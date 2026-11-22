#![allow(dead_code)]

//! A third batch of pure helpers from bdots' current source: plain text, approval previews,
//! grant scopes, error extraction, spend summaries, rate limits, and desk activity. All
//! deterministic and dependency-light so they can be tested without a host.

use serde_json::{Map, Value};

fn as_text(value: &Value) -> &str {
    value.as_str().unwrap_or("")
}

/// Store descriptions arrive as HTML or Markdown-ish text; keep the words (`shop.ts`).
pub fn plain_text(value: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = value.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '<' {
            if let Some(end) = chars[i..].iter().position(|c| *c == '>') {
                let tag: String = chars[i + 1..i + end].iter().collect();
                let lower = tag.trim().trim_start_matches('/').to_ascii_lowercase();
                if lower.starts_with("br")
                    || lower.starts_with("p")
                    || lower.starts_with("h")
                    || lower.starts_with("li")
                {
                    out.push('\n');
                }
                if lower.starts_with("li") && !tag.trim_start().starts_with('/') {
                    out.push_str("• ");
                }
                i += end + 1;
                continue;
            }
        }
        if chars[i] == '&' {
            if let Some(end) = chars[i..].iter().position(|c| *c == ';') {
                let name: String = chars[i + 1..i + end].iter().collect();
                let decoded = match name.as_str() {
                    "amp" => Some('&'),
                    "lt" => Some('<'),
                    "gt" => Some('>'),
                    "quot" => Some('"'),
                    "apos" | "#39" => Some('\''),
                    "nbsp" => Some(' '),
                    other if other.starts_with("#x") => {
                        u32::from_str_radix(&other[2..], 16).ok().and_then(char::from_u32)
                    }
                    other if other.starts_with('#') => other[1..].parse::<u32>().ok().and_then(char::from_u32),
                    _ => None,
                };
                if let Some(c) = decoded {
                    out.push(c);
                    i += end + 1;
                    continue;
                }
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    let out = out.split_whitespace().collect::<Vec<_>>().join(" ");
    out.trim().to_owned()
}

/// Button words that say what will happen, not just "Approve" (`approvalLabels`).
pub fn approval_labels(tool: &str, input: &Value) -> (&'static str, &'static str) {
    if tool == "send_email" || tool == "reply_email" || tool == "message_bot" {
        return ("Send", "Don't send");
    }
    if tool == "delete" || tool == "delete_app" || tool == "deleteBotApp" || tool == "removeSkill" {
        return ("Delete", "Keep");
    }
    if as_text(&input["command"]).starts_with("rm ") {
        return ("Delete", "Keep");
    }
    ("Approve", "Reject")
}

/// Read values at a dotted argument path; `[]` expands every item of a list (`valuesAt`).
pub fn values_at(args: &Value, at: &str) -> Vec<Value> {
    let mut nodes = vec![args.clone()];
    for part in at.split('.') {
        let each = part.ends_with("[]");
        let key = if each { &part[..part.len() - 2] } else { part };
        let mut next = Vec::new();
        for node in &nodes {
            if let Some(map) = node.as_object() {
                if let Some(value) = map.get(key) {
                    if each && value.is_array() {
                        next.extend(value.as_array().map(|a| a.clone()).unwrap_or_default());
                    } else {
                        next.push(value.clone());
                    }
                }
            }
        }
        nodes = next;
    }
    nodes
}

/// Whether arguments stay inside a tool grant's allowed values, and whether the call reads
/// (`checkScopes`). A missing value fails so a limit cannot be stepped around.
pub fn check_scopes(scopes: &[(String, Vec<(String, bool)>)], args: &Value) -> Result<bool, String> {
    let mut reads = true;
    for (at, allow) in scopes {
        let values = values_at(args, at);
        if values.is_empty() {
            return Err(format!("{at} is missing"));
        }
        for value in values {
            let text = value.as_str().unwrap_or("");
            match allow.iter().find(|(v, _)| v == text) {
                Some((_, read)) => {
                    if !*read {
                        reads = false;
                    }
                }
                None => {
                    return Err(format!(
                        "{at} \"{}\" is not one the user allowed for this app",
                        &text[..text.len().min(80)]
                    ));
                }
            }
        }
    }
    Ok(reads)
}

/// Extract an MCP `isError` result's message so apps don't misread failures (`toolResultError`).
pub fn tool_result_error(value: &Value) -> Option<String> {
    if value.get("isError") != Some(&Value::Bool(true)) {
        return None;
    }
    let content = value.get("content")?.as_array()?;
    let text = content
        .iter()
        .filter(|c| c["type"] == "text")
        .filter_map(|c| c["text"].as_str())
        .collect::<Vec<_>>()
        .join("\n");
    (!text.is_empty()).then_some(text)
}

/// The `filters` query for gateway logs after an ISO time (`afterFilter`), as a JSON array.
pub fn after_filter(_at: &str, one_ms_before: &str) -> Value {
    serde_json::json!([{"key":"created_at","operator":"gt","value":[one_ms_before]}])
}

/// A conversation/desk card's lane from its origin (`laneFor`): needs_you, fyi, done or none.
pub fn lane_for(origin: &str) -> Option<&'static str> {
    match origin {
        "permission" | "handoff" | "takeover" | "question" => Some("needs_you"),
        "mention" | "channel" | "email" => Some("fyi"),
        "routine" | "check_in" | "background" => Some("done"),
        _ => None,
    }
}

/// The "why is this card here" line from an origin and name (`whyOf`).
pub fn why_of(origin: &str, name: &str) -> String {
    match origin {
        "email" => format!("Email from {name}"),
        "routine" => format!("Routine · {name}"),
        "check_in" => format!("Check-in · {name}"),
        "mention" => format!("Mentioned in {name}"),
        other if !other.is_empty() => format!("{other} · {name}"),
        _ => name.to_owned(),
    }
}

/// A live status line for a tool call (`describeActivity`): "Reading a.com", "Writing file.md".
pub fn describe_activity(name: &str, input: &Value) -> String {
    let s = |key: &str| as_text(&input[key]);
    match name {
        "read" => format!("Reading {}", s("path")),
        "write" | "edit" => format!("Writing {}", s("path")),
        "delete" => format!("Deleting {}", s("path")),
        "exec" => format!("Running {}", s("command")),
        "grep" | "find" => format!("Searching for {}", s("pattern")),
        "browser_markdown" | "browser_scrape" => format!("Reading {}", s("url")),
        "message_bot" => format!("Messaging {}", s("to")),
        other => format!("Using {other}"),
    }
}

/// True when a background reply deliberately said nothing (`isQuietReply`).
pub fn is_quiet_reply(text: Option<&str>) -> bool {
    matches!(text.map(str::trim), Some(t) if t.is_empty() || t == "(no update)" || t == "NO_UPDATE")
}

/// Drop the bracketed origin line meant for the model (`stripOriginHeader`).
pub fn strip_origin_header(text: &str) -> &str {
    let trimmed = text.trim_start();
    if trimmed.starts_with('[')
        && let Some(end) = trimmed.find("]\n")
    {
        return trimmed[end + 2..].trim_start();
    }
    text
}

/// A one-line human connection status with tool pluralisation (`connectionStatus`).
pub fn connection_status(state: &str, tools: usize, needs_sign_in: bool) -> String {
    if needs_sign_in {
        return "Needs you to sign in".to_owned();
    }
    let label = if tools == 1 { "tool" } else { "tools" };
    format!("{state} · {tools} {label}")
}

/// Resolve a model with fallbacks (`pickModel`).
pub fn pick_model<'a>(chat: Option<&'a str>, background: Option<&'a str>, is_background: bool) -> &'a str {
    if is_background { background.or(chat).unwrap_or("default") } else { chat.unwrap_or("default") }
}

/// The most recent matching run index, or `None` (`currentStep`).
pub fn current_step(states: &[&str]) -> Option<usize> {
    states.iter().position(|s| *s == "running" || *s == "waiting")
}

/// Elapsed milliseconds of a step, or `None` when it never started (`stepSpan`).
pub fn step_span(started: Option<i64>, finished: Option<i64>, now: i64) -> Option<i64> {
    started.map(|start| (finished.unwrap_or(now) - start).max(0))
}

/// A submit-link rate limiter: one slot for `visitor` if the window allows (`takeSubmitSlot`).
pub struct SubmitDecision {
    pub ok: bool,
    pub retry_after_ms: i64,
    pub all: Vec<i64>,
    pub visitors: Vec<(String, Vec<i64>)>,
}

pub fn take_submit_slot(
    all: &[i64],
    visitors: &[(String, Vec<i64>)],
    visitor: &str,
    now: i64,
    window_ms: i64,
    per_link_limit: usize,
    per_visitor_limit: usize,
) -> SubmitDecision {
    let recent = |hits: &[i64]| hits.iter().copied().filter(|t| *t > now - window_ms && *t <= now).collect::<Vec<_>>();
    let all = recent(all);
    let mut kept: Vec<(String, Vec<i64>)> =
        visitors.iter().map(|(k, v)| (k.clone(), recent(v))).filter(|(_, v)| !v.is_empty()).collect();
    let mine: Vec<i64> = kept.iter().find(|(k, _)| k == visitor).map(|(_, v)| v.clone()).unwrap_or_default();
    let wait = |hits: &[i64], limit: usize| {
        if hits.len() >= limit { (hits[hits.len() - limit] + window_ms - now).max(0) } else { 0 }
    };
    let retry = wait(&mine, per_visitor_limit).max(wait(&all, per_link_limit));
    if retry > 0 {
        return SubmitDecision { ok: false, retry_after_ms: retry, all, visitors: kept };
    }
    let mut next_mine = mine;
    next_mine.push(now);
    if let Some(entry) = kept.iter_mut().find(|(k, _)| k == visitor) {
        entry.1 = next_mine;
    } else {
        kept.push((visitor.to_owned(), next_mine));
    }
    let mut next_all = all;
    next_all.push(now);
    SubmitDecision { ok: true, retry_after_ms: 0, all: next_all, visitors: kept }
}

/// Recursively replace receipt ids (in values and keys) with "private" (`hideReceipts`).
pub fn hide_receipts(value: Value, receipts: &[String]) -> Value {
    match value {
        Value::String(s) if receipts.iter().any(|r| r == &s) => Value::String("private".into()),
        Value::Array(items) => Value::Array(items.into_iter().map(|v| hide_receipts(v, receipts)).collect()),
        Value::Object(map) => {
            let mut out = Map::new();
            for (key, value) in map {
                let safe_key = if receipts.iter().any(|r| r == &key) { "private".to_owned() } else { key };
                out.insert(safe_key, hide_receipts(value, receipts));
            }
            Value::Object(out)
        }
        other => other,
    }
}

/// A tolerance parse of stored public-link access; garbage becomes no access (`parseAccess`).
pub fn parse_access(raw: Option<&str>) -> (Vec<String>, Vec<String>) {
    let Some(raw) = raw else { return (vec![], vec![]) };
    match serde_json::from_str::<Value>(raw) {
        Ok(Value::Object(map)) => {
            let list = |key: &str| -> Vec<String> {
                map.get(key)
                    .and_then(Value::as_array)
                    .map(|a| a.iter().filter_map(Value::as_str).map(str::to_owned).collect())
                    .unwrap_or_default()
            };
            (list("read"), list("submit"))
        }
        _ => (vec![], vec![]),
    }
}

/// Currency digits: JPY 0, KWD 3, most others 2 (`fractionDigits`).
pub fn fraction_digits(currency: &str) -> u32 {
    match currency {
        "JPY" | "KRW" | "VND" | "CLP" | "ISK" => 0,
        "KWD" | "BHD" | "OMR" | "JOD" | "TND" => 3,
        _ => 2,
    }
}

/// Minor units from a major amount (`toMinorUnits`): 40 USD → 4000 cents.
pub fn to_minor_units(amount: f64, currency: &str) -> i64 {
    (amount * 10f64.powi(fraction_digits(currency) as i32)).round() as i64
}

/// A cheap money label without Intl (`formatMoney`): 2999 USD cents → "$29.99".
pub fn format_money(minor: i64, currency: &str) -> String {
    let digits = fraction_digits(currency);
    let symbol = match currency {
        "USD" => "$",
        "EUR" => "€",
        "GBP" => "£",
        "JPY" => "¥",
        "INR" => "₹",
        _ => "",
    };
    let value = minor as f64 / 10f64.powi(digits as i32);
    let precision = digits as usize;
    let number = if digits == 0 || minor % 10i64.pow(digits) == 0 {
        format!("{value:.0}")
    } else {
        format!("{value:.precision$}")
    };
    if symbol.is_empty() { format!("{number} {currency}") } else { format!("{symbol}{number}") }
}

/// A patch preview of a finished edit (`editedPreview`): path + patch, or `None`.
pub fn edited_preview(input: &Value, output: &Value) -> Option<(String, String)> {
    let patch = output.get("patch")?.as_str()?;
    let file_path = output["path"].as_str().or_else(|| input["path"].as_str())?;
    (!file_path.is_empty() && !patch.is_empty()).then(|| (file_path.to_owned(), patch.to_owned()))
}

/// True when a preview has nothing worth showing (`isEmptyPreview`).
pub fn is_empty_preview(kind: &str, rows: usize, has_extra: bool) -> bool {
    kind == "fields" && rows == 0 && !has_extra
}

/// Tool arguments as label/value rows (`genericRows`), skipping long values into `extra`.
pub fn generic_rows(input: &Value) -> (Vec<(String, String)>, Vec<String>) {
    let mut rows = Vec::new();
    let mut extra = Vec::new();
    if let Some(map) = input.as_object() {
        for (key, value) in map {
            let label = crate::bdots_text::human_key(key);
            match value {
                Value::String(s) if s.chars().count() <= 90 => rows.push((label, s.clone())),
                Value::String(s) => extra.push(format!("{label}: {}", s.chars().take(200).collect::<String>())),
                Value::Number(n) => rows.push((label, n.to_string())),
                Value::Bool(b) => rows.push((label, b.to_string())),
                _ => {}
            }
        }
    }
    (rows, extra)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn plain_text_keeps_words_and_decodes_entities() {
        assert_eq!(plain_text("<p>Hello &amp; <b>world</b>&#39;s</p><br>next"), "Hello & world's next");
        assert_eq!(plain_text("  a   b  \n\n\n c "), "a b c");
    }

    #[test]
    fn approvals_and_activity() {
        assert_eq!(approval_labels("send_email", &json!({})), ("Send", "Don't send"));
        assert_eq!(approval_labels("delete", &json!({})), ("Delete", "Keep"));
        assert_eq!(approval_labels("shell", &json!({"command":"ls"})), ("Approve", "Reject"));
        assert_eq!(describe_activity("read", &json!({"path":"a.rs"})), "Reading a.rs");
        assert!(is_quiet_reply(Some("(no update)")));
    }

    #[test]
    fn scopes_and_error_extraction() {
        let scopes = vec![("calls[].path".into(), vec![("free/busy".into(), true), ("insert event".into(), false)])];
        assert_eq!(check_scopes(&scopes, &json!({"calls":[{"path":"free/busy"}]})), Ok(true));
        assert!(check_scopes(&scopes, &json!({"calls":[{"path":"delete"}]})).is_err());
        assert_eq!(values_at(&json!({"a":[{"b":1},{"b":2}]}), "a[].b"), vec![json!(1), json!(2)]);
        assert_eq!(
            tool_result_error(&json!({"isError":true,"content":[{"type":"text","text":"boom"}]})).as_deref(),
            Some("boom")
        );
    }

    #[test]
    fn desk_lines_and_status() {
        assert_eq!(lane_for("permission"), Some("needs_you"));
        assert_eq!(lane_for("routine"), Some("done"));
        assert_eq!(why_of("email", "alice"), "Email from alice");
        assert_eq!(connection_status("Connected", 3, false), "Connected · 3 tools");
        assert_eq!(strip_origin_header("[Origin]\nHello"), "Hello");
        assert_eq!(current_step(&["done", "running"]), Some(1));
        assert_eq!(step_span(Some(100), None, 400), Some(300));
    }

    #[test]
    fn rate_limit_and_receipts() {
        let decision = take_submit_slot(&[], &[], "v1", 1000, 60_000, 5, 2);
        assert!(decision.ok);
        assert_eq!(decision.all, vec![1000]);
        let blocked = take_submit_slot(&decision.all, &decision.visitors, "v1", 1100, 60_000, 5, 1);
        assert!(!blocked.ok);
        assert!(blocked.retry_after_ms > 0);
        let hidden = hide_receipts(json!({"id":"abc","items":[{"ref":"abc"}]}), &["abc".into()]);
        assert_eq!(hidden["id"], "private");
        assert_eq!(hidden["items"][0]["ref"], "private");
        assert_eq!(
            parse_access(Some("{\"read\":[\"status\"],\"submit\":[\"form\"]}")),
            (vec!["status".into()], vec!["form".into()])
        );
    }

    #[test]
    fn money_and_preview_helpers() {
        assert_eq!(fraction_digits("JPY"), 0);
        assert_eq!(fraction_digits("KWD"), 3);
        assert_eq!(to_minor_units(40.0, "USD"), 4000);
        assert_eq!(format_money(2999, "USD"), "$29.99");
        assert_eq!(format_money(1000, "JPY"), "¥1000");
        let preview = edited_preview(&json!({"path":"a.rs"}), &json!({"patch":"-old\n+new","path":"a.rs"})).unwrap();
        assert_eq!(preview.0, "a.rs");
        assert!(is_empty_preview("fields", 0, false));
        let (rows, extra) = generic_rows(&json!({"query":"x"}));
        assert!(rows.iter().any(|(k, _)| k == "Search for"));
        assert!(extra.is_empty());
    }
}
