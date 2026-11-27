//! Agents say what they will do before slow work (agentes.chat ADR 0207).
//!
//! The first words of a turn go out within a second so the person knows the request was taken, while
//! the full reply still arrives whole at the end. Codync already holds streamed text; this is the
//! bounded intent line the client can show on the activity bar.

use serde_json::{Value, json};

pub const MAX_INTENT: usize = 120;

/// The intent line: short, single-line, bounded.
pub fn intent(text: &str) -> Result<Value, &'static str> {
    let one_line = text.trim().replace(['\n', '\r'], " ");
    if one_line.is_empty() {
        return Err("intent needs text");
    }
    let clipped: String = one_line.chars().take(MAX_INTENT).collect();
    Ok(json!({"type": "agent_intent", "text": clipped, "waitForReply": true}))
}

/// The line is a promise, not the reply: the client must keep it distinct from the final message.
pub fn is_intent(value: &Value) -> bool {
    value["type"] == "agent_intent"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_intent_is_one_short_line_and_not_a_reply() {
        let i = intent("  Let me look at the logs\n and the diff  ").unwrap();
        assert_eq!(i["text"], "Let me look at the logs  and the diff");
        assert_eq!(i["waitForReply"], true);
        assert!(is_intent(&i));
        assert!(!is_intent(&json!({"type": "agent"})));
        assert!(intent("   ").is_err());
    }
}
