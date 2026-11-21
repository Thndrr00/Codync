//! A bounded completion check after a turn that used tools. OpenBot nightly 0.74.0 finishes a request
//! without the person saying "continue": when a tool-using turn ends, the model compares the request with
//! the work it did and the answer it gave, and carries on with anything still missing. A question, an
//! approval or a permission card still waiting ends the chain at once. The check is another normal turn
//! on the same session, not a second agent loop.

use super::{Actor, Done};
use crate::hub::BotStatus;
use crate::store::Lane;
use tokio::sync::mpsc;

/// OpenBot nightly 0.74.0 (`CHANGELOG.md`): "the agent continues with that part, at most 3 times for one
/// request. A question that needs your choice or approval still stops the agent."
pub(super) const MAX_SELF_CHECKS: u32 = 3;

/// What the turn that just ended hands to the check that follows it.
pub(super) struct SelfCheck {
    /// The person's request, kept unchanged through the whole chain.
    request: String,
}

impl SelfCheck {
    /// One short prompt in the same session: compare, then finish any gap or say there is none.
    fn prompt(&self) -> String {
        format!(
            "[Codync: check your work] Compare the person's request below with what you did and the answer you gave. \
             If any part is still not done, finish that part now and then stop. If it is all done, reply exactly \
             \"(pass)\" and say nothing else.\n\nThe request was:\n{}",
            self.request
        )
    }
}

/// Whether a completion check may run now. Pure, so every stopping rule has a test without agent I/O.
pub(super) fn may_check(
    used_tools: bool,
    has_answer: bool,
    stopped: bool,
    pending_input: bool,
    checks_done: u32,
) -> bool {
    used_tools && has_answer && !stopped && !pending_input && checks_done < MAX_SELF_CHECKS
}

/// A short "continue" (OpenBot's "keep going"/"go on") is unfinished work, not a new request:
/// schedule this bounded check on the same session instead of starting another agent loop. The
/// phrase matcher lives in `plan_continuation` so its tests need no actor I/O.
pub(super) fn continuation_uses_self_check(text: &str, checks_done: u32) -> bool {
    super::plan_continuation::continues(text, checks_done)
}

impl Actor {
    /// Schedules the check for after this turn. The first check fixes the request for the whole chain, so every
    /// later check compares against what the person actually asked, not against a previous check's answer.
    pub(super) fn schedule_self_check(&mut self, request: String) {
        if self.self_checks == 0 {
            self.self_check_request = Some(request);
        }
        self.self_checks += 1;
        self.self_check = Some(SelfCheck { request: self.self_check_request.clone().unwrap_or_default() });
    }

    /// Runs a scheduled check as another hidden turn on the same session; the normal loop carries on from there.
    pub(super) async fn start_self_check(&mut self, check: SelfCheck, done_tx: &mpsc::UnboundedSender<Done>) {
        // Check again at the point of use: a Stop or a card that appeared since the turn ended wins.
        if self.stop_requested
            || !self.perms.is_empty()
            || self.hub.runtime(&self.cfg.id).status == BotStatus::NeedsInput
        {
            self.self_check = None;
            return;
        }
        let lane: Lane = self.lane.clone();
        let prompt = check.prompt();
        // A host-started turn: reported as such by the existing `turn_finished` event's `source` field,
        // and no second "Finished" alert now that the person already had the answer.
        self.active_prompt = true;
        // The check goes as it is, without the profile, memory notices or file reminders.
        self.raw_prompt = true;
        if let Err(error) = self.start_turn(lane, &[], &prompt, true, done_tx).await {
            // `start_turn` may have failed before it took the flags; don't leak them to the next turn.
            self.raw_prompt = false;
            self.start_failed(&error);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn at_most_three_checks_for_one_request() {
        for done in 0..MAX_SELF_CHECKS {
            assert!(may_check(true, true, false, false, done), "check {} should run", done + 1);
        }
        assert!(!may_check(true, true, false, false, MAX_SELF_CHECKS), "no fourth check");
        assert!(!may_check(true, true, false, false, MAX_SELF_CHECKS + 1));
    }

    #[test]
    fn a_pending_question_or_approval_stops_the_check() {
        assert!(!may_check(true, true, false, true, 0), "a question, approval or permission card waits for the person");
        assert!(!may_check(true, true, true, false, 0), "Stop ends the chain at once");
        assert!(!may_check(false, true, false, false, 0), "a turn that used no tools doesn't check");
        assert!(!may_check(true, false, false, false, 0), "a turn with no final answer doesn't check");
        assert!(may_check(true, true, false, false, 0));
    }

    #[test]
    fn a_short_continue_reuses_the_bounded_check() {
        assert!(continuation_uses_self_check("keep going", 0));
        assert!(continuation_uses_self_check("continue", MAX_SELF_CHECKS - 1));
        assert!(!continuation_uses_self_check("continue", MAX_SELF_CHECKS), "no fourth check");
        assert!(!continuation_uses_self_check("continue?", 0), "a question is not a continuation");
    }

    #[test]
    fn the_check_prompt_keeps_the_original_request() {
        let prompt = SelfCheck { request: "Fix the login bug".into() }.prompt();
        assert!(prompt.contains("Fix the login bug"));
        assert!(prompt.contains("(pass)"));
        assert!(prompt.len() < 600, "one short prompt");
    }
}
