//! Connection settings and revocation epochs are published together under the connector lock.

use crate::hub::Hub;
use anyhow::{Context, Result, ensure};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::Value;

fn publish(hub: &Hub, key: &str, value: &str, epoch: &str) -> Result<()> {
    hub.store.with_connection(|connection| {
        let tx = connection.unchecked_transaction()?;
        for (key, value) in [(key, value.to_owned()), (epoch, uuid::Uuid::new_v4().to_string())] {
            tx.execute(
                "INSERT INTO kv(k,v) VALUES(?1,?2) ON CONFLICT(k) DO UPDATE SET v=excluded.v",
                params![key, value],
            )?;
        }
        if key == "whatsapp.link.account" {
            tx.execute_batch("DELETE FROM whatsapp_threads; DELETE FROM whatsapp_cards;")?;
        }
        tx.commit()?;
        Ok(())
    })
}

/// One allowed chat. OpenBot 0.77.0: `whatsapp.allowed` used to hold only plain jid strings; a
/// read-only chat is stored as `{"jid":"…","readOnly":true}`. The older strings stay valid and read
/// as writable, so the setting migrates when read, not when written.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AllowedChat {
    pub jid: String,
    #[serde(default)]
    pub read_only: bool,
}

/// Reads either shape of the allowlist. One malformed entry is skipped rather than losing every
/// chat, so a hand-edited setting cannot lock the number's controls.
pub(crate) fn read_allowed(store: &crate::store::Store) -> Result<Vec<AllowedChat>> {
    let Some(raw) = store.kv_read("whatsapp.allowed")? else { return Ok(Vec::new()) };
    let Ok(values) = serde_json::from_str::<Vec<Value>>(&raw) else { return Ok(Vec::new()) };
    Ok(values
        .into_iter()
        .filter_map(|value| match value {
            Value::String(jid) => Some(AllowedChat { jid, read_only: false }),
            Value::Object(_) => serde_json::from_value::<AllowedChat>(value).ok().filter(|entry| !entry.jid.is_empty()),
            _ => None,
        })
        .collect())
}

/// The jids older clients expect: the wire list stays plain strings, with read-only chats told to
/// newer clients in a separate `readOnly` field (`whatsappState`).
pub(crate) fn allowed_jids(hub: &Hub) -> Result<Vec<String>> {
    Ok(read_allowed(&hub.store)?.into_iter().map(|entry| entry.jid).collect())
}

/// OpenBot 0.77.0: a read-only chat is still kept in history for an agent to read, but it must not
/// open a bot turn. The owner's own chat is never read-only. Pure so the decision is testable
/// without a store.
pub(crate) fn can_reply(entry: Option<&AllowedChat>) -> bool {
    entry.is_none_or(|entry| !entry.read_only)
}

/// The busy note ("too many requests") reads as the owner's own post in another person's chat or a
/// group on a personal number, so only the failed reaction goes there (OpenBot 0.77.0). A number
/// with no allowlist answers anyone and is a number used only for OpenBot; it and the owner's own
/// chat still get the note while the "Working on it…" message is on. Pure so the decision is
/// testable without a link.
pub(crate) fn post_busy_notice(own_chat: bool, personal_number: bool, working_message: bool) -> bool {
    (own_chat || !personal_number) && working_message
}

pub(super) fn record_account(hub: &Hub, me: &str) -> Result<()> {
    ensure!(!me.is_empty(), "WhatsApp did not identify the linked account");
    let _guard = crate::LockExt::locked(&hub.store.connector_lock);
    if hub.store.kv_read("whatsapp.link.account")?.as_deref() != Some(me) {
        publish(hub, "whatsapp.link.account", me, "whatsapp.link.generation")?;
    }
    Ok(())
}

pub(crate) fn set_bot(hub: &Hub, bot: &str) -> Result<()> {
    let _guard = crate::LockExt::locked(&hub.store.connector_lock);
    if !bot.is_empty() {
        let row = hub.store.bot(bot)?.filter(|row| !row.deleted && !row.config.hidden).context("Bot is unavailable")?;
        ensure!(!row.config.is_group() && !crate::external::is_external(&row.config), "Choose an agent bot");
    }
    if hub.store.kv_read("whatsapp.bot")?.as_deref() != Some(bot) {
        publish(hub, "whatsapp.bot", bot, "whatsapp.bot.generation")?;
    }
    Ok(())
}

/// Adds or removes one allowed chat, and sets its read-only flag when given. Older clients omit
/// `read_only`: adding a chat leaves it writable, and toggling another setting keeps the flag it
/// already had. OpenBot 0.77.0. The jid shape and 200-chat cap are unchanged.
pub(crate) fn set_allowed(hub: &Hub, jid: &str, allowed: bool, read_only: Option<bool>) -> Result<Vec<String>> {
    ensure!(
        !jid.is_empty() && jid.len() <= 120 && !jid.contains(char::is_whitespace),
        "Use a WhatsApp id, such as 5215555555555@s.whatsapp.net."
    );
    let _guard = crate::LockExt::locked(&hub.store.connector_lock);
    let mut list = read_allowed(&hub.store)?;
    let before = list.clone();
    let was_read_only = before.iter().any(|entry| entry.jid == jid && entry.read_only);
    list.retain(|entry| entry.jid != jid);
    if allowed {
        ensure!(list.len() < 200, "That is as many chats as one number allows (200).");
        list.push(AllowedChat { jid: jid.to_owned(), read_only: read_only.unwrap_or(was_read_only) });
    }
    list.sort_by(|a, b| a.jid.cmp(&b.jid));
    let mut previous = before;
    previous.sort_by(|a, b| a.jid.cmp(&b.jid));
    if previous != list {
        // Keep plain strings for writable chats: a reader that predates read-only still works.
        let stored: Vec<Value> = list
            .iter()
            .map(
                |entry| {
                    if entry.read_only { serde_json::to_value(entry) } else { Ok(Value::String(entry.jid.clone())) }
                },
            )
            .collect::<Result<Vec<_>>>()?;
        publish(hub, "whatsapp.allowed", &serde_json::to_string(&stored)?, "whatsapp.allowed.generation")?;
    }
    Ok(list.into_iter().map(|entry| entry.jid).collect())
}

/// A message in the linked number's own "Message yourself" chat is always from its
/// owner. Keeping this one decision here means the inbox, the live path and the
/// routine path all treat that chat the same: it reaches the bot even when the
/// number is used only for OpenBot and `whatsapp.allowed` is not empty
/// (OpenBot nightly triage-13865).
pub(crate) fn is_own_chat(me: &str, chat: &str, sender: &str) -> bool {
    !me.is_empty() && (me == chat || me == sender)
}

/// The "Working on it…" acknowledgement is on unless its owner turned it off for this number.
pub(crate) fn working_message(store: &crate::store::Store) -> bool {
    store.kv_get("whatsapp.workingMessage").is_none_or(|value| value != "0")
}

/// Turns that acknowledgement on or off; a request that already runs keeps its post.
pub(crate) fn set_working_message(hub: &Hub, on: bool) -> Result<bool> {
    let _guard = crate::LockExt::locked(&hub.store.connector_lock);
    publish(hub, "whatsapp.workingMessage", if on { "1" } else { "0" }, "whatsapp.workingMessage.generation")?;
    Ok(on)
}

/// Status reactions (👀 on arrival, then ✅ / ❌ / 🫡 at the end) are on unless this number's owner
/// turned them off. The answers and error messages are not reactions and never depend on this.
pub(crate) fn status_reactions(store: &crate::store::Store) -> bool {
    store.kv_get("whatsapp.statusReactions").is_none_or(|value| value != "0")
}

/// Turns the number's status reactions on or off.
pub(crate) fn set_status_reactions(hub: &Hub, on: bool) -> Result<bool> {
    let _guard = crate::LockExt::locked(&hub.store.connector_lock);
    publish(hub, "whatsapp.statusReactions", if on { "1" } else { "0" }, "whatsapp.statusReactions.generation")?;
    Ok(on)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::identity::Identity;
    use serde_json::json;

    fn hub() -> (std::sync::Arc<Hub>, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("codync-whatsapp-settings-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let store = crate::store::Store::open(std::path::Path::new(":memory:")).expect("store");
        (Hub::new(store, "whatsapp-test".into(), Identity::load_or_create(&dir).expect("identity"), "t".into(), 0), dir)
    }

    #[test]
    fn the_owners_own_chat_is_a_place_even_on_a_number_only_for_openbot() {
        let me = "5215555555555@s.whatsapp.net";
        assert!(is_own_chat(me, me, "someone@s.whatsapp.net"), "chatting to yourself");
        assert!(is_own_chat(me, "family-group", me), "posting in a group");
        assert!(!is_own_chat(me, "family-group", "someone@s.whatsapp.net"));
        assert!(!is_own_chat("", me, me), "no link, no owner");
    }

    #[test]
    fn the_working_message_is_on_until_its_owner_turns_it_off() {
        let (hub, dir) = hub();
        assert!(working_message(&hub.store));
        assert!(!set_working_message(&hub, false).unwrap());
        assert!(!working_message(&hub.store));
        assert!(set_working_message(&hub, true).unwrap());
        assert!(working_message(&hub.store));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn the_status_reactions_are_on_until_their_owner_turns_them_off() {
        let (hub, dir) = hub();
        assert!(status_reactions(&hub.store));
        assert!(!set_status_reactions(&hub, false).unwrap());
        assert!(!status_reactions(&hub.store));
        assert!(set_status_reactions(&hub, true).unwrap());
        assert!(status_reactions(&hub.store));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_plain_jid_list_reads_as_writable_and_a_mixed_list_keeps_its_flags() {
        let (hub, dir) = hub();
        let old = json!(["a@s.whatsapp.net", "b@s.whatsapp.net"]).to_string();
        hub.store.kv_set("whatsapp.allowed", &old).unwrap();
        let read = read_allowed(&hub.store).unwrap();
        assert_eq!(read.len(), 2);
        assert!(read.iter().all(|entry| !entry.read_only), "old plain strings stay read-write");

        let mixed = json!(["a@s.whatsapp.net", {"jid":"g@g.us","readOnly":true}]).to_string();
        hub.store.kv_set("whatsapp.allowed", &mixed).unwrap();
        let read = read_allowed(&hub.store).unwrap();
        assert_eq!(read[1].jid, "g@g.us");
        assert!(read[1].read_only, "a read-only group is read as read-only");
        assert_eq!(allowed_jids(&hub).unwrap(), vec!["a@s.whatsapp.net", "g@g.us"]);

        hub.store.kv_set("whatsapp.allowed", json!([{"unexpected":true}, 7]).to_string().as_str()).unwrap();
        assert!(read_allowed(&hub.store).unwrap().is_empty(), "bad entries are skipped, not fatal");
        hub.store.kv_set("whatsapp.allowed", "not json").unwrap();
        assert!(read_allowed(&hub.store).unwrap().is_empty(), "a corrupt list does not lock the number");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_linked_number_offers_only_its_writable_chats_and_its_own_chat_as_places() {
        let (hub, dir) = hub();
        assert!(crate::whatsapp::delivery::places(&hub.store).unwrap().is_empty(), "no link, no place");
        hub.store.kv_set("whatsapp.link.account", "5215@s.whatsapp.net").unwrap();
        hub.store.kv_set("whatsapp.bot", "b1").unwrap();
        hub.store
            .kv_set("whatsapp.allowed", &json!(["5216@s.whatsapp.net", {"jid":"g@g.us","readOnly":true}]).to_string())
            .unwrap();
        let rows = crate::whatsapp::delivery::places(&hub.store).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, "5215@s.whatsapp.net");
        assert_eq!(rows[0].1, ["5216@s.whatsapp.net", "5215@s.whatsapp.net"], "read-only chats are not destinations");
        assert_eq!(rows[0].2, ["b1"]);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn only_a_read_only_entry_stops_a_bot_reply() {
        let writable = AllowedChat { jid: "a@s.whatsapp.net".into(), read_only: false };
        let read_only = AllowedChat { jid: "g@g.us".into(), read_only: true };
        assert!(can_reply(None), "a chat with no entry is an ordinary chat");
        assert!(can_reply(Some(&writable)));
        assert!(!can_reply(Some(&read_only)));
    }

    #[test]
    fn toggling_read_only_keeps_the_chat_allowed_and_removing_it_forgets_the_flag() {
        let (hub, dir) = hub();
        let jid = "a@s.whatsapp.net";
        assert_eq!(set_allowed(&hub, jid, true, Some(true)).unwrap(), vec![jid]);
        let read = read_allowed(&hub.store).unwrap();
        assert_eq!(read.len(), 1);
        assert!(read[0].read_only);
        // An old client allowing the same chat again must not silently clear read-only.
        set_allowed(&hub, jid, true, None).unwrap();
        assert!(read_allowed(&hub.store).unwrap()[0].read_only);
        set_allowed(&hub, jid, true, Some(false)).unwrap();
        assert!(!read_allowed(&hub.store).unwrap()[0].read_only, "the toggle turns it back off");
        // Writable entries are stored as plain strings for older readers.
        let raw = hub.store.kv_read("whatsapp.allowed").unwrap().unwrap();
        assert_eq!(raw, json!([jid]).to_string());
        set_allowed(&hub, jid, false, None).unwrap();
        assert!(read_allowed(&hub.store).unwrap().is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn the_allowlist_keeps_its_shape_check_and_two_hundred_chat_cap() {
        let (hub, dir) = hub();
        assert!(set_allowed(&hub, "has space@s.whatsapp.net", true, Some(true)).is_err());
        assert!(set_allowed(&hub, "", true, None).is_err());
        assert!(set_allowed(&hub, "x".repeat(121).as_str(), true, None).is_err());
        for n in 0..200 {
            set_allowed(&hub, &format!("{n}@s.whatsapp.net"), true, None).unwrap();
        }
        assert_eq!(read_allowed(&hub.store).unwrap().len(), 200);
        assert!(set_allowed(&hub, "201@s.whatsapp.net", true, Some(true)).is_err());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn the_busy_note_stays_in_the_owners_chat_and_on_a_number_used_only_for_openbot() {
        assert!(post_busy_notice(true, true, true), "the owner's own chat still gets it");
        assert!(!post_busy_notice(true, true, false), "off is off there too");
        assert!(post_busy_notice(false, false, true), "a number only for OpenBot still gets it");
        assert!(!post_busy_notice(false, false, false));
        assert!(!post_busy_notice(false, true, true), "a personal number posts no note in another chat");
        assert!(!post_busy_notice(false, true, false));
    }
}
