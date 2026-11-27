//! What we do to the computer is said, never a bare "Offline" (agentes.chat ADR 0257).
//!
//! A restart, update, move or sleep has a kind, a since and an expected back. Codync's host knows
//! its own health state; this is the vocabulary the clients show instead of a bare offline badge.

use serde_json::{Value, json};

pub const KINDS: [&str; 5] = ["moving", "restarting", "updating", "converting", "maintenance"];
/// A word sticks for at most two hours without an ETA, and 30 minutes past one.
pub const NO_ETA_MS: i64 = 2 * 60 * 60 * 1000;
pub const OVERRUN_MS: i64 = 30 * 60 * 1000;

/// A maintenance word, with a bounded visible window.
pub fn state(kind: Option<&str>, since: i64, eta: Option<i64>, now: i64) -> Value {
    let Some(kind) = kind.filter(|k| KINDS.contains(k)) else {
        return json!({"kind": Value::Null, "offline": true});
    };
    let until = eta.map_or(since.saturating_add(NO_ETA_MS), |e| e.saturating_add(OVERRUN_MS));
    json!({
        "kind": kind,
        "since": since,
        "eta": eta,
        "visible": now <= until,
        "offline": false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_maintenance_word_is_shown_instead_of_offline() {
        let s = state(Some("restarting"), 1000, Some(2000), 1500);
        assert_eq!(s["kind"], "restarting");
        assert_eq!(s["offline"], false);
        assert_eq!(s["visible"], true);
        assert_eq!(state(None, 1000, None, 1500)["offline"], true);
        assert_eq!(state(Some("updating"), 1000, None, 1000 + NO_ETA_MS + 1)["visible"], false);
    }
}
