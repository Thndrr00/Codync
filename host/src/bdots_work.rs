#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from agentes-chat's `work.ts`: work metadata, folded tool steps, activity
//! grouping and duration words. Composes helpers already ported into this host. Only `botInsights`
//! calls this.

use serde_json::{Value, json};

/// Marker of a call whose arguments were refused (SLIP).
pub const SLIP: &str = "Invalid input:";

/// After this long, a step phrase reads as elapsed words (STEP_WORDS_AFTER_MS).
pub const STEP_WORDS_AFTER_MS: u64 = 5_000;

/// Most app logos shown for one work block (LOGOS_MAX).
pub const LOGOS_MAX: usize = 3;

/// States a tool row may show (ToolState).
pub const TOOL_STATES: [&str; 4] = ["run", "done", "fail", "stop"];

/// Kinds that never appear in the work list (reactions and tool lookups).
pub const UNLISTED_KINDS: [&str; 2] = ["reaction", "toolLookup"];

/// Activity kinds that work on files (DRIVE).
pub const DRIVE_KINDS: [&str; 4] = ["readingDrive", "writingDrive", "exportingFile", "importingFile"];

/// Whether an org shows the finished work block (showsActivity).
pub fn shows_activity(org: Option<&Value>) -> bool {
    org.and_then(|value| value.get("showActivity")).and_then(Value::as_bool).unwrap_or(false)
}

/// Read `meta.work` into a normalized object (workMeta).
pub fn work_meta(meta: Option<&Value>) -> Option<Value> {
    let work = meta?.get("work")?;
    let steps = work["steps"].as_u64()?;
    let calls = work["calls"]
        .as_array()
        .map_or(Vec::new(), |values| values.iter().filter(|call| call["name"].as_str().is_some()).cloned().collect());
    let duration = work["durationMs"].as_u64();
    Some(json!({"steps": steps, "calls": Value::Array(calls), "durationMs": duration}))
}

/// Number of steps shown, dropping pure lookups (workSteps).
pub fn work_steps(work: &Value, skip: &[String]) -> u64 {
    let calls = work["calls"].as_array().cloned().unwrap_or_default();
    if !calls.is_empty()
        && calls.iter().all(|call| {
            let name = call["name"].as_str().unwrap_or("");
            name.contains("lookup") || skip.iter().any(|value| value == name)
        })
    {
        return 0;
    }
    let steps = work["steps"].as_u64().unwrap_or(0);
    calls.iter().fold(steps, |total, call| {
        let name = call["name"].as_str().unwrap_or("");
        if skip.iter().any(|value| value == name) {
            total.saturating_sub(call["n"].as_u64().unwrap_or(0))
        } else {
            total
        }
    })
}

/// Total calls of a work block (workCalls).
pub fn work_calls(work: &Value) -> usize {
    work["steps"]
        .as_array()
        .map(|steps| steps.iter().map(|step| step["count"].as_u64().unwrap_or(0) as usize).sum())
        .unwrap_or_else(|| work["steps"].as_u64().unwrap_or(0) as usize)
}

/// Fold consecutive steps with the same text into one line with a count (foldSteps).
pub fn fold_steps(steps: &[Value]) -> Vec<Value> {
    let mut out: Vec<Value> = Vec::new();
    for step in steps {
        let text = step["text"].as_str().unwrap_or("").to_owned();
        if let Some(last) = out.last_mut() {
            if last["text"].as_str() == Some(text.as_str()) {
                last["count"] = json!(last["count"].as_u64().unwrap_or(0) + 1);
                last["step"] = step.clone();
                continue;
            }
        }
        out.push(json!({"step": step, "text": text, "count": 1}));
    }
    out
}

/// Whether a live work row is a plain call with nothing to show.
fn is_plain_call(work: &Value) -> bool {
    work["kind"].as_str() == Some("call")
        && work["code"].is_null()
        && work["output"].as_array().is_none_or(Vec::is_empty)
        && work["autoApproved"].is_null()
}

/// Drop a plain call immediately followed by the same activity (foldLiveWork).
pub fn fold_live_work(work: &[Value]) -> Vec<Value> {
    work.iter()
        .enumerate()
        .filter(|(index, current)| {
            if !is_plain_call(current) {
                return true;
            }
            work.get(index + 1)
                .is_none_or(|next| !is_plain_call(next) || next["tool"].as_str() != current["tool"].as_str())
        })
        .map(|(_, value)| value.clone())
        .collect()
}

/// Duration in words: "12 s", "3 min", "21 h 55 min" (formatDuration).
pub fn format_duration(ms: u64) -> String {
    let seconds = (ms / 1_000).max(1);
    if seconds < 60 {
        return format!("{seconds} s");
    }
    let minutes = seconds / 60;
    if minutes < 60 {
        return format!("{minutes} min");
    }
    let hours = minutes / 60;
    let rest = minutes % 60;
    if rest == 0 { format!("{hours} h") } else { format!("{hours} h {rest} min") }
}

/// Pieces of the work header (workHeader).
pub fn work_header(work: &Value) -> Value {
    let connectors: Vec<String> = work["connectors"]
        .as_array()
        .map(|values| values.iter().filter_map(Value::as_str).map(str::to_owned).collect())
        .unwrap_or_default();
    json!({
        "steps": format!("{} steps", work["steps"].as_u64().unwrap_or(0)),
        "connectors": if connectors.is_empty() { Value::Null } else { Value::String(connectors.join(", ")) },
        "duration": work.get("durationMs").and_then(Value::as_u64).map(format_duration),
    })
}

/// Strip the server's cancellation note from a body (splitCanceled).
pub fn split_canceled(body: &str, meta: Option<&Value>) -> Value {
    let canceled = meta.and_then(|value| value.get("canceled"));
    let Some(canceled) = canceled.filter(|value| !value.is_null() && value != false) else {
        return json!({"body": body, "canceled": false, "byUserId": Value::Null, "note": Value::Null});
    };
    let by_user_id = canceled["byUserId"].as_str().map(str::to_owned);
    let said = canceled["text"].as_str();
    let trailing_note = body.trim_end().rsplit_once('_').map(|(_, rest)| rest.trim_end_matches('_').trim().to_owned());
    let note = trailing_note.filter(|value| !value.is_empty());
    json!({
        "body": said.unwrap_or(body),
        "canceled": true,
        "byUserId": by_user_id,
        "note": note,
    })
}

/// Grouping key of an activity (appKey).
pub fn app_key(activity: &Value) -> String {
    if let Some(service) = activity["service"].as_str() {
        format!("service:{service}")
    } else if activity["kind"].as_str() == Some("consulting") {
        format!("app:{}", activity["app"].as_str().unwrap_or(""))
    } else {
        format!("kind:{}", activity["kind"].as_str().unwrap_or(""))
    }
}

/// Each app once, in first-use order, capped at LOGOS_MAX (uniqueApps).
pub fn unique_apps(activities: &[Value]) -> Vec<Value> {
    let mut out = Vec::new();
    for activity in activities {
        if out.len() >= LOGOS_MAX {
            break;
        }
        let key = app_key(activity);
        if out.iter().any(|entry: &Value| app_key(entry) == key) {
            continue;
        }
        let mut app = json!({"kind": activity["kind"].as_str().unwrap_or("")});
        for field in ["app", "service", "icon"] {
            if let Some(value) = activity.get(field).filter(|value| value.is_string()) {
                app[field] = value.clone();
            }
        }
        out.push(app);
    }
    out
}

/// Apps used by tool rows (toolApps).
pub fn tool_apps(items: &[Value]) -> Vec<Value> {
    let activities: Vec<Value> = items.iter().filter_map(|item| item.get("activity").cloned()).collect();
    unique_apps(&activities)
}

/// Total calls across tool rows (toolCalls).
pub fn tool_calls(items: &[Value]) -> usize {
    items.iter().map(|item| item["count"].as_u64().unwrap_or(0) as usize).sum()
}

/// Whether exactly one call was made (oneCall).
pub fn one_call(items: &[Value]) -> bool {
    tool_calls(items) == 1
}

/// The last tool row, when any (toolNow).
pub fn tool_now(items: &[Value]) -> Value {
    items.last().cloned().unwrap_or(Value::Null)
}

/// Whether all rows are quiet (no output worth showing) (quietWork).
pub fn quiet_work(items: &[Value]) -> bool {
    !items.is_empty()
        && items.iter().all(|item| {
            item["code"].is_null() && item["output"].as_array().is_none_or(Vec::is_empty) && item["why"].is_null()
        })
}

/// State of a step after slip/failure/held flags.
pub fn step_state(slip: bool, failed: bool, held: bool) -> &'static str {
    if slip || held {
        "stop"
    } else if failed {
        "fail"
    } else {
        "done"
    }
}

/// A refused call was made right by a later call of the same tool (madeRight).
pub fn made_right(tool: &str, later: &[Value]) -> bool {
    later.iter().any(|call| call["tool"].as_str() == Some(tool) && call["failed"] != true && call["held"] != true)
}

/// Whether an activity kind is never listed.
pub fn is_unlisted(kind: &str) -> bool {
    UNLISTED_KINDS.contains(&kind)
}

/// Whether an activity kind works on files (DRIVE).
pub fn is_drive_kind(kind: &str) -> bool {
    DRIVE_KINDS.contains(&kind)
}

/// Words for a refusal code (toolWhy).
pub fn tool_why(refusal: Option<&str>) -> Option<&'static str> {
    match refusal.unwrap_or("") {
        "denied" => Some("A person refused this call."),
        "gateway" => Some("The tool gateway refused this call."),
        "invalid" => Some("The arguments were not accepted."),
        _ => None,
    }
}

/// One line describing a work block (workLine).
pub fn work_line(steps: u64, calls: usize, connectors: usize, duration_ms: Option<u64>) -> String {
    format!(
        "{} steps · {} calls · {} apps · {}",
        steps,
        calls,
        connectors,
        duration_ms.map(format_duration).unwrap_or_else(|| "—".to_owned())
    )
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(steps: u64, calls: usize, apps: usize, quiet: bool) -> Vec<(&'static str, String)> {
    vec![
        ("steps", count_label(steps as usize)),
        ("calls", count_label(calls)),
        ("apps", count_label(apps)),
        ("quiet", if quiet { "yes" } else { "no" }.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meta_and_steps() {
        assert!(shows_activity(Some(&json!({"showActivity": true}))));
        assert!(!shows_activity(None));
        let meta =
            json!({"work":{"steps":8,"calls":[{"name":"read","n":2},{"name":"lookup","n":3}],"durationMs":42000}});
        let work = work_meta(Some(&meta)).unwrap();
        assert_eq!(work["steps"], 8);
        assert_eq!(work["calls"].as_array().map_or(0, Vec::len), 2);
        assert_eq!(work_steps(&work, &["read".to_owned()]), 6);
        assert_eq!(work_calls(&json!({"steps":[{"count":2},{"count":3}]})), 5);
        assert!(work_meta(Some(&json!({}))).is_none());
    }

    #[test]
    fn folding_and_duration() {
        let steps =
            vec![json!({"text":"read","count":1}), json!({"text":"read","count":1}), json!({"text":"write","count":1})];
        let folded = fold_steps(&steps);
        assert_eq!(folded.len(), 2);
        assert_eq!(folded[0]["count"], 2);
        assert_eq!(format_duration(12_000), "12 s");
        assert_eq!(format_duration(180_000), "3 min");
        assert_eq!(format_duration((21 * 3600 + 55 * 60) * 1000), "21 h 55 min");
        let work = vec![
            json!({"kind":"call","tool":"browser","output":[]}),
            json!({"kind":"call","tool":"browser","output":[]}),
        ];
        assert_eq!(fold_live_work(&work).len(), 1);
    }

    #[test]
    fn grouping_and_states() {
        let activity = json!({"kind":"consulting","app":"Drive","service":"google"});
        assert_eq!(app_key(&activity), "service:google");
        assert_eq!(app_key(&json!({"kind":"consulting","app":"X"})), "app:X");
        assert_eq!(app_key(&json!({"kind":"done"})), "kind:done");
        let apps = unique_apps(&[activity.clone(), activity.clone(), json!({"kind":"readingDrive"})]);
        assert_eq!(apps.len(), 2);
        assert_eq!(tool_apps(&[json!({"activity":activity})]).len(), 1);
        let items = vec![json!({"count":1}), json!({"count":2})];
        assert_eq!(tool_calls(&items), 3);
        assert!(!one_call(&items));
        assert_eq!(tool_now(&items)["count"], 2);
        assert!(quiet_work(&[json!({"output":[]})]));
        assert_eq!(step_state(true, false, false), "stop");
        assert_eq!(step_state(false, true, false), "fail");
        assert_eq!(step_state(false, false, false), "done");
        assert!(made_right("bash", &[json!({"tool":"bash","failed":false})]));
        assert!(is_unlisted("reaction"));
        assert!(is_drive_kind("readingDrive"));
        assert!(tool_why(Some("denied")).is_some());
    }

    #[test]
    fn presentation() {
        let header = work_header(&json!({"steps":4,"connectors":["Drive","Gmail"],"durationMs":12000}));
        assert_eq!(header["steps"], "4 steps");
        assert_eq!(header["duration"], "12 s");
        let canceled =
            split_canceled("Hello _Interrompido por Ana._", Some(&json!({"canceled":{"byUserId":"7","text":"Hello"}})));
        assert_eq!(canceled["body"], "Hello");
        assert_eq!(canceled["byUserId"], "7");
        assert_eq!(work_line(4, 6, 2, Some(12_000)).contains("steps"), true);
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(4, 6, 2, true).len(), 4);
    }
}
