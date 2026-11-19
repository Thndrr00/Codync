//! A check before each catch-up after a usage pause (agentes.chat ADR 0136).
//!
//! After a plan or budget pauses the computer, the catch-up that follows must first check that the
//! pause really ended — otherwise it replays work the person already saw while the computer slept.

use serde_json::{Value, json};

/// Whether a catch-up may proceed: only when there is no active pause.
pub fn may_catch_up(paused_until: Option<i64>, now: i64) -> Value {
    let clear = paused_until.is_none_or(|until| now >= until);
    json!({"allowed": clear, "reason": if clear { "none" } else { "usage_pause" }})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_catch_up_waits_for_the_pause_to_end() {
        assert_eq!(may_catch_up(None, 500)["allowed"], true);
        assert_eq!(may_catch_up(Some(1000), 999)["allowed"], false);
        assert_eq!(may_catch_up(Some(1000), 1000)["allowed"], true);
        assert_eq!(may_catch_up(Some(1000), 999)["reason"], "usage_pause");
    }
}
