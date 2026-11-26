#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from hermes-agent's `review_idle_queue.py`: idle deferral for background
//! reviews, session coalescing, age-out and dispatch gating. Composes helpers already ported into
//! this host. Only `botInsights` calls this.

use std::collections::BTreeMap;

/// Quiet window before a machine counts as idle, in seconds (_IDLE_SETTLE_S).
pub const IDLE_SETTLE_S: f64 = 15.0;

/// Poll cadence while the queue is non-empty, in seconds (_POLL_INTERVAL_S).
pub const POLL_INTERVAL_S: f64 = 5.0;

/// Age after which a review dispatches regardless of idleness (_MAX_AGE_DEFAULT_S).
pub const MAX_AGE_DEFAULT_S: f64 = 1_800.0;

/// Defer mode from config: `auto` (default) or `never` (deferMode).
pub fn defer_mode(raw: Option<&str>) -> &'static str {
    match raw.unwrap_or("auto").trim().to_ascii_lowercase().as_str() {
        "never" => "never",
        _ => "auto",
    }
}

/// Maximum deferral age, or the default when the raw value is invalid (deferMaxAgeS).
pub fn defer_max_age_s(raw: Option<&str>) -> f64 {
    raw.and_then(|value| value.trim().parse::<f64>().ok())
        .filter(|value| value.is_finite() && *value > 0.0)
        .unwrap_or(MAX_AGE_DEFAULT_S)
}

/// Whether a review should be deferred at all.
pub fn is_deferred(mode: &str, managed_target: bool) -> bool {
    mode.eq_ignore_ascii_case("auto") && managed_target
}

/// Live-turn count after a turn starts.
pub fn note_turn_started(live: u32) -> u32 {
    live.saturating_add(1)
}

/// Live-turn count and quiet stamp after a turn finishes.
pub fn note_turn_finished(live: u32, now: f64) -> (u32, Option<f64>) {
    let live = live.saturating_sub(1);
    (live, (live == 0).then_some(now))
}

/// Whether no turn is live.
pub fn is_quiet(live: u32, quiet_since: Option<f64>, now: f64) -> bool {
    live == 0 && quiet_since.is_some_and(|stamp| now >= stamp)
}

/// Whether the quiet window has settled.
pub fn is_settled(live: u32, quiet_since: Option<f64>, now: f64, settle: f64) -> bool {
    quiet_since.is_some_and(|stamp| is_quiet(live, quiet_since, now) && now - stamp >= settle.max(0.0))
}

/// Enqueue time on coalesce: a new snapshot keeps the ORIGINAL time.
pub fn enqueue_time(existing: Option<f64>, now: f64) -> f64 {
    existing.unwrap_or(now)
}

/// Coalesce a session's pending review and return the queue length.
pub fn coalesce(pending: &mut BTreeMap<String, f64>, session: &str, now: f64) -> usize {
    let stamp = enqueue_time(pending.get(session).copied(), now);
    pending.insert(session.to_owned(), stamp);
    pending.len()
}

/// Whether a pending review has aged out of the defer window.
pub fn is_aged_out(enqueued_at: f64, now: f64, max_age: f64) -> bool {
    now - enqueued_at >= max_age.max(0.0)
}

/// Whether a pending review may dispatch now.
pub fn should_dispatch(deferred: bool, settled: bool, aged_out: bool) -> bool {
    deferred && (settled || aged_out)
}

/// Sessions dispatchable now, in stable order (never dispatches un-deferred reviews).
pub fn dispatchable_sessions(
    pending: &BTreeMap<String, f64>,
    now: f64,
    max_age: f64,
    settled: bool,
    deferred: bool,
) -> Vec<String> {
    if !deferred {
        return Vec::new();
    }
    pending
        .iter()
        .filter(|(_, stamp)| settled || is_aged_out(**stamp, now, max_age))
        .map(|(session, _)| session.clone())
        .collect()
}

/// Short session suffix used in log lines.
pub fn session_suffix(session: &str) -> String {
    session.chars().rev().take(12).collect::<Vec<_>>().into_iter().rev().collect()
}

/// Queued count for the card.
pub fn queued_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Human label for a defer mode.
pub fn review_mode_label(mode: &str) -> &'static str {
    if mode.eq_ignore_ascii_case("never") { "Immediate" } else { "Defer on idle" }
}

/// SF Symbol for a defer mode.
pub fn review_mode_icon(mode: &str) -> &'static str {
    if mode.eq_ignore_ascii_case("never") { "bolt.fill" } else { "moon.zzz.fill" }
}

/// Human label for machine idleness.
pub fn idle_label(settled: bool) -> &'static str {
    if settled { "Idle" } else { "Busy" }
}

/// Semantic tone for machine idleness.
pub fn idle_tone(settled: bool) -> &'static str {
    if settled { "good" } else { "attention" }
}

/// Poll interval in milliseconds.
pub fn next_poll_ms() -> u64 {
    (POLL_INTERVAL_S * 1_000.0).round() as u64
}

/// Validation message for an unknown defer mode, if any.
pub fn defer_error(raw: &str) -> String {
    let mode = raw.trim().to_ascii_lowercase();
    if matches!(mode.as_str(), "auto" | "never") {
        String::new()
    } else {
        format!("Unknown defer mode '{raw}'; use auto or never.")
    }
}

/// Summary rows for the card.
pub fn summary(queued: usize, live: u32, settled: bool, mode: &str) -> Vec<(&'static str, String)> {
    vec![
        ("queued", queued_label(queued)),
        ("live", live.to_string()),
        ("machine", idle_label(settled).to_owned()),
        ("mode", review_mode_label(mode).to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configuration() {
        assert_eq!(IDLE_SETTLE_S, 15.0);
        assert_eq!(POLL_INTERVAL_S, 5.0);
        assert_eq!(MAX_AGE_DEFAULT_S, 1800.0);
        assert_eq!(defer_mode(Some(" NEVER ")), "never");
        assert_eq!(defer_mode(Some("bad")), "auto");
        assert_eq!(defer_max_age_s(Some("60")), 60.0);
        assert_eq!(defer_max_age_s(Some("bad")), MAX_AGE_DEFAULT_S);
        assert_eq!(defer_max_age_s(Some("-1")), MAX_AGE_DEFAULT_S);
        assert!(is_deferred("auto", true));
        assert!(!is_deferred("auto", false));
        assert!(!is_deferred("never", true));
        assert!(defer_error("bad").contains("auto"));
        assert!(defer_error("auto").is_empty());
    }

    #[test]
    fn turn_lifecycle() {
        assert_eq!(note_turn_started(0), 1);
        assert_eq!(note_turn_finished(1, 100.0), (0, Some(100.0)));
        assert_eq!(note_turn_finished(2, 100.0), (1, None));
        assert!(is_quiet(0, Some(100.0), 110.0));
        assert!(!is_quiet(1, Some(100.0), 110.0));
        assert!(is_settled(0, Some(100.0), 120.0, 15.0));
        assert!(!is_settled(0, Some(100.0), 110.0, 15.0));
    }

    #[test]
    fn coalescing_and_age() {
        assert_eq!(enqueue_time(None, 100.0), 100.0);
        assert_eq!(enqueue_time(Some(90.0), 100.0), 90.0);
        let mut pending = BTreeMap::new();
        assert_eq!(coalesce(&mut pending, "a", 100.0), 1);
        assert_eq!(coalesce(&mut pending, "a", 200.0), 1);
        assert_eq!(pending["a"], 100.0);
        assert_eq!(coalesce(&mut pending, "b", 200.0), 2);
        assert!(is_aged_out(100.0, 2000.0, 1800.0));
        assert!(!is_aged_out(100.0, 1000.0, 1800.0));
    }

    #[test]
    fn dispatch_gating() {
        assert!(should_dispatch(true, true, false));
        assert!(should_dispatch(true, false, true));
        assert!(!should_dispatch(false, true, true));
        assert!(!should_dispatch(true, false, false));
        let mut pending = BTreeMap::new();
        pending.insert("a".to_owned(), 100.0);
        pending.insert("b".to_owned(), 1900.0);
        assert_eq!(dispatchable_sessions(&pending, 2000.0, 1800.0, true, true).len(), 2);
        assert_eq!(dispatchable_sessions(&pending, 2000.0, 1800.0, false, true), vec!["a"]);
        assert!(dispatchable_sessions(&pending, 2000.0, 1800.0, true, false).is_empty());
    }

    #[test]
    fn labels_and_summary() {
        assert_eq!(session_suffix("abcdefghijklmnop"), "efghijklmnop");
        assert_eq!(queued_label(1500), "1.5K");
        assert_eq!(review_mode_label("never"), "Immediate");
        assert_eq!(review_mode_icon("auto"), "moon.zzz.fill");
        assert_eq!(idle_label(true), "Idle");
        assert_eq!(idle_tone(false), "attention");
        assert_eq!(next_poll_ms(), 5000);
        assert_eq!(summary(2, 0, true, "auto").len(), 4);
    }
}
