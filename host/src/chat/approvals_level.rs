//! An approvals level per agent (agentes.chat ADR 0173): always ask, only risky, or never ask.
//!
//! Codync already has a per-bot `permission` (ask/auto) and sensitive tool rules. This is the third
//! setting a person can choose, with a fixed precedence code decides: `never` always refuses the
//! question itself; `always` asks on every write; `risky` keeps the current behavior.

use serde_json::{Value, json};

/// The three levels, in the order a client shows them.
pub const LEVELS: [&str; 3] = ["always", "risky", "never"];

/// Resolves the level a bot saved; an unknown value falls back to risky (today's behavior).
pub fn level(saved: Option<&str>) -> Value {
    let chosen = saved.filter(|l| LEVELS.contains(l)).unwrap_or("risky");
    json!({"level": chosen, "available": LEVELS})
}

/// Whether a tool call should ask, given the level and whether the tool is a sensitive write.
pub fn asks(level: &str, sensitive: bool) -> bool {
    match level {
        "never" => false,
        "always" => true,
        _ => sensitive,
    }
}

/// What to record when a call ran unasked: `never` leaves an audit reason, others do not.
pub fn unasked_reason(level: &str) -> Option<&'static str> {
    (level == "never").then_some("approvals_level_never")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_level_decides_who_asks() {
        assert!(asks("always", false));
        assert!(asks("risky", true));
        assert!(!asks("risky", false));
        assert!(!asks("never", true));
        assert_eq!(unasked_reason("never"), Some("approvals_level_never"));
        assert_eq!(unasked_reason("risky"), None);
        assert_eq!(level(Some("bogus"))["level"], "risky");
    }
}
