//! Paused workflows, ported from OpenBot nightly's workflow pause rule.
//!
//! A paused workflow does not start from a schedule, webhook, chat mention or GitHub event, but a
//! person can still run it by hand, and a schedule occurrence that came due while it was paused is
//! skipped rather than run late. The canvas shows **Paused** with a Resume button. Duplicating a
//! workflow makes a paused copy, so the copy cannot fire from the same triggers as the original
//! before the person has changed it.
//!
//! The pause state lives in one small kv flag per routine, next to the flow's other kv data, so a
//! client that doesn't know the field simply keeps showing the routine as before. The decisions are
//! pure; the engine and the API call them before starting or changing a flow.

use crate::store::Store;
use anyhow::Result;
use serde_json::Value;

/// kv prefix of a paused workflow flag, followed by the routine id.
pub const PAUSED_PREFIX: &str = "routineflow.paused.";

/// Whether `routine` is paused. An absent flag (a workflow saved before pauses existed) is running.
pub fn is_paused(store: &Store, routine: &str) -> bool {
    store.kv_get(&format!("{PAUSED_PREFIX}{routine}")).as_deref() == Some("1")
}

/// Pauses or resumes `routine`. Resuming removes the flag rather than writing `0`, so a workflow
/// that was never paused reads exactly like one that was resumed.
pub fn set(store: &Store, routine: &str, paused: bool) -> Result<()> {
    let key = format!("{PAUSED_PREFIX}{routine}");
    if paused { store.kv_set(&key, "1") } else { store.kv_delete(&key) }
}

/// Whether a run may start. A manual run is always allowed; every automatic trigger is refused while
/// the workflow is paused.
pub fn automatic_start_allowed(paused: bool, manual: bool) -> bool {
    manual || !paused
}

/// A schedule occurrence that came due while the workflow was paused is skipped, not deferred.
pub fn occurrence_skipped(paused: bool) -> bool {
    paused
}

/// A duplicated workflow is a paused copy: its triggers must not start both workflows.
pub fn duplicate_copy_paused() -> bool {
    true
}

/// Whether a flow's start event is a person running the workflow now: a test flow or "run from this
/// step". Everything else (schedule, webhook, chat mention, GitHub event) is automatic.
pub fn manual_event(event: &Value) -> bool {
    matches!(event["source"].as_str(), Some("test" | "manual"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn only_automatic_starts_are_refused_while_paused() {
        assert!(automatic_start_allowed(false, false));
        assert!(automatic_start_allowed(true, true), "a person can always run a paused workflow");
        assert!(!automatic_start_allowed(true, false));
    }

    #[test]
    fn a_paused_schedule_skips_its_occurrence() {
        assert!(occurrence_skipped(true));
        assert!(!occurrence_skipped(false));
    }

    #[test]
    fn a_duplicate_starts_paused() {
        assert!(duplicate_copy_paused());
    }

    #[test]
    fn test_and_step_runs_are_manual_but_events_are_not() {
        assert!(manual_event(&json!({"source": "test"})));
        assert!(manual_event(&json!({"source": "manual"})));
        assert!(!manual_event(&json!({"source": "github", "event": "issues.opened"})));
        assert!(!manual_event(&Value::Null));
    }

    #[test]
    fn a_pause_flag_is_stored_and_removed() {
        let store = Store::open(std::path::Path::new(":memory:")).expect("a store");
        assert!(!is_paused(&store, "r"), "an old workflow is running");
        set(&store, "r", true).expect("a pause");
        assert!(is_paused(&store, "r"));
        set(&store, "r", false).expect("a resume");
        assert!(!is_paused(&store, "r"));
        assert!(store.kv_get(&format!("{PAUSED_PREFIX}r")).is_none(), "resuming drops the flag");
        assert!(!is_paused(&store, "other"), "a flag belongs to one routine");
    }
}
