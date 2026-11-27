//! Agents apply their improvements: a proposal is reviewed, can be undone, and is recorded (ADR 0219).
//!
//! Codync already proposes memory rules from two denials (`approval_lessons`) and keeps an approval
//! ledger. This is the rule for applying a proposed improvement: only a person (or a bounded right
//! hand) applies it, the record says who decided and why, and it can be undone for a bounded time.

use serde_json::{Value, json};

/// How long a proposal stays undoable (ADR 0219: one week).
pub const UNDO_MS: i64 = 7 * 24 * 60 * 60 * 1000;

/// Applies a proposal once, recording who decided and when.
pub fn apply(text: &str, decided_by: &str, now: i64) -> Value {
    json!({
        "type": "improvement",
        "text": text.trim(),
        "status": "applied",
        "decidedBy": decided_by,
        "appliedAt": now,
        "undoUntil": now + UNDO_MS,
    })
}

/// Whether a proposal can still be undone.
pub fn undoable(applied: &Value, now: i64) -> bool {
    applied["status"] == "applied" && applied["undoUntil"].as_i64().is_some_and(|u| u > now)
}

/// Undoing keeps the record: it says the agent's change was rolled back by a person.
pub fn undo(mut applied: Value, by: &str, now: i64) -> Value {
    if undoable(&applied, now) {
        applied["status"] = "undone".into();
        applied["undoneBy"] = json!(by);
        applied["undoneAt"] = json!(now);
    }
    applied
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applying_is_recorded_and_undoable_for_a_week() {
        let a = apply("  keep answers short ", "owner", 1000);
        assert_eq!(a["status"], "applied");
        assert_eq!(a["text"], "keep answers short");
        assert!(undoable(&a, 1000));
        assert!(!undoable(&a, 1000 + UNDO_MS + 1));
        assert_eq!(undo(a.clone(), "owner", 2000)["status"], "undone");
        assert_eq!(undo(a.clone(), "owner", 1000 + UNDO_MS + 1)["status"], "applied");
    }
}
