//! One `signal-cli` envelope: who may write, whether the bot answers, the prompt that is queued,
//! and the numbered reply that answers an approval card.
//!
//! Signal has no channel user directory and no buttons: a direct chat answers its allowed people,
//! a group answers a mention (or once the group has a conversation), and an approval is a number
//! sent as a quote of the card's message.

use super::{Binding, Config, Hub, chat_label, client};
use crate::api::devices::Caller;
use crate::store::signal::SignalIncoming;
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
    format!("signal-{}", crate::chat::files::digest_hex(&digest.finalize()))
}

/// The thread root of the one conversation a chat has with its bot.
pub(super) fn root_id(generation: &str, chat: &str) -> String {
    stable_id(&[generation, chat, "root"])
}

/// The envelope's data message, or `None` for receipts, typing, sync traffic and edits.
fn data_message(envelope: &Value) -> Option<&Value> {
    let data = envelope.get("dataMessage")?;
    if data.get("isExpired").and_then(Value::as_bool) == true {
        return None;
    }
    if data.get("editMessage").is_some() {
        return None;
    }
    Some(data)
}

/// The chat an envelope belongs to: its group, or the person who wrote.
fn chat_of(envelope: &Value, data: &Value) -> Option<String> {
    if let Some(group) = data["groupInfo"]["groupId"].as_str().filter(|id| !id.is_empty()) {
        return Some(format!("group:{group}"));
    }
    source(envelope).map(str::to_owned)
}

fn source(envelope: &Value) -> Option<&str> {
    envelope["sourceNumber"]
        .as_str()
        .or_else(|| envelope["source"].as_str())
        .filter(|source| !source.is_empty() && *source != "unknown")
}

fn source_uuid(envelope: &Value) -> Option<&str> {
    envelope["sourceUuid"].as_str().filter(|uuid| !uuid.is_empty())
}

fn message_id(envelope: &Value, data: &Value) -> Option<String> {
    let value = data["timestamp"].as_u64().or_else(|| envelope["timestamp"].as_u64())?;
    Some(value.to_string())
}

fn binding_for<'a>(c: &'a Config, chat: &str, envelope: &Value) -> Option<&'a Binding> {
    let number = source(envelope);
    let uuid = source_uuid(envelope);
    c.bindings.iter().find(|binding| {
        binding.chat == chat
            && binding.allowed_users.iter().any(|user| {
                number.is_some_and(|number| user.as_str() == number) || uuid.is_some_and(|uuid| user.as_str() == uuid)
            })
    })
}

/// A Signal mention is the bot's number in `mentions`, or its number in the text.
fn mentions(data: &Value, number: &str) -> bool {
    data["mentions"]
        .as_array()
        .is_some_and(|mentions| mentions.iter().any(|mention| mention["author"].as_str() == Some(number)))
        || data["message"].as_str().is_some_and(|text| text.contains(number))
}

/// The bot's number is not part of what was said.
fn strip_mention(text: &str, number: &str) -> String {
    text.replace(number, "").trim().to_owned()
}

fn attachment_name(media: &Value, index: usize) -> String {
    media["fileName"]
        .as_str()
        .or_else(|| media["filename"].as_str())
        .filter(|name| !name.trim().is_empty())
        .map(|name| name.chars().filter(|c| !c.is_control()).take(255).collect())
        .unwrap_or_else(|| format!("attachment-{}", index + 1))
}

/// Downloads and keeps each attachment under the bot's uploads, so the agent can open it. A
/// failed download still tells the bot the file came, with its name.
async fn files(c: &Config, message: &str, data: &Value, bot: &crate::store::BotConfig) -> (String, Vec<Value>) {
    let mut prompt = String::new();
    let mut saved = Vec::new();
    let mut metadata = Vec::new();
    let mut total = 0_u64;
    for (index, media) in data["attachments"].as_array().into_iter().flatten().take(10).enumerate() {
        let name = attachment_name(media, index);
        let id = media["id"].as_str().filter(|id| !id.is_empty());
        let Some(id) = id else {
            saved.push(name);
            continue;
        };
        let cache = crate::service::data_dir().join("signal-media").join(&c.generation);
        let dest = cache.join(format!("{message}-{index}"));
        match client::download(&c.daemon, &c.number, id, &dest).await {
            Ok(()) => {
                let Ok(bytes) = tokio::fs::read(&dest).await else {
                    saved.push(name);
                    continue;
                };
                let _ = tokio::fs::remove_file(&dest).await;
                total += bytes.len() as u64;
                if total > 20 * 1024 * 1024 || bytes.is_empty() {
                    saved.push(name);
                    continue;
                }
                let root = crate::chat::uploads::root(bot);
                let upload = uuid::Uuid::new_v4().to_string();
                let copy = bytes.clone();
                let file_name = name.clone();
                match tokio::task::spawn_blocking(move || {
                    crate::chat::uploads::append(&root, &upload, &file_name, 0, &copy, true)
                })
                .await
                {
                    Ok(Ok(Some(meta))) => {
                        metadata.push(meta);
                        saved.push(name);
                    }
                    _ => saved.push(name),
                }
            }
            Err(_) => saved.push(name),
        }
    }
    if !saved.is_empty() {
        prompt.push_str(&format!("\n\n[Attached files: {}]", saved.join(", ")));
    }
    (prompt, metadata)
}

pub(super) async fn accept(hub: &Arc<Hub>, c: &Config, params: &Value) -> Result<()> {
    if let Some(account) = params["account"].as_str()
        && account != c.number
    {
        return Ok(());
    }
    let envelope = &params["envelope"];
    let Some(data) = data_message(envelope) else { return Ok(()) };
    let Some(chat) = chat_of(envelope, data) else { return Ok(()) };
    let Some(sender) = source(envelope) else { return Ok(()) };
    let Some(binding) = binding_for(c, &chat, envelope) else { return Ok(()) };
    let message = message_id(envelope, data).unwrap_or_default();
    if message.is_empty() {
        return Ok(());
    }
    let body = data["message"].as_str().unwrap_or_default().trim();
    // A quoted number answers the card it quotes; it is never a new turn.
    let card = data["quote"]["id"]
        .as_u64()
        .map(|id| id.to_string())
        .or_else(|| data["quote"]["id"].as_str().map(str::to_owned));
    if let Some(card) = card
        && answer(hub, c, &chat, binding, &card, body).await?
    {
        return Ok(());
    }
    let is_group = chat.starts_with("group:");
    let mention = mentions(data, &c.number);
    if is_group && !mention && !hub.store.signal_thread_exists(&c.generation, &chat)? {
        return Ok(());
    }
    let text = if mention { strip_mention(body, &c.number) } else { body.to_owned() };
    let attachments = data["attachments"].as_array().map_or(0, Vec::len);
    if text.is_empty() && attachments == 0 {
        return Ok(());
    }
    if text.len() > 32 * 1024 {
        return Ok(());
    }
    let Some(bot) = hub.store.bot(&binding.bot_id)?.filter(|b| !b.deleted && !b.config.is_group()) else {
        return Ok(());
    };
    let (files_prompt, files) = files(c, &message, data, &bot.config).await;
    let prompt = if text.is_empty() { files_prompt.trim_start().to_owned() } else { format!("{text}{files_prompt}") };
    let incoming = SignalIncoming {
        message: message.clone(),
        generation: c.generation.clone(),
        bot: binding.bot_id.clone(),
        chat: chat.clone(),
        entry: stable_id(&[&c.generation, &chat, &message]),
        text: prompt,
        files,
        user: sender.into(),
    };
    let event_name = if mention { "message.mention" } else { "message.created" };
    hub.routines.receive_event(
        &hub.store,
        &json!({
            "source": "signal", "event": event_name, "chat": chat, "user": sender,
            "text": text, "messageId": message,
        }),
        Some(message.clone()),
    );
    hub.store.enqueue_signal(&incoming)?;
    hub.store.map_signal_thread(&incoming, &root_id(&c.generation, &chat))?;
    hub.store.record_channel_activity(&crate::store::ChannelActivity {
        id: format!("signal-{}", incoming.entry),
        source: "signal".into(),
        bot_id: incoming.bot.clone(),
        thread_id: Some(root_id(&c.generation, &chat)),
        channel: chat.clone(),
        mentioned: mention,
        at: crate::store::now_ms(),
    })?;
    Ok(())
}

/// Answers the card a message quotes, when the body is a number. Returns whether the message was
/// a card answer (and so must not become a turn).
async fn answer(hub: &Arc<Hub>, c: &Config, chat: &str, binding: &Binding, card: &str, body: &str) -> Result<bool> {
    let text = body.trim().trim_end_matches(['.', ')']);
    let Ok(index) = text.parse::<usize>() else { return Ok(false) };
    if index == 0 || index > 9 {
        return Ok(false);
    }
    answer_at(hub, c, chat, binding, card, index - 1).await
}

/// Answers option `index` of the card shown by `card`, when the person may and the bot asked.
/// `true` when the message was a card of this chat, even if it was already settled.
async fn answer_at(
    hub: &Arc<Hub>,
    c: &Config,
    chat: &str,
    binding: &Binding,
    card: &str,
    index: usize,
) -> Result<bool> {
    let Some(entry_id) = hub.store.signal_card_entry(&c.generation, chat, card)? else { return Ok(false) };
    let Some(entry) = hub.store.entry(&entry_id).filter(|entry| entry.kind == "permission") else {
        return Ok(true);
    };
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

/// The label a log line uses for a chat.
pub(super) fn label(chat: &str) -> String {
    chat_label(chat)
}
#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> Config {
        Config {
            number: "+15550000001".into(),
            daemon: "http://127.0.0.1:7583".into(),
            generation: "g".into(),
            bindings: vec![
                Binding {
                    chat: "+15550000002".into(),
                    bot_id: "b".into(),
                    memory: String::new(),
                    icon: String::new(),
                    allowed_users: vec!["+15550000002".into()],
                },
                Binding {
                    chat: "group:abc=".into(),
                    bot_id: "b".into(),
                    memory: String::new(),
                    icon: String::new(),
                    allowed_users: vec!["uuid-1111-2222".into()],
                },
            ],
        }
    }

    #[test]
    fn only_the_bound_chat_and_person_match() {
        let c = config();
        let dm = json!({"sourceNumber": "+15550000002"});
        assert!(binding_for(&c, "+15550000002", &dm).is_some());
        let other = json!({"sourceNumber": "+15550000003"});
        assert!(binding_for(&c, "+15550000002", &other).is_none());
        assert!(binding_for(&c, "group:abc=", &other).is_none());
        let member = json!({"sourceNumber": "+15550000003", "sourceUuid": "uuid-1111-2222"});
        assert!(binding_for(&c, "group:abc=", &member).is_some());
    }

    #[test]
    fn the_chat_is_the_group_or_the_writer() {
        let dm = json!({"sourceNumber": "+15550000002"});
        let data = json!({"message": "hi"});
        assert_eq!(chat_of(&dm, &data).as_deref(), Some("+15550000002"));
        let group = json!({"sourceNumber": "+15550000002"});
        let data = json!({"groupInfo": {"groupId": "abc="}});
        assert_eq!(chat_of(&group, &data).as_deref(), Some("group:abc="));
        assert_eq!(chat_of(&json!({}), &json!({})), None);
    }

    #[test]
    fn receipts_and_edits_are_not_messages() {
        assert!(data_message(&json!({"receiptMessage": {}})).is_none());
        assert!(data_message(&json!({"dataMessage": {"editMessage": {}}})).is_none());
        assert!(data_message(&json!({"dataMessage": {"message": "hi"}})).is_some());
    }

    #[test]
    fn a_number_answers_the_quoted_card() {
        assert_eq!("2.".trim_end_matches(['.', ')']).parse::<usize>(), Ok(2));
        assert_eq!("no".parse::<usize>(), Err("no".parse::<usize>().unwrap_err()));
        assert!(mentions(&json!({"mentions": [{"author": "+15550000001"}]}), "+15550000001"));
        assert!(mentions(&json!({"message": "hola +15550000001"}), "+15550000001"));
        assert!(!mentions(&json!({"message": "hola"}), "+15550000001"));
        assert_eq!(strip_mention("+15550000001 deploy", "+15550000001"), "deploy");
    }

    #[test]
    fn an_attachment_keeps_a_readable_name() {
        assert_eq!(attachment_name(&json!({"fileName": "a.png"}), 0), "a.png");
        assert_eq!(attachment_name(&json!({"contentType": "image/png"}), 2), "attachment-3");
    }
}
