//! One Matrix event: who may write, whether the bot answers, the prompt that is queued, and the
//! replies and reactions that answer an approval card.
//!
//! A bound room answers a message that mentions the bot and, once the room has a conversation, the
//! messages of the people allowed there. Anything else is dropped without error, so the bot can sit
//! in a busy room.

use super::render::{self, KEYS};
use super::{Config, Hub, room_label};
use crate::api::devices::Caller;
use crate::store::matrix::MatrixIncoming;
use anyhow::Result;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::sync::Arc;

pub(super) fn stable_id(parts: &[&str]) -> String {
    let mut digest = Sha256::new();
    for part in parts {
        digest.update((part.len() as u64).to_le_bytes());
        digest.update(part.as_bytes());
    }
    format!("matrix-{}", crate::chat::files::digest_hex(&digest.finalize()))
}

/// The thread root of the one conversation a room has with its bot.
pub(super) fn root_id(generation: &str, room: &str) -> String {
    stable_id(&[generation, room, "root"])
}

pub(super) fn accept(hub: &Arc<Hub>, c: &Config, joined: &[super::sync::Joined]) -> Result<()> {
    for room in joined {
        if !c.bindings.iter().any(|binding| binding.room_id == room.room) {
            continue;
        }
        for event in &room.events {
            match event["type"].as_str() {
                Some("m.room.message") => {
                    if let Err(error) = message(hub, c, &room.room, event) {
                        tracing::warn!(
                            room = room_label(&room.room),
                            error = format!("{error:#}"),
                            "Matrix message was not accepted"
                        );
                    }
                }
                Some("m.reaction") => {
                    if let Err(error) = reaction(hub, c, &room.room, event) {
                        tracing::warn!(
                            room = room_label(&room.room),
                            error = format!("{error:#}"),
                            "Matrix reaction was not accepted"
                        );
                    }
                }
                _ => {}
            }
        }
    }
    Ok(())
}

fn message(hub: &Arc<Hub>, c: &Config, room: &str, event: &Value) -> Result<()> {
    let Some(id) = event["event_id"].as_str().filter(|id| (1..=255).contains(&id.len())) else { return Ok(()) };
    let Some(sender) = event["sender"].as_str().filter(|sender| *sender != c.user_id.as_str()) else { return Ok(()) };
    let Some(binding) = allowed(c, room, sender) else { return Ok(()) };
    let content = &event["content"];
    if content["m.relates_to"]["rel_type"] == "m.replace" {
        return Ok(());
    }
    let msgtype = content["msgtype"].as_str().unwrap_or_default();
    if !matches!(msgtype, "m.text" | "m.notice" | "m.emote" | "m.image" | "m.file" | "m.audio" | "m.video") {
        return Ok(());
    }
    let body = content["body"].as_str().unwrap_or_default().trim();
    // A reply to an approval card is an answer, not a new turn.
    if let Some(card) = replied_to(content)
        && answer(hub, c, room, binding, card, body)?
    {
        return Ok(());
    }
    if body.is_empty() || body.len() > 32 * 1024 {
        return Ok(());
    }
    let media = matches!(msgtype, "m.image" | "m.file" | "m.audio" | "m.video");
    let text = if media { String::new() } else { strip_mention(body, &c.user_id) };
    let mention = mentions(content, &c.user_id) || body.contains(&c.user_id);
    if !mention && !hub.store.matrix_thread_exists(&c.generation, room)? {
        return Ok(());
    }
    if text.trim().is_empty() && !media {
        return Ok(());
    }
    let files =
        if media { vec![crate::channels::FileRef { name: body.chars().take(255).collect() }] } else { Vec::new() };
    let incoming = MatrixIncoming {
        event: id.into(),
        generation: c.generation.clone(),
        bot: binding.bot_id.clone(),
        room: room.into(),
        entry: stable_id(&[&c.generation, room, id]),
        text: crate::channels::Incoming {
            channel: room.into(),
            person: sender.into(),
            thread: room.into(),
            text: text.clone(),
            files,
        }
        .prompt(),
        user: sender.into(),
    };
    let event_name = if mention { "message.mention" } else { "message.created" };
    hub.routines.receive_event(
        &hub.store,
        &json!({
            "source": "matrix", "event": event_name, "room": room, "user": sender,
            "text": text, "eventId": id, "url": format!("https://matrix.to/#/{room}/{id}"),
        }),
        Some(id.to_owned()),
    );
    hub.store.enqueue_matrix(&incoming)?;
    hub.store.map_matrix_thread(&incoming, &root_id(&c.generation, room))?;
    hub.store.record_channel_activity(&crate::store::ChannelActivity {
        id: format!("matrix-{}", incoming.entry),
        source: "matrix".into(),
        bot_id: incoming.bot.clone(),
        thread_id: Some(root_id(&c.generation, room)),
        channel: room.into(),
        mentioned: mention,
        at: crate::store::now_ms(),
    })?;
    Ok(())
}

fn reaction(hub: &Arc<Hub>, c: &Config, room: &str, event: &Value) -> Result<()> {
    let Some(sender) = event["sender"].as_str().filter(|sender| *sender != c.user_id.as_str()) else { return Ok(()) };
    let Some(binding) = allowed(c, room, sender) else { return Ok(()) };
    let Some(key) = event["content"]["m.relates_to"]["key"].as_str() else { return Ok(()) };
    let Some(card) = event["content"]["m.relates_to"]["event_id"].as_str() else { return Ok(()) };
    let Some(index) = render::reaction_index(key) else { return Ok(()) };
    answer_at(hub, c, room, binding, card, index)?;
    Ok(())
}

/// Answers the card a message replies to, when the body is a number. Returns whether the message
/// was a card answer (and so must not become a turn).
fn answer(hub: &Arc<Hub>, c: &Config, room: &str, binding: &super::Binding, card: &str, body: &str) -> Result<bool> {
    let text = body.trim().trim_end_matches(['.', ')']);
    let Ok(index) = text.parse::<usize>() else { return Ok(false) };
    if index == 0 {
        return Ok(false);
    }
    answer_at(hub, c, room, binding, card, index - 1)
}

/// Answers option `index` of the card shown by `card`, when the person may and the bot asked.
/// `true` when the event was a card of this room, even if it was already settled.
fn answer_at(
    hub: &Arc<Hub>,
    c: &Config,
    room: &str,
    binding: &super::Binding,
    card: &str,
    index: usize,
) -> Result<bool> {
    let Some(entry_id) = hub.store.matrix_card_entry(&c.generation, room, card)? else { return Ok(false) };
    let Some(entry) = hub.store.entry(&entry_id).filter(|entry| entry.kind == "permission") else { return Ok(true) };
    if entry.bot_id != binding.bot_id || entry.data["status"] != "pending" {
        return Ok(true);
    }
    let options = entry.data["options"].as_array().cloned().unwrap_or_default();
    let Some(option) = options.get(index).and_then(|option| option["optionId"].as_str()) else { return Ok(true) };
    let option = option.to_owned();
    let hub = hub.clone();
    tokio::spawn(async move {
        let _ = crate::api::dispatch(
            &hub,
            &Caller::Local,
            "respondPermission",
            json!({"entryId": entry_id, "optionId": option}),
        )
        .await;
    });
    Ok(true)
}

/// The card event this content replies to, modern or legacy shape.
fn replied_to(content: &Value) -> Option<&str> {
    let relation = &content["m.relates_to"];
    if relation["rel_type"] == "m.in_reply_to" {
        return relation["event_id"].as_str();
    }
    relation["m.in_reply_to"]["event_id"].as_str()
}

fn allowed<'a>(c: &'a Config, room: &str, sender: &str) -> Option<&'a super::Binding> {
    c.bindings.iter().find(|binding| binding.room_id == room && binding.allowed_users.iter().any(|user| user == sender))
}

fn mentions(content: &Value, user: &str) -> bool {
    content["m.mentions"]["user_ids"].as_array().is_some_and(|users| users.iter().any(|id| id == user))
}

/// The bot's own mention is not part of what was said.
fn strip_mention(text: &str, user: &str) -> String {
    text.replace(user, "").trim().to_owned()
}

/// The reaction keys the worker puts on a card, so a test and the sender agree.
pub(super) fn keys(count: usize) -> &'static [&'static str] {
    &KEYS[..count.min(KEYS.len())]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_bots_mention_leaves_the_text_and_replies_are_found() {
        assert_eq!(strip_mention("@bot:example.org deploy please", "@bot:example.org"), "deploy please");
        let modern = json!({"m.relates_to": {"rel_type": "m.in_reply_to", "event_id": "$c"}});
        assert_eq!(replied_to(&modern), Some("$c"));
        let legacy = json!({"m.relates_to": {"m.in_reply_to": {"event_id": "$c"}}});
        assert_eq!(replied_to(&legacy), Some("$c"));
        assert_eq!(replied_to(&json!({})), None);
    }

    #[test]
    fn only_the_configured_room_and_person_match() {
        let c = Config {
            homeserver: "https://m.example".into(),
            user_id: "@bot:example.org".into(),
            access_token: "t".into(),
            device_id: String::new(),
            generation: "g".into(),
            bindings: vec![super::super::Binding {
                room_id: "!a:example.org".into(),
                bot_id: "b".into(),
                memory: String::new(),
                icon: String::new(),
                allowed_users: vec!["@ana:example.org".into()],
            }],
        };
        assert!(allowed(&c, "!a:example.org", "@ana:example.org").is_some());
        assert!(allowed(&c, "!a:example.org", "@beto:example.org").is_none());
        assert!(allowed(&c, "!b:example.org", "@ana:example.org").is_none());
    }

    #[test]
    fn a_card_answer_is_a_number_and_a_reaction_is_a_key() {
        assert_eq!(render::reaction_index("2"), Some(1));
        assert_eq!(keys(3), &["1️⃣", "2️⃣", "3️⃣"]);
        let c = super::super::client::text_content("2", "");
        assert_eq!(c["body"], "2");
    }
}
