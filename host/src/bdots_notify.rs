#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from hermes-agent's notification presentation and delivery ledger: which
//! events are freeform, when a diagnostic wake mutes a turn, flood/retry timing and obligation
//! states. Composes helpers already ported into this host. Only `botInsights` calls this.

use serde_json::Value;

/// Maximum length of a platform reply before it must be bounded (delivery.py).
pub const MAX_PLATFORM_OUTPUT: usize = 4000;

/// Attempt budget of the delivery obligation ledger (MAX_ATTEMPTS).
pub const MAX_ATTEMPTS: u32 = 3;

/// Default wait after a flood refusal, in seconds.
pub const FLOOD_RETRY_DEFAULT: f64 = 60.0;

/// Upper bound for a flood wait, in seconds.
pub const FLOOD_RETRY_CAP: f64 = 900.0;

/// Slack added to a platform-supplied flood wait, in seconds.
pub const FLOOD_RETRY_SLACK: f64 = 2.0;

/// Backoff after each spent attempt, in seconds (_RETRY_BACKOFF_SECONDS).
pub const RETRY_BACKOFF: [f64; 2] = [30.0, 120.0];

/// Text/media events whose presentation a muted diagnostic turn suppresses (FREEFORM_EVENTS).
pub fn is_freeform_event(event: &str) -> bool {
    matches!(
        event,
        "message.start"
            | "message.delta"
            | "message.interim"
            | "message.complete"
            | "reasoning.delta"
            | "thinking.delta"
            | "status.update"
            | "notification.show"
            | "tool.start"
            | "tool.complete"
            | "tool.generating"
            | "error"
            | "reaction"
    )
}

/// Whether a freeform event is muted for a turn bound to a surface (eventPresentationMuted).
pub fn event_presentation_muted(event: &str, muted_surface: Option<&str>, session_id: &str) -> bool {
    muted_surface == Some(session_id) && is_freeform_event(event)
}

/// Early failure and watch diagnostics, not an explicitly requested final result.
pub fn is_diagnostic_process_event(task_failure_notice: bool, event_type: &str) -> bool {
    task_failure_notice || matches!(event_type, "watch_disabled" | "watch_overflow_tripped" | "watch_overflow_released")
}

/// Callbacks frozen for a muted diagnostic turn (PRESENTATION_CALLBACKS).
pub fn presentation_callbacks() -> &'static [&'static str] {
    &[
        "stream_delta_callback",
        "interim_assistant_callback",
        "reasoning_callback",
        "tool_progress_callback",
        "tool_start_callback",
        "tool_complete_callback",
        "tool_gen_callback",
        "reaction_callback",
    ]
}

/// Whether a name is one of the presentation callbacks.
pub fn is_presentation_callback(name: &str) -> bool {
    presentation_callbacks().contains(&name)
}

/// A warning status or producer-owned diagnostic text (isWarningStatus).
pub fn is_warning_status(event_type: &str, diagnostic_text: bool) -> bool {
    event_type == "warn" || diagnostic_text
}

/// A notice is diagnostic when it warns/errors or is a credit-service notice (isDiagnosticNotice).
pub fn is_diagnostic_notice(level: Option<&str>, key: Option<&str>) -> bool {
    level.is_some_and(|l| matches!(l, "warn" | "error")) || key.is_some_and(|k| k.starts_with("credits."))
}

/// Only trusted internal wakes may classify a turn as diagnostic (diagnosticMetadata).
pub fn is_diagnostic_metadata(internal: bool, category: Option<&str>) -> bool {
    internal && category == Some("diagnostic")
}

/// Diagnostics are visible unless the owning surface opts out (warningNotificationsEnabled).
pub fn warning_notifications_enabled(suppress: bool) -> bool {
    !suppress
}

/// A diagnostic-category wake mutes a turn only when its policy hides diagnostics.
pub fn diagnostic_turn_muted(category: Option<&str>, enabled: bool) -> bool {
    category == Some("diagnostic") && !enabled
}

/// A wake mutes its turn only when internal and trusted, unless a snapshot overrides it.
pub fn diagnostic_wake_muted(internal: bool, category: Option<&str>, enabled: bool, snapshot: Option<bool>) -> bool {
    snapshot.unwrap_or(internal && diagnostic_turn_muted(category, enabled))
}

/// Whether a presentation sink renders this content (renderNotification).
pub fn should_render(diagnostic: bool, enabled: bool) -> bool {
    !diagnostic || enabled
}

/// Text a platform emits when it produced nothing, which must not be narrated (_isSilenceNarration).
pub fn is_silence_narration(text: &str) -> bool {
    let lower = text.trim().to_ascii_lowercase();
    lower.is_empty()
        || lower.contains("no output")
        || lower.contains("no response")
        || lower.contains("nothing to report")
        || lower.contains("ran silently")
}

/// Whether an id looks like a Telegram private chat id (looksLikeTelegramPrivateChatId).
pub fn looks_like_private_chat_id(id: Option<&str>) -> bool {
    id.is_some_and(|value| {
        let digits = value.trim();
        !digits.is_empty() && digits.len() <= 19 && digits.chars().all(|c| c.is_ascii_digit())
    })
}

/// Keys that route a reply to a thread rather than the main chat (_THREAD_ROUTING_KEYS).
pub fn is_thread_routing_key(key: &str) -> bool {
    matches!(key, "thread_id" | "message_thread_id" | "direct_messages_topic_id" | "telegram_direct_messages_topic_id")
}

/// Reply text bounded to the platform's output limit, reusing the shared shortener.
pub fn bounded_output(text: &str) -> String {
    crate::bdots_more::shorten(text, MAX_PLATFORM_OUTPUT)
}

/// A deterministic id for a delivery obligation (computeObligationId), as FNV-1a hex.
pub fn obligation_id(session: &str, message_ref: &str, content: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in [session, message_ref, content].join("\0").bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// A flood-control refusal carries the platform's own wait (FLOOD_ERROR_PREFIX).
pub fn is_flood_error(error: &str) -> bool {
    let lower = error.trim().to_ascii_lowercase();
    lower.starts_with("flood_control:") || lower.contains("flood control") || lower.contains("429")
}

/// An error whose send contract proves the platform never saw the request.
pub fn is_reconnect_only(error: &str) -> bool {
    error.trim().eq_ignore_ascii_case("send_path_degraded")
}

/// A whole-chat death is final: the target is gone.
pub fn is_dead_target(error: &str) -> bool {
    let lower = error.trim().to_ascii_lowercase();
    ["chat not found", "blocked", "deactivated", "user deleted", "bot was kicked", "group deleted"]
        .iter()
        .any(|needle| lower.contains(needle))
}

/// Wait encoded in a flood error, or the default, bounded by the platform cap.
pub fn flood_wait_seconds(error: &str) -> f64 {
    let lower = error.trim().to_ascii_lowercase();
    let parsed = lower
        .strip_prefix("flood_control:")
        .and_then(|rest| rest.trim().parse::<f64>().ok())
        .or_else(|| {
            lower
                .find("retry after ")
                .and_then(|at| lower[at + 12..].split_whitespace().next())
                .and_then(|value| value.parse::<f64>().ok())
        })
        .unwrap_or(FLOOD_RETRY_DEFAULT);
    flood_retry_delay(parsed)
}

/// Clamp a platform wait into the retry window (floodRetryDelay).
pub fn flood_retry_delay(seconds: f64) -> f64 {
    if !seconds.is_finite() || seconds <= 0.0 {
        FLOOD_RETRY_DEFAULT
    } else {
        (seconds + FLOOD_RETRY_SLACK).min(FLOOD_RETRY_CAP)
    }
}

/// Earliest moment a failed obligation may be resent, if any (retryNotBefore).
pub fn retry_not_before(updated_at: f64, error: &str, attempts: u32) -> Option<f64> {
    if is_flood_error(error) {
        return Some(updated_at + flood_wait_seconds(error));
    }
    if is_reconnect_only(error) {
        return Some(updated_at);
    }
    if is_dead_target(error) {
        return None;
    }
    let spent = attempts.max(0) as usize;
    if spent >= MAX_ATTEMPTS as usize - 1 {
        return None;
    }
    Some(updated_at + RETRY_BACKOFF[spent.min(RETRY_BACKOFF.len() - 1)])
}

/// Human label for a ledger state.
pub fn state_label(state: &str) -> &'static str {
    match state {
        "pending" => "Pending",
        "attempting" => "Attempting",
        "delivered" => "Delivered",
        "failed" => "Failed",
        "abandoned" => "Abandoned",
        _ => "Unknown",
    }
}

/// Semantic tone for a ledger state.
pub fn state_tone(state: &str) -> &'static str {
    match state {
        "delivered" => "good",
        "pending" | "attempting" => "accent",
        "failed" | "abandoned" => "attention",
        _ => "secondary",
    }
}

/// Honest marker for a redelivery that may duplicate an earlier message.
pub fn marker_for(state: &str) -> &'static str {
    match state {
        "attempting" => "Recovered reply — the gateway restarted during delivery.",
        "failed" => "Recovered reply — the platform reconnected after the failure.",
        _ => "",
    }
}

/// Icon for a ledger state.
pub fn state_icon(state: &str) -> &'static str {
    match state {
        "delivered" => "checkmark.circle.fill",
        "failed" | "abandoned" => "xmark.circle.fill",
        "attempting" => "arrow.triangle.2.circlepath",
        _ => "clock.circle",
    }
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// One plain sentence about delivery health.
pub fn delivery_headline(delivered: usize, failed: usize, muted: bool) -> String {
    let state = if failed > 0 { "needs attention" } else { "healthy" };
    format!(
        "{} · {} delivered · {} failed{}",
        state,
        count_label(delivered),
        count_label(failed),
        if muted { " · muted" } else { "" }
    )
}

/// Summary rows for the card.
pub fn summary(delivered: usize, failed: usize, pending: usize, muted: bool) -> Vec<(&'static str, String)> {
    vec![
        ("delivered", count_label(delivered)),
        ("failed", count_label(failed)),
        ("pending", count_label(pending)),
        ("presentation", if muted { "muted" } else { "live" }.to_owned()),
    ]
}

/// Diagnostic notice classification of a raw event row.
pub fn classify_event(row: &Value) -> bool {
    let event = row["type"].as_str().unwrap_or("");
    is_diagnostic_process_event(row["task_failure_notice"] == true, event)
        || is_warning_status(event, false)
        || is_diagnostic_notice(row["level"].as_str(), row["key"].as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn presentation_events() {
        assert!(is_freeform_event("message.delta"));
        assert!(!is_freeform_event("approval.request"));
        assert!(event_presentation_muted("tool.start", Some("s1"), "s1"));
        assert!(!event_presentation_muted("tool.start", Some("s2"), "s1"));
        assert!(is_diagnostic_process_event(false, "watch_overflow_tripped"));
        assert!(!is_diagnostic_process_event(false, "message.complete"));
        assert_eq!(presentation_callbacks().len(), 8);
        assert!(is_presentation_callback("reaction_callback"));
    }

    #[test]
    fn diagnostic_policy() {
        assert!(is_warning_status("warn", false));
        assert!(is_warning_status("info", true));
        assert!(is_diagnostic_notice(Some("warn"), None));
        assert!(is_diagnostic_notice(Some("info"), Some("credits.low")));
        assert!(!is_diagnostic_notice(Some("info"), None));
        assert!(is_diagnostic_metadata(true, Some("diagnostic")));
        assert!(!is_diagnostic_metadata(false, Some("diagnostic")));
        assert!(!warning_notifications_enabled(true));
        assert!(diagnostic_turn_muted(Some("diagnostic"), false));
        assert!(!diagnostic_turn_muted(Some("chat"), false));
        assert!(diagnostic_wake_muted(true, Some("diagnostic"), false, None));
        assert!(!diagnostic_wake_muted(true, Some("diagnostic"), false, Some(false)));
        assert!(!should_render(true, false));
        assert!(should_render(false, false));
    }

    #[test]
    fn transport_shaping() {
        assert!(is_silence_narration("  "));
        assert!(is_silence_narration("No output."));
        assert!(!is_silence_narration("Here is the answer."));
        assert!(looks_like_private_chat_id(Some("123456789")));
        assert!(!looks_like_private_chat_id(Some("abc")));
        assert!(is_thread_routing_key("message_thread_id"));
        assert_eq!(bounded_output("hello").len(), 5);
        assert_eq!(obligation_id("s", "m", "c"), obligation_id("s", "m", "c"));
        assert_ne!(obligation_id("s", "m", "c"), obligation_id("s", "m", "d"));
    }

    #[test]
    fn flood_and_retry() {
        assert!(is_flood_error("flood_control:120"));
        assert!(is_flood_error("HTTP 429"));
        assert!(!is_flood_error("ok"));
        assert!(is_reconnect_only("send_path_degraded"));
        assert!(is_dead_target("chat not found"));
        assert!(!is_dead_target("timeout"));
        assert_eq!(flood_wait_seconds("flood_control:10"), 12.0);
        assert_eq!(flood_wait_seconds("nothing"), FLOOD_RETRY_DEFAULT);
        assert_eq!(flood_retry_delay(2000.0), FLOOD_RETRY_CAP);
        assert_eq!(retry_not_before(100.0, "flood_control:10", 0), Some(112.0));
        assert_eq!(retry_not_before(100.0, "send_path_degraded", 2), Some(100.0));
        assert_eq!(retry_not_before(100.0, "chat not found", 0), None);
        assert_eq!(retry_not_before(100.0, "timeout", 0), Some(130.0));
        assert_eq!(retry_not_before(100.0, "timeout", 2), None);
    }

    #[test]
    fn states_and_summary() {
        assert_eq!(state_label("delivered"), "Delivered");
        assert_eq!(state_tone("failed"), "attention");
        assert!(marker_for("attempting").contains("restarted"));
        assert_eq!(state_icon("pending"), "clock.circle");
        assert_eq!(count_label(1500), "1.5K");
        assert!(delivery_headline(10, 0, false).contains("healthy"));
        assert!(delivery_headline(10, 1, true).contains("muted"));
        assert_eq!(summary(2, 1, 0, false).len(), 4);
        assert!(classify_event(&json!({"type":"warn"})));
        assert!(!classify_event(&json!({"type":"message.complete"})));
    }
}
