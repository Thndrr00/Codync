#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from coucou's `windows/src/core/plan.ts`: Claude and Codex plan usage,
//! window parsing, percentages, pill labels and reset times. Only `botInsights` calls this.

use serde_json::{Value, json};

/// Milliseconds in a day (DAY_MS).
pub const DAY_MS: i64 = 86_400_000;

/// How long a Codex answer stays fresh (CODEX_STALE_MS).
pub const CODEX_STALE_MS: i64 = 60_000;

/// Titles of the two plan cards and pills.
pub const PLAN_TITLES: [&str; 4] = ["Claude plan", "Codex plan", "Claude plan usage", "Codex plan usage"];

/// Names of the two usage windows.
pub const PLAN_WINDOWS: [&str; 2] = ["5 hours", "Week"];

/// Colors of a plan percentage: no data, good, warning, full (planColor).
pub const PLAN_COLORS: [&str; 4] = ["#6B7079", "#22C55E", "#F59E0B", "#F4505E"];

/// Text shown when no plan data arrived.
pub const PLAN_NONE: &str = "–";

/// Whether a JSON value is a finite number (isNum).
pub fn is_num(value: &Value) -> bool {
    value.as_f64().is_some_and(f64::is_finite)
}

/// A JSON object, or null (asObj).
pub fn as_obj(value: Option<&Value>) -> Option<&Value> {
    value.filter(|value| value.is_object())
}

/// A window with a clamped used percentage (parseClaudeWindow).
pub fn parse_window(value: Option<&Value>) -> Option<Value> {
    let object = as_obj(value)?;
    let used = object.get("usedPct")?;
    let resets = object.get("resetsAt")?;
    if !is_num(used) || !is_num(resets) {
        return None;
    }
    Some(json!({
        "usedPct": used.as_f64()?.clamp(0.0, 100.0),
        "resetsAt": resets.as_i64()?,
    }))
}

/// Claude's plan usage from a status-line payload (parseClaudePlan).
pub fn parse_claude_plan(value: Option<&Value>, now: i64) -> Value {
    let object = as_obj(value);
    let five_hour = parse_window(object.and_then(|object| object.get("five_hour")));
    let seven_day = parse_window(object.and_then(|object| object.get("seven_day")));
    if five_hour.is_none() && seven_day.is_none() {
        return Value::Null;
    }
    let mut usage = json!({"updatedAt": now});
    if let Some(window) = five_hour {
        usage["fiveHour"] = window;
    }
    if let Some(window) = seven_day {
        usage["sevenDay"] = window;
    }
    usage
}

/// Codex's plan usage from its rate-limits response (parseCodexPlan).
pub fn parse_codex_plan(value: Option<&Value>, now: i64) -> Value {
    let object = as_obj(value);
    let Some(limits) = object.and_then(|object| as_obj(object.get("rateLimits"))) else {
        return Value::Null;
    };
    let mut usage = json!({"updatedAt": now});
    if let Some(plan_type) = limits.get("planType").and_then(Value::as_str).filter(|value| !value.is_empty()) {
        usage["planType"] = json!(plan_type);
    }
    for key in ["primary", "secondary"] {
        if let Some(window) = parse_window(limits.get(key)) {
            usage[key] = window;
        }
    }
    usage
}

/// Percentage of a window right now (effectivePct).
pub fn effective_pct(window: &Value, now: i64) -> f64 {
    if window["resetsAt"].as_i64().unwrap_or(0) <= now { 0.0 } else { window["usedPct"].as_f64().unwrap_or(0.0) }
}

/// The most used window of a plan (dominantPct).
pub fn dominant_pct(usage: Option<&Value>, now: i64) -> Option<f64> {
    let usage = as_obj(usage)?;
    let mut values = Vec::new();
    for key in ["fiveHour", "sevenDay", "primary", "secondary"] {
        if let Some(window) = as_obj(usage.get(key)) {
            values.push(effective_pct(window, now));
        }
    }
    values.into_iter().reduce(f64::max)
}

/// Gauge color of a percentage (planColor).
pub fn plan_color(pct: Option<f64>) -> &'static str {
    match pct {
        None => PLAN_COLORS[0],
        Some(pct) if pct < 50.0 => PLAN_COLORS[1],
        Some(pct) if pct < 80.0 => PLAN_COLORS[2],
        Some(_) => PLAN_COLORS[3],
    }
}

/// Pill label: "Claude 73%" or "Claude –" (pillLabel).
pub fn pill_label(name: &str, usage: Option<&Value>, now: i64) -> String {
    match dominant_pct(usage, now) {
        Some(pct) => format!("{name} {}%", pct.round() as i64),
        None => format!("{name} {PLAN_NONE}"),
    }
}

/// How long ago the numbers arrived (ageLabel).
pub fn age_label(updated_at: i64, now: i64) -> String {
    let secs = (now - updated_at).max(0) / 1_000;
    if secs < 60 {
        return "just now".to_owned();
    }
    let mins = secs / 60;
    if mins < 60 { format!("{mins} min ago") } else { format!("{} h ago", mins / 60) }
}

/// When a window resets (resetLabel).
pub fn reset_label(window: &Value, weekly: bool, now: i64) -> String {
    let secs = (window["resetsAt"].as_i64().unwrap_or(0) - now) / 1_000;
    if secs <= 0 {
        return "Resetting…".to_owned();
    }
    if weekly {
        return format!(
            "{} {:02}:{:02}",
            weekday_short(window["resetsAt"].as_i64().unwrap_or(0)),
            hour(window["resetsAt"].as_i64().unwrap_or(0)),
            minute(window["resetsAt"].as_i64().unwrap_or(0))
        );
    }
    let hours = secs / 3_600;
    let mins = (secs % 3_600) / 60;
    if hours > 0 { format!("in {hours} h {mins}") } else { format!("in {mins} min") }
}

/// Short weekday of an epoch-millisecond timestamp (weekdayShort).
pub fn weekday_short(ms: i64) -> &'static str {
    const DAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    let days = ms.div_euclid(DAY_MS);
    DAYS[(days + 4).rem_euclid(7) as usize]
}

/// Hour of an epoch-millisecond timestamp, UTC.
pub fn hour(ms: i64) -> i64 {
    ms.div_euclid(3_600_000).rem_euclid(24)
}

/// Minute of an epoch-millisecond timestamp, UTC.
pub fn minute(ms: i64) -> i64 {
    ms.div_euclid(60_000).rem_euclid(60)
}

/// Claude card subtitle (claudeSubtitle).
pub fn claude_subtitle(usage: Option<&Value>, now: i64) -> String {
    match as_obj(usage) {
        Some(usage) => age_label(usage["updatedAt"].as_i64().unwrap_or(now), now),
        None => "Waiting for a response from Claude Code".to_owned(),
    }
}

/// Codex card subtitle (codexSubtitle).
pub fn codex_subtitle(usage: Option<&Value>, now: i64) -> String {
    let Some(usage) = as_obj(usage) else {
        return "Asking Codex…".to_owned();
    };
    let prefix = usage["planType"]
        .as_str()
        .filter(|value| !value.is_empty())
        .map_or_else(String::new, |value| format!("{value} · "));
    format!("{}{}", prefix, age_label(usage["updatedAt"].as_i64().unwrap_or(now), now))
}

/// Codex reset credits label (codexResetsLabel).
pub fn codex_resets_label(usage: Option<&Value>) -> String {
    let Some(usage) = as_obj(usage) else {
        return PLAN_NONE.to_owned();
    };
    let Some(credits) = usage.get("resetCredits").and_then(Value::as_i64) else {
        return PLAN_NONE.to_owned();
    };
    let mut text = format!("{credits} available");
    if credits > 0 {
        if let Some(expires) = usage["resetCreditExpiresAt"].as_i64() {
            text.push_str(&format!(" — until {}", weekday_short(expires)));
        }
    }
    text
}

/// Whether the Codex numbers are missing or older than a minute (codexIsStale).
pub fn codex_is_stale(usage: Option<&Value>, now: i64) -> bool {
    match as_obj(usage) {
        None => true,
        Some(usage) => now - usage["updatedAt"].as_i64().unwrap_or(0) >= CODEX_STALE_MS,
    }
}

/// A stored Claude usage, if it still looks like one (restorePlanUsage).
pub fn restore_plan_usage(raw: Option<&str>) -> Value {
    let Some(raw) = raw.filter(|raw| !raw.is_empty()) else {
        return Value::Null;
    };
    let Ok(value) = serde_json::from_str::<Value>(raw) else {
        return Value::Null;
    };
    let Some(object) = as_obj(Some(&value)) else {
        return Value::Null;
    };
    let Some(updated_at) = object.get("updatedAt").filter(|value| is_num(value)) else {
        return Value::Null;
    };
    let five_hour = parse_window(object.get("fiveHour"));
    let seven_day = parse_window(object.get("sevenDay"));
    if five_hour.is_none() && seven_day.is_none() {
        return Value::Null;
    }
    let mut usage = json!({"updatedAt": updated_at});
    if let Some(window) = five_hour {
        usage["fiveHour"] = window;
    }
    if let Some(window) = seven_day {
        usage["sevenDay"] = window;
    }
    usage
}

/// One line describing a plan (planLine).
pub fn plan_line(name: &str, usage: Option<&Value>, now: i64) -> String {
    format!("{} · {}", pill_label(name, usage, now), plan_color(dominant_pct(usage, now)))
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(titles: usize, windows: usize, colors: usize, stale: bool) -> Vec<(&'static str, String)> {
    vec![
        ("titles", count_label(titles)),
        ("windows", count_label(windows)),
        ("colors", count_label(colors)),
        ("stale", if stale { "yes" } else { "no" }.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_and_windows() {
        assert_eq!(DAY_MS, 86_400_000);
        assert_eq!(CODEX_STALE_MS, 60_000);
        assert_eq!(PLAN_TITLES.len(), 4);
        assert_eq!(PLAN_WINDOWS.len(), 2);
        assert_eq!(PLAN_COLORS.len(), 4);
        assert_eq!(PLAN_NONE, "–");
        let window = json!({"usedPct": 150, "resetsAt": 10_000});
        assert_eq!(parse_window(Some(&window))["usedPct"], 100.0);
        assert_eq!(parse_window(None), None);
        assert_eq!(parse_window(Some(&json!({"usedPct":"x","resetsAt":1}))), None);
        assert_eq!(effective_pct(&parse_window(Some(&window)).unwrap(), 9_000), 100.0);
        assert_eq!(effective_pct(&parse_window(Some(&window)).unwrap(), 20_000), 0.0);
    }

    #[test]
    fn parsing() {
        let claude = json!({"five_hour":{"usedPct":73,"resetsAt":9_000},"seven_day":{"usedPct":12,"resetsAt":9_000}});
        let usage = parse_claude_plan(Some(&claude), 1_000);
        assert_eq!(usage["fiveHour"]["usedPct"], 73.0);
        assert_eq!(usage["updatedAt"], 1_000);
        assert_eq!(parse_claude_plan(None, 1_000), Value::Null);
        let codex = json!({"rateLimits":{"planType":"plus","primary":{"usedPct":42,"resetsAt":9_000},"secondary":{"usedPct":5,"resetsAt":9_000}}});
        let usage = parse_codex_plan(Some(&codex), 1_000);
        assert_eq!(usage["planType"], "plus");
        assert_eq!(usage["primary"]["usedPct"], 42.0);
        assert_eq!(parse_codex_plan(None, 1_000), Value::Null);
        let restored = restore_plan_usage(Some(&usage.to_string()));
        assert_eq!(restored["primary"]["usedPct"], 42.0);
        assert_eq!(restore_plan_usage(Some("not json")), Value::Null);
        assert_eq!(restore_plan_usage(None), Value::Null);
    }

    #[test]
    fn percentages_and_colors() {
        let usage = json!({"fiveHour":{"usedPct":73,"resetsAt":9_000},"updatedAt":1_000});
        assert_eq!(dominant_pct(Some(&usage), 1_000), Some(73.0));
        assert_eq!(dominant_pct(None, 1_000), None);
        assert_eq!(plan_color(None), "#6B7079");
        assert_eq!(plan_color(Some(30.0)), "#22C55E");
        assert_eq!(plan_color(Some(60.0)), "#F59E0B");
        assert_eq!(plan_color(Some(90.0)), "#F4505E");
        assert_eq!(pill_label("Claude", Some(&usage), 1_000), "Claude 73%");
        assert_eq!(pill_label("Claude", None, 1_000), "Claude –");
        assert!(plan_line("Codex", Some(&usage), 1_000).contains("Codex"));
    }

    #[test]
    fn labels() {
        assert_eq!(age_label(1_000, 10_000), "just now");
        assert_eq!(age_label(1_000, 61_000), "1 min ago");
        assert_eq!(age_label(0, 7_200_000), "2 h ago");
        assert_eq!(weekday_short(0), "Thu");
        assert_eq!(hour(3_600_000), 1);
        assert_eq!(minute(60_000), 1);
        let window = json!({"usedPct":50,"resetsAt":3_600_000});
        assert_eq!(reset_label(&window, false, 0), "in 1 h 0");
        assert_eq!(reset_label(&json!({"usedPct":50,"resetsAt":0}), false, 1_000), "Resetting…");
        assert!(reset_label(&window, true, 0).contains(":00"));
        assert_eq!(claude_subtitle(None, 1_000), "Waiting for a response from Claude Code");
        assert_eq!(codex_subtitle(None, 1_000), "Asking Codex…");
        assert_eq!(codex_subtitle(Some(&json!({"planType":"plus","updatedAt":1_000})), 2_000), "plus · just now");
        assert_eq!(codex_resets_label(Some(&json!({"resetCredits":2}))), "2 available");
        assert_eq!(codex_resets_label(None), "–");
        assert!(codex_is_stale(None, 0));
        assert!(!codex_is_stale(Some(&json!({"updatedAt":1_000})), 10_000));
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(4, 2, 4, false).len(), 4);
    }
}
