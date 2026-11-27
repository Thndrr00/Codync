//! Agents say what must keep running (agentes.chat ADR 0176).
//!
//! A hold names who wants a program kept alive, what and why, and when it ends. Codync's host already
//! keeps the machine awake while runs are active (`keeps_awake`); this is the per-agent, listable
//! version with a bounded lifetime, so a quiet program can also hold the computer.

use serde_json::{Value, json};

pub const MAX_WHAT: usize = 80;
pub const MAX_WHY: usize = 200;
/// A hold may last at most seven days (ADR 0176).
pub const MAX_HOLD_MS: i64 = 7 * 24 * 60 * 60 * 1000;

/// A hold, validated: ends at the earlier of `until` and the seven-day cap.
pub fn hold(what: &str, why: &str, until: i64, now: i64) -> Result<Value, &'static str> {
    let what = what.trim();
    let why = why.trim();
    if what.is_empty() || what.chars().count() > MAX_WHAT {
        return Err("what must be 1-80 characters");
    }
    if why.chars().count() > MAX_WHY {
        return Err("why must be at most 200 characters");
    }
    if until <= now {
        return Err("until must be in the future");
    }
    let end = until.min(now.saturating_add(MAX_HOLD_MS));
    Ok(json!({"what": what, "why": why, "until": end, "capped": end < until}))
}

/// Whether a hold still applies now.
pub fn active(hold: &Value, now: i64) -> bool {
    hold["until"].as_i64().is_some_and(|u| u > now)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hold_is_bounded_and_expires() {
        let h = hold("mail bot", "waits for replies", 1000 + MAX_HOLD_MS + 5, 1000).unwrap();
        assert_eq!(h["capped"], true);
        assert_eq!(h["until"], 1000 + MAX_HOLD_MS);
        assert!(active(&h, 2000));
        assert!(!active(&h, 1000 + MAX_HOLD_MS));
        assert!(hold("", "why", 2000, 1000).is_err());
    }
}
