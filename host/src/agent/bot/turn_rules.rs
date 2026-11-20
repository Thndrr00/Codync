//! Pure decisions about when a turn ends, when a question stops holding the bot, and when silence needs a notice.
//!
//! The actor's steering, queue, question and turn-end paths all ask the same small questions. They live here, with
//! no I/O, so the rules can be reasoned about and tested on their own. Ported from OpenBot's nightly turn lifecycle
//! (triage-12470, -12968, -12975, -13050, -13142); the code is Codync's.

use super::Done;

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

/// The notice for a turn that did steps and said nothing. The work stays in the chat and the turn is not run again.
pub(super) const NO_ANSWER_AFTER_STEPS: &str =
    "The agent ended its turn without an answer. Check what it did before you send the same request again.";
/// The notice for a turn that did nothing and has no backup to hand the message to.
pub(super) const NO_ANSWER_EMPTY: &str = "The agent ended its turn with no answer.";

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
}
