#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from agentes-chat's `brain-picker.ts`: model descriptions, pick options,
//! search text, grouping and sign-in needs. Composes helpers already ported into this host. Only
//! `botInsights` calls this.

use serde_json::Value;

/// Claude model families used for descriptions (CLAUDE_FAMILIES).
pub const CLAUDE_FAMILIES: [&str; 3] = ["sonnet", "opus", "haiku"];

/// Value of the automatic pick.
pub const AUTO_VALUE: &str = "auto";

/// Keyword inserted for free models.
pub const FREE_KEYWORD: &str = "free";

/// Claude family of a model id, when present (claudeFamily).
pub fn claude_family(model_id: &str, provider: &str) -> Option<&'static str> {
    if !provider.eq_ignore_ascii_case("anthropic") {
        return None;
    }
    let lower = model_id.to_ascii_lowercase();
    CLAUDE_FAMILIES.iter().copied().find(|family| lower.contains(family))
}

/// Whether a model costs nothing (freeModel).
pub fn is_free_model(input_usd: f64, output_usd: f64) -> bool {
    input_usd <= 0.0 && output_usd <= 0.0
}

/// Description of a model in a locale (modelDescription).
pub fn model_description(model_id: &str, provider: &str, input: f64, output: f64, locale: &str) -> String {
    if let Some(family) = claude_family(model_id, provider) {
        return crate::bdots_textfmt::upper_first(family);
    }
    if is_free_model(input, output) {
        return "Free".to_owned();
    }
    format!(
        "{} in / {} out · {}",
        crate::bdots_localfmt::format_decimal(locale, input),
        crate::bdots_localfmt::format_decimal(locale, output),
        crate::bdots_i18n::locale_native(locale)
    )
}

/// Text a search reads of an option (brainPickText).
pub fn pick_text(label: &str, value: &str, group: Option<&str>, keywords: &str) -> String {
    format!("{label} {value} {} {keywords}", group.unwrap_or("")).split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Model label for a pick option.
pub fn pick_label(row: &Value) -> String {
    row["label"].as_str().or_else(|| row["name"].as_str()).unwrap_or("").to_owned()
}

/// Whether a pick option is enabled.
pub fn is_enabled(row: &Value) -> bool {
    row["available"] == true || row["disabled"] != true
}

/// Whether a pick needs its own sign-in (needsOwnSignIn).
pub fn needs_own_sign_in(connected: bool, account: Option<&str>) -> bool {
    !connected && account.is_none()
}

/// Build a search keyword string for a free model.
pub fn free_keywords(free: bool) -> &'static str {
    if free { FREE_KEYWORD } else { "" }
}

/// Model line of a plan pick (brainModelLine), from its free/tag hints.
pub fn model_line(free: bool, tag: Option<&str>, in_plan: bool, cursor_plan_says: bool) -> String {
    if cursor_plan_says {
        return if in_plan { "In plan" } else { "API price" }.to_owned();
    }
    let mut parts = Vec::new();
    if free {
        parts.push("Free");
    }
    if let Some(tag) = tag {
        parts.push(tag);
    }
    parts.join(" · ")
}

/// Group key of a pick option, defaulting to Other.
pub fn group_key(row: &Value) -> String {
    row["group"].as_str().map_or_else(|| "Other".to_owned(), crate::bdots_textfmt::upper_first)
}

/// Group names in first-seen order (pickGroups).
pub fn pick_groups(rows: &[Value]) -> Vec<String> {
    let mut out = Vec::new();
    for row in rows {
        let group = group_key(row);
        if !out.contains(&group) {
            out.push(group);
        }
    }
    out
}

/// Search keyword string built from free and plan membership.
pub fn pick_keywords(free: bool, in_plan: bool) -> String {
    let mut parts = Vec::new();
    if free {
        parts.push(FREE_KEYWORD);
    }
    if in_plan {
        parts.push("plan");
    }
    parts.join(" ")
}

/// Whether a provider label is the automatic pick.
pub fn is_auto(provider: &str) -> bool {
    provider.eq_ignore_ascii_case(AUTO_VALUE)
}

/// Provider label, reusing the text formatter.
pub fn provider_label(provider: &str) -> String {
    crate::bdots_textfmt::upper_first(provider)
}

/// Whether a pick is a connect-only choice (no key yet).
pub fn is_connect_pick(kind: &str) -> bool {
    kind.eq_ignore_ascii_case("connect")
}

/// SF Symbol for a pick kind.
pub fn pick_icon(kind: &str) -> &'static str {
    match kind {
        "model" => "cpu",
        "connect" => "plug",
        "auto" => "lightbulb.fill",
        _ => "person.crop.circle",
    }
}

/// Semantic tone for a pick state.
pub fn pick_tone(enabled: bool) -> &'static str {
    if enabled { "good" } else { "secondary" }
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(options: usize, groups: usize, free: usize, auto: bool) -> Vec<(&'static str, String)> {
    vec![
        ("options", count_label(options)),
        ("groups", count_label(groups)),
        ("free", count_label(free)),
        ("auto", if auto { "yes" } else { "no" }.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn model_helpers() {
        assert_eq!(claude_family("claude-sonnet-4", "anthropic"), Some("sonnet"));
        assert_eq!(claude_family("gpt-5", "openai"), None);
        assert!(is_free_model(0.0, 0.0));
        assert!(!is_free_model(0.0, 1.0));
        assert_eq!(model_description("claude-opus-4", "anthropic", 15.0, 75.0, "en"), "Opus");
        assert_eq!(model_description("x", "openai", 0.0, 0.0, "en"), "Free");
        assert!(model_description("x", "openai", 1.0, 2.0, "en").contains("in"));
    }

    #[test]
    fn picks_and_sign_in() {
        assert_eq!(pick_text("Writer", "w", Some("Models"), "free"), "Writer w Models free");
        assert_eq!(pick_text("Writer", "w", None, ""), "Writer w");
        let row = json!({"label":"Writer","available":true});
        assert_eq!(pick_label(&row), "Writer");
        assert!(is_enabled(&row));
        assert!(!is_enabled(&json!({"disabled":true})));
        assert!(needs_own_sign_in(false, None));
        assert!(!needs_own_sign_in(false, Some("acct")));
        assert_eq!(free_keywords(true), "free");
    }

    #[test]
    fn lines_and_groups() {
        assert_eq!(model_line(true, Some("fast"), false, false), "Free · fast");
        assert_eq!(model_line(false, None, true, true), "In plan");
        assert_eq!(group_key(&json!({"group":"api"})), "Api");
        assert_eq!(group_key(&json!({})), "Other");
        let rows = vec![json!({"group":"a"}), json!({"group":"b"}), json!({"group":"a"})];
        assert_eq!(pick_groups(&rows), vec!["A", "B"]);
        assert_eq!(pick_keywords(true, true), "free plan");
    }

    #[test]
    fn presentation() {
        assert!(is_auto("AUTO"));
        assert_eq!(provider_label("openai"), "Openai");
        assert!(is_connect_pick("connect"));
        assert_eq!(pick_icon("model"), "cpu");
        assert_eq!(pick_tone(false), "secondary");
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(4, 2, 1, true).len(), 4);
    }
}
