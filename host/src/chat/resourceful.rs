//! Resourceful agents: one permission model, giving up needs evidence (agentes.chat ADR 0109).
//!
//! An agent that loses access to a tool or account does not give up on the work: it reports what
//! it needs, and only a repeated, evidenced failure ends the turn. This is the rule behind that
//! decision, using signals the host already has.

use serde_json::{Value, json};

/// Whether an agent should keep going after a failed step: yes unless the failure is both repeated
/// and evidenced. A person's stop always wins.
pub fn keep_going(failures: usize, evidenced: bool, user_stopped: bool) -> Value {
    let stop = user_stopped || (failures >= 3 && evidenced);
    json!({"keepGoing": !stop, "reason": if user_stopped { "user" } else if stop { "evidenced" } else { "retry" }})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_single_failure_is_retried_but_an_evidenced_loop_stops() {
        assert_eq!(keep_going(1, true, false)["keepGoing"], true);
        assert_eq!(keep_going(3, true, false)["keepGoing"], false);
        assert_eq!(keep_going(3, false, false)["keepGoing"], true);
        assert_eq!(keep_going(0, false, true)["reason"], "user");
    }
}
