//! The fixed safeguards around routines: a 15-minute floor for interval routines, at most 20
//! enabled routines on a computer, and pausing one after 10 failures in a row.
//!
//! They are not settings: there is nothing to raise, so a routine that can't run within them is
//! better paused than left spending the person's quota. The scheduler applies them to routines saved
//! before they existed too.
//!
//! The same limits are in `CopilotKit/OpenBot`'s automation server (Apache-2.0).

use super::{Routine, State, Trigger};
use anyhow::{Result, bail};

/// The shortest interval a routine may have.
pub const MIN_INTERVAL_SECONDS: i64 = 15 * 60;
/// How many routines one computer may keep enabled.
pub const MAX_ENABLED: usize = 20;
/// Failed runs in a row before a routine pauses.
pub const FAILURE_LIMIT: u32 = 10;

/// The first interval trigger under the floor, and its length in seconds.
pub fn interval_below_floor(triggers: &[Trigger]) -> Option<i64> {
    triggers.iter().find_map(|trigger| match trigger {
        Trigger::Interval { seconds } if *seconds < MIN_INTERVAL_SECONDS => Some(*seconds),
        _ => None,
    })
}

/// Refuses interval triggers under the floor, naming the shortest one.
pub fn check_interval(triggers: &[Trigger]) -> Result<()> {
    match interval_below_floor(triggers) {
        Some(seconds) => bail!(
            "An interval routine's shortest interval is 15 minutes ({MIN_INTERVAL_SECONDS} seconds); {seconds} is shorter. Use a cron schedule for more frequent checks."
        ),
        None => Ok(()),
    }
}

/// Refuses one more enabled routine than the computer may keep. `except` is the routine being enabled or
/// saved, which doesn't count twice.
pub fn check_enabled_cap(state: &State, except: Option<&str>) -> Result<()> {
    let enabled = state
        .routines
        .iter()
        .filter(|routine| !routine.deleted && routine.enabled && Some(routine.id.as_str()) != except)
        .count();
    if enabled >= MAX_ENABLED {
        bail!("A computer keeps at most {MAX_ENABLED} routines enabled; pause one before turning on another.");
    }
    Ok(())
}

/// The enabled routines beyond the cap, oldest kept, as ids to pause. Runs when routines saved before the
/// cap leave more on than it allows.
pub fn over_cap(state: &State) -> Vec<String> {
    let mut enabled: Vec<&Routine> =
        state.routines.iter().filter(|routine| !routine.deleted && routine.enabled).collect();
    if enabled.len() <= MAX_ENABLED {
        return Vec::new();
    }
    enabled.sort_by(|a, b| (a.created_at, &a.id).cmp(&(b.created_at, &b.id)));
    enabled.into_iter().skip(MAX_ENABLED).map(|routine| routine.id.clone()).collect()
}

/// The count of failed runs in a row after `failed`: a success starts over.
pub fn next_failures(previous: u32, failed: bool) -> u32 {
    if failed { previous.saturating_add(1) } else { 0 }
}

/// Whether `failures` failed runs in a row are enough to pause the routine.
pub fn reached_limit(failures: u32) -> bool {
    failures >= FAILURE_LIMIT
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routines::tests::routine;

    #[test]
    fn intervals_under_fifteen_minutes_are_refused() {
        assert!(check_interval(&[Trigger::Interval { seconds: 899 }]).is_err());
        assert!(check_interval(&[Trigger::Interval { seconds: MIN_INTERVAL_SECONDS }]).is_ok());
        assert!(check_interval(&[Trigger::Webhook, Trigger::Interval { seconds: 60 }]).is_err());
        assert!(check_interval(&[Trigger::Cron { expression: "*/5 * * * *".into(), time_zone: "UTC".into() }]).is_ok());
    }

    #[test]
    fn a_computer_keeps_at_most_twenty_enabled_routines() {
        let mut state = State::default();
        for i in 0..MAX_ENABLED {
            let mut r = routine();
            r.id = format!("routine-{i:02}");
            r.created_at = i as i64;
            state.routines.push(r);
        }
        assert!(check_enabled_cap(&state, None).is_err(), "the twentieth already fills the computer");
        assert!(check_enabled_cap(&state, Some("routine-00")).is_ok(), "the one being enabled doesn't count");
        let mut paused = routine();
        paused.id = "paused".into();
        paused.enabled = false;
        state.routines.push(paused);
        assert!(check_enabled_cap(&state, Some("paused")).is_err());
        assert!(check_enabled_cap(&state, Some("routine-00")).is_ok());

        // Routines enabled before the cap: the oldest twenty stay, the rest are named.
        let mut over = State::default();
        for i in 0..(MAX_ENABLED + 2) {
            let mut r = routine();
            r.id = format!("routine-{i:02}");
            r.created_at = i as i64;
            over.routines.push(r);
        }
        assert_eq!(over_cap(&over), ["routine-20", "routine-21"]);
    }

    #[test]
    fn ten_failures_in_a_row_reach_the_limit_and_a_success_starts_over() {
        let mut failures = 0;
        for _ in 0..FAILURE_LIMIT {
            failures = next_failures(failures, true);
        }
        assert!(reached_limit(failures));
        assert_eq!(next_failures(failures, false), 0);
        assert!(!reached_limit(FAILURE_LIMIT - 1));
    }
}
