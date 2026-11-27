#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from rakazo's `tool-step-calls.ts`: bounded tool text, credential-field
//! redaction, kept arguments/results and replay permission. Composes helpers already ported into
//! this host. Only `botInsights` calls this.

use serde_json::Value;

/// Most of one call's text kept for later turns (TOOL_CALL_TEXT_MAX_CHARS).
pub const TOOL_CALL_TEXT_MAX_CHARS: usize = 1_000;

/// Most of all results one reply keeps (TOOL_REPLY_RESULTS_MAX_CHARS).
pub const TOOL_REPLY_RESULTS_MAX_CHARS: usize = 8_000;

/// Minimum room left before another result is worth keeping (TOOL_RESULT_MIN_CHARS).
pub const TOOL_RESULT_MIN_CHARS: usize = 200;

/// Maximum items or entries kept in an argument (ARGUMENT_ITEMS_MAX).
pub const ARGUMENT_ITEMS_MAX: usize = 100;

/// Maximum nesting kept in an argument (ARGUMENT_DEPTH_MAX).
pub const ARGUMENT_DEPTH_MAX: usize = 8;

/// Stand-in for an image a call returned (IMAGE_NOT_KEPT).
pub const IMAGE_NOT_KEPT: &str = "[image not kept]";

/// Marker used for a redacted credential value.
pub const REDACTED_MARKER: &str = "\"[redacted]\"";

/// Words that mark a JSON field as holding a credential (CREDENTIAL_FIELD).
pub const CREDENTIAL_WORDS: [&str; 7] = ["api_key", "apikey", "api-key", "token", "password", "passwd", "secret"];

/// End index and content of a JSON string starting at `start`.
fn scan_string(bytes: &[u8], start: usize) -> Option<(usize, String)> {
    let mut index = start + 1;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index += 2,
            b'"' => {
                return Some((index + 1, String::from_utf8_lossy(&bytes[start + 1..index]).into_owned()));
            }
            _ => index += 1,
        }
    }
    None
}

/// Index after any JSON whitespace at `start`.
fn skip_ws(bytes: &[u8], mut start: usize) -> usize {
    while start < bytes.len() && matches!(bytes[start], b' ' | b'\n' | b'\r' | b'\t') {
        start += 1;
    }
    start
}

/// Whether a field name says it holds a credential.
pub fn is_credential_field(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    CREDENTIAL_WORDS.iter().any(|word| lower.contains(word))
        || lower.contains("authorization")
        || lower.contains("credential")
}

/// Replace credential-like JSON values with the redaction marker, preserving the field name.
pub fn redact_credential_fields(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'"' {
            if let Some((key_end, key)) = scan_string(bytes, index) {
                if is_credential_field(&key) {
                    let colon = skip_ws(bytes, key_end);
                    if bytes.get(colon) == Some(&b':') {
                        let value_start = skip_ws(bytes, colon + 1);
                        if bytes.get(value_start) == Some(&b'"') {
                            if let Some((value_end, _)) = scan_string(bytes, value_start) {
                                out.push_str(&text[index..value_start]);
                                out.push_str(REDACTED_MARKER);
                                index = value_end;
                                continue;
                            }
                        }
                    }
                }
                out.push_str(&text[index..key_end]);
                index = key_end;
                continue;
            }
        }
        let ch = text[index..].chars().next().unwrap_or('\0');
        out.push(ch);
        index += ch.len_utf8();
    }
    out
}

/// Redact known secrets and credential fields from a piece of text.
pub fn sanitize_text(text: &str, secrets: &[String]) -> String {
    let mut clean = text.to_owned();
    for secret in secrets {
        clean = crate::bdots_more::redact(&clean, secret);
    }
    redact_credential_fields(&clean)
}

/// Number of Unicode characters in a string.
pub fn char_len(text: &str) -> usize {
    text.chars().count()
}

/// First `count` characters of a string, safe on UTF-8 boundaries.
pub fn take_chars(text: &str, count: usize) -> String {
    text.chars().take(count).collect()
}

/// Last `count` characters of a string, safe on UTF-8 boundaries.
pub fn take_last_chars(text: &str, count: usize) -> String {
    let total = char_len(text);
    if count >= total {
        return text.to_owned();
    }
    text.chars().skip(total - count).collect()
}

/// Text bounded to `max` characters, keeping start and end and saying what was left out.
pub fn bounded(text: &str, max: usize, secrets: &[String]) -> String {
    let total = char_len(text);
    if total <= max {
        return sanitize_text(text, secrets);
    }
    let margin = 200;
    let head_len = max * 7 / 10;
    let tail_len = max - head_len;
    let head_source = take_chars(text, head_len + margin);
    let tail_source = take_last_chars(text, tail_len + margin);
    let head = take_chars(&sanitize_text(&head_source, secrets), head_len);
    let tail = take_last_chars(&sanitize_text(&tail_source, secrets), tail_len);
    format!("{head}\n… [{} characters not kept] …\n{tail}", total.saturating_sub(max))
}

/// One argument value, bounded and redacted recursively.
pub fn kept_argument(value: &Value, depth: usize, secrets: &[String]) -> Value {
    match value {
        Value::String(text) => Value::String(bounded(text, TOOL_CALL_TEXT_MAX_CHARS, secrets)),
        Value::Null | Value::Number(_) | Value::Bool(_) => value.clone(),
        Value::Array(items) => Value::Array(
            items.iter().take(ARGUMENT_ITEMS_MAX).map(|item| kept_argument(item, depth + 1, secrets)).collect(),
        ),
        Value::Object(entries) => Value::Object(
            entries
                .iter()
                .take(ARGUMENT_ITEMS_MAX)
                .map(|(key, item)| (key.clone(), kept_argument(item, depth + 1, secrets)))
                .collect(),
        ),
    }
}

/// Redact credential-named fields of an argument tree before it is kept (redactToolArgsForReview).
pub fn redact_argument_fields(value: &Value, secrets: &[String]) -> Value {
    match value {
        Value::Object(entries) => Value::Object(
            entries
                .iter()
                .map(|(key, item)| {
                    if is_credential_field(key) {
                        (key.clone(), Value::String("[redacted]".to_owned()))
                    } else {
                        (key.clone(), redact_argument_fields(item, secrets))
                    }
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(|item| redact_argument_fields(item, secrets)).collect()),
        Value::String(text) => Value::String(sanitize_text(text, secrets)),
        other => other.clone(),
    }
}

/// A call's arguments as a JSON string, or `{}` when they are not an object.
pub fn kept_tool_arguments(args: &Value, secrets: &[String]) -> String {
    if !args.is_object() {
        return "{}".to_owned();
    }
    let kept = kept_argument(&redact_argument_fields(args, secrets), 0, secrets);
    serde_json::to_string(&kept).unwrap_or_else(|_| "{}".to_owned())
}

/// Whether a tool's name marks it as carrying secrets.
pub fn is_secret_tool(name: &str) -> bool {
    name.to_ascii_lowercase().contains("secret")
}

/// Text parts of a tool result, with images replaced by the stand-in.
pub fn content_text(result: &Value) -> Option<String> {
    let content = result.get("content")?.as_array()?;
    let parts: Vec<String> = content
        .iter()
        .filter_map(|part| match part["type"].as_str()? {
            "text" => part["text"].as_str().map(str::to_owned),
            "image" => Some(IMAGE_NOT_KEPT.to_owned()),
            _ => None,
        })
        .collect();
    if parts.is_empty() { None } else { Some(parts.join("\n")) }
}

/// What the model read back from a call, redacted and bounded, when it may be kept.
pub fn kept_tool_result(
    name: &str,
    error: Option<&str>,
    result: &Value,
    paused: bool,
    secrets: &[String],
    max: usize,
) -> Option<String> {
    if paused || is_secret_tool(name) {
        return None;
    }
    let text = if let Some(error) = error {
        sanitize_text(error, secrets)
    } else {
        content_text(result).map(|text| sanitize_text(text.trim(), secrets))?
    };
    let trimmed = text.trim();
    if trimmed.is_empty() { None } else { Some(bounded(trimmed, max, secrets)) }
}

/// Whether a run may replay earlier tool history for its trigger and source.
pub fn run_replays_tool_history(trigger: &str, from_user: bool, non_owner_chat: bool) -> bool {
    if non_owner_chat {
        return false;
    }
    match trigger {
        "user" | "call_end" | "messaging" => true,
        "follow_up" => from_user,
        _ => false,
    }
}

/// Room left in a reply's result budget.
pub fn result_budget_left(kept: usize, total: usize) -> usize {
    total.saturating_sub(kept)
}

/// Whether enough budget remains to keep another result.
pub fn can_keep_result(kept: usize, total: usize) -> bool {
    result_budget_left(kept, total) >= TOOL_RESULT_MIN_CHARS
}

/// Per-result limit inside the remaining reply budget.
pub fn result_limit(kept: usize, total: usize) -> usize {
    result_budget_left(kept, total).min(TOOL_CALL_TEXT_MAX_CHARS)
}

/// Outcome of a completion from its error and exit code.
pub fn completion_outcome(error: Option<&str>, exit_code: Option<i64>) -> &'static str {
    if error.is_some() || exit_code.is_some_and(|code| code != 0) { "failed" } else { "ok" }
}

/// SF Symbol for a completion outcome.
pub fn outcome_icon(outcome: &str) -> &'static str {
    if outcome == "failed" { "xmark.circle.fill" } else { "checkmark.circle.fill" }
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(calls: usize, kept: usize, replay: bool, failed: bool) -> Vec<(&'static str, String)> {
    vec![
        ("calls", count_label(calls)),
        ("kept", count_label(kept)),
        ("replay", if replay { "yes" } else { "no" }.to_owned()),
        ("failed", if failed { "yes" } else { "no" }.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn secrets() -> Vec<String> {
        vec!["supersecret".to_owned()]
    }

    #[test]
    fn credential_redaction() {
        assert!(is_credential_field("api_key"));
        assert!(is_credential_field("Authorization"));
        assert!(!is_credential_field("message"));
        let text = r#"{"api_key":"abc123","message":"hi"}"#;
        let clean = redact_credential_fields(text);
        assert!(clean.contains("\"api_key\":\"[redacted]\""));
        assert!(clean.contains("\"message\":\"hi\""));
        assert!(sanitize_text("key supersecret", &secrets()).contains("[redacted]"));
    }

    #[test]
    fn bounding_keeps_ends() {
        assert_eq!(char_len("héllo"), 5);
        assert_eq!(take_chars("héllo", 2), "hé");
        assert_eq!(take_last_chars("héllo", 2), "lo");
        let text = "x".repeat(3000);
        let kept = bounded(&text, 100, &[]);
        assert!(kept.contains("characters not kept"));
        assert!(char_len(&kept) < 200);
        assert_eq!(bounded("short", 100, &[]), "short");
    }

    #[test]
    fn arguments_are_bounded() {
        let args = json!({"command":"hello","api_key":"abc123","items":[1,2,3]});
        let kept = kept_tool_arguments(&args, &[]);
        assert!(kept.contains("\"api_key\":\"[redacted]\""));
        assert!(kept.contains("hello"));
        assert_eq!(kept_tool_arguments(&json!([1, 2]), &[]), "{}");
        let deep = json!({"a":{"b":{"c":"d"}}});
        assert!(kept_tool_arguments(&deep, &[]).contains("\"c\":\"d\""));
    }

    #[test]
    fn results_and_replay() {
        assert!(is_secret_tool("ReadSecret"));
        assert!(!is_secret_tool("read_file"));
        let result = json!({"content":[{"type":"text","text":"  done  "},{"type":"image"}]});
        let kept = kept_tool_result("read_file", None, &result, false, &[], 100).unwrap();
        assert!(kept.contains("done"));
        assert!(kept.contains(IMAGE_NOT_KEPT));
        assert!(kept_tool_result("secret_tool", None, &result, false, &[], 100).is_none());
        assert!(kept_tool_result("read_file", None, &result, true, &[], 100).is_none());
        assert!(run_replays_tool_history("user", false, false));
        assert!(run_replays_tool_history("follow_up", true, false));
        assert!(!run_replays_tool_history("follow_up", false, false));
        assert!(!run_replays_tool_history("user", false, true));
        assert!(!run_replays_tool_history("schedule", true, false));
    }

    #[test]
    fn budgets_and_presentation() {
        assert_eq!(result_budget_left(100, 8000), 7900);
        assert!(can_keep_result(0, TOOL_REPLY_RESULTS_MAX_CHARS));
        assert!(!can_keep_result(7900, TOOL_REPLY_RESULTS_MAX_CHARS));
        assert_eq!(result_limit(0, 8000), TOOL_CALL_TEXT_MAX_CHARS);
        assert_eq!(completion_outcome(None, Some(0)), "ok");
        assert_eq!(completion_outcome(Some("boom"), None), "failed");
        assert_eq!(outcome_icon("failed"), "xmark.circle.fill");
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(3, 2, true, false).len(), 4);
    }
}
