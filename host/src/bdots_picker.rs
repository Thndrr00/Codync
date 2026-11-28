#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from hermes-agent's `model_picker_usage.py`: allowance gauges, usage bars,
//! model ordering and picker tags. Composes helpers already ported into this host. Only
//! `botInsights` calls this.

use serde_json::Value;

/// Width of a usage bar (BAR_WIDTH).
pub const BAR_WIDTH: usize = 10;

/// Remaining-fraction below which the gauge turns yellow.
pub const YELLOW_BELOW: f64 = 0.5;

/// Remaining-fraction below which the gauge turns red.
pub const RED_BELOW: f64 = 0.2;

/// Gauge ids Hermes names itself.
pub const GAUGE_LABELS: [&str; 4] = ["day", "week", "month", "top_tier"];

/// Levels shown as a tag on a picker row.
pub const SHOWN_LEVELS: [&str; 2] = ["top_tier", "free"];

/// Clamp a fraction into 0..=1 (clamp_fraction).
pub fn clamp_fraction(value: f64) -> f64 {
    if !value.is_finite() || value < 0.0 { 0.0 } else { value.min(1.0) }
}

/// Used fraction from a remaining fraction (UsageGauge.used_fraction).
pub fn used_fraction(left: f64) -> f64 {
    1.0 - clamp_fraction(left)
}

/// Gauge color from remaining allowance (gauge_style).
pub fn gauge_style(left: f64) -> &'static str {
    let left = clamp_fraction(left);
    if left < RED_BELOW {
        "red"
    } else if left < YELLOW_BELOW {
        "yellow"
    } else {
        "green"
    }
}

/// Filled cells of a bar: non-zero shows one, non-full leaves one empty (_filled_cells).
pub fn filled_cells(fraction: f64, width: usize) -> usize {
    let value = clamp_fraction(fraction);
    let width = width.max(1);
    let mut filled = (value * width as f64).round() as usize;
    if value > 0.0 && filled == 0 {
        filled = 1;
    }
    if value < 1.0 && filled == width {
        filled = width - 1;
    }
    filled.min(width)
}

/// Render a usage bar as `[||||      ]` (renderBar).
pub fn render_bar(fraction: f64, width: usize) -> String {
    let filled = filled_cells(fraction, width);
    format!("[{}{}]", "|".repeat(filled), " ".repeat(width.min(BAR_WIDTH.max(width)) - filled.min(width)))
}

/// Whole-percent used: `<1%` for a sliver, `100%` only when exhausted (formatPercentUsed).
pub fn format_percent_used(used: f64) -> String {
    let bp = (clamp_fraction(used) * 10_000.0).round() as u64;
    if bp == 0 {
        "0%".to_owned()
    } else if bp < 100 {
        "<1%".to_owned()
    } else {
        format!("{}%", bp / 100)
    }
}

/// Compact duration until a reset in ms (compactDuration).
pub fn compact_duration(resets_at_ms: Option<i64>, now_ms: i64) -> String {
    let Some(reset) = resets_at_ms else {
        return String::new();
    };
    let seconds = (reset - now_ms).max(0) / 1000;
    if seconds < 60 {
        return String::new();
    }
    let days = seconds / 86_400;
    let hours = (seconds % 86_400) / 3_600;
    let minutes = (seconds % 3_600) / 60;
    if days > 0 {
        if hours > 0 { format!("{days}d {hours}h") } else { format!("{days}d") }
    } else if hours > 0 {
        format!("{hours}h")
    } else {
        format!("{minutes}m")
    }
}

/// Reset phrase for a gauge (formatResetsIn).
pub fn format_resets_in(resets_at_ms: Option<i64>, now_ms: i64) -> String {
    let duration = compact_duration(resets_at_ms, now_ms);
    if duration.is_empty() { "resets soon".to_owned() } else { format!("resets in {duration}") }
}

/// Featured picks first, then the rest in catalog order (featuredFirst).
pub fn featured_first(model_ids: &[String], featured: &[String]) -> Vec<String> {
    let (head, tail) = split_recommended(model_ids, featured);
    let mut out = head;
    out.extend(tail);
    out
}

/// Split models into recommended and other (splitRecommended).
pub fn split_recommended(model_ids: &[String], featured: &[String]) -> (Vec<String>, Vec<String>) {
    let mut head: Vec<String> = Vec::new();
    for model in featured {
        if model_ids.contains(model) && !head.contains(model) {
            head.push(model.clone());
        }
    }
    let tail: Vec<String> = model_ids.iter().filter(|m| !head.contains(m)).cloned().collect();
    (head, tail)
}

/// Whether a pricing row is free: $0 in and $0 out (isFreeModel).
pub fn is_free_model(pricing: Option<&Value>) -> bool {
    let Some(row) = pricing else {
        return false;
    };
    let price = |key: &str| row[key].as_f64().filter(|value| value.is_finite());
    match (price("prompt"), price("completion")) {
        (Some(input), Some(output)) => input <= 0.0 && output <= 0.0,
        _ => false,
    }
}

/// Whether a pricing row is billed to a held subscription.
pub fn is_subscription_row(pricing: Option<&Value>) -> bool {
    pricing.and_then(|row| row["billing_mode"].as_str()).is_some_and(|mode| mode.eq_ignore_ascii_case("subscription"))
}

/// Usage level for one model from pricing, display only (usage_levels).
pub fn usage_level(pricing: Option<&Value>, stamped: Option<&str>) -> &'static str {
    if let Some(level) = stamped {
        if SHOWN_LEVELS.contains(&level) || matches!(level, "low" | "medium") {
            return match level {
                "top_tier" => "top_tier",
                "free" => "free",
                "medium" => "medium",
                _ => "low",
            };
        }
    }
    if is_free_model(pricing) { "free" } else { "medium" }
}

/// Tag shown next to a row, empty for most levels (usageLevelLabel).
pub fn level_tag(level: &str) -> &'static str {
    match level {
        "top_tier" => "Large",
        "free" => "Free",
        _ => "",
    }
}

/// CLI picker row label from stamped usage levels (windowRowLabels).
pub fn window_row_label(id: &str, level: &str, subscribed: bool) -> String {
    if subscribed {
        return id.to_owned();
    }
    let tag = level_tag(level);
    if tag.is_empty() { id.to_owned() } else { format!("{id}   {tag}") }
}

/// Human label for a gauge id, reusing the text formatter.
pub fn gauge_label(id: &str) -> String {
    crate::bdots_textfmt::upper_first(id.replace('_', " ").as_str())
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(left: f64, used: f64, gauges: usize, featured: usize) -> Vec<(&'static str, String)> {
    vec![
        ("style", gauge_style(left).to_owned()),
        ("used", format_percent_used(used)),
        ("gauges", count_label(gauges)),
        ("featured", count_label(featured)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn fractions_and_styles() {
        assert_eq!(clamp_fraction(-1.0), 0.0);
        assert_eq!(clamp_fraction(2.0), 1.0);
        assert_eq!(used_fraction(0.25), 0.75);
        assert_eq!(gauge_style(0.1), "red");
        assert_eq!(gauge_style(0.3), "yellow");
        assert_eq!(gauge_style(0.7), "green");
    }

    #[test]
    fn bars_and_percents() {
        assert_eq!(filled_cells(0.0, 10), 0);
        assert_eq!(filled_cells(0.01, 10), 1);
        assert_eq!(filled_cells(0.99, 10), 9);
        let bar = render_bar(0.5, 10);
        assert_eq!(bar.len(), 12);
        assert!(bar.contains("|||||"));
        assert_eq!(format_percent_used(0.0), "0%");
        assert_eq!(format_percent_used(0.001), "<1%");
        assert_eq!(format_percent_used(0.5), "50%");
        assert_eq!(format_percent_used(1.0), "100%");
    }

    #[test]
    fn resets() {
        assert_eq!(compact_duration(None, 0), "");
        assert_eq!(compact_duration(Some(30_000), 0), "");
        assert_eq!(compact_duration(Some(300_000), 0), "5m");
        assert_eq!(compact_duration(Some(7_200_000), 0), "2h");
        assert_eq!(compact_duration(Some(180_000_000), 0), "2d 2h");
        assert_eq!(format_resets_in(None, 0), "resets soon");
        assert_eq!(format_resets_in(Some(300_000), 0), "resets in 5m");
    }

    #[test]
    fn models_and_pricing() {
        let ids = vec!["a".to_owned(), "b".to_owned(), "c".to_owned()];
        let featured = vec!["c".to_owned(), "a".to_owned()];
        assert_eq!(featured_first(&ids, &featured), vec!["c", "a", "b"]);
        let (head, tail) = split_recommended(&ids, &featured);
        assert_eq!(head, vec!["c", "a"]);
        assert_eq!(tail, vec!["b"]);
        assert!(is_free_model(Some(&json!({"prompt":0,"completion":0}))));
        assert!(!is_free_model(Some(&json!({"prompt":1,"completion":0}))));
        assert!(!is_free_model(None));
        assert!(is_subscription_row(Some(&json!({"billing_mode":"subscription"}))));
        assert_eq!(usage_level(None, Some("top_tier")), "top_tier");
        assert_eq!(usage_level(Some(&json!({"prompt":0,"completion":0})), None), "free");
    }

    #[test]
    fn presentation() {
        assert_eq!(level_tag("free"), "Free");
        assert_eq!(level_tag("low"), "");
        assert_eq!(window_row_label("m", "top_tier", false), "m   Large");
        assert_eq!(window_row_label("m", "top_tier", true), "m");
        assert_eq!(gauge_label("top_tier"), "Top tier");
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(0.7, 0.3, 3, 2).len(), 4);
    }
}
