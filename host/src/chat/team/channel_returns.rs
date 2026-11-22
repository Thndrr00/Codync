//! Results of channel (group) tasks coming back to the agent that posted the request
//! (OpenBot nightly `t3-cross-agent-channel-messages`).
//!
//! A member's final answer is delivered as a teammate reply in the asking agent's own chat, just
//! like a `message_bot` reply. A failed, stopped or cancelled task carries a note instead of a
//! result, so the agent can decide whether to retry rather than telling the user to follow the
//! channel. The decisions here are pure: what comes back, when, and with what text.

/// A channel task's terminal state, after the task's own bookkeeping has settled it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskState {
    Completed,
    Failed,
    Cancelled,
    Paused,
}

/// The state a stored channel task reports, when it is one that returns a result.
pub fn returned(state: &str) -> Option<TaskState> {
    match state {
        "completed" => Some(TaskState::Completed),
        "failed" => Some(TaskState::Failed),
        "cancelled" => Some(TaskState::Cancelled),
        "paused" => Some(TaskState::Paused),
        _ => None,
    }
}

/// A routing task the lead cancelled and handed to an open task has no result of its own: the open
/// task's id is now on the request message, and that task sends the result later.
pub fn moved_to_open_task(state: TaskState, owner_is_none: bool, request_task_id: Option<&str>) -> bool {
    state == TaskState::Cancelled && owner_is_none && request_task_id.is_some()
}

/// Whether a finished task's outcome goes back to the agent that asked. Only a root task with a
/// requester that isn't the member answering produces a reply; an agent never gets its own result.
pub fn should_return(state: TaskState, is_root: bool, requester: Option<&str>, member: &str) -> bool {
    is_root && requester.is_some_and(|r| r != member) && returned_for(state)
}

fn returned_for(state: TaskState) -> bool {
    matches!(state, TaskState::Completed | TaskState::Failed | TaskState::Cancelled | TaskState::Paused)
}

/// The header a channel result carries into the asking agent's chat.
pub fn header(channel_name: &str, channel_id: &str, member_name: &str) -> String {
    format!("Channel: {channel_name} ({channel_id}) · {member_name}")
}

/// The teammate-reply text for a task outcome. `answer` is the member's written result (completed
/// tasks only); `reason` is the task's error or stop note, already trimmed by the caller.
pub fn return_text(
    channel_name: &str,
    channel_id: &str,
    state: TaskState,
    answer: Option<&str>,
    reason: &str,
) -> String {
    let origin = format!("Channel: {channel_name} ({channel_id}). This answers your channel request.");
    let reason = reason.trim();
    let line = |prefix: &str| if reason.is_empty() { prefix.to_owned() } else { format!("{prefix} {reason}") };
    match state {
        TaskState::Completed => {
            let written = answer
                .map(str::trim)
                .filter(|a| !a.is_empty())
                .unwrap_or("The member finished without a written result.");
            format!("{origin}\n{written}")
        }
        TaskState::Paused => {
            format!(
                "{origin}\nStatus: paused\n{}\nNext: Do not send the request again. A person can continue the task in the channel, and its result then comes back here.",
                line("Result: Codync note, not written by this member: the channel task stopped before it finished.")
            )
        }
        TaskState::Failed => {
            format!(
                "{origin}\nStatus: blocked\n{}\nUnblock: Read the channel, then send the request again if you still need the result.",
                line("Result: Codync note, not written by this member: the channel task failed before it finished.")
            )
        }
        TaskState::Cancelled => {
            format!(
                "{origin}\nStatus: blocked\n{}\nUnblock: Read the channel, then send the request again if you still need the result.",
                line(
                    "Result: Codync note, not written by this member: the channel task was cancelled before it finished."
                )
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_terminal_root_tasks_with_a_requester_return() {
        assert_eq!(returned("completed"), Some(TaskState::Completed));
        assert_eq!(returned("failed"), Some(TaskState::Failed));
        assert_eq!(returned("cancelled"), Some(TaskState::Cancelled));
        assert_eq!(returned("paused"), Some(TaskState::Paused));
        assert_eq!(returned("working"), None);
        assert!(should_return(TaskState::Completed, true, Some("ada"), "grace"));
        assert!(!should_return(TaskState::Completed, false, Some("ada"), "grace"), "only root tasks return");
        assert!(!should_return(TaskState::Completed, true, None, "grace"), "no requester, no reply");
        assert!(!should_return(TaskState::Completed, true, Some("ada"), "ada"), "an agent doesn't get its own result");
    }

    #[test]
    fn a_routing_task_handed_to_an_open_task_skips_its_result() {
        assert!(moved_to_open_task(TaskState::Cancelled, true, Some("open-task")));
        assert!(!moved_to_open_task(TaskState::Cancelled, false, Some("open-task")));
        assert!(!moved_to_open_task(TaskState::Cancelled, true, None));
        assert!(!moved_to_open_task(TaskState::Failed, true, Some("open-task")));
    }

    #[test]
    fn a_completed_task_sends_the_members_written_answer() {
        let text = return_text("Design", "g1", TaskState::Completed, Some("  Shipped the mock.  "), "");
        assert!(text.contains("Channel: Design (g1)"));
        assert!(text.contains("Shipped the mock."));
        let empty = return_text("Design", "g1", TaskState::Completed, Some("  "), "");
        assert!(empty.contains("finished without a written result"));
    }

    #[test]
    fn failed_stopped_and_cancelled_tasks_send_notes_not_results() {
        for (state, word) in
            [(TaskState::Failed, "failed"), (TaskState::Cancelled, "was cancelled"), (TaskState::Paused, "stopped")]
        {
            let text = return_text("Ops", "g2", state, Some("a stale result"), "provider timed out");
            assert!(text.contains("Status:"), "{state:?}");
            assert!(text.contains(word), "{state:?}");
            assert!(text.contains("provider timed out"));
            assert!(!text.contains("a stale result"), "a note never carries a member's result");
        }
        let paused = return_text("Ops", "g2", TaskState::Paused, None, "");
        assert!(paused.contains("Do not send the request again"));
    }

    #[test]
    fn a_non_terminal_state_has_no_reply_at_all() {
        assert_eq!(returned("working"), None);
        assert_eq!(returned("queued"), None);
    }
}
