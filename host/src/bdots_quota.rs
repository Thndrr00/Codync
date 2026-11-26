#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from hermes-agent's `account_usage.py`: plan windows, basis-point gauges,
//! reset labels and depletion status. Composes helpers already ported into this host. Only the
//! `botInsights` endpoint calls this.

/// A full gauge in basis points (BP_FULL).
pub const BP_FULL: i64 = 10_000;

/// A label cleaned of `_` and `-`, written in title case (_titleCaseSlug).
pub fn title_case_slug(value: Option<&str>) -> Option<String> {
    let cleaned = value.unwrap_or("").trim();
    if cleaned.is_empty() {
        return None;
    }
    let words: Vec<String> = cleaned
        .split(|c| c == '_' || c == '-' || c == ' ')
        .filter(|w| !w.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_ascii_lowercase(),
                None => String::new(),
            }
        })
        .collect();
    Some(words.join(" "))
}

/// Whether a percentage is finite and in range.
pub fn is_percent(value: Option<f64>) -> bool {
    value.is_some_and(|v| v.is_finite() && (0.0..=100.0).contains(&v))
}

/// Percent still available in a window.
pub fn remaining_percent(used: f64) -> f64 {
    (100.0 - used.clamp(0.0, 100.0)).max(0.0)
}

/// Basis points as a percent, when the gauge is valid (_headerGaugePercent).
pub fn bp_percent(bp: Option<i64>) -> Option<f64> {
    let bp = bp?;
    if is_header_gauge_valid(bp) { Some(bp as f64 / BP_FULL as f64 * 100.0) } else { None }
}

/// Whether a header gauge value is in the valid basis-point range.
pub fn is_header_gauge_valid(bp: i64) -> bool {
    (0..=BP_FULL).contains(&bp)
}

/// Human label for a reset, given seconds until it (_formatReset, without the stamp).
pub fn reset_label(seconds: Option<i64>) -> String {
    let Some(seconds) = seconds else {
        return "unknown".to_owned();
    };
    if seconds <= 0 {
        return "now".to_owned();
    }
    let days = seconds / 86_400;
    let hours = (seconds % 86_400) / 3_600;
    let minutes = (seconds % 3_600) / 60;
    if days > 0 {
        format!("in {days}d {hours}h")
    } else if hours > 0 {
        format!("in {hours}h {minutes}m")
    } else {
        format!("in {minutes}m")
    }
}

/// Short reset phrase for a TUI column (_resetPhrase).
pub fn reset_phrase(seconds: Option<i64>) -> String {
    match reset_label(seconds).strip_prefix("in ") {
        Some(rest) => format!("resets in {rest}"),
        None => "resets soon".to_owned(),
    }
}

/// Whether a window is fully used up.
pub fn is_depleted(used_percent: f64) -> bool {
    used_percent >= 100.0
}

/// Whether a window is close to running out.
pub fn is_low(used_percent: f64, threshold: f64) -> bool {
    (threshold..100.0).contains(&used_percent)
}

/// Semantic tone for a usage level.
pub fn usage_tone(used_percent: f64) -> &'static str {
    if is_depleted(used_percent) {
        "danger"
    } else if used_percent >= 80.0 {
        "attention"
    } else if used_percent >= 0.0 {
        "good"
    } else {
        "secondary"
    }
}

/// SF Symbol for a usage level.
pub fn usage_icon(used_percent: f64) -> &'static str {
    if is_depleted(used_percent) {
        "battery.0"
    } else if used_percent >= 80.0 {
        "battery.25"
    } else {
        "battery.100"
    }
}

/// Human label for a window's scope.
pub fn scope_label(scope: &str) -> &'static str {
    if scope.eq_ignore_ascii_case("model") { "Model" } else { "Account" }
}

/// Whether exhausting this window exhausts the whole login.
pub fn is_account_scope(scope: &str) -> bool {
    !scope.eq_ignore_ascii_case("model")
}

/// One line describing a usage window.
pub fn window_headline(label: &str, used: f64, reset: &str) -> String {
    format!("{} · {}% · {}", crate::bdots_textfmt::upper_first(label), used.round() as i64, reset)
}

/// Plural suffix for a count (_plural).
pub fn plural_suffix(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}

/// A dollar amount in the shared money format (_fmtUsd).
pub fn format_usd_short(value: f64) -> String {
    crate::bdots_extra::format_money(crate::bdots_extra::to_minor_units(value, "USD"), "USD")
}

/// Whether a snapshot has anything to show (AccountUsageSnapshot.available).
pub fn is_available(windows: usize, details: usize, unavailable: bool) -> bool {
    (windows > 0 || details > 0) && !unavailable
}

/// The first fully depleted window, if any.
pub fn depleted_window<'a>(windows: &'a [(String, f64)]) -> Option<&'a str> {
    windows.iter().find(|(_, used)| is_depleted(*used)).map(|(label, _)| label.as_str())
}

/// The window with the least remaining quota.
pub fn lowest_remaining<'a>(windows: &'a [(String, f64)]) -> Option<(&'a str, f64)> {
    windows
        .iter()
        .min_by(|a, b| remaining_percent(a.1).total_cmp(&remaining_percent(b.1)))
        .map(|(label, used)| (label.as_str(), remaining_percent(*used)))
}

/// Windows sorted from most used to least used.
pub fn sorted_windows<'a>(windows: &'a [(String, f64)]) -> Vec<&'a (String, f64)> {
    let mut sorted: Vec<&(String, f64)> = windows.iter().collect();
    sorted.sort_by(|a, b| b.1.total_cmp(&a.1));
    sorted
}

/// Status line for a depleted or paused account.
pub fn status_line(depleted: bool, paused: bool) -> &'static str {
    if paused {
        "Included usage paused"
    } else if depleted {
        "Access depleted — top up to restore"
    } else {
        "Active"
    }
}

/// SF Symbol for a window scope.
pub fn scope_icon(scope: &str) -> &'static str {
    if is_account_scope(scope) { "person.crop.circle" } else { "cpu" }
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(used: f64, remaining: f64, windows: usize, reset: &str) -> Vec<(&'static str, String)> {
    vec![
        ("used", format!("{}%", used.round() as i64)),
        ("left", format!("{}%", remaining.round() as i64)),
        ("windows", count_label(windows)),
        ("reset", reset.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_and_gauges() {
        assert_eq!(title_case_slug(Some("weekly_limit-5h")).as_deref(), Some("Weekly Limit 5h"));
        assert_eq!(title_case_slug(Some("  ")), None);
        assert!(is_percent(Some(37.5)));
        assert!(!is_percent(Some(101.0)));
        assert!(!is_percent(Some(f64::NAN)));
        assert_eq!(remaining_percent(37.5), 62.5);
        assert_eq!(remaining_percent(150.0), 0.0);
        assert_eq!(bp_percent(Some(5_000)), Some(50.0));
        assert_eq!(bp_percent(Some(20_000)), None);
        assert!(is_header_gauge_valid(BP_FULL));
    }

    #[test]
    fn resets() {
        assert_eq!(reset_label(None), "unknown");
        assert_eq!(reset_label(Some(0)), "now");
        assert_eq!(reset_label(Some(120)), "in 2m");
        assert_eq!(reset_label(Some(7_200)), "in 2h 0m");
        assert_eq!(reset_label(Some(90_000)), "in 1d 1h");
        assert_eq!(reset_phrase(Some(120)), "resets in 2m");
        assert_eq!(reset_phrase(None), "resets soon");
    }

    #[test]
    fn levels_and_scopes() {
        assert!(is_depleted(100.0));
        assert!(is_low(85.0, 80.0));
        assert!(!is_low(100.0, 80.0));
        assert_eq!(usage_tone(100.0), "danger");
        assert_eq!(usage_tone(85.0), "attention");
        assert_eq!(usage_icon(10.0), "battery.100");
        assert_eq!(scope_label("model"), "Model");
        assert!(is_account_scope("account"));
        assert!(!is_account_scope("model"));
        assert_eq!(scope_icon("model"), "cpu");
    }

    #[test]
    fn lines_windows_and_status() {
        assert_eq!(plural_suffix(1), "");
        assert_eq!(plural_suffix(2), "s");
        assert!(format_usd_short(0.42).contains('$'));
        assert!(is_available(1, 0, false));
        assert!(!is_available(1, 0, true));
        let windows = vec![("session".to_owned(), 90.0), ("week".to_owned(), 40.0)];
        assert_eq!(depleted_window(&windows), None);
        assert_eq!(lowest_remaining(&windows), Some(("session", 10.0)));
        assert_eq!(sorted_windows(&windows)[0].1, 90.0);
        assert!(window_headline("session", 90.0, "in 2m").contains("90%"));
        assert!(status_line(true, false).contains("depleted"));
        assert_eq!(status_line(false, true), "Included usage paused");
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(40.0, 60.0, 2, "in 2m").len(), 4);
    }
}
