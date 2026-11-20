//! An agent's context window (agentes.chat ADR 0086).
//!
//! Each agent publishes how many tokens its context holds and how much is free. The host maps that
//! to the compaction threshold it already applies, so the same numbers drive both. No new state:
//! this is the rule that turns a reported window into a limit.

use serde_json::{Value, json};

/// The fraction of the window left below which the host compacts.
pub const COMPACT_AT: f64 = 0.2;

/// Whether a window should be compacted, from used/total tokens reported by the agent.
pub fn should_compact(used: u64, total: u64) -> Value {
    let free = total.saturating_sub(used);
    let ratio = if total == 0 { 1.0 } else { free as f64 / total as f64 };
    let compact = total > 0 && ratio <= COMPACT_AT;
    json!({"compact": compact, "freePercent": (ratio * 100.0).round() as u64})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compaction_starts_when_the_window_is_nearly_full() {
        assert_eq!(should_compact(90, 100)["compact"], true);
        assert_eq!(should_compact(70, 100)["compact"], false);
        assert_eq!(should_compact(0, 0)["compact"], false);
        assert_eq!(should_compact(90, 100)["freePercent"], 10);
    }
}
