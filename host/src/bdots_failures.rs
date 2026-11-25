#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers for a bot's recent tool failures. Reuses `bdots_extra`, `bdots_more`,
//! `bdots_activity` and `bdots_text` instead of duplicating them. Only `botInsights` calls this.

/// One failed step reduced to what the card shows.
#[derive(Clone, Debug, PartialEq)]
pub struct Failure {
    pub name: String,
    pub kind: &'static str,
    pub reason: String,
    pub age: String,
    pub icon: &'static str,
    pub tone: &'static str,
}

/// SF Symbol for a failure's severity.
pub fn failure_icon(repeat: bool) -> &'static str {
    if repeat { "exclamationmark.octagon.fill" } else { "exclamationmark.triangle.fill" }
}

/// Semantic tone for a failure.
pub fn failure_tone(repeat: bool) -> &'static str {
    if repeat { "danger" } else { "attention" }
}

/// Plain reason from a tool error, reusing the host's extractor.
pub fn plain_reason(value: &serde_json::Value) -> String {
    crate::bdots_extra::tool_result_error(value).unwrap_or_else(|| "Failed without a message".to_owned())
}

/// Short display reason, reusing the shared shortener.
pub fn short_reason(reason: &str) -> String {
    crate::bdots_more::shorten(reason.trim(), 72)
}

/// Whether an error body looks like a permission/refusal message.
pub fn is_refusal(reason: &str) -> bool {
    let lower = reason.to_ascii_lowercase();
    ["not allowed", "denied", "blocked", "forbidden", "permission"].iter().any(|w| lower.contains(w))
}

/// Whether an error body looks like a network/connection failure.
pub fn is_network(reason: &str) -> bool {
    let lower = reason.to_ascii_lowercase();
    ["timeout", "timed out", "offline", "connection", "dns", "unreachable"].iter().any(|w| lower.contains(w))
}

/// Whether an error body looks like a rate limit.
pub fn is_rate_limit(reason: &str) -> bool {
    let lower = reason.to_ascii_lowercase();
    lower.contains("too many") || lower.contains("rate limit") || lower.contains("429")
}

/// Category label for a reason.
pub fn reason_category(reason: &str) -> &'static str {
    if is_refusal(reason) {
        "permission"
    } else if is_rate_limit(reason) {
        "rate limit"
    } else if is_network(reason) {
        "network"
    } else {
        "other"
    }
}

/// SF Symbol for a reason category.
pub fn category_icon(category: &str) -> &'static str {
    match category {
        "permission" => "hand.raised.fill",
        "rate limit" => "gauge.badge.exclamationmark",
        "network" => "wifi.exclamationmark",
        _ => "exclamationmark.circle",
    }
}

/// Semantic tone for a reason category.
pub fn category_tone(category: &str) -> &'static str {
    match category {
        "permission" | "rate limit" => "danger",
        "network" => "attention",
        _ => "secondary",
    }
}

/// Human age for a failure, reusing the shared span formatter.
pub fn age_label(age_ms: i64) -> String {
    crate::bdots_text::format_span(age_ms.max(0))
}

/// Build a failure from raw fields.
pub fn failure(name: &str, error: Option<&serde_json::Value>, age_ms: i64) -> Failure {
    let reason = error.map_or_else(|| "Failed without a message".to_owned(), plain_reason);
    let repeat = is_refusal(&reason) || is_rate_limit(&reason);
    Failure {
        name: name.to_owned(),
        kind: crate::bdots_more::step_kind(name),
        reason: short_reason(&reason),
        age: age_label(age_ms),
        icon: failure_icon(repeat),
        tone: failure_tone(repeat),
    }
}

/// Count failures per reason category, in first-seen order.
pub fn count_categories(failures: &[Failure]) -> Vec<(&'static str, usize)> {
    let mut out: Vec<(&'static str, usize)> = Vec::new();
    for f in failures {
        let cat = reason_category(&f.reason);
        if let Some(entry) = out.iter_mut().find(|(c, _)| *c == cat) {
            entry.1 += 1;
        } else {
            out.push((cat, 1));
        }
    }
    out
}

/// Number of failures.
pub fn total(failures: &[Failure]) -> usize {
    failures.len()
}

/// Whether any failure happened.
pub fn has_any(failures: &[Failure]) -> bool {
    !failures.is_empty()
}

/// Number of failures with a permission/refusal reason.
pub fn refusal_count(failures: &[Failure]) -> usize {
    failures.iter().filter(|f| is_refusal(&f.reason)).count()
}

/// Number of failures with a rate-limit reason.
pub fn rate_limit_count(failures: &[Failure]) -> usize {
    failures.iter().filter(|f| is_rate_limit(&f.reason)).count()
}

/// Number of failures with a network reason.
pub fn network_count(failures: &[Failure]) -> usize {
    failures.iter().filter(|f| is_network(&f.reason)).count()
}

/// Percent of failures in a category.
pub fn category_percent(failures: &[Failure], category: &str) -> u32 {
    let n = failures.iter().filter(|f| reason_category(&f.reason) == category).count();
    crate::bdots_activity::percent(n, failures.len())
}

/// Bar of filled cells for a category's share.
pub fn category_bar(failures: &[Failure], category: &str, width: usize) -> String {
    let n = failures.iter().filter(|f| reason_category(&f.reason) == category).count();
    "▮".repeat(crate::bdots_activity::bar_width(n, failures.len(), width))
}

/// One plain sentence describing the failures.
pub fn failure_headline(failures: &[Failure]) -> String {
    if failures.is_empty() {
        return "No recent failures.".to_owned();
    }
    let cats = count_categories(failures);
    let top = cats.first().map_or("other", |(c, _)| *c);
    format!("{} failure{} · mostly {top}", failures.len(), if failures.len() == 1 { "" } else { "s" })
}

/// Semantic tone for the failure card.
pub fn failure_tone_headline(failures: &[Failure]) -> &'static str {
    if failures.is_empty() { "good" } else { "attention" }
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(failures: &[Failure]) -> String {
    crate::bdots_desk::compact_count(failures.len() as u64)
}

/// Failures sorted newest first by age label, then name.
pub fn sorted(failures: &[Failure]) -> Vec<&Failure> {
    let mut out: Vec<&Failure> = failures.iter().collect();
    out.sort_by(|a, b| b.age.cmp(&a.age).then_with(|| a.name.cmp(&b.name)));
    out
}

/// Summary rows for the card.
pub fn summary(failures: &[Failure]) -> Vec<(&'static str, String)> {
    vec![
        ("total", count_label(failures)),
        ("refusals", refusal_count(failures).to_string()),
        ("rate", rate_limit_count(failures).to_string()),
        ("headline", failure_headline(failures)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn failures() -> Vec<Failure> {
        vec![
            failure("exec", Some(&json!({"isError":true,"content":[{"type":"text","text":"not allowed"}]})), 60_000),
            failure(
                "send_email",
                Some(&json!({"isError":true,"content":[{"type":"text","text":"too many requests"}]})),
                120_000,
            ),
            failure("read", None, 30_000),
        ]
    }

    #[test]
    fn icons_tones_and_categories() {
        assert_eq!(failure_icon(true), "exclamationmark.octagon.fill");
        assert_eq!(failure_tone(false), "attention");
        assert_eq!(category_icon("permission"), "hand.raised.fill");
        assert_eq!(category_tone("network"), "attention");
        assert_eq!(reason_category("not allowed"), "permission");
        assert_eq!(reason_category("429 too many"), "rate limit");
        assert_eq!(reason_category("connection timeout"), "network");
        assert_eq!(reason_category("weird"), "other");
    }

    #[test]
    fn detectors_and_reasons() {
        assert!(is_refusal("permission denied"));
        assert!(is_rate_limit("rate limit"));
        assert!(is_network("dns failed"));
        assert!(!is_refusal("hello"));
        let err = json!({"isError":true,"content":[{"type":"text","text":"boom"}]});
        assert_eq!(plain_reason(&err), "boom");
        assert_eq!(short_reason("  a  "), "a");
        assert_eq!(age_label(120_000), "2 min");
    }

    #[test]
    fn build_and_count() {
        let f = failures();
        assert_eq!(total(&f), 3);
        assert!(has_any(&f));
        assert!(!has_any(&[]));
        assert_eq!(refusal_count(&f), 1);
        assert_eq!(rate_limit_count(&f), 1);
        assert_eq!(network_count(&f), 0);
        assert_eq!(count_categories(&f).len(), 3);
    }

    #[test]
    fn shares_headlines_and_order() {
        let f = failures();
        assert_eq!(category_percent(&f, "permission"), 33);
        assert!(!category_bar(&f, "permission", 8).is_empty());
        assert!(failure_headline(&f).contains("failure"));
        assert!(failure_headline(&[]).contains("No"));
        assert_eq!(failure_tone_headline(&[]), "good");
        assert_eq!(count_label(&f), "3");
        assert_eq!(sorted(&f).len(), 3);
        assert_eq!(summary(&f).len(), 4);
    }
}
