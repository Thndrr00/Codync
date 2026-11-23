//! Connection settings and revocation epochs are published together under the connector lock.

use crate::hub::Hub;
use anyhow::{Context, Result, ensure};
use rusqlite::params;

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

pub(crate) fn set_allowed(hub: &Hub, jid: &str, allowed: bool) -> Result<Vec<String>> {
    ensure!(
        !jid.is_empty() && jid.len() <= 120 && !jid.contains(char::is_whitespace),
        "Use a WhatsApp id, such as 5215555555555@s.whatsapp.net."
    );
    let _guard = crate::LockExt::locked(&hub.store.connector_lock);
    let mut list: Vec<String> =
        hub.store.kv_read("whatsapp.allowed")?.as_deref().map(serde_json::from_str).transpose()?.unwrap_or_default();
    let before = list.clone();
    list.retain(|entry| entry != jid);
    if allowed {
        ensure!(list.len() < 200, "That is as many chats as one number allows (200).");
        list.push(jid.to_owned());
    }
    list.sort();
    let mut previous = before;
    previous.sort();
    if previous != list {
        publish(hub, "whatsapp.allowed", &serde_json::to_string(&list)?, "whatsapp.allowed.generation")?;
    }
    Ok(list)
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
}
