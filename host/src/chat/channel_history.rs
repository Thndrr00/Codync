//! A bot's read-back of the messages a connected channel brought it. Ported from `OpenBot` 0.69.0
//! ("a routine, or an agent in your own chat, can read a connected Slack, Discord, Telegram or
//! `WhatsApp` chat"). `OpenBot` reads the server side of every chat it hosts; Codync keeps what each
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
        "description": "Read the messages that a connected Slack, Telegram, Discord or WhatsApp chat sent you, newest first: the text, who it was from, and when. Search them by words or find what came after a moment. Use it to summarize a channel, to find an earlier message, or to check what was said before you answer. It sends nothing and reads only this bot's own chat.",
        "inputSchema": {"type": "object", "properties": {
            "source": {"type": "string", "enum": ["slack", "telegram", "discord", "whatsapp"], "description": "Only this channel; leave it out for all four."},
            "query": {"type": "string", "description": "Words to look for; every one of them has to appear in the message, in any case."},
            "since": {"type": "integer", "description": "Only messages at or after this time, in Unix milliseconds."},
            "limit": {"type": "integer", "minimum": 1, "maximum": 50}
        }},
        "annotations": {"readOnlyHint": true},
    })
}

/// The words of `query`, lowercased, all of which a message has to hold.
fn words(query: &str) -> Vec<String> {
    query.split_whitespace().map(str::to_lowercase).collect()
}

/// Whether `text` holds every word of `query` (an empty query holds for every message).
fn holds(text: &str, query: &[String]) -> bool {
    let text = text.to_lowercase();
    query.iter().all(|word| text.contains(word.as_str()))
}

/// The most recent channel messages in `bot`'s chat, newest first.
pub fn read(hub: &Hub, bot: &str, args: &Value) -> Result<Value> {
    let source = args["source"].as_str().unwrap_or_default();
    if !source.is_empty() && !SOURCES.contains(&source) {
        bail!("source must be slack, telegram, discord or whatsapp");
    }
    let query = words(args["query"].as_str().unwrap_or_default());
    let since = args["since"].as_i64().unwrap_or(0);
    let limit = usize::try_from(args["limit"].as_u64().unwrap_or(20).clamp(1, LIMIT as u64)).unwrap_or(LIMIT);
    let mut messages = Vec::with_capacity(limit);
    let mut before = i64::MAX;
    'pages: while messages.len() < limit {
        let page = hub.store.history(bot, before, PAGE)?;
        let Some(oldest) = page.last() else { break };
        before = oldest.seq;
        for entry in &page {
            if since > 0 && entry.created_at < since {
                // The chat is newest first: everything after this is too old.
                break 'pages;
            }
            let from = entry.data["source"].as_str().unwrap_or_default();
            if !SOURCES.contains(&from) || (!source.is_empty() && from != source) {
                continue;
            }
            let text = entry.data["text"].as_str().unwrap_or_default().trim();
            if text.is_empty() || !holds(text, &query) {
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
        if page.len() < usize::try_from(PAGE).unwrap_or(usize::MAX) {
            break;
        }
    }
    // Ported from `OpenBot` 0.72.0: the number keeps what it received for 90 days, so older lines and
    // the group messages that never reached the bot are only in that store.
    if source.is_empty() || source == "whatsapp" {
        let kept = crate::whatsapp::history::search(&hub.store, &query, since, i64::MAX, limit).unwrap_or_default();
        for hit in kept {
            messages.push(json!({
                "source": "whatsapp",
                "at": hit["at"].clone(),
                "from": hit["from"].clone(),
                "text": hit["text"].clone(),
            }));
        }
        messages.sort_by_key(|message| std::cmp::Reverse(message["at"].as_i64().unwrap_or_default()));
        messages.dedup_by(|a, b| a["at"] == b["at"] && a["text"] == b["text"]);
        messages.truncate(limit);
    }
    Ok(json!({"messages": messages}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_word_of_the_query_has_to_appear_in_any_case() {
        let query = words("  Invoice  PAID ");
        assert!(holds("INVOICE 42 was paid today", &query));
        assert!(!holds("Invoice 42 is due", &query), "an unpaid invoice is not the one asked for");
        let none: Vec<String> = Vec::new();
        assert!(holds("anything at all", &none), "an empty query matches every message");
        assert!(words("").is_empty());
    }
}
