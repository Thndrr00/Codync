//! Counting failed runs in a row and pausing the routine at the limit (`guardrails::FAILURE_LIMIT`).
//! A success starts the count over; a run the person stopped is neither a success nor another failure.

use super::{State, Status, cancel_run, guardrails, now_ms};

/// Notes how the run at `run_index` ended. Returns the bot, routine and name to tell the person about when
/// that failure paused the routine.
pub fn note_outcome(s: &mut State, run_index: usize, status: Status) -> Option<(String, String, String)> {
    let routine_id = s.runs.get(run_index)?.routine_id.clone();
    let index = s.routines.iter().position(|routine| routine.id == routine_id && !routine.deleted)?;
    match status {
        Status::Succeeded => s.routines[index].failure_runs = 0,
        Status::Failed => {
            let failures = guardrails::next_failures(s.routines[index].failure_runs, true);
            s.routines[index].failure_runs = failures;
            if guardrails::reached_limit(failures) && s.routines[index].enabled {
                s.routines[index].enabled = false;
                s.routines[index].last_error =
                    Some(format!("Paused after {} failed runs in a row.", guardrails::FAILURE_LIMIT));
                s.routines[index].updated_at = now_ms();
                let paused = (s.runs[run_index].bot_id.clone(), routine_id.clone(), s.routines[index].name.clone());
                // Nothing waits on a routine that won't run: its queued work, retry and outputs stop here,
                // and its failure is for the person to look at.
                s.pending_flows.retain(|flow| flow.routine_id != routine_id);
                s.runs[run_index].retry_at = None;
                s.runs[run_index].retryable = false;
                for run in &mut s.runs {
                    if run.routine_id == routine_id && run.status == Status::Pending {
                        cancel_run(run);
                    }
                }
                return Some(paused);
            }
        }
        _ => {}
    }
    None
}
