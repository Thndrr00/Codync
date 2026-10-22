//! A bot's working notes for one conversation (its main chat, or one thread): a plan, what's done,
//! what to check next. They outlive compactions and session restarts, which the agent's own
//! context doesn't, and stay out of the transcript and memory. The `chat` server's
//! `scratchpad_read` and `scratchpad_write` keep them; writing nothing clears them.
//!
//! Idea from rakazo's `packages/adapters/src/scratchpad-tools.ts` (Apache-2.0); no code copied.

use crate::LockExt;
use crate::hub::Hub;
use crate::store::{Lane, lane_key};
use anyhow::{Result, bail};
use serde_json::{Value, json};

/// Longest scratchpad, in characters.
const MAX_CHARS: usize = 16_000;

pub const TOOLS: [&str; 2] = ["scratchpad_read", "scratchpad_write"];

pub fn tools() -> [Value; 2] {
    [
        json!({
            "name": "scratchpad_read",
            "description": "Read your working notes for this conversation (empty when you have none). Read them after a compaction or when you pick a long task back up.",
            "inputSchema": {"type": "object", "properties": {}},
            "annotations": {"readOnlyHint": true},
        }),
        json!({
            "name": "scratchpad_write",
            "description": format!("Keep working notes for this conversation (a plan, what's done, what's next); they survive compactions and restarts. The user doesn't see them. replace (default) rewrites them, append adds a line; empty text with replace clears them. At most {MAX_CHARS} characters; never put passwords or keys here."),
            "inputSchema": {
                "type": "object",
                "properties": {
                    "text": {"type": "string"},
                    "mode": {"type": "string", "enum": ["replace", "append"]},
                },
                "required": ["text"],
            },
            "annotations": {"readOnlyHint": false},
        }),
    ]
}

/// One read-modify-write at a time, so two appends can't drop one.
static WRITE: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn call(hub: &Hub, bot: &str, name: &str, args: &Value) -> Result<Value> {
    // The conversation of the turn that called; the bot's own notes there, even in a group.
    let lane = hub.runtime(bot).lane.unwrap_or_else(|| Lane::main(bot));
    let key = lane_key("scratchpad", bot, &lane);
    let _one = WRITE.locked();
    let notes = hub.store.kv_read(&key)?.unwrap_or_default();
    match name {
        "scratchpad_read" => Ok(json!({"text": notes})),
        "scratchpad_write" => {
            let text = args["text"].as_str().unwrap_or_default();
            let next = match args["mode"].as_str().unwrap_or("replace") {
                "replace" => text.trim().to_owned(),
                "append" if notes.is_empty() => text.trim().to_owned(),
                "append" => format!("{notes}\n{}", text.trim()),
                other => bail!("mode is replace or append, not {other}"),
            };
            let chars = next.chars().count();
            if chars > MAX_CHARS {
                bail!(
                    "the scratchpad would be {chars} characters; keep it under {MAX_CHARS} (replace it with a shorter summary)"
                );
            }
            if next.is_empty() {
                hub.store.kv_delete(&key)?;
            } else {
                hub.store.kv_set(&key, &next)?;
            }
            Ok(json!({"saved": true, "chars": chars}))
        }
        _ => bail!("unknown scratchpad tool: {name}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::identity::Identity;
    use crate::store::{BotConfig, Store};

    #[test]
    fn notes_are_kept_per_conversation_and_bounded() {
        let dir = std::env::temp_dir().join(format!("codync-scratchpad-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let cfg: BotConfig =
            serde_json::from_value(json!({"id": "a", "name": "A", "backend": "claude", "cwd": dir.to_string_lossy()}))
                .unwrap();
        store.save_bot(&cfg).unwrap();
        let hub = Hub::new(store, "scratchpad-test".into(), Identity::load_or_create(&dir).unwrap(), "test".into(), 0);
        call(&hub, "a", "scratchpad_write", &json!({"text": "1. read the spec"})).unwrap();
        call(&hub, "a", "scratchpad_write", &json!({"text": "2. write tests", "mode": "append"})).unwrap();
        assert_eq!(call(&hub, "a", "scratchpad_read", &json!({})).unwrap()["text"], "1. read the spec\n2. write tests");
        assert!(call(&hub, "a", "scratchpad_write", &json!({"text": "x".repeat(MAX_CHARS + 1)})).is_err());
        assert!(call(&hub, "a", "scratchpad_write", &json!({"text": "x", "mode": "prepend"})).is_err());
        // A thread's notes are its own.
        assert_eq!(hub.store.kv_read(&lane_key("scratchpad", "a", &Lane::in_thread("a", "root"))).unwrap(), None);
        call(&hub, "a", "scratchpad_write", &json!({"text": ""})).unwrap();
        assert_eq!(call(&hub, "a", "scratchpad_read", &json!({})).unwrap()["text"], "");
        call(&hub, "a", "scratchpad_write", &json!({"text": "keep"})).unwrap();
        // Lane state goes with the bot.
        hub.store.delete_bot("a").unwrap();
        assert_eq!(call(&hub, "a", "scratchpad_read", &json!({})).unwrap()["text"], "");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
