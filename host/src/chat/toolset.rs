//! Tool sets declared per run kind (agentes.chat ADR 0043 and 0047).
//!
//! A routine run is read-only by default: it may read and search but not post, ask a person, message
//! another bot or write. What it leaves out carries the reason the agent hears when it tries, so a
//! refusal teaches instead of failing silently. The full conversation run keeps every tool.

use serde_json::{Value, json};

/// The tools a quiet routine run may use, and why the rest are withheld.
pub fn routine() -> Value {
    json!({
        "allows": ["search", "read", "query_database", "hand_off"],
        "refuses": {
            "send_message": "Routines reach people only through hand_off.",
            "message_bot": "A routine cannot message another bot; link a routine flow with the user.",
            "ask_member": "A routine cannot ask a person; hand off to the conversation instead.",
            "write": "A routine run is read-only unless the user turns on actions for it."
        }
    })
}

/// A plain conversation run has the whole tool set.
pub fn conversation() -> Value {
    json!({"allows": ["all"], "refuses": {}})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_routine_is_read_only_and_says_why() {
        let set = routine();
        assert!(set["allows"].as_array().unwrap().contains(&json!("hand_off")));
        assert!(!set["allows"].as_array().unwrap().contains(&json!("send_message")));
        let why = set["refuses"]["send_message"].as_str().unwrap();
        assert!(why.contains("hand_off"), "{why}");
        assert_eq!(conversation()["allows"][0], json!("all"));
    }
}
