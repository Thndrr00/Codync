#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from agentes-chat's `missions.ts`: plan strips, waiting parts, elapsed spans,
//! task sessions, pull requests, composer commands and message meta. Composes helpers already
//! ported into this host. Only `botInsights` calls this.

use serde_json::{Value, json};

/// Cells of a mission plan strip (PlanCell).
pub const PLAN_CELLS: [&str; 6] = ["done", "doing", "waiting", "stopped", "dropped", "todo"];

/// Waiting parts, in card order (WaitingKey).
pub const WAITING_KEYS: [&str; 6] = ["decisions", "provisional", "reviews", "suggestions", "commands", "deliveries"];

/// Composer commands of a mission (MISSION_COMMANDS).
pub const MISSION_COMMANDS: [&str; 6] = ["mission", "stop", "pause", "resume", "summary", "plan"];

/// States of a session row (SessionState).
pub const SESSION_STATES: [&str; 6] = ["running", "done", "failed", "interrupted", "canceled", "queued"];

/// Lines of `/summary` (SummaryKey).
pub const SUMMARY_KEYS: [&str; 7] = ["plan", "doing", "running", "decisions", "provisional", "reviews", "waiting"];

/// Most output lines kept for a session (appendOutputLine max).
pub const OUTPUT_MAX_LINES: usize = 200;

/// Cell of one plan item (planCell).
pub fn plan_cell(status: &str, offline: bool) -> &'static str {
    match status {
        "done" => "done",
        "doing" => {
            if offline {
                "stopped"
            } else {
                "doing"
            }
        }
        "waiting_you" => "waiting",
        "dropped" => "dropped",
        _ => "todo",
    }
}

/// Strip from plan items, dropping dropped items (planCells).
pub fn plan_cells(items: &[Value], offline: bool) -> Vec<&'static str> {
    items
        .iter()
        .filter(|item| item["status"].as_str() != Some("dropped"))
        .map(|item| plan_cell(item["status"].as_str().unwrap_or(""), offline))
        .collect()
}

/// Strip from counts only (cellsFromCounts).
pub fn cells_from_counts(
    done: usize,
    total: usize,
    waiting: usize,
    running: usize,
    offline: bool,
) -> Vec<&'static str> {
    let total = total;
    let done = done.min(total);
    let waiting = waiting.min(total - done);
    let doing = running.min(total - done - waiting);
    let mut cells = vec!["done"; done];
    cells.extend(vec!["waiting"; waiting]);
    cells.extend(vec![if offline { "stopped" } else { "doing" }; doing]);
    cells.extend(vec!["todo"; total - done - waiting - doing]);
    cells
}

/// Strip of a mission view: items when present, counts otherwise (missionCells).
pub fn mission_cells(mission: &Value, offline: bool) -> Vec<&'static str> {
    let items = mission["plan"]["items"].as_array();
    if let Some(items) = items {
        if items.iter().any(|item| item["status"].as_str() != Some("dropped")) {
            return plan_cells(items, offline);
        }
    }
    let plan = &mission["plan"];
    let doing =
        plan["doing"].as_u64().map_or(mission["runningTasks"].as_u64().unwrap_or(0) as usize, |value| value as usize);
    cells_from_counts(
        plan["done"].as_u64().unwrap_or(0) as usize,
        plan["total"].as_u64().unwrap_or(0) as usize,
        plan["waiting"].as_u64().unwrap_or(0) as usize,
        doing,
        offline,
    )
}

/// Counts of live plan items (planCounts).
pub fn plan_counts(items: &[Value]) -> Value {
    let live: Vec<&Value> = items.iter().filter(|item| item["status"].as_str() != Some("dropped")).collect();
    json!({
        "total": live.len(),
        "done": live.iter().filter(|item| item["status"].as_str() == Some("done")).count(),
        "doing": live.iter().filter(|item| item["status"].as_str() == Some("doing")).count(),
        "waiting": live.iter().filter(|item| item["status"].as_str() == Some("waiting_you")).count(),
    })
}

/// Whether a mission is still being worked on (isOpenMission).
pub fn is_open_mission(status: &str) -> bool {
    matches!(status, "proposed" | "active" | "paused" | "awaiting_close")
}

/// What waits for people, in card order (waitingParts).
pub fn waiting_parts(mission: &Value) -> Vec<Value> {
    if mission["canAct"] != true {
        return Vec::new();
    }
    WAITING_KEYS
        .iter()
        .filter_map(|key| {
            let count = mission["pending"][key].as_u64().unwrap_or(0);
            (count > 0).then(|| json!({"key": key, "count": count}))
        })
        .collect()
}

/// How often an elapsed label must tick (tickFor), in milliseconds.
pub fn tick_for(age_seconds: u64) -> u64 {
    if age_seconds < 60 {
        1_000
    } else if age_seconds < 3_600 {
        15_000
    } else {
        60_000
    }
}

/// A span in words (formatSpan).
pub fn format_span(seconds: u64) -> String {
    let total = seconds;
    if total < 60 {
        return format!("{total} s");
    }
    let minutes = total / 60;
    if minutes < 60 {
        return format!("{minutes} min");
    }
    let hours = minutes / 60;
    if hours < 24 {
        let rest = minutes % 60;
        return if rest == 0 { format!("{hours} h") } else { format!("{hours} h {rest:02}") };
    }
    format!("{} d", hours / 24)
}

/// Epoch milliseconds of an ISO timestamp, or None.
fn parse_iso_ms(text: &str) -> Option<i64> {
    let text = text.trim();
    let date = text.get(0..10)?;
    let year: i64 = date.get(0..4)?.parse().ok()?;
    let month: i64 = date.get(5..7)?.parse().ok()?;
    let day: i64 = date.get(8..10)?.parse().ok()?;
    let time = text.get(11..19).unwrap_or("00:00:00");
    let hour: i64 = time.get(0..2)?.parse().ok()?;
    let minute: i64 = time.get(3..5)?.parse().ok()?;
    let second: i64 = time.get(6..8)?.parse().ok()?;
    let shifted = year - i64::from(month <= 2);
    let era = if shifted >= 0 { shifted } else { shifted - 399 } / 400;
    let year_of_era = shifted - era * 400;
    let month_prime = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * month_prime + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = era * 146_097 + day_of_era - 719_468;
    let mut millis = ((days * 24 + hour) * 60 + minute) * 60 * 1_000 + second * 1_000;
    if let Some(fraction) = text.get(19..).and_then(|rest| rest.strip_prefix('.')) {
        let digits: String = fraction.chars().take_while(char::is_ascii_digit).take(3).collect();
        for (index, digit) in digits.chars().enumerate() {
            millis += i64::from(digit.to_digit(10).unwrap_or(0)) * 10_i64.pow(2 - index as u32);
        }
    }
    Some(millis)
}

/// How long a task has run or ran, in seconds (taskSeconds).
pub fn task_seconds(task: &Value, now_ms: i64) -> u64 {
    if task["status"].as_str() == Some("running") {
        if let Some(started) = task["startedAt"].as_str().and_then(parse_iso_ms) {
            return (now_ms - started).max(0) as u64 / 1_000;
        }
    }
    task["seconds"].as_u64().unwrap_or(0)
}

/// Tasks of a mission, running first then newest (missionTasks).
pub fn mission_tasks(tasks: &[Value], mission_id: &str) -> Vec<Value> {
    let rank = |task: &Value| match task["status"].as_str().unwrap_or("") {
        "running" => 0,
        "queued" => 1,
        _ => 2,
    };
    let mut out: Vec<Value> = tasks
        .iter()
        .filter(|task| task["missionId"].as_str() == Some(mission_id) && task["resumedTaskId"].is_null())
        .cloned()
        .collect();
    out.sort_by(|a, b| {
        rank(a)
            .cmp(&rank(b))
            .then_with(|| b["createdAt"].as_str().unwrap_or("").cmp(a["createdAt"].as_str().unwrap_or("")))
    });
    out
}

/// A pull request named in text (prOf).
pub fn pr_of(text: Option<&str>) -> Option<Value> {
    let text = text?;
    if let Some(index) = text.find("https://github.com/") {
        let rest = &text[index..];
        if let Some(pull) = rest.find("/pull/") {
            let digits: String = rest[pull + 6..].chars().take_while(char::is_ascii_digit).collect();
            if !digits.is_empty() {
                let url: String = rest.chars().take_while(|c| !c.is_whitespace()).collect();
                return Some(json!({"number": digits, "url": url}));
            }
        }
    }
    if let Some(index) = text.to_ascii_lowercase().find("pr #") {
        let digits: String = text[index + 4..].chars().take_while(char::is_ascii_digit).collect();
        if !digits.is_empty() {
            return Some(json!({"number": digits, "url": Value::Null}));
        }
    }
    None
}

/// Pull request of a session (taskPr).
pub fn task_pr(task: &Value) -> Option<Value> {
    if let Some(url) = task["prUrl"].as_str().filter(|value| !value.is_empty()) {
        let number = if let Some(index) = url.find("/pull/") {
            url[index + 6..].chars().take_while(char::is_ascii_digit).collect::<String>()
        } else if let Some(index) = url.find("/merge_requests/") {
            url[index + 16..].chars().take_while(char::is_ascii_digit).collect::<String>()
        } else {
            String::new()
        };
        return Some(json!({"number": number, "url": url}));
    }
    pr_of(task["lastLine"].as_str())
}

/// Append a live output line when it is new (appendOutputLine).
pub fn append_output_line(lines: &[String], line: Option<&str>, max: usize) -> Vec<String> {
    let clean = line.unwrap_or("").trim();
    if clean.is_empty() || lines.last().is_some_and(|last| last == clean) {
        return lines.to_vec();
    }
    let mut out = lines.to_vec();
    out.push(clean.to_owned());
    if out.len() > max {
        out.drain(..out.len() - max);
    }
    out
}

/// Titles of an item list in a plan change.
fn item_titles(items: Option<&Vec<Value>>) -> Vec<String> {
    items
        .map(|values| values.iter().filter_map(|item| item["title"].as_str().map(str::to_owned)).collect())
        .unwrap_or_default()
}

/// Parts of a plan change line (planChangeParts).
pub fn plan_change_parts(change: &Value) -> Value {
    let added = change["added"].as_array();
    let removed = change["removed"].as_array();
    let changed = change["changed"].as_array().cloned().unwrap_or_default();
    let done = changed
        .iter()
        .filter(|item| item["to"].as_str() == Some("done"))
        .filter_map(|item| item["title"].as_str().map(str::to_owned))
        .collect::<Vec<_>>();
    let renamed = changed
        .iter()
        .filter(|item| item["renamedFrom"].as_str().is_some())
        .map(|item| json!({"from": item["renamedFrom"].as_str().unwrap_or(""), "to": item["title"].as_str().unwrap_or("")}))
        .collect::<Vec<_>>();
    json!({"added": item_titles(added), "removed": item_titles(removed), "done": done, "renamed": renamed})
}

/// Lines of `/summary` (summaryParts).
pub fn summary_parts(summary: &Value) -> Vec<Value> {
    let plan = &summary["plan"];
    let mut parts = vec![
        json!({"key": "plan", "count": plan["done"].as_u64().unwrap_or(0), "total": plan["total"].as_u64().unwrap_or(0)}),
    ];
    let rest = [
        ("doing", plan["doing"].as_u64().unwrap_or(0)),
        ("running", summary["running"].as_u64().unwrap_or(0)),
        ("decisions", summary["pendingDecisions"].as_u64().unwrap_or(0)),
        ("provisional", summary["provisionalDecisions"].as_u64().unwrap_or(0)),
        ("reviews", summary["pendingReviews"].as_u64().unwrap_or(0)),
        ("waiting", plan["waiting"].as_u64().unwrap_or(0)),
    ];
    for (key, count) in rest {
        if count > 0 {
            parts.push(json!({"key": key, "count": count}));
        }
    }
    parts
}

/// Composer command and its argument (parseMissionCommand).
pub fn parse_mission_command(text: &str) -> Option<Value> {
    let trimmed = text.trim();
    let body = trimmed.strip_prefix('/')?;
    let (name, arg) = match body.split_once(char::is_whitespace) {
        Some((name, arg)) => (name, arg.trim()),
        None => (body, ""),
    };
    if !MISSION_COMMANDS.iter().any(|command| command.eq_ignore_ascii_case(name)) {
        return None;
    }
    let command = name.to_ascii_lowercase();
    if command == "mission" && arg.is_empty() {
        return None;
    }
    if command != "mission" && !arg.is_empty() {
        return None;
    }
    Some(json!({"command": command, "arg": arg}))
}

/// A mission link from message meta (missionLinkOf).
pub fn mission_link_of(meta: &Value) -> Value {
    let raw = &meta["missionLink"];
    let mission_id = raw["missionId"].as_str().unwrap_or("");
    let thread_id = raw["threadId"].as_str().unwrap_or("");
    if !raw.is_object() || mission_id.is_empty() || thread_id.is_empty() {
        Value::Null
    } else {
        json!({"missionId": mission_id, "threadId": thread_id})
    }
}

/// A mission plan from message meta (missionPlanOf).
pub fn mission_plan_of(meta: &Value) -> Value {
    let raw = &meta["missionPlan"];
    let mission_id = raw["missionId"].as_str().unwrap_or("");
    if !raw.is_object() || mission_id.is_empty() {
        return Value::Null;
    }
    json!({
        "missionId": mission_id,
        "title": raw["title"].as_str().unwrap_or(""),
        "goal": raw["goal"].as_str().unwrap_or(""),
        "computer": raw["computer"].as_str().unwrap_or(""),
        "executor": if raw["executor"].as_str().unwrap_or("").is_empty() { "claude" } else { raw["executor"].as_str().unwrap_or("") },
    })
}

/// A decision/review/delivery card reference from message meta (missionCardRef).
pub fn mission_card_ref(meta: &Value, key: &str) -> Value {
    let raw = &meta[key];
    let id = raw["id"].as_str().unwrap_or("");
    let mission_id = raw["missionId"].as_str().unwrap_or("");
    if !raw.is_object() || id.is_empty() || mission_id.is_empty() {
        Value::Null
    } else {
        json!({"id": id, "missionId": mission_id})
    }
}

/// Last non-empty lines of a log (lastLines).
pub fn last_lines(text: Option<&str>, count: usize) -> String {
    let text = text.unwrap_or("").trim_end();
    let lines: Vec<&str> = text.split('\n').skip_while(|line| line.trim().is_empty()).collect();
    lines.iter().rev().take(count).collect::<Vec<_>>().into_iter().rev().copied().collect::<Vec<_>>().join("\n")
}

/// Commit range a task changed, when both ends are known (taskRange).
pub fn task_range(task: &Value) -> Option<String> {
    let start = task["startSha"].as_str()?;
    let end = task["endSha"].as_str()?;
    if start.is_empty() || end.is_empty() || start == end {
        return None;
    }
    Some(format!("{}..{}", start.chars().take(7).collect::<String>(), end.chars().take(7).collect::<String>()))
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(cells: usize, waiting: usize, open: bool, running: usize) -> Vec<(&'static str, String)> {
    vec![
        ("cells", count_label(cells)),
        ("waiting", count_label(waiting)),
        ("open", if open { "yes" } else { "no" }.to_owned()),
        ("running", count_label(running)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_strips() {
        assert_eq!(plan_cell("done", false), "done");
        assert_eq!(plan_cell("doing", true), "stopped");
        assert_eq!(plan_cell("waiting_you", false), "waiting");
        assert_eq!(plan_cell("dropped", false), "dropped");
        let items = vec![json!({"status":"done"}), json!({"status":"dropped"}), json!({"status":"doing"})];
        assert_eq!(plan_cells(&items, false), vec!["done", "doing"]);
        assert_eq!(cells_from_counts(1, 4, 1, 1, false), vec!["done", "waiting", "doing", "todo"]);
        assert_eq!(mission_cells(&json!({"plan":{"items":items,"doing":0},"runningTasks":0}), false).len(), 2);
        assert_eq!(plan_counts(&items)["total"], 2);
        assert!(is_open_mission("active"));
        assert!(!is_open_mission("done"));
    }

    #[test]
    fn waiting_and_spans() {
        let mission = json!({"canAct":true,"pending":{"decisions":2,"reviews":1}});
        assert_eq!(waiting_parts(&mission).len(), 2);
        assert!(waiting_parts(&json!({"canAct":false,"pending":{}})).is_empty());
        assert_eq!(tick_for(30), 1_000);
        assert_eq!(tick_for(120), 15_000);
        assert_eq!(tick_for(7200), 60_000);
        assert_eq!(format_span(40), "40 s");
        assert_eq!(format_span(125), "2 min");
        assert_eq!(format_span(5_100), "1 h 25");
        assert_eq!(format_span(86_400), "1 d");
    }

    #[test]
    fn tasks_and_prs() {
        let task = json!({"status":"running","startedAt":"2026-01-01T00:00:00Z","seconds":5});
        let now = parse_iso_ms("2026-01-01T00:01:00Z").unwrap();
        assert_eq!(task_seconds(&task, now), 60);
        assert_eq!(task_seconds(&json!({"status":"done","seconds":9}), now), 9);
        assert_eq!(parse_iso_ms("2026-01-01T00:00:00.500Z").unwrap() % 1000, 500);
        let tasks = vec![
            json!({"id":"a","missionId":"m","status":"done","createdAt":"2026-01-02"}),
            json!({"id":"b","missionId":"m","status":"running","createdAt":"2026-01-01"}),
        ];
        assert_eq!(mission_tasks(&tasks, "m")[0]["id"], "b");
        assert_eq!(pr_of(Some("PR #431")).unwrap()["number"], "431");
        assert_eq!(pr_of(Some("https://github.com/o/r/pull/12")).unwrap()["url"], "https://github.com/o/r/pull/12");
        assert_eq!(task_pr(&json!({"prUrl":"https://gitlab.com/o/r/-/merge_requests/7"})).unwrap()["number"], "7");
        assert_eq!(
            task_range(&json!({"startSha":"abcdef123","endSha":"123456789"})).as_deref(),
            Some("abcdef1..1234567")
        );
    }

    #[test]
    fn output_and_commands() {
        let lines = append_output_line(&[], Some(" first "), 3);
        assert_eq!(lines, vec!["first"]);
        assert_eq!(append_output_line(&lines, Some("first"), 3), lines);
        assert_eq!(append_output_line(&lines, Some("second"), 3).len(), 2);
        let change = json!({"added":[{"title":"A"}],"removed":[],"changed":[{"title":"B","to":"done"},{"title":"C","renamedFrom":"D"}]});
        let parts = plan_change_parts(&change);
        assert_eq!(parts["added"][0], "A");
        assert_eq!(parts["done"][0], "B");
        assert_eq!(parts["renamed"][0]["from"], "D");
        let summary = json!({"plan":{"done":2,"total":5,"doing":1,"waiting":1},"running":1,"pendingDecisions":3});
        assert_eq!(summary_parts(&summary).len(), 5);
        assert_eq!(parse_mission_command("/mission build it").unwrap()["command"], "mission");
        assert_eq!(parse_mission_command("/stop").unwrap()["command"], "stop");
        assert!(parse_mission_command("/mission").is_none());
        assert!(parse_mission_command("/stop now").is_none());
    }

    #[test]
    fn meta_and_presentation() {
        let meta = json!({"missionLink":{"missionId":"m","threadId":"t"},"missionPlan":{"missionId":"m","title":"T"}});
        assert_eq!(mission_link_of(&meta)["threadId"], "t");
        assert_eq!(mission_plan_of(&meta)["executor"], "claude");
        assert_eq!(
            mission_card_ref(&json!({"missionDecision":{"id":"d","missionId":"m"}}), "missionDecision")["id"],
            "d"
        );
        assert_eq!(last_lines(Some("\n\nfirst\nsecond\n"), 1), "second");
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(4, 1, true, 2).len(), 4);
    }
}
