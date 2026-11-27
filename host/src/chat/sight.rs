//! One "is anyone looking" state (agentes.chat ADR 0228 §1).
//!
//! A client tells the host whether a person is present; the host uses one state for quiet work and
//! notices instead of each surface guessing from focus or timers. Codync's proactive settings and
//! routine notices can read this single signal.

use serde_json::{Value, json};

pub const STATES: [&str; 4] = ["hidden", "away", "idle", "present"];
/// No key, click or pointer move for two minutes is idle (ADR 0228).
pub const IDLE_MS: i64 = 2 * 60 * 1000;

/// Resolves the state from sight, focus and last input. The most attentive signal wins.
pub fn sight(visible: bool, focused: bool, pointer_over: bool, last_input: i64, now: i64) -> Value {
    let state = if !visible {
        "hidden"
    } else if !focused && !pointer_over {
        "away"
    } else if now.saturating_sub(last_input) > IDLE_MS {
        "idle"
    } else {
        "present"
    };
    json!({"state": state, "anyoneLooking": state == "present" || state == "idle"})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_most_attentive_signal_wins() {
        assert_eq!(sight(false, true, true, 1000, 1000)["state"], "hidden");
        assert_eq!(sight(true, false, false, 1000, 1000)["state"], "away");
        assert_eq!(sight(true, true, true, 1000, 1000 + IDLE_MS + 1)["state"], "idle");
        assert_eq!(sight(true, true, true, 1000, 1000)["state"], "present");
        assert_eq!(sight(false, true, true, 1000, 1000)["anyoneLooking"], false);
    }
}
