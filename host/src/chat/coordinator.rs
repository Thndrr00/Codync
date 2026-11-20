//! The team's coordinator (agentes.chat ADR 0093).
//!
//! A lead bot coordinates a group: it decides who answers, and members hand their result back to
//! it. Codync already has a `lead` bot and a `lead` flag; this is the rule for choosing it from a
//! roster and naming the hand-back, without duplicating the roster logic.

use serde_json::{Value, json};

/// The coordinator is the lead, or the first pinned bot when no lead is set.
pub fn pick(bots: &[Value]) -> Option<String> {
    bots.iter()
        .find(|b| b["lead"] == true && b["deleted"] != true)
        .or_else(|| bots.iter().find(|b| b["pinned"] == true && b["deleted"] != true))
        .and_then(|b| b["id"].as_str().map(str::to_owned))
}

/// The line a member uses when it hands its result back to the coordinator.
pub fn hand_back(coordinator: &str) -> Value {
    json!({"to": coordinator, "kind": "hand_back", "text": format!("Handing back to {coordinator}")})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lead_coordinates_and_a_deleted_one_is_skipped() {
        let bots = vec![
            json!({"id": "a", "pinned": true, "deleted": false}),
            json!({"id": "b", "lead": true, "deleted": false}),
        ];
        assert_eq!(pick(&bots), Some("b".into()));
        let deleted = vec![json!({"id": "b", "lead": true, "deleted": true}), json!({"id": "a", "pinned": true})];
        assert_eq!(pick(&deleted), Some("a".into()));
        assert_eq!(hand_back("b")["to"], "b");
    }
}
