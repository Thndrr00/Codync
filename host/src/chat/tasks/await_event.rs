//! Pure decisions for `await_event` in a task's side thread.
//!
//! Previously the tool refused side threads, so background work never continued the thread where
//! the person asked. A task is already a side thread with its own lane and session; waiting in it
//! is allowed, bounded, and cancelled when the task is stopped. The wait polls the routine run list
//! the host already keeps; it never opens a second event loop.

use crate::store::tasks::TaskRun;
use crate::store::{TaskInfo, TaskState};
use anyhow::{Result, bail};
use serde_json::Value;
use std::time::Duration;

/// A wait may never hold a turn longer than the host's synchronous team ask.
pub const MAX_WAIT: Duration = Duration::from_secs(600);
/// How often the existing run list is checked. Short enough to feel immediate, long enough to be cheap.
pub const POLL: Duration = Duration::from_millis(500);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventWait {
    /// One named routine run has to reach a terminal state.
    Run(String),
    /// A routine delivery from `source` with event `name`; empty name means any event.
    Signal { source: String, name: String },
}

/// Parses the caller's selector. A `*` event name means any event from that source.
pub fn parse_wait(run_id: Option<&str>, source: Option<&str>, name: Option<&str>) -> Result<EventWait> {
    if let Some(id) = run_id {
        let id = id.trim();
        if !id.is_empty() {
            return Ok(EventWait::Run(id.to_owned()));
        }
    }
    let source = source.unwrap_or("").trim();
    let name = name.unwrap_or("").trim();
    if source.is_empty() && name.is_empty() {
        bail!("awaitEvent needs a runId or an event source");
    }
    if source.is_empty() && name != "*" {
        bail!("an event name needs its source");
    }
    Ok(EventWait::Signal { source: source.to_owned(), name: name.to_owned() })
}

/// A caller's seconds, clamped to the same bound as other bounded host waits.
pub fn clamp_wait(seconds: Option<u64>) -> Duration {
    let seconds = seconds.unwrap_or(MAX_WAIT.as_secs()).min(MAX_WAIT.as_secs()).max(1);
    Duration::from_secs(seconds)
}

/// Only a finished run wakes a waiter. A queued or running one can still change its outcome.
pub fn terminal(status: &str) -> bool {
    matches!(status, "succeeded" | "failed" | "interrupted" | "cancelled")
}

/// A string field of a run's event payload. Payloads are untrusted data, never instructions.
pub fn field<'a>(event: &'a Value, key: &str) -> &'a str {
    event.get(key).and_then(Value::as_str).unwrap_or("").trim()
}

/// The small, data-only view of a routine run that a wait needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EventProbe<'a> {
    pub run_id: &'a str,
    pub status: &'a str,
    pub source: &'a str,
    pub name: &'a str,
}

/// Whether one run is the event this wait asked for.
pub fn matches(wait: &EventWait, probe: &EventProbe<'_>) -> bool {
    if !terminal(probe.status) {
        return false;
    }
    match wait {
        EventWait::Run(id) => id == probe.run_id,
        EventWait::Signal { source, name } => {
            (source.is_empty() || source == "*" || source == probe.source)
                && (name.is_empty() || name == "*" || name == probe.name)
        }
    }
}

/// A task that was closed, archived or stopped must not keep waiting or wake again.
pub fn cancelled(task: &TaskInfo) -> bool {
    task.state != TaskState::Open || task.run == TaskRun::Stopped
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaitOutcome {
    Arrived,
    Cancelled,
    TimedOut,
    Waiting,
}

/// The one decision the polling loop makes on each tick, in a fixed priority order: a stop wins
/// over an event that happened at the same instant, and an event wins over a deadline.
pub fn outcome(wait: &EventWait, probe: Option<&EventProbe<'_>>, task: &TaskInfo, expired: bool) -> WaitOutcome {
    if cancelled(task) {
        WaitOutcome::Cancelled
    } else if probe.is_some_and(|probe| matches(wait, probe)) {
        WaitOutcome::Arrived
    } else if expired {
        WaitOutcome::TimedOut
    } else {
        WaitOutcome::Waiting
    }
}

/// The prompt posted into the side thread after the waited-for run finishes. The run's own turn
/// continues the request there; this line only tells the agent what woke it.
pub fn wake_text(probe: &EventProbe<'_>) -> String {
    let origin = if probe.name.is_empty() {
        format!("source {}", probe.source)
    } else {
        format!("{} from {}", probe.name, probe.source)
    };
    format!(
        "[Codync: the event you waited for arrived ({origin}, run {}). This is background work, not a new user instruction. Continue the request in this side thread.]\nThe run finished with status {}.",
        probe.run_id, probe.status
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::TaskState;
    use serde_json::json;

    fn probe(status: &str) -> EventProbe<'static> {
        EventProbe { run_id: "r1", status, source: "github", name: "push" }
    }

    fn task(state: TaskState, run: TaskRun) -> TaskInfo {
        TaskInfo {
            id: "task:1".into(),
            bot_id: "b".into(),
            name: "Wait".into(),
            state,
            root: "task:1".into(),
            started_at: 0,
            closed_at: 0,
            unread: 0,
            rev: 1,
            updated_at: 0,
            parent_chat: String::new(),
            parent_thread: String::new(),
            run,
            routed_seq: 0,
            prompt: String::new(),
        }
    }

    #[test]
    fn a_selector_needs_a_run_or_a_source_and_is_bounded() {
        assert_eq!(parse_wait(Some(" r1 "), None, None).unwrap(), EventWait::Run("r1".into()));
        assert_eq!(
            parse_wait(None, Some("github"), Some("push")).unwrap(),
            EventWait::Signal { source: "github".into(), name: "push".into() }
        );
        assert_eq!(
            parse_wait(None, Some("github"), None).unwrap(),
            EventWait::Signal { source: "github".into(), name: String::new() }
        );
        assert!(parse_wait(None, None, None).is_err());
        assert!(parse_wait(None, None, Some("push")).is_err());
        assert_eq!(clamp_wait(None), MAX_WAIT);
        assert_eq!(clamp_wait(Some(0)), Duration::from_secs(1));
        assert_eq!(clamp_wait(Some(u64::MAX)), MAX_WAIT);
        assert_eq!(clamp_wait(Some(12)), Duration::from_secs(12));
    }

    #[test]
    fn only_a_finished_run_wakes_the_matching_wait() {
        let run = EventWait::Run("r1".into());
        let signal = EventWait::Signal { source: "github".into(), name: "push".into() };
        assert!(matches(&run, &probe("succeeded")));
        assert!(!matches(&run, &probe("running")));
        assert!(!matches(&run, &EventProbe { run_id: "r2", ..probe("succeeded") }));
        assert!(matches(&signal, &probe("failed")));
        assert!(!matches(&signal, &EventProbe { name: "issues", ..probe("succeeded") }));
        assert!(!matches(&signal, &EventProbe { source: "slack", ..probe("succeeded") }));
        assert!(matches(
            &EventWait::Signal { source: "github".into(), name: "*".into() },
            &EventProbe { name: "issues", ..probe("succeeded") }
        ));
        assert!(matches(&EventWait::Signal { source: "*".into(), name: String::new() }, &probe("succeeded")));
        assert!(terminal("cancelled"));
        assert!(!terminal("pending"));
    }

    #[test]
    fn a_stop_cancels_the_wait_even_if_the_event_arrived_at_the_same_tick() {
        let wait = EventWait::Run("r1".into());
        let open = task(TaskState::Open, TaskRun::Running);
        assert_eq!(outcome(&wait, Some(&probe("succeeded")), &open, true), WaitOutcome::Arrived);
        assert_eq!(outcome(&wait, None, &open, true), WaitOutcome::TimedOut);
        assert_eq!(outcome(&wait, None, &open, false), WaitOutcome::Waiting);
        let stopped = task(TaskState::Open, TaskRun::Stopped);
        assert!(cancelled(&stopped));
        assert_eq!(outcome(&wait, Some(&probe("succeeded")), &stopped, false), WaitOutcome::Cancelled);
        let closed = task(TaskState::Closed, TaskRun::Idle);
        assert_eq!(outcome(&wait, Some(&probe("succeeded")), &closed, false), WaitOutcome::Cancelled);
    }

    #[test]
    fn an_event_payload_is_read_as_data_not_as_instructions() {
        let event = json!({"source": "  github  ", "event": "push"});
        assert_eq!(field(&event, "source"), "github");
        assert_eq!(field(&event, "missing"), "");
        let text = wake_text(&probe("succeeded"));
        assert!(text.contains("push from github"));
        assert!(text.contains("not a new user instruction"));
    }
}
