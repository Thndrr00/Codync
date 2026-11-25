//! Pure decisions about when a turn ends, when a question stops holding the bot, and when silence needs a notice.
//!
//! The actor's steering, queue, question and turn-end paths all ask the same small questions. They live here, with
//! no I/O, so the rules can be reasoned about and tested on their own. Ported from OpenBot's nightly turn lifecycle
//! (triage-12470, -12968, -12975, -13050, -13142); the code is Codync's.

use super::Done;
use std::time::Duration;

/// A final answer ends the turn at once, even while a message steered into the turn is still waiting for the
/// agent's steering answer. The late answer leaves the message queued for the next turn instead of keeping the
/// working line and holding that next message (triage-12470).
pub(super) fn final_answer_ends_turn(done: &Done) -> bool {
    matches!(done, Ok(value) if value["stopReason"].as_str() == Some("end_turn"))
}

/// What the chat and the provider still know about a turn, for the check that releases a turn that stopped
/// everywhere else. "Interrupted counting" is this rule: a stopped turn only still counts as active while a
/// provider or a detached child is doing something for it (triage-12968).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct TurnPresence {
    /// The prompt response arrived but is held for a steering answer.
    pub ended: bool,
    /// The person, a deadline or an error stopped the turn.
    pub stopped: bool,
    /// The runtime still carries a start time, so a provider may be running the turn.
    pub started_at: Option<i64>,
    /// A process the agent started still runs detached under the turn.
    pub running_children: bool,
}

impl TurnPresence {
    /// A turn that ended or stopped, that no provider started and that left no child running only holds the queue:
    /// it is marked interrupted and the waiting messages start. A turn that is still starting stays, even when the
    /// host has not seen a `turn/started` for it yet.
    pub(super) fn stale(self) -> bool {
        (self.ended || self.stopped) && self.started_at.is_none() && !self.running_children
    }
}

/// The parts of an open question that decide whether it may be detached while an update or restart waits for the
/// provider to go idle (triage-12975).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Question {
    /// A choice card with fixed options: it still waits.
    pub choice: bool,
    /// A secret or Vault question: it still waits.
    pub secret: bool,
    /// A question inside a routine thread: it still waits.
    pub routine_thread: bool,
    /// The answer is being saved right now: it still waits.
    pub saving: bool,
}

/// An ordinary question stays open in the chat, the agent ends its turn, and a later answer arrives as a new
/// message. Choice cards, secret questions and routine-thread questions are not detachable.
pub(super) fn detachable_question(question: &Question) -> bool {
    !question.choice && !question.secret && !question.routine_thread && !question.saving
}

/// Whether a turn that started a long task (research, and anything else that does the work in the background)
/// is waiting for it (triage-13050).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct BackgroundTurn {
    pub started_task: bool,
    pub waiting: bool,
}

/// The task does the work and keeps its own result, so the turn ends instead of sending "Automatic continuation"
/// notes that make the agent redo the work in the chat.
pub(super) fn background_task_ends_turn(turn: &BackgroundTurn) -> bool {
    turn.started_task && turn.waiting
}

/// What to tell the user when a completed turn ends without any answer text (triage-13142).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NoAnswer {
    /// The turn did steps: show the "check what it did" notice and keep the work; the turn does not run again.
    CheckSteps,
    /// The turn did nothing and has a backup: let the backup take the message, quietly.
    BackupTakesOver,
    /// The turn did nothing and has no backup: say the agent ended the turn with no answer.
    EndedWithNoAnswer,
    /// The turn started background work and waits for it: say nothing, the agent answers when it ends.
    None,
}

pub(super) fn no_answer_action(steps: u32, background: bool, has_backup: bool) -> NoAnswer {
    if background {
        NoAnswer::None
    } else if steps > 0 {
        NoAnswer::CheckSteps
    } else if has_backup {
        NoAnswer::BackupTakesOver
    } else {
        NoAnswer::EndedWithNoAnswer
    }
}

/// When a message the person asked to send now reaches the actor (OpenBot v0.78.0).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SteerStage {
    /// No turn is running: the message waits for its own turn.
    BetweenTurns,
    /// The turn has started but its ACP session has not opened yet.
    Starting,
}

/// Where that message goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SteerTiming {
    /// It stays in the queue for its own turn.
    Wait,
    /// Hold it for this turn: hand it to the agent as soon as the session opens (v0.78.0).
    HoldForSession,
    /// Hand it to the agent now.
    Now,
}

/// A steer that arrives before its turn has a session must not wait for the *whole* turn: keep it for the
/// session, then join that turn (OpenBot v0.78.0).
pub(super) fn steer_during_start(stage: SteerStage, has_session: bool) -> SteerTiming {
    match (stage, has_session) {
        (SteerStage::BetweenTurns, _) => SteerTiming::Wait,
        (SteerStage::Starting, false) => SteerTiming::HoldForSession,
        (SteerStage::Starting, true) => SteerTiming::Now,
    }
}

/// OpenBot v0.78.0: Antigravity (and Devin, which has no Codync backend) only take a steer after their first
/// `session/update`; every other backend takes it at once. The caller retries briefly while it waits.
pub(super) fn steer_ready(backend: &str, first_update_seen: bool) -> bool {
    backend != "antigravity" || first_update_seen
}

/// A Stop the provider never carried out leaves `stop_requested` set. When the provider is still running, a
/// later message may steer into it and the stale flag is reconciled instead of blocking it (v0.78.0).
pub(super) fn steer_after_stop(stop_requested: bool, provider_still_running: bool) -> bool {
    stop_requested && provider_still_running
}

/// What to do with a tool call that has run past its bound (OpenBot nightly triage-15551).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum HungOutcome {
    /// Still inside the bound: the long tool may simply be doing its work.
    KeepWaiting,
    /// The bound passed: end the turn as stopped. Open cards are detached so the person can still answer them.
    End { detach_cards: bool },
}

/// Never end a turn before the bound, even when a card is open; at the bound free the queue and keep cards
/// answerable (triage-15551).
pub(super) fn hung_outcome(elapsed: Duration, limit: Duration, cards_open: bool) -> HungOutcome {
    if elapsed < limit { HungOutcome::KeepWaiting } else { HungOutcome::End { detach_cards: cards_open } }
}

/// OpenBot nightly triage-15555: a Claude process that dies while its subagents still run needs one recovery
/// turn, not an endless "waiting for background work" line. `announced` keeps that to once per user request.
pub(super) fn recovery_after_exit(children: bool, background: bool, announced: bool) -> bool {
    children && background && !announced
}

/// Whether a new message may interrupt the step that is running now (OpenBot nightly triage-15730). A
/// teammate's or a routine's message waits for the step to end; the person's own reaches a busy agent at once.
pub(super) fn interrupt_allowed(message_source: &str, is_user: bool, step_running: bool) -> bool {
    if !step_running {
        return true;
    }
    let automated = matches!(message_source, "routine" | "scheduled_task" | "bot" | "host");
    is_user && !automated
}

/// The notice for a turn that did steps and said nothing. The work stays in the chat and the turn is not run again.
pub(super) const NO_ANSWER_AFTER_STEPS: &str =
    "The agent ended its turn without an answer. Check what it did before you send the same request again.";
/// The notice for a turn that did nothing and has no backup to hand the message to.
pub(super) const NO_ANSWER_EMPTY: &str = "The agent ended its turn with no answer.";

/// What to do with a late message when the provider closed its session while the bot was busy
/// (OpenBot nightly 16361/16364). A message read by a busy Claude must go back into the queue when
/// that session closes, or it is lost; a reply the agent continued on its own (Kimi Code) still
/// reaches the chat instead of being mistaken for a lost message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LateMessage {
    /// Keep it in the queue for the next turn; nothing can deliver it now.
    Requeue,
    /// Put it in front so the next turn takes it (or let the agent's own reply through).
    Deliver,
}

pub(super) fn late_message_action(session_closed: bool, busy: bool) -> LateMessage {
    if session_closed && busy { LateMessage::Requeue } else { LateMessage::Deliver }
}

/// Whether a provider's repeated background-command answer is new to the chat (OpenBot nightly
/// 16149). Gemini can report the same answer twice; the second copy must not show again.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum BackgroundAnswer {
    Show,
    Skip,
}

pub(super) fn background_answer_seen(text: &str, seen: bool) -> BackgroundAnswer {
    if seen || text.trim().is_empty() { BackgroundAnswer::Skip } else { BackgroundAnswer::Show }
}

/// The plain words for a provider that stopped before it answered (OpenBot triage-15306). `reason`
/// may be empty; the user is told they can resend rather than being left with a silent turn.
pub(super) fn stopped_before_answer(reason: &str) -> String {
    let lead = "The agent stopped before it answered";
    let reason = reason.trim();
    let lead = if reason.is_empty() { lead.to_owned() } else { format!("{lead} ({reason})") };
    format!("{lead}. Send your message again if you still need an answer.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_final_answer_ends_the_turn_but_a_pause_does_not() {
        assert!(final_answer_ends_turn(&Ok(serde_json::json!({"stopReason": "end_turn"}))));
        assert!(!final_answer_ends_turn(&Ok(serde_json::json!({"stopReason": "max_turn_requests"}))));
        assert!(!final_answer_ends_turn(&Ok(serde_json::json!({"stopReason": "cancelled"}))));
        assert!(!final_answer_ends_turn(&Err(anyhow::anyhow!("gone"))));
        assert!(!final_answer_ends_turn(&Ok(serde_json::json!({}))));
    }

    #[test]
    fn interrupted_counting_only_releases_a_turn_that_stopped_everywhere() {
        let stopped = TurnPresence { ended: true, stopped: false, started_at: None, running_children: false };
        assert!(stopped.stale(), "an ended turn with no provider and no child holds only the queue");
        let stopped = TurnPresence { ended: false, stopped: true, started_at: None, running_children: false };
        assert!(stopped.stale(), "a turn stopped by the person is interrupted too");
        assert!(!TurnPresence { started_at: Some(1), ..stopped }.stale(), "a running provider keeps it");
        assert!(!TurnPresence { running_children: true, ..stopped }.stale(), "so does detached work");
        assert!(!TurnPresence { ended: false, stopped: false, started_at: None, running_children: false }.stale());
    }

    #[test]
    fn only_an_ordinary_question_can_be_detached_for_an_update() {
        let plain = Question { choice: false, secret: false, routine_thread: false, saving: false };
        assert!(detachable_question(&plain));
        assert!(!detachable_question(&Question { choice: true, ..plain }), "a choice card still waits");
        assert!(!detachable_question(&Question { secret: true, ..plain }), "a secret still waits");
        assert!(!detachable_question(&Question { routine_thread: true, ..plain }), "a routine thread still waits");
        assert!(!detachable_question(&Question { saving: true, ..plain }), "an answer being saved still waits");
    }

    #[test]
    fn a_background_task_ends_its_turn_without_a_continuation() {
        assert!(background_task_ends_turn(&BackgroundTurn { started_task: true, waiting: true }));
        assert!(!background_task_ends_turn(&BackgroundTurn { started_task: true, waiting: false }));
        assert!(!background_task_ends_turn(&BackgroundTurn { started_task: false, waiting: true }));
    }

    #[test]
    fn no_answer_notice_choice_depends_on_steps_background_and_backup() {
        assert_eq!(no_answer_action(3, false, false), NoAnswer::CheckSteps, "steps keep the work and show a notice");
        assert_eq!(no_answer_action(0, false, true), NoAnswer::BackupTakesOver, "a backup takes an empty turn");
        assert_eq!(no_answer_action(0, false, false), NoAnswer::EndedWithNoAnswer);
        assert_eq!(no_answer_action(3, true, false), NoAnswer::None, "background work waits for its own answer");
        assert_eq!(no_answer_action(0, true, true), NoAnswer::None);
    }

    #[test]
    fn a_steer_that_arrives_before_the_session_waits_for_the_session_not_the_turn() {
        assert_eq!(steer_during_start(SteerStage::BetweenTurns, false), SteerTiming::Wait);
        assert_eq!(steer_during_start(SteerStage::BetweenTurns, true), SteerTiming::Wait);
        assert_eq!(steer_during_start(SteerStage::Starting, false), SteerTiming::HoldForSession);
        assert_eq!(steer_during_start(SteerStage::Starting, true), SteerTiming::Now);
    }

    #[test]
    fn antigravity_needs_its_first_update_but_other_backends_do_not() {
        assert!(!steer_ready("antigravity", false));
        assert!(steer_ready("antigravity", true));
        for backend in ["claude", "grok", "codex", "pi", "gemini"] {
            assert!(steer_ready(backend, false), "{backend} takes a steer at once");
        }
    }

    #[test]
    fn only_a_still_running_provider_can_take_a_message_after_a_stop() {
        assert!(steer_after_stop(true, true));
        assert!(!steer_after_stop(true, false), "a provider that stopped cannot be steered");
        assert!(!steer_after_stop(false, true), "no stop to reconcile");
    }

    #[test]
    fn a_hung_tool_ends_the_turn_only_at_the_bound_and_keeps_cards_answerable() {
        let limit = Duration::from_mins(30);
        assert_eq!(hung_outcome(limit - Duration::from_millis(1), limit, true), HungOutcome::KeepWaiting);
        assert_eq!(hung_outcome(Duration::ZERO, limit, false), HungOutcome::KeepWaiting);
        assert_eq!(hung_outcome(limit, limit, false), HungOutcome::End { detach_cards: false });
        assert_eq!(hung_outcome(limit * 2, limit, true), HungOutcome::End { detach_cards: true });
    }

    #[test]
    fn a_dead_claude_with_live_children_gets_one_recovery_turn() {
        assert!(recovery_after_exit(true, true, false));
        assert!(!recovery_after_exit(false, true, false), "no live work to resume");
        assert!(!recovery_after_exit(true, false, false), "no background work to recover");
        assert!(!recovery_after_exit(true, true, true), "it is already announced");
    }

    #[test]
    fn a_busy_step_admits_only_the_persons_own_message() {
        assert!(interrupt_allowed("bot", false, false), "an idle step takes any message");
        assert!(interrupt_allowed("", true, true), "the person reaches a busy agent at once");
        assert!(interrupt_allowed("slack", true, true), "a person in a channel is still the person");
        assert!(!interrupt_allowed("bot", false, true), "a teammate waits");
        assert!(!interrupt_allowed("routine", false, true), "a routine waits");
        assert!(!interrupt_allowed("scheduled_task", true, true), "a scheduled message is not the person");
    }

    #[test]
    fn a_late_message_is_requeued_only_when_a_busy_turn_lost_its_session() {
        assert_eq!(late_message_action(true, true), LateMessage::Requeue, "the busy turn can no longer deliver it");
        assert_eq!(late_message_action(true, false), LateMessage::Deliver, "an idle bot takes the next turn");
        assert_eq!(late_message_action(false, true), LateMessage::Deliver, "a self-continued reply reaches the chat");
        assert_eq!(late_message_action(false, false), LateMessage::Deliver);
    }

    #[test]
    fn a_background_answer_is_shown_once() {
        assert_eq!(background_answer_seen("done", false), BackgroundAnswer::Show);
        assert_eq!(background_answer_seen("done", true), BackgroundAnswer::Skip);
        assert_eq!(background_answer_seen("  ", false), BackgroundAnswer::Skip, "no answer to show");
    }

    #[test]
    fn a_provider_that_stopped_says_so_and_offers_a_resend() {
        let plain = stopped_before_answer("");
        assert!(plain.contains("stopped before it answered") && plain.contains("Send your message again"));
        assert!(!plain.contains("()"), "no empty parentheses");
        let named = stopped_before_answer(" session closed ");
        assert!(named.contains("(session closed)"), "the reason is kept when there is one");
    }
}
