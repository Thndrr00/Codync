#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure shaping helpers for a bot's recent activity timeline. Ported from bdots' step display
//! (`message-parts.ts`) and desk cards (`agents/desk.ts`), composed with the helpers already in
//! `bdots_more` / `bdots_text` / `bdots_desk`. No store, no client, no I/O: the host endpoint
//! `botActivity` is the only caller, so every helper here is live.

use serde_json::Value;

/// One tool call in the window, already reduced to what the display needs.
#[derive(Clone, Debug)]
pub struct Call {
    pub kind: &'static str,
    pub name: String,
    pub input: Value,
    pub error: Option<String>,
    pub exit: Option<i64>,
    pub at_ms: i64,
}

/// A run of calls of the same kind.
#[derive(Clone, Debug, PartialEq)]
pub struct Group {
    pub kind: &'static str,
    pub label: String,
    pub icon: String,
    pub tone: String,
    pub count: usize,
    pub writes: bool,
}

/// SF Symbol for a step kind, so all four clients show the same icon.
pub fn kind_icon(kind: &str) -> &'static str {
    match kind {
        "shell" => "terminal",
        "files" => "doc.text",
        "search" => "magnifyingglass",
        "history" => "clock.arrow.circlepath",
        "memory" => "brain",
        "routine" => "calendar.badge.clock",
        "bot" => "bubble.left.and.bubble.right",
        "app" => "plugs",
        "shop" => "bag",
        "web" => "globe",
        _ => "ellipsis.circle",
    }
}

/// Semantic tone for a step kind; views map it to their own palette.
pub fn kind_tone(kind: &str) -> &'static str {
    match kind {
        "shell" | "app" => "accent",
        "files" | "search" => "secondary",
        "web" | "shop" => "good",
        "bot" | "routine" => "warn",
        "memory" | "history" => "tertiary",
        _ => "secondary",
    }
}

/// Whether a step changed something (affects the group's verb: "Changed" vs "Read").
pub fn is_write(kind: &str, name: &str) -> bool {
    kind == "files" && matches!(name, "write" | "edit" | "delete")
        || matches!(name, "delete_app" | "save_app" | "app_data_put" | "app_data_delete")
}

/// A short detail for a step: the path, URL, pattern or command it acted on.
pub fn call_detail(input: &Value) -> String {
    for key in ["path", "url", "pattern", "to", "command", "tool"] {
        if let Some(text) = input.get(key).and_then(Value::as_str) {
            let text = first_line(text);
            return shorten(&text, 64);
        }
    }
    String::new()
}

/// First line of a multi-line value, with surrounding space removed.
pub fn first_line(text: &str) -> &str {
    text.lines().next().unwrap_or("").trim()
}

/// Shorten a value to `max` characters, adding an ellipsis when cut.
pub fn shorten(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_owned();
    }
    let kept: String = text.chars().take(max.saturating_sub(1)).collect();
    format!("{kept}…")
}

/// Strip ANSI escape sequences from a tool result before showing it in a card.
pub fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            for esc in chars.by_ref() {
                if esc.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// A call's status: "error" when it failed, "ok" for a successful write, else "info".
pub fn call_status(error: Option<&str>, exit: Option<i64>) -> &'static str {
    if error.is_some() || exit.is_some_and(|code| code != 0) { "error" } else { "info" }
}

/// A grammatical label for a run: singular for one step, bdots' group verb for more.
pub fn group_label(kind: &str, count: usize, writes: bool) -> String {
    if count == 1 {
        return match kind {
            "shell" => "Ran a command".to_owned(),
            "files" if writes => "Changed a file".to_owned(),
            "files" => "Read a file".to_owned(),
            "search" => "Searched once".to_owned(),
            "web" => "Visited a page".to_owned(),
            _ => "1 step".to_owned(),
        };
    }
    crate::bdots_more::describe_group(kind, &vec![String::new(); count], writes)
}

/// Group consecutive calls of the same kind into labelled runs.
pub fn group_calls(calls: &[Call]) -> Vec<Group> {
    let mut out: Vec<Group> = Vec::new();
    for call in calls {
        let writes = is_write(call.kind, &call.name);
        if let Some(last) = out.last_mut()
            && last.kind == call.kind
            && last.writes == writes
        {
            last.count += 1;
            last.label = group_label(last.kind, last.count, writes);
            continue;
        }
        let label = group_label(call.kind, 1, writes);
        out.push(Group {
            kind: call.kind,
            label,
            icon: kind_icon(call.kind).to_owned(),
            tone: kind_tone(call.kind).to_owned(),
            count: 1,
            writes,
        });
    }
    out
}

/// Coalesce two adjacent runs of the same kind (used when merging paginated windows).
pub fn merge_adjacent(groups: &[Group]) -> Vec<Group> {
    let mut out: Vec<Group> = Vec::new();
    for group in groups {
        if let Some(last) = out.last_mut()
            && last.kind == group.kind
            && last.writes == group.writes
        {
            last.count += group.count;
            last.label = group_label(last.kind, last.count, last.writes);
            continue;
        }
        out.push(group.clone());
    }
    out
}

/// Count calls by kind, in first-seen order, for the summary strip.
pub fn count_by_kind(calls: &[Call]) -> Vec<(&'static str, usize)> {
    let mut out: Vec<(&'static str, usize)> = Vec::new();
    for call in calls {
        if let Some(entry) = out.iter_mut().find(|(kind, _)| *kind == call.kind) {
            entry.1 += 1;
        } else {
            out.push((call.kind, 1));
        }
    }
    out
}

/// The kind with the most calls, or "other" for an empty window.
pub fn dominant_kind(calls: &[Call]) -> &'static str {
    count_by_kind(calls).into_iter().max_by_key(|(_, n)| *n).map_or("other", |(kind, _)| kind)
}

/// Number of calls that failed in the window.
pub fn error_count(calls: &[Call]) -> usize {
    calls.iter().filter(|c| call_status(c.error.as_deref(), c.exit) == "error").count()
}

/// One plain sentence for the summary strip: what this bot mostly did.
pub fn activity_headline(calls: &[Call]) -> String {
    if calls.is_empty() {
        return "Nothing in this window yet.".to_owned();
    }
    let kind = dominant_kind(calls);
    let groups = group_calls(calls);
    let label = groups.iter().find(|g| g.kind == kind).map_or_else(|| "steps".to_owned(), |g| g.label.clone());
    let errors = error_count(calls);
    if errors == 0 { label } else { format!("{label} · {errors} failed") }
}

/// Whole percent, clamped to 100, for a bar or progress view.
pub fn percent(n: usize, total: usize) -> u32 {
    if total == 0 {
        return 0;
    }
    ((n * 100) / total).min(100) as u32
}

/// How many cells of a small bar to fill for `n` against `max`.
pub fn bar_width(n: usize, max: usize, width: usize) -> usize {
    if max == 0 || width == 0 {
        return 0;
    }
    ((n * width) / max).clamp(0, width)
}

/// UTC day key (`YYYY-MM-DD`) for an epoch-millisecond timestamp.
pub fn bucket_day(ms: i64) -> String {
    let days = ms.div_euclid(86_400_000);
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}")
}

/// Days between two day keys (`b - a`), ignoring the sign of malformed keys.
pub fn days_between(a: &str, b: &str) -> i64 {
    let parse = |key: &str| -> Option<i64> {
        let mut parts = key.split('-');
        let y = parts.next()?.parse::<i64>().ok()?;
        let m = parts.next()?.parse::<i64>().ok()?;
        let d = parts.next()?.parse::<i64>().ok()?;
        Some(days_from_civil(y, m, d))
    };
    match (parse(a), parse(b)) {
        (Some(x), Some(y)) => y - x,
        _ => 0,
    }
}

/// Human label for a day key relative to today.
pub fn day_label(day: &str, today: &str) -> String {
    match days_between(day, today) {
        0 => "Today".to_owned(),
        1 => "Yesterday".to_owned(),
        _ => day.to_owned(),
    }
}

/// Bucket calls into days, oldest first, with a count per day.
pub fn daily_counts(calls: &[Call]) -> Vec<(String, usize)> {
    let mut out: Vec<(String, usize)> = Vec::new();
    for call in calls {
        let day = bucket_day(call.at_ms);
        if let Some(entry) = out.iter_mut().find(|(key, _)| *key == day) {
            entry.1 += 1;
        } else {
            out.push((day, 1));
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// Earliest timestamp in the window, or `now` when empty.
pub fn window_start(calls: &[Call], now: i64) -> i64 {
    calls.iter().map(|c| c.at_ms).min().unwrap_or(now)
}

/// Span of the window as a compact label, e.g. "3h 12m".
pub fn window_span(calls: &[Call], now: i64) -> String {
    let start = window_start(calls, now);
    crate::bdots_text::format_span((now - start).max(0))
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    (y + i64::from(m <= 2), u32::try_from(m).unwrap_or(1), u32::try_from(d).unwrap_or(1))
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = year - i64::from(month <= 2);
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = month + if month > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn call(kind: &'static str, name: &str, at_ms: i64) -> Call {
        Call { kind, name: name.to_owned(), input: json!({"path": "a.rs"}), error: None, exit: None, at_ms }
    }

    #[test]
    fn icons_and_tones_are_stable_for_all_clients() {
        assert_eq!(kind_icon("shell"), "terminal");
        assert_eq!(kind_icon("web"), "globe");
        assert_eq!(kind_tone("files"), "secondary");
        assert_eq!(kind_tone("bot"), "warn");
    }

    #[test]
    fn writes_and_details_drive_the_group_verb() {
        assert!(is_write("files", "write"));
        assert!(!is_write("files", "read"));
        assert_eq!(call_detail(&json!({"path": "a.rs"})), "a.rs");
        assert_eq!(call_detail(&json!({"command": "npm\ntest"})), "npm");
    }

    #[test]
    fn text_helpers_keep_cards_short_and_plain() {
        assert_eq!(first_line(" a\nb "), "a");
        assert_eq!(shorten("abcdef", 4), "abc…");
        assert_eq!(shorten("ab", 4), "ab");
        assert_eq!(strip_ansi("\u{1b}[31mred\u{1b}[0m"), "red");
    }

    #[test]
    fn statuses_and_counts_summarise_a_window() {
        assert_eq!(call_status(Some("boom"), None), "error");
        assert_eq!(call_status(None, Some(1)), "error");
        assert_eq!(call_status(None, Some(0)), "info");
        let calls = [call("files", "read", 0), call("files", "write", 1), call("web", "read", 2)];
        assert_eq!(count_by_kind(&calls), vec![("files", 2), ("web", 1)]);
        assert_eq!(dominant_kind(&calls), "files");
        assert_eq!(error_count(&calls), 0);
        assert!(!activity_headline(&calls).is_empty());
    }

    #[test]
    fn groups_coalesce_runs_and_merge_keeps_order() {
        let calls = [call("files", "read", 0), call("files", "write", 1), call("web", "read", 2)];
        let groups = group_calls(&calls);
        assert_eq!(groups.len(), 3);
        assert_eq!(merge_adjacent(&groups).len(), 2);
        assert_eq!(group_label("files", 1, false), "Read a file");
        assert_eq!(group_label("shell", 3, false), "Ran 3 commands");
    }

    #[test]
    fn bars_and_days_shape_the_strip() {
        assert_eq!(percent(1, 4), 25);
        assert_eq!(percent(1, 0), 0);
        assert_eq!(bar_width(1, 4, 8), 2);
        assert_eq!(bucket_day(0), "1970-01-01");
        assert_eq!(days_between("2026-01-01", "2026-01-02"), 1);
        assert_eq!(day_label("2026-01-01", "2026-01-01"), "Today");
        assert_eq!(day_label("2026-01-01", "2026-01-02"), "Yesterday");
    }

    #[test]
    fn daily_buckets_and_window_are_oldest_first() {
        let calls = [call("web", "read", 86_400_000), call("shell", "exec", 0), call("web", "read", 86_400_000)];
        let days = daily_counts(&calls);
        assert_eq!(days[0].0, "1970-01-01");
        assert_eq!(days[0].1, 1);
        assert_eq!(days[1].1, 2);
        assert_eq!(window_start(&calls, 99), 0);
        assert_eq!(window_span(&calls, 86_400_000), "1d");
    }
}
