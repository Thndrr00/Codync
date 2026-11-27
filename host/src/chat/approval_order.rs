//! Approved actions run in the order the agent asked (agentes.chat ADR 0182).
//!
//! A card is a line: requests from one agent keep their ask order. A person may decide in any order,
//! but a run waits until every earlier request on the same card is settled. Codync's approval ledger
//! keeps one decision per card; this is the order rule on top of it, so parallel approvals cannot
//! let a later action overtake an earlier one.

use serde_json::Value;

/// A line of requests, oldest first: `created_at` then the id for ties.
pub fn line(requests: &[Value]) -> Vec<String> {
    let mut sorted = requests.to_vec();
    sorted.sort_by(|a, b| {
        a["createdAt"]
            .as_i64()
            .unwrap_or(0)
            .cmp(&b["createdAt"].as_i64().unwrap_or(0))
            .then_with(|| a["id"].as_str().unwrap_or("").cmp(b["id"].as_str().unwrap_or("")))
    });
    sorted.iter().filter_map(|r| r["id"].as_str().map(str::to_owned)).collect()
}

/// Whether a run may act now: every request before its place on the card must be settled.
pub fn may_run(requests: &[Value], run_id: &str) -> bool {
    let order = line(requests);
    let Some(at) = order.iter().position(|id| id == run_id) else { return false };
    requests
        .iter()
        .filter(|r| order.iter().position(|id| Some(id.as_str()) == r["id"].as_str()).unwrap_or(usize::MAX) < at)
        .all(|r| r["settled"] == true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_later_action_waits_for_an_earlier_one() {
        let reqs = vec![
            json!({"id": "a", "createdAt": 1, "settled": false}),
            json!({"id": "b", "createdAt": 2, "settled": true}),
        ];
        assert_eq!(line(&reqs), ["a", "b"]);
        assert!(!may_run(&reqs, "b"), "b asked after a, which is still waiting");
        assert!(may_run(&reqs, "a"));
        let settled =
            vec![json!({"id":"a","createdAt":1,"settled":true}), json!({"id":"b","createdAt":2,"settled":true})];
        assert!(may_run(&settled, "b"));
    }
}
