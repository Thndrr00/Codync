//! What an approval card asks for, in one comparable value: its tool kind, its command (or title),
//! the folder it runs in and the diffs it would apply, hashed. Two requests with the same
//! fingerprint ask the same thing, so the ledger can tell "that action again" from "another one",
//! and an approval stops covering the moment what it would do changes.
//!
//! Idea from agentes.chat's single-use approvals (`packages/engine`, AGPL-3.0-only): read, not
//! copied.

use crate::store::approvals::MAX_SUBJECT;
use serde_json::Value;

/// The fingerprint of a card's request, from the `data` its entry holds.
pub fn fingerprint(data: &Value) -> String {
    let kind = data["toolKind"].as_str().unwrap_or("other");
    let subject = data["command"].as_str().or_else(|| data["title"].as_str()).unwrap_or_default();
    let cwd = data["cwd"].as_str().unwrap_or_default();
    let diffs = data["diffs"].as_array().map(|d| serde_json::to_string(d).unwrap_or_default()).unwrap_or_default();
    crate::agent::registry::sha256_hex(format!("{kind}\u{1f}{subject}\u{1f}{cwd}\u{1f}{diffs}").as_bytes())
}

/// What the person saw the card ask: its command, else its title, capped for the record.
pub fn subject(data: &Value) -> String {
    let text = data["command"].as_str().or_else(|| data["title"].as_str()).unwrap_or_default().trim();
    if text.chars().count() <= MAX_SUBJECT { text.to_owned() } else { text.chars().take(MAX_SUBJECT).collect() }
}

/// The decision an answer is: `allow`, `deny`, or `cancelled` when nobody chose.
pub fn decision(data: &Value, option_id: Option<&str>) -> &'static str {
    let Some(option_id) = option_id else { return "cancelled" };
    let kind = data["options"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|o| o["optionId"].as_str() == Some(option_id))
        .and_then(|o| o["kind"].as_str())
        .unwrap_or_default();
    if kind.starts_with("reject") { "deny" } else { "allow" }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn card() -> Value {
        json!({
            "title": "Run git push",
            "toolKind": "execute",
            "command": "git push origin dev",
            "cwd": "/repo",
            "diffs": [],
            "options": [
                {"optionId": "a", "name": "Allow", "kind": "allow_once"},
                {"optionId": "r", "name": "Deny", "kind": "reject_once"}
            ]
        })
    }

    #[test]
    fn the_same_request_has_the_same_fingerprint() {
        assert_eq!(fingerprint(&card()), fingerprint(&card()));
    }

    #[test]
    fn a_changed_action_is_another_request() {
        let mut other = card();
        other["command"] = json!("git push origin main");
        assert_ne!(fingerprint(&card()), fingerprint(&other));
        other = card();
        other["cwd"] = json!("/elsewhere");
        assert_ne!(fingerprint(&card()), fingerprint(&other));
        other = card();
        other["diffs"] = json!([{"path": "a.rs", "newText": "x"}]);
        assert_ne!(fingerprint(&card()), fingerprint(&other));
    }

    #[test]
    fn a_title_alone_still_fingerprints() {
        let mut titled = card();
        titled["command"] = Value::Null;
        assert_eq!(subject(&titled), "Run git push");
        let mut renamed = titled.clone();
        renamed["title"] = json!("Run the tests");
        assert_ne!(fingerprint(&titled), fingerprint(&renamed));
    }

    #[test]
    fn the_answer_says_allow_deny_or_cancelled() {
        assert_eq!(decision(&card(), Some("a")), "allow");
        assert_eq!(decision(&card(), Some("r")), "deny");
        assert_eq!(decision(&card(), None), "cancelled");
    }

    #[test]
    fn a_long_subject_is_capped() {
        let mut long = card();
        long["command"] = json!("x".repeat(MAX_SUBJECT + 50));
        assert_eq!(subject(&long).chars().count(), MAX_SUBJECT);
    }
}
