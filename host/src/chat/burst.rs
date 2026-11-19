//! Messages in a row: a burst is one request, a message mid-task updates it (agentes.chat ADR 0112).
//!
//! A person who sends several lines quickly should not start several turns. A burst groups them
//! into one request; a message that arrives while the turn runs joins it as an update, not a new
//! turn. This module holds the grouping rule the host applies before it enqueues a turn.

use serde_json::{Value, json};

/// Messages sent within this many milliseconds of each other belong to one burst.
pub const BURST_MS: i64 = 1_500;

/// Whether a message arriving at `at` joins the burst whose last message was at `last`.
pub fn joins_burst(last: i64, at: i64) -> bool {
    at >= last && at.saturating_sub(last) <= BURST_MS
}

/// A burst is one request: its text is the messages joined in order, with blank lines between.
pub fn request(messages: &[String]) -> Value {
    let text = messages.join("\n\n");
    json!({"text": text, "count": messages.len()})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_quick_follow_up_joins_the_burst_but_a_late_one_does_not() {
        assert!(joins_burst(1000, 1000 + BURST_MS));
        assert!(!joins_burst(1000, 1000 + BURST_MS + 1));
        assert!(!joins_burst(1000, 999));
    }

    #[test]
    fn a_burst_is_one_request_in_order() {
        let r = request(&["first".into(), "second".into()]);
        assert_eq!(r["text"], "first\n\nsecond");
        assert_eq!(r["count"], 2);
    }
}
