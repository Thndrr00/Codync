//! A bot's read-back of the messages a connected channel brought it. Ported from `OpenBot` 0.69.0
//! ("a routine, or an agent in your own chat, can read a connected Slack, Discord, Telegram or
//! WhatsApp chat"). `OpenBot` reads the server side of every chat it hosts; Codync keeps what each
//! channel delivered to the bot, so a routine summarizes what this computer actually received.

use crate::hub::Hub;
use anyhow::{Result, bail};
use serde_json::{Value, json};

pub const TOOL: &str = "read_channel_messages";
/// The channels whose delivered messages carry `source` in their entry.
const SOURCES: [&str; 4] = ["slack", "telegram", "discord", "whatsapp"];
/// Most messages one call returns.
const LIMIT: usize = 50;
/// Entries read per page of the bot's history while looking for channel messages.
const PAGE: i64 = 200;

pub fn tool() -> Value {
    json!({
        "name": TOOL,
        "description": "Read the messages that a connected Slack, Telegram, Discord or WhatsApp chat sent you, newest first: the text, who it was from, and when. Use it to summarize a channel or to check what was said before you answer. It sends nothing and reads only this bot's own chat.",
        "inputSchema": {"type": "object", "properties": {
            "source": {"type": "string", "enum": ["slack", "telegram", "discord", "whatsapp"], "description": "Only this channel; leave it out for all four."},
            "limit": {"type": "integer", "minimum": 1, "maximum": 50}
        }},
        "annotations": {"readOnlyHint": true},
    })
}

/// The most recent channel messages in `bot`'s chat, newest first.
pub fn read(hub: &Hub, bot: &str, args: &Value) -> Result<Value> {
    let source = args["source"].as_str().unwrap_or_default();
    if !source.is_empty() && !SOURCES.contains(&source) {
        bail!("source must be slack, telegram, discord or whatsapp");
    }
    let limit = args["limit"].as_u64().unwrap_or(20).clamp(1, LIMIT as u64) as usize;
    let mut messages = Vec::with_capacity(limit);
    let mut before = i64::MAX;
    'pages: while messages.len() < limit {
        let page = hub.store.history(bot, before, PAGE)?;
        let Some(oldest) = page.last() else { break };
        before = oldest.seq;
        for entry in &page {
            let from = entry.data["source"].as_str().unwrap_or_default();
            if !SOURCES.contains(&from) || (!source.is_empty() && from != source) {
                continue;
            }
            let text = entry.data["text"].as_str().unwrap_or_default().trim();
            if text.is_empty() {
                continue;
            }
            messages.push(json!({
                "source": from,
                "at": entry.created_at,
                "from": entry.data["from"]
                    .as_str()
                    .or_else(|| entry.data["name"].as_str())
                    .or_else(|| entry.data["whatsappFence"]["sender"].as_str())
                    .unwrap_or_default(),
                "text": text,
            }));
            if messages.len() == limit {
                break 'pages;
            }
        }
        if page.len() < PAGE as usize {
            break;
        }
    }
    Ok(json!({"messages": messages}))
}
