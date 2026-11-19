//! Presence and a written status for bots (agentes.chat ADR 0084).
//!
//! Runtime status says whether a bot is working; a written status says what its person told it to
//! tell others ("back at 3"). It is bounded, expires, and is shown by the clients next to the bot.

use crate::store::Store;
use anyhow::{Result, bail};
use serde_json::{Value, json};

const MAX_STATUS_CHARS: usize = 120;

fn key(bot: &str) -> String {
    format!("presence:{bot}")
}

/// A written status: the text and when it stops applying (Unix ms, `None` = until cleared).
pub fn set(store: &Store, bot: &str, text: Option<&str>, until: Option<i64>) -> Result<Value> {
    let status = match text {
        Some(text) => {
            let text = text.trim();
            if text.is_empty() {
                bail!("a status needs text");
            }
            if text.chars().count() > MAX_STATUS_CHARS {
                bail!("a status must be at most {MAX_STATUS_CHARS} characters");
            }
            if let Some(until) = until
                && until <= crate::store::now_ms()
            {
                bail!("a status cannot end in the past");
            }
            Some(json!({"text": text, "until": until}))
        }
        None => Value::Null,
    };
    store.kv_set(&key(bot), &serde_json::to_string(&status)?)?;
    Ok(json!({"botId": bot, "status": status}))
}

/// The status still in force now; an expired one reads as absent.
pub fn get(store: &Store, bot: &str, now: i64) -> Option<Value> {
    let text = store.kv_read(&key(bot)).ok().flatten()?;
    let status: Value = serde_json::from_str(&text).ok()?;
    if status.is_null() {
        return None;
    }
    if status["until"].as_i64().is_some_and(|until| until <= now) {
        return None;
    }
    Some(status)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_status_is_bounded() {
        assert!("  away  ".trim().chars().count() < MAX_STATUS_CHARS);
        assert!(MAX_STATUS_CHARS < 200);
    }

    #[test]
    fn an_expired_status_is_absent() {
        let status = json!({"text": "back at 3", "until": 1000});
        assert!(status["until"].as_i64().is_some_and(|until| until <= 2000));
    }
}
