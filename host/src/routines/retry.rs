//! Trying failed runs again. A run that failed before its agent used a tool that may change
//! something (a rate limit, an agent that didn't start, a session it couldn't reload) has done
//! nothing yet, so it runs again 1, 5 and 15 minutes later. One that acted may have done part of
//! its work (sent the mail, then failed), and one that hit its time limit could hold the bot that
//! long again: only the user retries those, from the failure notice.
use super::{
    Context, Hub, Refused, Result, Routines, Run, State, Status, Value, bail, busy, cancel_run, json, may_run, new_run,
    notice_id, result_notice_id,
};

/// Seconds before each automatic retry; a run that fails after the last one waits for the user.
const DELAYS: [i64; 3] = [60, 300, 900];
/// A routine that failed before its first step gets exactly one more try, a few minutes later.
/// Ported from OpenBot nightly's "retry a routine that failed to start once" (a provider that
/// wasn't up, or a setup that timed out): no step of the first try could have done any work.
const START_RETRY_MS: i64 = 5 * 60 * 1000;

/// Whether a failed run is a transient start failure that gets one more try. `started` says its
/// first step began (`Run::started_at`), `acted` says a tool changed something, and `attempt` is
/// how many tries already failed. Only the very first try, which reached no step and acted on
/// nothing, is retried; a run that began a step may have done work, and the one automatic retry
/// itself never queues another. Pure so both decisions are testable without a host.
pub(crate) fn retry_start(attempt: u32, started: bool, acted: bool) -> bool {
    attempt == 0 && !started && !acted
}

/// Whether a failed run is safe to retry on the 1/5/15-minute ladder. A quiet run posts no start
/// notice, so `acted()` can't read its tool steps; use the stored step status (`started_at`) instead —
/// a step that began may have changed something, so leave that run for the user. Pure so the rule has
/// a test without a store or a host.
pub(crate) fn retry_ladder_safe(quiet: bool, started: bool, acted: bool, timed_out: bool) -> bool {
    !timed_out && !acted && !(quiet && started)
}

/// Queues that one retry on the same `not_before` delay path as an ordinary failure. The queued run
/// is marked past the retry ladder so that, if it also fails to start, it waits for the user rather
/// than looping. Returns earlier failures that no longer offer a retry.
pub fn after_start_failure(s: &mut State, failed: usize, now: i64) -> Vec<String> {
    let (routine_id, event) = {
        let run = &s.runs[failed];
        (run.routine_id.clone(), run.event.clone())
    };
    let Some(routine) = s.routines.iter().find(|r| r.id == routine_id && !r.deleted) else {
        return Vec::new();
    };
    if !may_run(routine, &event) {
        // A paused routine (unless this was a test run) waits for the user, like an ordinary failure.
        let withdrawn = forget(s, |r| r.routine_id == routine_id);
        s.runs[failed].retryable = true;
        return withdrawn;
    }
    // Past the ladder: this is the only automatic try, whatever happens to it.
    let attempt = DELAYS.len() as u32;
    let scheduled = event["source"] == "schedule";
    let wake = s
        .runs
        .iter()
        .position(|r| scheduled && r.routine_id == routine_id && r.status == Status::Pending && r.event == event);
    let at = if let Some(wake) = wake {
        s.runs[wake].attempt = attempt;
        s.runs[wake].not_before.unwrap_or(now)
    } else {
        let mut next = new_run(routine, event.clone(), None);
        next.attempt = attempt;
        next.not_before = Some(now + START_RETRY_MS);
        s.runs.push(next);
        now + START_RETRY_MS
    };
    s.runs[failed].retry_at = Some(at);
    Vec::new()
}

/// Whether run `id` used a tool that may have changed something. A run that never began didn't;
/// one whose check fails counts as acted.
pub fn acted(hub: &Hub, id: &str) -> bool {
    let Some(notice) = hub.store.entry(&notice_id(id)) else { return false };
    hub.store.acted_in_turn(&notice.bot_id, notice.turn).unwrap_or_else(|error| {
        tracing::warn!(run = id, error = format!("{error:#}"), "couldn't check what a failed run did; not retrying it");
        true
    })
}

/// What follows the run at `failed`, which just failed: a retry queued for later when running it
/// again is `safe`, or the run kept for the user to retry. Returns the earlier failures that no
/// longer offer a retry.
pub fn after_failure(s: &mut State, failed: usize, safe: bool, now: i64) -> Vec<String> {
    let run = &s.runs[failed];
    let Some(routine) = s.routines.iter().find(|r| r.id == run.routine_id && !r.deleted) else {
        return Vec::new();
    };
    if let Some(delay) = DELAYS.get(run.attempt as usize).filter(|_| safe && may_run(routine, &run.event)) {
        let attempt = run.attempt + 1;
        // A scheduled fire already queued does the same work: it is the retry (as in `schedule_due`).
        let scheduled = run.event["source"] == "schedule";
        let wake = s.runs.iter().position(|r| {
            scheduled && r.routine_id == routine.id && r.status == Status::Pending && r.event == run.event
        });
        let at = if let Some(wake) = wake {
            s.runs[wake].attempt = attempt;
            s.runs[wake].not_before.unwrap_or(now)
        } else {
            let mut next = new_run(routine, run.event.clone(), None);
            next.attempt = attempt;
            next.not_before = Some(now + delay * 1000);
            s.runs.push(next);
            now + delay * 1000
        };
        s.runs[failed].retry_at = Some(at);
        return Vec::new();
    }
    // Only a routine's latest failure keeps its event, so failed webhook payloads can't pile up.
    let routine_id = routine.id.clone();
    let withdrawn = forget(s, |r| r.routine_id == routine_id);
    s.runs[failed].retryable = true;
    withdrawn
}

/// Takes the retry off the failed runs `matches` picks, dropping the events they kept; returns their ids.
pub fn forget(s: &mut State, matches: impl Fn(&Run) -> bool) -> Vec<String> {
    let mut forgotten = Vec::new();
    for r in s.runs.iter_mut().filter(|r| r.retryable && matches(r)) {
        r.retryable = false;
        r.event = Value::Null;
        forgotten.push(r.id.clone());
    }
    forgotten
}

/// The note on a failure that is tried again automatically, `wait_ms` after it failed.
pub fn retry_note(wait_ms: i64) -> String {
    match (wait_ms + 30_000) / 60_000 {
        ..=0 => "Trying again now.".into(),
        1 => "Trying again in 1 minute.".into(),
        minutes => format!("Trying again in {minutes} minutes."),
    }
}

/// Takes the retry button off these runs' failure notices.
pub fn withdraw(hub: &Hub, runs: &[String]) {
    for id in runs {
        let notice = result_notice_id(id);
        if let Some(mut e) = hub.store.entry(&notice).filter(|e| e.data["retry"] == true) {
            e.data["retry"] = false.into();
            hub.set_entry(&notice, &e.data);
        }
    }
}

/// Ends a failed run's failure notice: no retry, no automatic try queued, and the text no longer
/// promises one. The run's messages stay.
pub fn end(hub: &Hub, run_id: &str, retry_at: Option<i64>, finished_at: Option<i64>) {
    let notice = result_notice_id(run_id);
    let Some(mut e) = hub.store.entry(&notice) else { return };
    if let (Some(at), Some(done)) = (retry_at, finished_at) {
        let promise = format!(". {}", retry_note(at - done));
        if let Some(base) = e.data["text"].as_str().and_then(|text| text.strip_suffix(&promise)).map(str::to_owned) {
            e.data["text"] = base.into();
        }
    }
    e.data["retry"] = false.into();
    e.data["cancelled"] = true.into();
    hub.set_entry(&notice, &e.data);
}

impl Routines {
    /// `retryRoutineRun`: does a failed run's work again, with the event it had.
    pub fn retry(&self, hub: &Hub, bot: &str, run_id: &str) -> Result<Value> {
        let run = self.change(&hub.store, |s| {
            let old = s
                .runs
                .iter()
                .position(|r| r.id == run_id && r.bot_id == bot && r.retryable)
                .context("this run can't be retried anymore")?;
            let routine = s
                .routines
                .iter()
                .find(|r| r.id == s.runs[old].routine_id && !r.deleted)
                .context("routine not found")?;
            if !may_run(routine, &s.runs[old].event) {
                bail!(Refused::Paused);
            }
            if busy(s, &routine.id) {
                bail!(Refused::Busy);
            }
            let run = new_run(routine, std::mem::take(&mut s.runs[old].event), None);
            s.runs[old].retryable = false;
            s.runs.push(run.clone());
            Ok(run)
        })?;
        withdraw(hub, &[run_id.to_owned()]);
        Ok(json!({"run": run}))
    }

    /// `cancelTask` on a failure notice: ends the run there. A retry already queued for later is
    /// cancelled with it, and the event the run kept goes.
    pub fn cancel(&self, hub: &Hub, bot: &str, run_id: &str) -> Result<Value> {
        let (retry_at, finished_at) = self.change(&hub.store, |s| {
            let i = s.runs.iter().position(|r| r.id == run_id && r.bot_id == bot).context("unknown routine run")?;
            if !matches!(s.runs[i].status, Status::Failed | Status::Interrupted) {
                bail!("This run is still going; stop it instead");
            }
            if s.runs[i].retry_at.is_none() && !s.runs[i].retryable {
                bail!("This run already ended");
            }
            let (attempt, routine, retry_at, finished_at) =
                (s.runs[i].attempt, s.runs[i].routine_id.clone(), s.runs[i].retry_at.take(), s.runs[i].finished_at);
            if let Some(at) = retry_at
                && let Some(next) = s.runs.iter_mut().find(|r| {
                    r.routine_id == routine
                        && r.status == Status::Pending
                        && r.attempt == attempt + 1
                        && r.not_before == Some(at)
                })
            {
                cancel_run(next);
            }
            forget(s, |r| r.id == run_id);
            Ok((retry_at, finished_at))
        })?;
        end(hub, run_id, retry_at, finished_at);
        Ok(json!({}))
    }
}

#[cfg(test)]
mod tests {
    use super::super::{EntryKind, Lane, LockExt, Store, tests::routine};
    use super::*;
    use crate::remote::identity::Identity;

    fn failed(attempt: u32) -> (State, usize) {
        let r = routine();
        let mut run: Run = new_run(&r, json!({"source":"schedule"}), None);
        run.status = Status::Failed;
        run.finished_at = Some(0);
        run.attempt = attempt;
        (State { routines: vec![r], runs: vec![run], ..State::default() }, 0)
    }

    #[test]
    fn a_run_that_did_nothing_is_retried_later_with_its_event() {
        let (mut s, i) = failed(0);
        assert_eq!(after_failure(&mut s, i, true, 1_000), Vec::<String>::new());
        assert_eq!(s.runs[0].retry_at, Some(61_000));
        let next = &s.runs[1];
        assert_eq!(next.status, Status::Pending);
        assert_eq!((next.attempt, next.not_before), (1, Some(61_000)));
        assert_eq!(next.event["source"], "schedule");
        assert_eq!(retry_note(60_000), "Trying again in 1 minute.");
        assert_eq!(retry_note(900_000), "Trying again in 15 minutes.");
    }

    #[test]
    fn only_a_first_try_that_reached_no_step_is_retried_on_start_failure() {
        assert!(retry_start(0, false, false), "a provider not up before the first step is transient");
        assert!(!retry_start(0, true, false), "a step began, so it may have done work");
        assert!(!retry_start(0, false, true), "a tool acted, so work may have been done");
        assert!(!retry_start(1, false, false), "the one automatic retry is not retried again");
    }

    #[test]
    fn a_quiet_run_that_began_a_step_waits_for_the_user_instead_of_the_retry_ladder() {
        // An ordinary run with read-only steps may still be retried (that starts the ladder).
        assert!(retry_ladder_safe(false, true, false, false));
        // A quiet run has no notice for `acted()` to read: a stored step means it may have acted.
        assert!(!retry_ladder_safe(true, true, false, false), "a started quiet run is not auto-retried");
        assert!(retry_ladder_safe(true, false, false, false), "a quiet run that reached no step may be retried");
        assert!(!retry_ladder_safe(true, false, true, false), "a tool that acted is never retried");
        assert!(!retry_ladder_safe(true, false, false, true), "a timeout is never retried on the ladder");
    }

    #[test]
    fn a_failure_before_the_first_step_gets_one_retry_five_minutes_later() {
        let (mut s, i) = failed(0);
        assert_eq!(after_start_failure(&mut s, i, 1_000), Vec::<String>::new());
        assert_eq!(s.runs[0].retry_at, Some(301_000));
        let next = &s.runs[1];
        assert_eq!(next.status, Status::Pending);
        assert_eq!((next.attempt, next.not_before), (3, Some(301_000)));
        assert_eq!(next.event["source"], "schedule", "the retry keeps the event");
        assert_eq!(retry_note(START_RETRY_MS), "Trying again in 5 minutes.");
        assert!(!retry_start(next.attempt, false, false), "a failed retry waits for the user");
    }

    #[test]
    fn a_queued_scheduled_fire_stands_in_as_the_start_retry() {
        let (mut s, i) = failed(0);
        let wake = new_run(&s.routines[0], json!({"source":"schedule"}), None);
        s.runs.push(wake);
        after_start_failure(&mut s, i, 1_000);
        assert_eq!(s.runs.len(), 2, "no second run of the same work");
        assert_eq!(s.runs[1].attempt, DELAYS.len() as u32);
        assert_eq!(s.runs[0].retry_at, Some(1_000));
        assert_eq!(retry_note(0), "Trying again now.");
    }

    #[test]
    fn a_paused_routine_waits_for_the_user_after_a_start_failure() {
        let (mut s, i) = failed(0);
        s.routines[0].enabled = false;
        after_start_failure(&mut s, i, 0);
        assert_eq!(s.runs.len(), 1, "no automatic retry while paused");
        assert!(s.runs[0].retryable && s.runs[0].retry_at.is_none());
    }

    #[test]
    fn a_scheduled_fire_already_queued_is_the_retry() {
        let (mut s, i) = failed(0);
        let wake = new_run(&s.routines[0], json!({"source":"schedule"}), None);
        s.runs.push(wake);
        after_failure(&mut s, i, true, 1_000);
        assert_eq!(s.runs.len(), 2, "no second run of the same work");
        assert_eq!((s.runs[1].attempt, s.runs[0].retry_at), (1, Some(1_000)));
        assert_eq!(retry_note(0), "Trying again now.");
    }

    #[test]
    fn a_run_that_acted_or_ran_out_of_retries_waits_for_the_user() {
        for (attempt, safe) in [(0, false), (3, true)] {
            let (mut s, i) = failed(attempt);
            after_failure(&mut s, i, safe, 0);
            assert_eq!(s.runs.len(), 1, "no automatic retry");
            assert!(s.runs[0].retryable && s.runs[0].retry_at.is_none());
        }
    }

    #[test]
    fn a_paused_routine_is_not_retried_automatically() {
        let (mut s, i) = failed(0);
        s.routines[0].enabled = false;
        after_failure(&mut s, i, true, 0);
        assert_eq!(s.runs.len(), 1);
        assert!(s.runs[0].retryable);
    }

    #[test]
    fn only_the_latest_failure_keeps_its_event() {
        let (mut s, _) = failed(3);
        after_failure(&mut s, 0, false, 0);
        let mut second = s.runs[0].clone();
        second.id = "second".into();
        second.retryable = false;
        s.runs.push(second);
        assert_eq!(after_failure(&mut s, 1, false, 0), vec![s.runs[0].id.clone()]);
        assert!(!s.runs[0].retryable && s.runs[0].event.is_null());
        assert!(s.runs[1].retryable && !s.runs[1].event.is_null());
    }

    #[test]
    fn a_user_retry_needs_a_free_resumed_routine_and_deleting_it_withdraws_the_retry() {
        let dir = std::env::temp_dir().join(format!("codync-retry-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let hub = Hub::new(store, "retry-test".into(), Identity::load_or_create(&dir).unwrap(), "test".into(), 0);
        let (mut s, i) = failed(3);
        after_failure(&mut s, i, false, 0);
        let id = s.runs[i].id.clone();
        let mut running = new_run(&s.routines[0], Value::Null, None);
        running.status = Status::Running;
        s.runs.push(running);
        hub.routines
            .change(&hub.store, |state| {
                *state = s;
                Ok(())
            })
            .unwrap();
        let refused = |e: anyhow::Error| e.downcast::<Refused>().unwrap();
        assert_eq!(refused(hub.routines.retry(&hub, "bot", &id).unwrap_err()), Refused::Busy);
        hub.routines
            .change(&hub.store, |s| {
                s.runs[1].status = Status::Succeeded;
                s.routines[0].enabled = false;
                Ok(())
            })
            .unwrap();
        assert_eq!(refused(hub.routines.retry(&hub, "bot", &id).unwrap_err()), Refused::Paused);

        let notice = result_notice_id(&id);
        let data = json!({"text": "failed", "retry": true});
        hub.add_entry_once(&notice, &Lane::main("bot"), EntryKind::Notice, 0, &data).unwrap();
        assert!(hub.routines.remove(&hub, "other", "routine").is_err(), "another bot can't delete it");
        hub.routines.remove(&hub, "bot", "routine").unwrap();
        let state = hub.routines.state.locked();
        assert!(!state.runs[i].retryable && state.runs[i].event.is_null(), "the payload goes with the routine");
        assert_eq!(hub.store.entry(&notice).unwrap().data["retry"], false, "and so does the retry button");
    }

    fn with_state(runs: State) -> (std::sync::Arc<Hub>, String) {
        let dir = std::env::temp_dir().join(format!("codync-cancel-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let hub = Hub::new(store, "cancel-test".into(), Identity::load_or_create(&dir).unwrap(), "test".into(), 0);
        let id = runs.runs.iter().find(|r| r.status == Status::Failed).unwrap().id.clone();
        hub.routines
            .change(&hub.store, |state| {
                *state = runs;
                Ok(())
            })
            .unwrap();
        (hub, id)
    }

    fn notice(hub: &Hub, id: &str, text: &str, retry: bool) {
        let data = json!({"text": text, "retry": retry, "runId": id, "source": "scheduled_task"});
        hub.add_entry_once(&result_notice_id(id), &Lane::main("bot"), EntryKind::Notice, 0, &data).unwrap();
    }

    #[test]
    fn a_user_cancel_ends_a_failed_run_and_the_retry_queued_for_later() {
        let (mut s, i) = failed(0);
        after_failure(&mut s, i, true, 0);
        let (hub, id) = with_state(s);
        notice(&hub, &id, "Routine could not finish: boom. Trying again in 1 minute.", false);
        hub.routines.cancel(&hub, "bot", &id).unwrap();
        let state = hub.routines.state.locked();
        assert_eq!(state.runs[0].retry_at, None);
        assert_eq!(state.runs[1].status, Status::Cancelled, "the queued try stops too");
        assert!(state.runs[1].event.is_null(), "and its copy of the event goes");
        drop(state);
        let entry = hub.store.entry(&result_notice_id(&id)).unwrap();
        assert_eq!(entry.data["cancelled"], true);
        assert_eq!(entry.data["retry"], false);
        assert_eq!(entry.data["text"], "Routine could not finish: boom", "the promise of a retry goes");
        assert!(hub.routines.cancel(&hub, "bot", &id).is_err(), "an ended run can't be ended twice");
    }

    #[test]
    fn a_user_cancel_ends_a_run_that_waited_for_the_user() {
        let (mut s, i) = failed(3);
        after_failure(&mut s, i, false, 0);
        let (hub, id) = with_state(s);
        notice(&hub, &id, "Routine could not finish: boom.", true);
        hub.routines.cancel(&hub, "other", &id).unwrap_err();
        hub.routines.cancel(&hub, "bot", &id).unwrap();
        let state = hub.routines.state.locked();
        assert!(!state.runs[0].retryable);
        assert_eq!(state.runs[0].event, Value::Null);
        drop(state);
        assert_eq!(hub.store.entry(&result_notice_id(&id)).unwrap().data["cancelled"], true);
        assert!(hub.routines.retry(&hub, "bot", &id).is_err(), "a cancelled run is not retried");
    }
}
