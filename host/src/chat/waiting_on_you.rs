//! "Waiting on you" is a view of the inbox, never a second list (agentes.chat ADR 0172).
//!
//! Codync already has open items and pending approval cards. This is the filter that answers "what
//! waits on a person" from those rows: not done, not snoozed, and an approval only while the person
//! can still decide it. It never keeps its own state, so the two views cannot drift apart.

use serde_json::{Value, json};

/// Whether an inbox row is waiting on the person now.
pub fn waiting(row: &Value, now: i64) -> bool {
    if row["done"] == true || row["snoozedUntil"].as_i64().is_some_and(|s| s > now) {
        return false;
    }
    if row["kind"] == "approval" {
        // An approval whose deadline passed can no longer be decided.
        return row["decidable"] == true && row["expiresAt"].as_i64().is_none_or(|e| e > now);
    }
    true
}

/// The view: rows oldest first, capped, from the inbox the caller already has.
pub fn list(rows: &[Value], now: i64, cap: usize) -> Value {
    let mut waiting: Vec<Value> = rows.iter().filter(|r| waiting(r, now)).cloned().collect();
    waiting.sort_by_key(|r| r["createdAt"].as_i64().unwrap_or(0));
    waiting.truncate(cap);
    json!({"items": waiting, "count": waiting.len()})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_actionable_rows_wait() {
        assert!(waiting(&json!({"kind": "notice"}), 1000));
        assert!(!waiting(&json!({"kind": "notice", "done": true}), 1000));
        assert!(!waiting(&json!({"kind": "notice", "snoozedUntil": 2000}), 1000));
        assert!(waiting(&json!({"kind": "approval", "decidable": true, "expiresAt": 2000}), 1000));
        assert!(!waiting(&json!({"kind": "approval", "decidable": false}), 1000));
        assert!(!waiting(&json!({"kind": "approval", "decidable": true, "expiresAt": 500}), 1000));
        let list = list(&[json!({"kind":"notice","createdAt":5}), json!({"kind":"notice","createdAt":2})], 1000, 10);
        assert_eq!(list["items"][0]["createdAt"], 2);
    }
}
