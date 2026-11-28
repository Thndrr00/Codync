#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from hermes-agent's activity tracking and cron delivery outcome.
//! Composes validators already ported into this host instead of duplicating them. Only the
//! `botInsights` endpoint calls this.

use serde_json::Value;

/// Cadence for durable activity persistence, in seconds (hermes SESSION_ACTIVITY_HEARTBEAT).
pub const HEARTBEAT_SECONDS: u64 = 60;

/// Human label for a liveness state.
pub fn liveness_label(active: bool) -> &'static str {
    if active { "Working" } else { "Idle" }
}

/// SF Symbol for a liveness state.
pub fn liveness_icon(active: bool) -> &'static str {
    if active { "bolt.circle.fill" } else { "moon.circle" }
}

/// Semantic tone for a liveness state.
pub fn liveness_tone(active: bool) -> &'static str {
    if active { "good" } else { "secondary" }
}

/// Whether an activity stamp is fresh against a now timestamp.
pub fn is_fresh(stamp_ms: i64, now_ms: i64, within_ms: i64) -> bool {
    stamp_ms > 0 && now_ms - stamp_ms <= within_ms.max(0)
}

/// Human age of an activity stamp, reusing the shared span formatter.
pub fn age_label(stamp_ms: i64, now_ms: i64) -> String {
    if stamp_ms <= 0 {
        return "Never".to_owned();
    }
    crate::bdots_text::format_span((now_ms - stamp_ms).max(0))
}

/// Generation after one touch (hermes increments a monotonic generation).
pub fn next_generation(current: i64) -> i64 {
    current.saturating_add(1)
}

/// Whether a stall observation binds to the generation it sampled (hermes fencing).
pub fn claim_valid(observed: i64, current: i64) -> bool {
    observed == current
}

/// Bounded description text for an activity card.
pub fn bound_description(desc: &str) -> String {
    crate::bdots_more::shorten(desc.trim(), 120)
}

/// Whether a write is due at this timestamp (rate-limited by HEARTBEAT_SECONDS).
pub fn persist_due(last_ms: i64, now_ms: i64) -> bool {
    last_ms <= 0 || now_ms - last_ms >= (HEARTBEAT_SECONDS as i64) * 1000
}

/// Next due timestamp for a heartbeat.
pub fn next_due(last_ms: i64) -> i64 {
    last_ms + (HEARTBEAT_SECONDS as i64) * 1000
}

/// Ledger outcome for a queue terminal status (hermes _LEDGER_OUTCOME).
pub fn ledger_outcome(status: &str) -> Option<&'static str> {
    match status {
        "delivered" => Some("delivered"),
        "suppressed" => Some("suppressed"),
        "failed" | "unknown" => Some("failed"),
        _ => None,
    }
}

/// Human label for a ledger outcome.
pub fn outcome_label(outcome: &str) -> &'static str {
    match outcome {
        "delivered" => "Delivered",
        "suppressed" => "Suppressed",
        "failed" => "Failed",
        _ => "Queued",
    }
}

/// SF Symbol for a ledger outcome.
pub fn outcome_icon(outcome: &str) -> &'static str {
    match outcome {
        "delivered" => "checkmark.circle.fill",
        "suppressed" => "minus.circle.fill",
        "failed" => "xmark.circle.fill",
        _ => "clock.circle",
    }
}

/// Semantic tone for a ledger outcome.
pub fn outcome_tone(outcome: &str) -> &'static str {
    match outcome {
        "delivered" => "good",
        "suppressed" => "secondary",
        "failed" => "danger",
        _ => "attention",
    }
}

/// Delivery error text from a queue row, in plain words.
pub fn delivery_error(status: &str, error: Option<&str>) -> String {
    if matches!(status, "failed" | "unknown") {
        let text = error.filter(|e| !e.trim().is_empty()).unwrap_or("delivery failed");
        crate::bdots_more::shorten(text, 120)
    } else {
        String::new()
    }
}

/// Outcome from a raw queue row.
pub fn settled(row: &Value) -> Option<&'static str> {
    row.get("status").and_then(Value::as_str).and_then(ledger_outcome)
}

/// Whether a queued delivery is still in progress.
pub fn is_queued(row: &Value) -> bool {
    row.get("status").and_then(Value::as_str).is_some_and(|s| ledger_outcome(s).is_none())
}

/// Job status after settling a delivery (hermes _settle_job_record).
pub fn settled_job_status(current: &str, error: Option<&str>) -> &'static str {
    if current == "delivery_queued" {
        if error.is_some_and(|e| !e.trim().is_empty()) { "delivery_failed" } else { "ok" }
    } else {
        "unchanged"
    }
}

/// Whether a job completion is the latest one (fencing by execution id).
pub fn is_latest_completion(completions: &[String], execution: &str) -> bool {
    completions.last().is_some_and(|last| last == execution)
}

/// One plain sentence describing liveness.
pub fn liveness_headline(active: bool, age: &str) -> String {
    if active { format!("Working · last activity {age}") } else { "Idle".to_owned() }
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(active: bool, age: &str, generations: i64) -> Vec<(&'static str, String)> {
    vec![
        ("state", liveness_label(active).to_owned()),
        ("age", age.to_owned()),
        ("generation", generations.to_string()),
        ("headline", liveness_headline(active, age)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn liveness_states() {
        assert_eq!(liveness_label(true), "Working");
        assert_eq!(liveness_icon(false), "moon.circle");
        assert_eq!(liveness_tone(true), "good");
        assert!(is_fresh(1000, 2000, 5000));
        assert!(!is_fresh(0, 2000, 5000));
        assert_eq!(age_label(0, 2000), "Never");
        assert_eq!(next_generation(3), 4);
        assert!(claim_valid(3, 3));
        assert!(!claim_valid(2, 3));
        assert_eq!(bound_description("  hello  "), "hello");
    }

    #[test]
    fn heartbeat_cadence() {
        assert_eq!(HEARTBEAT_SECONDS, 60);
        assert!(persist_due(0, 1000));
        assert!(persist_due(1000, 61_000));
        assert!(!persist_due(1000, 60_000));
        assert_eq!(next_due(1000), 61_000);
    }

    #[test]
    fn delivery_outcomes() {
        assert_eq!(ledger_outcome("delivered"), Some("delivered"));
        assert_eq!(ledger_outcome("unknown"), Some("failed"));
        assert_eq!(ledger_outcome("queued"), None);
        assert_eq!(outcome_label("suppressed"), "Suppressed");
        assert_eq!(outcome_icon("failed"), "xmark.circle.fill");
        assert_eq!(outcome_tone("delivered"), "good");
        assert!(delivery_error("failed", None).contains("delivery failed"));
        assert!(delivery_error("delivered", Some("x")).is_empty());
    }

    #[test]
    fn rows_and_fencing() {
        assert_eq!(settled(&json!({"status":"delivered"})), Some("delivered"));
        assert!(is_queued(&json!({"status":"queued"})));
        assert!(!is_queued(&json!({"status":"failed"})));
        assert_eq!(settled_job_status("delivery_queued", None), "ok");
        assert_eq!(settled_job_status("delivery_queued", Some("boom")), "delivery_failed");
        assert_eq!(settled_job_status("ok", None), "unchanged");
        assert!(is_latest_completion(&["a".into(), "b".into()], "b"));
        assert!(!is_latest_completion(&["a".into()], "b"));
    }

    #[test]
    fn headline_and_summary() {
        assert!(liveness_headline(true, "2 min").contains("Working"));
        assert_eq!(liveness_headline(false, "x"), "Idle");
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(true, "2 min", 4).len(), 4);
    }
}
