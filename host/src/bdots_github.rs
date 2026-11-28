#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from coucou's `windows/src/core/github.ts`: GitHub pulse parsing, CI verdicts,
//! safe links and the contribution grid. Composes helpers already ported into this host. Only
//! `botInsights` calls this.

use serde_json::{Value, json};

/// CI states of a pull request (CIState).
pub const CI_STATES: [&str; 4] = ["pending", "success", "failure", "unknown"];

/// Review states of a pull request (ReviewState).
pub const REVIEW_STATES: [&str; 4] = ["approved", "changesRequested", "pending", "unknown"];

/// Sections of the GitHub card (GitHubSection).
pub const GH_SECTIONS: [&str; 4] = ["myPRs", "toReview", "mainCI", "activity"];

/// Colors of CI states (CI_COLORS).
pub const CI_COLORS: [(&str, &str); 4] =
    [("pending", "#d29922"), ("success", "#3fb950"), ("failure", "#f85149"), ("unknown", "#8b949e")];

/// Size of a contribution square, in points (GRID_SQUARE).
pub const GRID_SQUARE: f64 = 7.0;

/// Gap between contribution squares, in points (GRID_GAP).
pub const GRID_GAP: f64 = 1.5;

/// Month short names for day labels.
const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/// Color of a contribution level (contributionColor).
pub fn contribution_color(level: u8) -> &'static str {
    match level {
        1 => "#0E4429",
        2 => "#006D32",
        3 => "#26A641",
        4 => "#39D353",
        _ => "rgba(255,255,255,0.06)",
    }
}

/// CI color by state.
fn ci_color(state: &str) -> &'static str {
    CI_COLORS.iter().find(|(kind, _)| *kind == state).map_or(CI_COLORS[3].1, |(_, color)| *color)
}

/// Read the pulse object sent by Rust (readPulse).
pub fn read_pulse(data: &Value) -> Value {
    let pulse = &data["pulse"];
    if !pulse.is_object() || !pulse["myPRs"].is_array() || !pulse["toReview"].is_array() || !pulse["mainCI"].is_array()
    {
        return Value::Null;
    }
    pulse.clone()
}

/// Read the contribution activity sent by Rust (readActivity).
pub fn read_activity(data: &Value) -> Value {
    let activity = &data["activity"];
    if !activity.is_object() || !activity["weeks"].is_array() || activity["total"].as_u64().is_none() {
        return Value::Null;
    }
    activity.clone()
}

/// Read repo and star totals (readStats).
pub fn read_stats(data: &Value) -> Value {
    if data["totalRepos"].is_null() {
        return Value::Null;
    }
    json!({
        "totalRepos": data["totalRepos"].as_u64().unwrap_or(0),
        "totalStars": data["totalStars"].as_u64().unwrap_or(0),
    })
}

/// Worst CI state, failure first (worstCI).
pub fn worst_ci(states: &[String]) -> &'static str {
    if states.iter().any(|state| state == "failure") {
        "failure"
    } else if states.iter().any(|state| state == "pending") {
        "pending"
    } else if states.iter().any(|state| state == "success") {
        "success"
    } else {
        "unknown"
    }
}

/// Value of the "My PRs" row (myPRsValue).
pub fn my_prs_value(prs: &[Value]) -> String {
    let count = prs.len();
    if count == 0 {
        return "0".to_owned();
    }
    let failing = prs.iter().filter(|pr| pr["ci"].as_str() == Some("failure")).count();
    if failing > 0 {
        return format!("{count} · {failing} failing");
    }
    if prs.iter().any(|pr| pr["ci"].as_str() == Some("pending")) {
        return format!("{count} · running");
    }
    count.to_string()
}

/// Summary of the default-branch CI row (mainCISummary).
pub fn main_ci_summary(repos: &[Value]) -> Value {
    let states: Vec<String> = repos.iter().map(|repo| repo["ci"].as_str().unwrap_or("unknown").to_owned()).collect();
    let worst = worst_ci(&states);
    match worst {
        "failure" => json!({
            "failing": true,
            "color": ci_color("failure"),
            "value": format!("{} failing", repos.iter().filter(|r| r["ci"].as_str() == Some("failure")).count()),
        }),
        "pending" => json!({"failing": false, "color": ci_color("pending"), "value": "running"}),
        "success" => json!({"failing": false, "color": ci_color("success"), "value": "all green"}),
        _ => json!({"failing": false, "color": ci_color("unknown"), "value": "—"}),
    }
}

/// Word for a CI state, when it has one (ciWord).
pub fn ci_word(state: &str) -> Option<&'static str> {
    match state {
        "failure" => Some("failing"),
        "pending" => Some("running"),
        "success" => Some("passing"),
        _ => None,
    }
}

/// Title of a card section (sectionTitle).
pub fn section_title(section: &str) -> &'static str {
    match section {
        "myPRs" => "My PRs",
        "toReview" => "To review",
        "mainCI" => "Default branch CI",
        "activity" => "Activity",
        _ => "GitHub",
    }
}

/// "1.2k" above a thousand (formatCount).
pub fn format_count(n: u64) -> String {
    if n >= 1_000 { format!("{:.1}k", n as f64 / 1_000.0) } else { n.to_string() }
}

/// "owner/repo" → "repo" (shortRepo).
pub fn short_repo(repo: &str) -> &str {
    repo.rsplit('/').next().unwrap_or(repo)
}

/// Only http(s) links to github.com open (safeGitHubUrl).
pub fn safe_github_url(raw: &str) -> Option<&str> {
    let value = raw.trim();
    if !value.starts_with("https://github.com") && !value.starts_with("http://github.com") {
        return None;
    }
    let rest = value.split_once("github.com").map(|(_, rest)| rest)?;
    if rest.starts_with('/') || rest.is_empty() { Some(value) } else { None }
}

/// A repository's Actions page (actionsUrl).
pub fn actions_url(repo_url: &str) -> String {
    if repo_url.ends_with('/') { format!("{repo_url}actions") } else { format!("{repo_url}/actions") }
}

/// A user's profile URL (profileUrl).
pub fn profile_url(login: &str) -> String {
    format!("https://github.com/{login}")
}

/// The last `n` weeks, oldest first (lastWeeks).
pub fn last_weeks(activity: &Value, n: usize) -> Vec<Value> {
    if n == 0 {
        return Vec::new();
    }
    let weeks = activity["weeks"].as_array().cloned().unwrap_or_default();
    let start = weeks.len().saturating_sub(n);
    weeks[start..].to_vec()
}

/// The last `n` days, regraded against the busiest of those days (lastDays).
pub fn last_days(activity: &Value, n: usize) -> Vec<Value> {
    if n == 0 {
        return Vec::new();
    }
    let days: Vec<Value> = activity["weeks"]
        .as_array()
        .map(|weeks| weeks.iter().flat_map(|week| week.as_array().cloned().unwrap_or_default()).collect())
        .unwrap_or_default();
    let start = days.len().saturating_sub(n);
    let chosen = &days[start..];
    let busiest = chosen.iter().map(|day| day["count"].as_u64().unwrap_or(0)).max().unwrap_or(0);
    if busiest == 0 {
        return chosen.to_vec();
    }
    chosen
        .iter()
        .map(|day| {
            let count = day["count"].as_u64().unwrap_or(0);
            let level = if count == 0 { 0 } else { ((4 * count + busiest - 1) / busiest).max(1) as u8 };
            let mut out = day.clone();
            out["level"] = json!(level);
            out
        })
        .collect()
}

/// One grid column, Sunday first, with nulls for missing days (weekColumn).
pub fn week_column(week: &[Value]) -> Vec<Value> {
    (0..7)
        .map(|dow| week.iter().find(|day| day["weekday"].as_u64() == Some(dow)).cloned().unwrap_or(Value::Null))
        .collect()
}

/// "2026-01-05" → "Jan 5" (dayLabel).
pub fn day_label(date: &str) -> String {
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() == 3 {
        if let Ok(month) = parts[1].parse::<usize>() {
            if (1..=12).contains(&month) {
                return format!("{} {}", MONTHS[month - 1], parts[2].trim_start_matches('0'));
            }
        }
    }
    date.to_owned()
}

/// Label of a contribution count (contributionsLabel).
pub fn contributions_label(count: u64) -> String {
    format!("{} contributions", format_count(count))
}

/// Label of a total (totalLabel).
pub fn total_label(n: u64) -> String {
    format_count(n)
}

/// Header of the activity detail (activityHeader).
pub fn activity_header(activity: &Value) -> String {
    let total = activity["total"].as_u64().unwrap_or(0);
    let weeks = activity["weeks"].as_array().map_or(0, Vec::len);
    format!("{} · {} weeks", contributions_label(total), weeks)
}

/// First alert worth showing (gitHubAlert).
pub fn github_alert(events: &[Value]) -> Value {
    events
        .iter()
        .find_map(|event| match event["kind"].as_str().unwrap_or("") {
            "ciFailed" => Some(json!({"kind": "ciFailed", "text": "CI failed", "id": event["prId"]})),
            "mainFailed" => Some(json!({"kind": "mainFailed", "text": "Default branch failed", "id": event["repo"]})),
            "reviewRequested" => {
                Some(json!({"kind": "reviewRequested", "text": "Review requested", "id": event["prId"]}))
            }
            "ciPassed" => Some(json!({"kind": "ciPassed", "text": "CI passed", "id": event["prId"]})),
            _ => None,
        })
        .unwrap_or(Value::Null)
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(prs: usize, repos: usize, failures: usize, green: bool) -> Vec<(&'static str, String)> {
    vec![
        ("prs", count_label(prs)),
        ("repos", count_label(repos)),
        ("failures", count_label(failures)),
        ("green", if green { "yes" } else { "no" }.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn colors_and_states() {
        assert_eq!(contribution_color(0), "rgba(255,255,255,0.06)");
        assert_eq!(contribution_color(4), "#39D353");
        assert_eq!(ci_color("success"), "#3fb950");
        let states = vec!["success".to_owned(), "failure".to_owned()];
        assert_eq!(worst_ci(&states), "failure");
        assert_eq!(worst_ci(&[]), "unknown");
        assert_eq!(ci_word("pending"), Some("running"));
        assert_eq!(ci_word("unknown"), None);
        assert_eq!(section_title("mainCI"), "Default branch CI");
    }

    #[test]
    fn parsing_and_values() {
        let pulse = json!({"pulse":{"myPRs":[{"ci":"failure"}],"toReview":[],"mainCI":[]}});
        assert!(read_pulse(&pulse).is_object());
        assert_eq!(read_pulse(&json!({})), Value::Null);
        let activity =
            json!({"activity":{"total":42,"weeks":[[{"date":"2026-01-05","count":2,"level":1,"weekday":1}]]}});
        assert_eq!(read_activity(&activity)["total"], 42);
        assert_eq!(read_activity(&json!({})), Value::Null);
        assert_eq!(read_stats(&json!({"totalRepos":3,"totalStars":10}))["totalStars"], 10);
        assert_eq!(my_prs_value(&[json!({"ci":"failure"})]), "1 · 1 failing");
        assert_eq!(my_prs_value(&[]), "0");
        let ci = main_ci_summary(&[json!({"ci":"failure"})]);
        assert_eq!(ci["failing"], true);
    }

    #[test]
    fn links_and_grid() {
        assert_eq!(format_count(999), "999");
        assert_eq!(format_count(1_200), "1.2k");
        assert_eq!(short_repo("owner/repo"), "repo");
        assert!(safe_github_url("https://github.com/o/r").is_some());
        assert_eq!(safe_github_url("javascript:alert(1)"), None);
        assert_eq!(actions_url("https://github.com/o/r"), "https://github.com/o/r/actions");
        assert_eq!(profile_url("ana"), "https://github.com/ana");
        let activity = json!({"weeks":[[{"date":"2026-01-05","count":4,"level":1,"weekday":1}],[{"date":"2026-01-06","count":8,"level":1,"weekday":2}]]});
        assert_eq!(last_weeks(&activity, 1).len(), 1);
        let days = last_days(&activity, 2);
        assert_eq!(days[1]["level"], 4);
        assert_eq!(week_column(&days).len(), 7);
        assert_eq!(day_label("2026-01-05"), "Jan 5");
    }

    #[test]
    fn alerts_and_summary() {
        assert_eq!(contributions_label(1200), "1.2k contributions");
        assert_eq!(total_label(42), "42");
        assert!(activity_header(&json!({"total":42,"weeks":[]})).contains("42"));
        let alert = github_alert(&[json!({"kind":"ciFailed","prId":"p1"})]);
        assert_eq!(alert["kind"], "ciFailed");
        assert_eq!(github_alert(&[]), Value::Null);
        assert_eq!(CI_STATES.len(), 4);
        assert_eq!(REVIEW_STATES.len(), 4);
        assert_eq!(GH_SECTIONS.len(), 4);
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(2, 1, 1, false).len(), 4);
    }
}
