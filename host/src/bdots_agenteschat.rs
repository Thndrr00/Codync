#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from agentes-chat 0.7.5 (context limits, timeline, auto model). Composes
//! validators already ported into this host. Only `botInsights` calls this.

/// A context size as the apps write it: 200K, 400K, 1M (contextSize).
pub fn context_size(tokens: u64) -> String {
    if tokens >= 1_000_000 { format!("{}M", tokens / 1_000_000) } else { format!("{}K", (tokens + 999) / 1000) }
}

/// Plan window from a model id with Claude Code's `[1m]` suffix (planWindow).
pub fn plan_window(model_id: Option<&str>) -> Option<u64> {
    model_id.filter(|m| m.ends_with("[1m]")).map(|_| 1_000_000)
}

/// Effective context window from a model's declared window or plan id.
pub fn effective_window(window: Option<u64>, model_id: Option<&str>) -> u64 {
    window.or_else(|| plan_window(model_id)).unwrap_or(200_000)
}

/// Whether `window` fits under a model's context limit.
pub fn fits_window(window: u64, limit: u64) -> bool {
    window <= limit
}

/// The smaller of two windows (the one that wins at run time).
pub fn smaller_window(a: u64, b: u64) -> u64 {
    a.min(b)
}

/// Human label for a context limit choice.
pub fn limit_label(tokens: u64) -> String {
    format!("{} context", context_size(tokens))
}

/// SF Symbol for a context limit.
pub fn limit_icon(tokens: u64) -> &'static str {
    if tokens >= 1_000_000 { "infinity.circle" } else { "circle.lefthalf.filled" }
}

/// Semantic tone for a context limit.
pub fn limit_tone(tokens: u64) -> &'static str {
    if tokens >= 1_000_000 { "good" } else { "accent" }
}

/// How long a posted placeholder is kept, in ms (PLACE_KEPT_MS).
pub const PLACE_KEPT_MS: u64 = 600_000;

/// Grouping window for timeline messages, in ms (GROUP_WINDOW_MS).
pub const GROUP_WINDOW_MS: u64 = 300_000;

/// Key for a message, preferring its client id (messageKey).
pub fn message_key(id: &str, client_id: Option<&str>) -> String {
    client_id.map_or_else(|| id.to_owned(), |c| format!("c:{c}"))
}

/// Whether a message landed after a timestamp (landedAfter).
pub fn landed_after(created_ms: i64, at_ms: i64) -> bool {
    created_ms > at_ms
}

/// Whether two timestamps fall in the same grouping window.
pub fn same_group(a_ms: i64, b_ms: i64) -> bool {
    a_ms.abs_diff(b_ms) <= GROUP_WINDOW_MS as i64
}

/// Whether a placeholder is still within its keep window.
pub fn placeholder_fresh(posted_ms: i64, now_ms: i64) -> bool {
    now_ms - posted_ms >= 0 && (now_ms - posted_ms) <= PLACE_KEPT_MS as i64
}

/// Auto efforts an agent can choose (AUTO_EFFORTS).
pub fn auto_efforts() -> &'static [&'static str] {
    &["low", "medium", "high"]
}

/// Whether an effort is an auto effort.
pub fn is_auto_effort(effort: &str) -> bool {
    auto_efforts().contains(&effort.to_ascii_lowercase().as_str())
}

/// Timeout for an auto model choice, in ms (AUTO_MODEL_TIMEOUT_MS).
pub const AUTO_MODEL_TIMEOUT_MS: u64 = 1500;

/// Whether an agent needs an auto choice (needsAutoChoice), approximated from mode and plan.
pub fn needs_auto_choice(mode: &str, plan_has_model: bool) -> bool {
    mode.eq_ignore_ascii_case("auto") && !plan_has_model
}

/// Fallback choice source label.
pub fn fallback_label(source: &str) -> &'static str {
    match source.to_ascii_lowercase().as_str() {
        "judge" => "Judge",
        "config" => "Configured",
        "default" => "Default",
        _ => "Fallback",
    }
}

/// SF Symbol for a fallback source.
pub fn fallback_icon(source: &str) -> &'static str {
    if source.eq_ignore_ascii_case("judge") { "scale.3d" } else { "arrow.uturn.left.circle" }
}

/// Human label for an auto decision.
pub fn auto_label(on: bool) -> &'static str {
    if on { "Automatic" } else { "Manual" }
}

/// Semantic tone for an auto decision.
pub fn auto_tone(on: bool) -> &'static str {
    if on { "good" } else { "secondary" }
}

/// One plain sentence describing context use.
pub fn context_headline(used: u64, window: u64) -> String {
    let pct = crate::bdots_activity::percent(used as usize, window as usize);
    format!("{} of {} · {}%", context_size(used), limit_label(window), pct)
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(used: u64, window: u64, auto: bool) -> Vec<(&'static str, String)> {
    vec![
        ("window", limit_label(window)),
        ("used", context_size(used)),
        ("mode", auto_label(auto).to_owned()),
        ("headline", context_headline(used, window)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_formatting() {
        assert_eq!(context_size(200_000), "200K");
        assert_eq!(context_size(1_000_000), "1M");
        assert_eq!(context_size(1001), "2K");
        assert_eq!(plan_window(Some("claude[1m]")), Some(1_000_000));
        assert_eq!(plan_window(Some("claude")), None);
        assert_eq!(effective_window(None, Some("x[1m]")), 1_000_000);
        assert_eq!(effective_window(None, None), 200_000);
    }

    #[test]
    fn fits_and_limits() {
        assert!(fits_window(100_000, 200_000));
        assert!(!fits_window(300_000, 200_000));
        assert_eq!(smaller_window(300_000, 200_000), 200_000);
        assert_eq!(limit_label(200_000), "200K context");
        assert_eq!(limit_icon(1_000_000), "infinity.circle");
        assert_eq!(limit_tone(200_000), "accent");
    }

    #[test]
    fn timeline_helpers() {
        assert_eq!(PLACE_KEPT_MS, 600_000);
        assert_eq!(GROUP_WINDOW_MS, 300_000);
        assert_eq!(message_key("id", None), "id");
        assert_eq!(message_key("id", Some("c1")), "c:c1");
        assert!(landed_after(2000, 1000));
        assert!(!landed_after(500, 1000));
        assert!(same_group(1000, 2000));
        assert!(!same_group(1000, 500_000));
        assert!(placeholder_fresh(1000, 100_000));
        assert!(!placeholder_fresh(1000, 700_000));
    }

    #[test]
    fn auto_helpers() {
        assert_eq!(auto_efforts(), &["low", "medium", "high"]);
        assert!(is_auto_effort("HIGH"));
        assert!(!is_auto_effort("max"));
        assert_eq!(AUTO_MODEL_TIMEOUT_MS, 1500);
        assert!(needs_auto_choice("auto", false));
        assert!(!needs_auto_choice("auto", true));
        assert_eq!(fallback_label("judge"), "Judge");
        assert_eq!(fallback_icon("config"), "arrow.uturn.left.circle");
        assert_eq!(auto_label(true), "Automatic");
        assert_eq!(auto_tone(false), "secondary");
    }

    #[test]
    fn headline_and_summary() {
        assert!(context_headline(100_000, 200_000).contains("50%"));
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(100_000, 200_000, true).len(), 4);
    }
}
