//! One controller decides how every turn ends (agentes.chat ADR 0097).
//!
//! A turn ends in one of a few ways; this is the single place that names them, so the client and the
//! host agree on what "done", "stopped" and "waiting" mean. Codync already applies these in its
//! runner; this module is the shared vocabulary behind them.

use serde_json::{Value, json};

/// How a turn ended, from the outcome the runner already computed.
pub fn verdict(stopped: bool, failed: bool, waiting: bool) -> Value {
    let state = if waiting {
        "waiting"
    } else if stopped {
        "stopped"
    } else if failed {
        "failed"
    } else {
        "done"
    };
    json!({"state": state, "continueAllowed": waiting || stopped})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_end_has_one_verdict() {
        assert_eq!(verdict(false, false, false)["state"], "done");
        assert_eq!(verdict(true, false, false)["state"], "stopped");
        assert_eq!(verdict(false, true, false)["state"], "failed");
        assert_eq!(verdict(false, false, true)["state"], "waiting");
        assert_eq!(verdict(true, false, false)["continueAllowed"], true);
        assert_eq!(verdict(false, true, false)["continueAllowed"], false);
    }
}
