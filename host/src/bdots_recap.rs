#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from coucou's `windows/src/recap/summary.ts`: week dates, auto-show, agent
//! names, merged turn durations and weekly summaries. Only `botInsights` calls this.

use serde_json::{Value, json};

/// Hour the Monday recap opens (RECAP_HOUR).
pub const RECAP_HOUR: u64 = 8;

/// Days in a recap week.
pub const WEEK_DAYS: i64 = 7;

/// Default recap preference (DEFAULT_PREFS.enabled).
pub const DEFAULT_ENABLED: bool = true;

/// Names of weekdays for labels.
pub const DAY_NAMES: [&str; 7] = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];

/// Known pill ids and their display names (AGENT_NAMES).
pub const AGENT_NAMES: [(&str, &str); 5] = [
    ("integration_claude", "Claude Code"),
    ("agent_codex", "Codex"),
    ("agent_cursor", "Cursor"),
    ("agent_gemini", "Gemini"),
    ("agent_hermes", "Hermes"),
];

/// Named project colors, shared with the island layout.
const PROJECTS: [&str; 4] = ["coucou", "codync", "side-project", "work"];

/// Add `n` days to a YYYY-MM-DD key, returning a new key (addDays).
pub fn add_days(day: &str, n: i64) -> String {
    let parts: Vec<i64> = day.split('-').filter_map(|p| p.parse().ok()).collect();
    if parts.len() != 3 {
        return day.to_owned();
    }
    let mut days = days_from_civil(parts[0], parts[1] as u32, parts[2] as u32) + n;
    let (year, month, date) = civil_from_days(days);
    format!("{year:04}-{month:02}-{date:02}")
}

/// Days from civil date (Howard Hinnant's algorithm).
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let y = year - i64::from(month <= 2);
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = i64::from(month) + if month > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + i64::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Civil date from days since epoch.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (year, month as u32, day as u32)
}

/// Weekday index, Sunday first (0..6), for a YYYY-MM-DD key.
pub fn weekday(day: &str) -> usize {
    (days_from_civil(
        day.split('-').next().and_then(|p| p.parse().ok()).unwrap_or(1970),
        day.split('-').nth(1).and_then(|p| p.parse().ok()).unwrap_or(1),
        day.split('-').nth(2).and_then(|p| p.parse().ok()).unwrap_or(1),
    ) + 4)
        .rem_euclid(7) as usize
}

/// Monday key of the week holding `day` (mondayOf).
pub fn monday_of(day: &str) -> String {
    let since_monday = (weekday(day) + 6) % 7;
    add_days(day, -(since_monday as i64))
}

/// Monday key of the last completed week (previousWeekStart).
pub fn previous_week_start(day: &str) -> String {
    add_days(&monday_of(day), -7)
}

/// Day key of a date (dayKey); here it's already YYYY-MM-DD.
pub fn day_key(day: &str) -> &str {
    day
}

/// Whether the Monday recap opens on its own (shouldAutoShow).
pub fn should_auto_show(day: &str, hour: u64, last_shown_week: &str) -> bool {
    weekday(day) == 1 && hour >= RECAP_HOUR && monday_of(day) != last_shown_week
}

/// Display name of a pill id (agentName).
pub fn agent_name(id: &str) -> String {
    if let Some((_, name)) = AGENT_NAMES.iter().find(|(candidate, _)| *candidate == id) {
        return (*name).to_owned();
    }
    let raw = id.strip_prefix("agent_").unwrap_or(id);
    let mut chars = raw.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
        None => id.to_owned(),
    }
}

/// Merge overlapping turn intervals into a total duration (mergedSeconds).
pub fn merged_seconds(turns: &[Value]) -> i64 {
    let mut sorted: Vec<(i64, i64)> = turns
        .iter()
        .filter_map(|turn| {
            let start = turn["start"].as_i64()?;
            let end = turn["end"].as_i64()?;
            (end > start).then_some((start, end))
        })
        .collect();
    sorted.sort_by_key(|(start, _)| *start);
    let mut total = 0_i64;
    let mut segment_start = 0_i64;
    let mut segment_end = i64::MIN;
    for (start, end) in sorted {
        if start <= segment_end {
            segment_end = segment_end.max(end);
        } else {
            if segment_end > segment_start {
                total += segment_end - segment_start;
            }
            segment_start = start;
            segment_end = end;
        }
    }
    if segment_end > segment_start {
        total += segment_end - segment_start;
    }
    total
}

/// Duration in words: 42m, 3h, 2h 15m (formatDuration).
pub fn format_duration(minutes: i64) -> String {
    if minutes < 60 {
        return format!("{minutes}m");
    }
    let hours = minutes / 60;
    let rest = minutes % 60;
    if rest == 0 { format!("{hours}h") } else { format!("{hours}h {rest}m") }
}

/// Stat-chip count: 1,234 up to 10k, then 12.3k (formatCount).
pub fn format_count(n: u64) -> String {
    if n >= 10_000 {
        let decimals = if n >= 100_000 { 0 } else { 1 };
        format!("{:.*}k", decimals, n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

/// Label of a week range: "Sep 28 - Oct 4" (weekRangeLabel).
pub fn week_range_label(start: &str, end: &str) -> String {
    let month_name = |day: &str| {
        let month: usize = day.split('-').nth(1).and_then(|p| p.parse().ok()).unwrap_or(1);
        (month > 0 && month <= 12).then(|| MONTHS[month - 1]).unwrap_or("")
    };
    let short = |day: &str| {
        let date = day.split('-').nth(2).unwrap_or("").trim_start_matches('0');
        format!("{} {}", month_name(day), date)
    };
    format!("{} - {}", short(start), short(end))
}

/// Months for range labels.
const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/// Whether a project is one of the named ones.
pub fn is_named_project(name: &str) -> bool {
    PROJECTS.contains(&name)
}

/// Summarize a week of turns: count by agent, project and day (summarize).
pub fn summarize(turns: &[Value], week_start: &str) -> Value {
    let end = add_days(week_start, 7);
    let in_week: Vec<&Value> = turns
        .iter()
        .filter(|turn| {
            let day = turn["day"].as_str().unwrap_or("");
            day >= week_start && day < end.as_str()
        })
        .collect();
    if in_week.is_empty() {
        return Value::Null;
    }
    let mut agents: Vec<String> = Vec::new();
    let mut projects: Vec<String> = Vec::new();
    let mut days: Vec<String> = Vec::new();
    for turn in &in_week {
        let agent = agent_name(turn["agent"].as_str().unwrap_or(""));
        if !agents.contains(&agent) {
            agents.push(agent);
        }
        let project = turn["project"].as_str().unwrap_or("").to_owned();
        if !project.is_empty() && !projects.contains(&project) {
            projects.push(project);
        }
        let day = turn["day"].as_str().unwrap_or("").to_owned();
        if !days.contains(&day) {
            days.push(day);
        }
    }
    json!({
        "weekStart": week_start,
        "weekEnd": end,
        "turns": in_week.len(),
        "agents": agents,
        "projects": projects,
        "days": days.len(),
        "seconds": merged_seconds(turns),
    })
}

/// One line describing a recap week (recapLine).
pub fn recap_line(start: &str, turns: usize, minutes: i64) -> String {
    format!("{} · {} turns · {}", week_range_label(start, &add_days(start, 6)), format_duration(minutes))
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(weeks: usize, turns: usize, agents: usize, auto: bool) -> Vec<(&'static str, String)> {
    vec![
        ("weeks", count_label(weeks)),
        ("turns", count_label(turns)),
        ("agents", count_label(agents)),
        ("auto", if auto { "yes" } else { "no" }.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn week_dates() {
        assert_eq!(RECAP_HOUR, 8);
        assert_eq!(DAY_NAMES.len(), 7);
        assert_eq!(weekday("2026-11-23"), 1);
        assert_eq!(monday_of("2026-11-27"), "2026-11-23");
        assert_eq!(monday_of("2026-11-23"), "2026-11-23");
        assert_eq!(previous_week_start("2026-11-27"), "2026-11-16");
        assert_eq!(add_days("2026-11-30", 1), "2026-12-01");
        assert_eq!(day_key("2026-11-27"), "2026-11-27");
        assert!(should_auto_show("2026-11-23", 9, "2026-11-09"));
        assert!(!should_auto_show("2026-11-23", 7, "2026-11-09"));
        assert!(!should_auto_show("2026-11-24", 9, "2026-11-09"));
    }

    #[test]
    fn names_and_durations() {
        assert_eq!(agent_name("integration_claude"), "Claude Code");
        assert_eq!(agent_name("agent_codex"), "Codex");
        assert_eq!(agent_name("custom"), "Custom");
        assert_eq!(format_duration(42), "42m");
        assert_eq!(format_duration(180), "3h");
        assert_eq!(format_duration(135), "2h 15m");
        assert_eq!(format_count(1234), "1234");
        assert_eq!(format_count(12_345), "12.3k");
        assert_eq!(format_count(123_456), "123k");
        assert_eq!(week_range_label("2026-09-28", "2026-10-04"), "Sep 28 - Oct 4");
        assert!(is_named_project("coucou"));
        assert!(!is_named_project("other"));
    }

    #[test]
    fn merging_and_summary() {
        let turns = vec![
            json!({"start":0,"end":60,"day":"2026-11-23","agent":"integration_claude","project":"coucou"}),
            json!({"start":30,"end":120,"day":"2026-11-23","agent":"agent_codex","project":"codync"}),
            json!({"start":200,"end":260,"day":"2026-11-24","agent":"integration_claude","project":"coucou"}),
        ];
        assert_eq!(merged_seconds(&turns), 180);
        let week = summarize(&turns, "2026-11-23");
        assert_eq!(week["turns"], 3);
        assert_eq!(week["agents"].as_array().map_or(0, Vec::len), 2);
        assert_eq!(week["days"], 2);
        assert_eq!(summarize(&turns, "2026-11-02"), Value::Null);
    }

    #[test]
    fn presentation() {
        assert!(recap_line("2026-11-23", 3, 135).contains("2h 15m"));
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(1, 3, 2, true).len(), 4);
    }
}
