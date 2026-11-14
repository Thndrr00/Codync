//! Pins: keeping a message at hand in a chat's pin list, and the agent tool that pins one itself.
//!
//! A pin is the `pinned` marker on the message (`store::pins`), so the entry's rev carries it to every
//! client. A person pins from the message menu; the agent pins with `pin_message`, naming the message
//! by a quote because it doesn't see message ids, and cannot take back a person's pin.

use super::outbox::MAX_MESSAGE_BYTES;
use crate::hub::Hub;
use crate::store::{BotConfig, Entry, LIST_LIMIT, Lane, PINS_LIMIT, message_text, now_ms};
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};
use std::sync::Arc;

/// The most characters of a pinned message the list carries; it shows only the first lines.
pub const TEXT_LIMIT: usize = 1_000;

pub(super) fn tool() -> Value {
    json!({
        "name": "pin_message",
        "description": "Pin a finished message of this chat so the people here find it again, or unpin it. Use it for a decision, a path or another detail that must not get lost. You can't take back a pin of a person's.",
        "inputSchema": {
            "type": "object", "properties": {
                "quote": {"type": "string", "description": "A short exact part of the message text; the newest message that contains it is the one pinned. Leave it out for the message you are answering."},
                "pinned": {"type": "boolean", "description": "False unpins it. Default true."},
            },
        },
        "annotations": {"readOnlyHint": false, "idempotentHint": true},
    })
}

/// Pins or unpins `message` of `chat`. `by` is the marker stored on the message: a person (with the
/// name of their device, or none at the computer itself) or the chat's agent.
pub fn set(hub: &Hub, chat: &str, message: &str, pinned: bool, by: &Value) -> Result<Value> {
    let bot = hub.store.bot(chat)?.filter(|row| !row.deleted).context("unknown bot")?;
    let entry =
        hub.store.entry_checked(message)?.filter(|entry| entry.bot_id == bot.config.id).context("unknown message")?;
    ensure!(matches!(entry.kind.as_str(), "user" | "agent"), "only messages can be pinned");
    if pinned && entry.data["pinned"].is_null() {
        ensure!(
            hub.store.pins_in(chat, entry.thread_id.as_deref())? < PINS_LIMIT,
            "a chat keeps at most {PINS_LIMIT} pins; unpin one first"
        );
    }
    let mut data = entry.data.clone();
    match data.as_object_mut() {
        Some(obj) if pinned => {
            obj.insert("pinned".into(), json!({"by": by, "at": now_ms()}));
        }
        Some(obj) => {
            obj.remove("pinned");
        }
        None => bail!("the message has no data"),
    }
    let updated = hub.set_entry(&entry.id, &data).context("couldn't save the pin")?;
    Ok(json!({"entry": updated, "pin": view(hub, &updated)}))
}

/// A pin as a list shows it.
fn view(hub: &Hub, entry: &Entry) -> Value {
    let title =
        entry.thread_id.as_deref().and_then(|id| hub.store.thread_by_id(id).ok().flatten()).map(|thread| thread.title);
    json!({
        "messageId": entry.id,
        "threadId": entry.thread_id,
        "threadTitle": title,
        "pinnedAt": entry.data["pinned"]["at"],
        "pinnedBy": entry.data["pinned"]["by"],
        "author": if entry.kind == "user" { "user" } else { "agent" },
        "authorBotId": entry.data["author"],
        "createdAt": entry.created_at,
        "text": message_text(&entry.data).chars().take(TEXT_LIMIT).collect::<String>(),
    })
}

/// The pins of every chat of `bot`, newest first.
pub fn list(hub: &Hub, bot: &str) -> Result<Vec<Value>> {
    let rows = hub.store.bot(bot)?.filter(|row| !row.deleted).context("unknown bot")?;
    Ok(hub.store.pins(&rows.config.id, LIST_LIMIT)?.iter().map(|entry| view(hub, entry)).collect())
}

/// `pin_message` for the bot of the running turn.
pub(super) fn pin(hub: &Arc<Hub>, source: &BotConfig, args: &Value) -> Result<Value> {
    let pinned = args["pinned"].as_bool().unwrap_or(true);
    let lane = hub.runtime(&source.id).lane.context("pin_message needs a running turn")?;
    let quote = args["quote"].as_str().map(str::trim).filter(|quote| !quote.is_empty());
    if let Some(quote) = quote {
        ensure!(quote.len() <= MAX_MESSAGE_BYTES, "the quote must be at most {MAX_MESSAGE_BYTES} bytes");
    }
    let message = match quote {
        Some(quote) => find_by_quote(hub, &lane, quote, pinned)?,
        None => hub.runtime(&source.id).message.context(
            "this turn has no message of the user's; pass quote with a short exact part of the message to pin",
        )?,
    };
    let entry = hub.store.entry_checked(&message)?.context("unknown message")?;
    if !pinned && entry.data["pinned"]["by"]["kind"] == "person" {
        bail!("a person pinned that message; only they can unpin it");
    }
    set(hub, &lane.chat, &message, pinned, &json!({"kind": "agent", "botId": source.id}))?;
    let pins: Vec<Value> = list(hub, &lane.chat)?
        .into_iter()
        .filter(|pin| pin["threadId"].as_str() == lane.thread.as_deref())
        .map(|pin| {
            let mine = pin["pinnedBy"]["botId"] == source.id;
            json!({
                "text": pin["text"].as_str().unwrap_or_default().chars().take(160).collect::<String>(),
                "pinnedBy": if mine { "you" } else { pin["pinnedBy"]["kind"].as_str().unwrap_or_default() },
            })
        })
        .collect();
    Ok(json!({"status": if pinned { "pinned" } else { "unpinned" }, "messageId": message, "pinsInThisChat": pins}))
}

/// The newest message of `lane` that contains `quote`: a finished one when pinning, a pinned one when
/// unpinning. The turn's own message is left out, as it is what a call without a quote pins.
fn find_by_quote(hub: &Hub, lane: &Lane, quote: &str, pinned: bool) -> Result<String> {
    let quote = quote.to_lowercase();
    let own = hub.runtime(&lane.chat).message;
    hub.store
        .pin_candidates(&lane.chat, lane.thread.as_deref(), 500)?
        .iter()
        .rev()
        .filter(|entry| Some(&entry.id) != own.as_ref())
        .filter(|entry| pinned || !entry.data["pinned"].is_null())
        .find(|entry| message_text(&entry.data).to_lowercase().contains(&quote))
        .map(|entry| entry.id.clone())
        .with_context(|| {
            if pinned {
                "no message of this chat contains that quote; copy a short exact part of it and try again"
            } else {
                "no pinned message of this chat contains that quote"
            }
        })
}

#[cfg(test)]
mod tests;
