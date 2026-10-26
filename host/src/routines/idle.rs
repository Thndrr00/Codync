//! A routine whose runs keep doing nothing is only spending the user's quota. After `IDLE_LIMIT` runs in a row
//! that ran no tool that changed or ran anything, the routine is paused and the bot's chat says so.
//!
//! Idea from mercury-agent's `bot-manager.ts` (MIT); no code copied.

use super::{EntryKind, Hub, Lane, Routines};
use crate::LockExt;
use anyhow::{Context, Result};
use serde_json::json;

/// Runs in a row that did nothing before the routine pauses.
pub const IDLE_LIMIT: u32 = 3;

/// The routine's count of idle runs after one more run: it starts over when a run acted.
pub fn next_count(previous: u32, worked: bool) -> u32 {
    if worked { 0 } else { previous + 1 }
}

impl Routines {
    /// Turns off every routine of `bot` that's on, and says how many it turned off.
    pub fn pause_all(&self, store: &crate::store::Store, bot: &str) -> Result<usize> {
        let on: Vec<String> = self
            .state
            .locked()
            .routines
            .iter()
            .filter(|r| r.bot_id == bot && r.enabled && !r.deleted)
            .map(|r| r.id.clone())
            .collect();
        for id in &on {
            self.set_enabled(store, bot, id, false)?;
        }
        Ok(on.len())
    }

    /// Notes how a finished run of `run_id` went. `worked`: it ran a tool or said something. Pauses the routine on the
    /// run that reaches `IDLE_LIMIT`. Does nothing for a run or routine that's gone.
    pub fn note_outcome(&self, hub: &Hub, run_id: &str, worked: bool) -> Result<()> {
        let paused = self.change(&hub.store, |s| {
            let routine_id = s.runs.iter().find(|r| r.id == run_id).map(|r| r.routine_id.clone());
            let Some(routine) = routine_id.and_then(|id| s.routines.iter_mut().find(|r| r.id == id && !r.deleted))
            else {
                return Ok(None);
            };
            routine.idle_runs = next_count(routine.idle_runs, worked);
            Ok((routine.idle_runs >= IDLE_LIMIT && routine.enabled)
                .then(|| (routine.bot_id.clone(), routine.id.clone(), routine.name.clone())))
        })?;
        let Some((bot, id, name)) = paused else { return Ok(()) };
        self.set_enabled(&hub.store, &bot, &id, false).context("pausing an idle routine")?;
        hub.add_entry(
            &Lane::main(&bot),
            EntryKind::Notice,
            0,
            &json!({
                "text": format!("Paused routine \"{name}\": its last {IDLE_LIMIT} runs did nothing. Turn it back on when it has work to do."),
                "routineId": id,
                "style": "info",
            }),
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_runs_count_up_and_a_run_that_acted_starts_over() {
        assert_eq!(next_count(0, false), 1);
        assert_eq!(next_count(2, false), IDLE_LIMIT);
        assert_eq!(next_count(2, true), 0);
    }
}
