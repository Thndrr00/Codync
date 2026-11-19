//! Back to the latest messages (agentes.chat ADR 0140).
//!
//! A client that was away resumes at the newest messages, not at the oldest unread. This computes
//! the anchor from the history it already has: the last sequence, capped so a huge gap still opens
//! at a bounded page.

use serde_json::{Value, json};

pub const MAX_BACKTRACK: i64 = 200;

/// The sequence to resume from: `since` when it is recent enough, otherwise the newest `cap` back.
pub fn anchor(newest: i64, since: Option<i64>) -> Value {
    let at = match since {
        Some(s) if newest.saturating_sub(s) <= MAX_BACKTRACK => s,
        _ => newest.saturating_sub(MAX_BACKTRACK).max(0),
    };
    json!({"since": at, "backtrack": newest.saturating_sub(at)})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_recent_cursor_is_kept_and_a_stale_one_is_bounded() {
        assert_eq!(anchor(1000, Some(990))["since"], 990);
        assert_eq!(anchor(1000, Some(10))["since"], 1000 - MAX_BACKTRACK);
        assert_eq!(anchor(1000, None)["backtrack"], MAX_BACKTRACK);
    }
}
