//! Automatic notification deduplication for routine results (Idea 9).
//!
//! Bots can already call `mark_reported`/`was_reported` for explicit keys. This wraps
//! the routine publisher too: a result identical to one reported by the same routine
//! in the last day is kept in the run history but doesn't notify the person again.
//! Failures are never deduplicated — a repeated failure is still worth seeing.

use crate::hub::Hub;
use sha2::{Digest as _, Sha256};
use std::fmt::Write as _;

/// How long an identical result stays "already reported".
const WINDOW_HOURS: f64 = 24.0;

/// A stable key for one routine's result: routine id plus whitespace/case-normalised text.
pub(super) fn result_key(routine_id: &str, text: &str) -> String {
    let normalised: String = text.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase();
    let digest = Sha256::digest(format!("routine:{routine_id}\n{normalised}"));
    let mut key = String::from("routine:");
    for byte in digest.iter().take(12) {
        let _ = write!(key, "{byte:02x}");
    }
    key
}

/// True when the same result was already reported for this routine in the window.
/// A storage error fails open: the person gets the notification rather than silence.
pub(super) fn already_reported(hub: &Hub, bot_id: &str, routine_id: &str, text: &str) -> bool {
    let key = result_key(routine_id, text);
    hub.store
        .was_reported(bot_id, std::slice::from_ref(&key), Some(WINDOW_HOURS))
        .is_ok_and(|check| check["reported"].as_array().is_some_and(|reported| !reported.is_empty()))
}

/// Remembers a result so an identical one doesn't notify again before the window ends.
pub(super) fn mark_reported(hub: &Hub, bot_id: &str, routine_id: &str, text: &str) {
    let key = result_key(routine_id, text);
    let note = format!("routine {routine_id}");
    if let Err(error) = hub.store.mark_reported(bot_id, &key, Some(&note)) {
        tracing::warn!(bot = bot_id, error = format!("{error:#}"), "couldn't remember a routine result for dedup");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_result_has_one_key_across_whitespace_and_case_changes() {
        let a = result_key("r1", "Invoice 101 is pending\nReview it");
        let b = result_key("r1", "  invoice 101 is PENDING   review it ");
        assert_eq!(a, b);
        assert_ne!(a, result_key("r2", "Invoice 101 is pending Review it"));
        assert_ne!(a, result_key("r1", "Invoice 102 is pending Review it"));
        assert!(a.starts_with("routine:"));
        assert_eq!(a.len(), "routine:".len() + 24);
    }

    #[test]
    fn a_failure_key_differs_from_a_success_key() {
        assert_ne!(result_key("r1", "ok"), result_key("r1", "ok (retry)"));
    }
}
