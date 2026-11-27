//! People hear when a computer stops answering, and when it is back (agentes.chat ADR 0241).
//!
//! A short drop is not an outage: a run or a CLI retry can survive it (ADR 0160). The notice waits
//! past a grace period, names what was in flight, and clears when the computer is heard from again.

use serde_json::{Value, json};

/// A drop shorter than this is retried, not announced (ADR 0241 §Decision).
pub const GRACE_MS: i64 = 60_000;

/// Whether a silent gap is long enough to announce, from the last word heard and now.
pub fn is_outage(last_seen: i64, now: i64) -> bool {
    now.saturating_sub(last_seen) > GRACE_MS
}

/// The notice, in the words people read; `runs` and `calls` are what was in flight.
pub fn notice(computer: &str, runs: usize, calls: usize, back: bool) -> Value {
    json!({
        "type": if back { "computer_back" } else { "computer_down" },
        "computer": computer,
        "runs": runs,
        "calls": calls,
        "text": if back {
            format!("{computer} is back.")
        } else {
            format!("{computer} stopped answering. Its agents' work waits and goes on when it returns.")
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_short_drop_is_retried_not_announced() {
        assert!(!is_outage(1000, 1000 + GRACE_MS - 1));
        assert!(is_outage(1000, 1000 + GRACE_MS + 1));
        let down = notice("Mac", 2, 1, false);
        assert_eq!(down["type"], "computer_down");
        assert_eq!(notice("Mac", 0, 0, true)["type"], "computer_back");
    }
}
