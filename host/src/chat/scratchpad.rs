//! A bot's working notes for one conversation (its main chat, or one thread): a plan, what's done,
//! what to check next. They outlive compactions and session restarts, which the agent's own
//! context doesn't, and stay out of the transcript and memory. The `chat` server's
//! `scratchpad_read` and `scratchpad_write` keep them; writing nothing clears them.
//!
//! Idea from rakazo's `packages/adapters/src/scratchpad-tools.ts` (Apache-2.0); no code copied.

use crate::LockExt;
use crate::hub::Hub;
use crate::store::{Lane, lane_key};
use anyhow::{Context, Result, bail, ensure};
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
    call_in_lane(hub, bot, &lane, name, args)
}

/// User quick capture always belongs to the bot's main chat, never the active turn's group or routine.
pub(crate) fn append_main(hub: &Hub, bot: &str, text: &str) -> Result<Value> {
    hub.store.bot(bot)?.filter(|row| !row.deleted).context("Unknown bot")?;
    ensure!(!text.trim().is_empty(), "A note is required");
    call_in_lane(hub, bot, &Lane::main(bot), "scratchpad_write", &json!({"text": text, "mode": "append"}))
}

fn call_in_lane(hub: &Hub, bot: &str, lane: &Lane, name: &str, args: &Value) -> Result<Value> {
    let key = lane_key("scratchpad", bot, lane);
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

    fn fixture() -> (std::path::PathBuf, std::sync::Arc<Hub>) {
        let dir = std::env::temp_dir().join(format!("codync-scratchpad-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let cfg: BotConfig =
            serde_json::from_value(json!({"id": "a", "name": "A", "backend": "claude", "cwd": dir.to_string_lossy()}))
                .unwrap();
        store.save_bot(&cfg).unwrap();
        let hub = Hub::new(store, "scratchpad-test".into(), Identity::load_or_create(&dir).unwrap(), "test".into(), 0);
        (dir, hub)
    }

    #[test]
    fn notes_are_kept_per_conversation_and_bounded() {
        let (dir, hub) = fixture();
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

    #[test]
    fn user_capture_appends_to_main_notes_while_the_agent_is_in_another_conversation() {
        let (dir, hub) = fixture();
        append_main(&hub, "a", "main note").unwrap();
        let group = Lane::in_thread("crew", "routine-thread-active");
        hub.set_runtime("a", |runtime| runtime.lane = Some(group.clone()));
        call(&hub, "a", "scratchpad_write", &json!({"text": "group note"})).unwrap();
        append_main(&hub, "a", " captured note ").unwrap();
        assert_eq!(
            hub.store.kv_read(&lane_key("scratchpad", "a", &Lane::main("a"))).unwrap().as_deref(),
            Some("main note\ncaptured note")
        );
        assert_eq!(call(&hub, "a", "scratchpad_read", &json!({})).unwrap()["text"], "group note");
        assert_eq!(hub.store.kv_read(&lane_key("scratchpad", "a", &group)).unwrap().as_deref(), Some("group note"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn capture_rejects_blank_unknown_and_deleted_bots_without_changing_notes() {
        let (dir, hub) = fixture();
        append_main(&hub, "a", "keep").unwrap();
        assert!(append_main(&hub, "a", " \n\t ").is_err());
        assert!(append_main(&hub, "missing", "note").is_err());
        assert_eq!(call(&hub, "a", "scratchpad_read", &json!({})).unwrap()["text"], "keep");
        hub.store.delete_bot("a").unwrap();
        assert!(append_main(&hub, "a", "must not return").is_err());
        assert_eq!(hub.store.kv_read(&lane_key("scratchpad", "a", &Lane::main("a"))).unwrap(), None);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn capture_respects_the_total_character_limit_without_replacing_existing_notes() {
        let (dir, hub) = fixture();
        append_main(&hub, "a", &"筆".repeat(15_000)).unwrap();
        assert!(append_main(&hub, "a", &"記".repeat(1_000)).is_err());
        assert_eq!(
            call(&hub, "a", "scratchpad_read", &json!({})).unwrap()["text"].as_str().unwrap().chars().count(),
            15_000
        );
        assert_eq!(append_main(&hub, "a", &"記".repeat(999)).unwrap()["chars"], MAX_CHARS);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
