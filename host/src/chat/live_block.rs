//! Live blocks: what an agent shows to be used, not kept (agentes.chat ADR 0113).
//!
//! An agent can show a small interactive block (a poll, a choice, a form) for the person to act on
//! right away. It is live — it does not become a permanent message. This is the shape of that block.

use anyhow::{Result, bail};
use serde_json::{Value, json};

const MAX_TITLE: usize = 200;

/// A live block with its kind and payload; a choice card is one kind of it.
pub fn block(kind: &str, title: &str, payload: &Value) -> Result<Value> {
    if title.trim().is_empty() {
        bail!("a live block needs a title");
    }
    if title.chars().count() > MAX_TITLE {
        bail!("a live block title must be at most {MAX_TITLE} characters");
    }
    Ok(json!({
        "type": "live_block",
        "kind": kind,
        "title": title.trim(),
        "payload": payload,
        "persistent": false,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_live_block_is_not_persistent() {
        let b = block("poll", "Which one?", &json!({"options": ["a", "b"]})).unwrap();
        assert_eq!(b["persistent"], false);
        assert_eq!(b["kind"], "poll");
        assert!(block("poll", "", &json!({})).is_err());
    }
}
