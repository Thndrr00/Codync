//! Hand-offs between agents come back (agentes.chat ADR 0095).
//!
//! When bot A hands work to bot B and B finishes, the result comes back to A's conversation as a
//! new turn, with a note naming where it came from. Codync already relays a requester's reply; this
//! is the rule that marks a hand-off's result so it is not mistaken for a person's words.

use serde_json::{Value, json};

/// A result from a hand-off, tagged with the bot that did the work and the one it was for.
pub fn result(from: &str, to: &str, text: &str) -> Value {
    json!({
        "kind": "handoff_result",
        "from": from,
        "to": to,
        "text": text,
        "author": from,
        "trusted": false,
    })
}

/// Whether a received turn is a hand-off result that should be announced as such.
pub fn is_handoff(value: &Value) -> bool {
    value["kind"] == "handoff_result"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_handoff_result_is_tagged_and_not_a_persons_words() {
        let r = result("b", "a", "done");
        assert!(is_handoff(&r));
        assert_eq!(r["trusted"], false);
        assert_eq!(r["author"], "b");
        assert!(!is_handoff(&json!({"kind": "user"})));
    }
}
