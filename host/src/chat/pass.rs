//! Agents may pass: silence is a normal outcome (agentes.chat ADR 0049).
//!
//! In a room or a request chain, an agent that has nothing to add passes instead of inventing an
//! answer. A pass is a first-class outcome, not a failure, and the runner moves on.

use serde_json::{Value, json};

/// The pass text the host uses when an agent has nothing to say.
pub const PASS: &str = "(pass)";

/// Whether a reply is a pass, comparing case-insensitively and ignoring surrounding spaces.
pub fn is_pass(text: &str) -> bool {
    text.trim().eq_ignore_ascii_case(PASS)
}

/// The outcome recorded for a turn, marking a pass as normal rather than failed.
pub fn outcome(reply: &str) -> Value {
    let pass = is_pass(reply);
    json!({"state": if pass { "passed" } else { "answered" }, "failed": false, "pass": pass})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pass_is_normal_not_a_failure() {
        assert!(is_pass("  (PASS) "));
        assert!(!is_pass("(pass) but here is more"));
        let o = outcome(PASS);
        assert_eq!(o["state"], "passed");
        assert_eq!(o["failed"], false);
        assert_eq!(outcome("a real answer")["state"], "answered");
    }
}
