//! The rule a denial proposes. Two denials of the same kind of action are the same "no" said twice,
//! so the bot offers to stop asking: a `deny` rule for that command (or title) and tool kind, which
//! the person may widen before keeping it.
//!
//! Idea from agentes.chat's memory that learns from rejected approvals (`packages/engine`,
//! AGPL-3.0-only): the behavior, not the code.

use serde_json::{Value, json};

/// The kind of action a denial is about: its tool kind and its command (or title), with case and
/// spacing normalized so the same "no" is recognized twice.
pub fn family(kind: &str, subject: &str) -> String {
    let normal = subject.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase();
    crate::agent::registry::sha256_hex(format!("{kind}\u{1f}{normal}").as_bytes())
}

/// The approval rule the bot proposes for a denied action.
pub fn rule(subject: &str, kind: &str) -> Value {
    json!({"pattern": subject.trim(), "kind": kind, "decision": "deny"})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_no_in_another_case_or_spacing_is_the_same_kind() {
        assert_eq!(family("execute", "git  push"), family("execute", " Git Push "));
        assert_ne!(family("execute", "git push"), family("edit", "git push"));
    }

    #[test]
    fn the_proposed_rule_denies_that_action_and_its_kind() {
        let rule = rule("  git push  ", "execute");
        assert_eq!(rule["pattern"], "git push");
        assert_eq!(rule["kind"], "execute");
        assert_eq!(rule["decision"], "deny");
    }
}
