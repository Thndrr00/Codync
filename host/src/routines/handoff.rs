//! A routine's handoff to the conversation (agentes.chat ADR 0047).
//!
//! A routine run works quietly: its own final text stays in the run history. When it has news worth
//! telling, it calls the `hand_off` tool with the complete message; a conversation run reads and clears
//! it with `routineHandoff` and writes it in the agent's voice. This keeps untrusted data from a
//! routine's read straight out of the chat: a second run, with the conversation's context, says it.

use crate::store::Store;
use anyhow::{Result, bail};
use serde_json::{Value, json};

/// A handoff's message cannot exceed this many characters (ADR 0047).
pub const MAX_HAND_OFF_CHARS: usize = 8_000;

fn key(bot: &str) -> String {
    format!("routine-handoff:{bot}")
}

/// Validates a handoff message: trimmed, nonempty, bounded. A second handoff replaces the first.
pub fn normalize(text: &str) -> Result<String> {
    let text = text.trim();
    if text.is_empty() {
        bail!("a handoff needs a message");
    }
    if text.chars().count() > MAX_HAND_OFF_CHARS {
        bail!("a handoff message must be at most {MAX_HAND_OFF_CHARS} characters");
    }
    Ok(text.to_owned())
}

/// Records the message a routine hands off to its bot's conversation.
pub fn save(store: &Store, bot: &str, text: &str) -> Result<Value> {
    let message = normalize(text)?;
    store.kv_set(&key(bot), &serde_json::to_string(&json!({"message": message}))?)?;
    Ok(json!({"handedOff": true}))
}

/// Takes the pending handoff, if any. Reading consumes it: a conversation says it once.
pub fn take(store: &Store, bot: &str) -> Option<String> {
    let text = store.kv_read(&key(bot)).ok().flatten()?;
    let value: Value = serde_json::from_str(&text).ok()?;
    let message = value["message"].as_str()?.to_owned();
    let _ = store.kv_delete(&key(bot));
    Some(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_handoff_is_trimmed_bounded_and_needs_text() {
        assert_eq!(normalize("  a finding  ").unwrap(), "a finding");
        assert!(normalize("   ").is_err());
        assert!(normalize(&"x".repeat(MAX_HAND_OFF_CHARS)).is_ok());
        assert!(normalize(&"x".repeat(MAX_HAND_OFF_CHARS + 1)).is_err());
    }
}
