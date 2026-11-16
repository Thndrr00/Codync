//! The scheduler's side of the fixed routine safeguards: pause a routine whose saved interval is under
//! the floor, or the ones enabled beyond the computer's cap, and say so in its chat.

use super::{EntryKind, Hub, Lane, LockExt, Result, Routines, Status, cancel_run, guardrails, json, now_ms};

impl Routines {
    /// Applies the fixed safeguards to routines saved before they existed: an interval under 15 minutes
    /// and the enabled routines beyond the computer's cap are paused, each with a notice naming it and how
    /// to bring it back. Nothing is written when every routine already fits.
    pub fn enforce_limits(&self, hub: &Hub) -> Result<()> {
        let needed = {
            let state = self.state.locked();
            guardrails::over_cap(&state).first().is_some()
                || state.routines.iter().any(|routine| {
                    !routine.deleted && routine.enabled && guardrails::interval_below_floor(&routine.triggers).is_some()
                })
        };
        if !needed {
            return Ok(());
        }
        let paused = self.change(&hub.store, |s| {
            let mut paused: Vec<(String, String, String, String)> = Vec::new();
            let short: Vec<(usize, i64)> = s
                .routines
                .iter()
                .enumerate()
                .filter(|(_, routine)| !routine.deleted && routine.enabled)
                .filter_map(|(index, routine)| {
                    guardrails::interval_below_floor(&routine.triggers).map(|seconds| (index, seconds))
                })
                .collect();
            for (index, seconds) in short {
                let id = s.routines[index].id.clone();
                let bot = s.routines[index].bot_id.clone();
                let name = s.routines[index].name.clone();
                s.routines[index].enabled = false;
                s.routines[index].last_error = Some(format!(
                    "Its interval of {seconds} seconds is under the {} second minimum.",
                    guardrails::MIN_INTERVAL_SECONDS
                ));
                s.routines[index].updated_at = now_ms();
                s.pending_flows.retain(|flow| flow.routine_id != id);
                for run in &mut s.runs {
                    if run.routine_id == id && run.status == Status::Pending {
                        cancel_run(run);
                    }
                }
                paused.push((
                    bot,
                    id,
                    name,
                    format!(
                        "Codync's shortest interval is {} minutes. Change its schedule, then turn it back on.",
                        guardrails::MIN_INTERVAL_SECONDS / 60
                    ),
                ));
            }
            for id in guardrails::over_cap(s) {
                let Some(routine) = s.routines.iter_mut().find(|routine| routine.id == id && !routine.deleted) else {
                    continue;
                };
                routine.enabled = false;
                routine.last_error =
                    Some(format!("A computer keeps at most {} routines enabled.", guardrails::MAX_ENABLED));
                routine.updated_at = now_ms();
                let (bot, name) = (routine.bot_id.clone(), routine.name.clone());
                s.pending_flows.retain(|flow| flow.routine_id != id);
                for run in &mut s.runs {
                    if run.routine_id == id && run.status == Status::Pending {
                        cancel_run(run);
                    }
                }
                paused.push((
                    bot,
                    id,
                    name,
                    format!(
                        "A computer keeps at most {} routines enabled. Pause another, then turn this one back on.",
                        guardrails::MAX_ENABLED
                    ),
                ));
            }
            Ok(paused)
        })?;
        for (bot, id, name, why) in paused {
            hub.store.cancel_routine_outputs(&id)?;
            self.flows.cancel_routine(&id);
            hub.add_entry(
                &Lane::main(&bot),
                EntryKind::Notice,
                0,
                &json!({"text": format!("Paused routine \"{name}\": {why}"), "routineId": id, "style": "info"}),
            );
        }
        Ok(())
    }
}
