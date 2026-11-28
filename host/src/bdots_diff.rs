#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from coucou's `windows/src/core/diff.ts`: file names, diff step markers,
//! one-line previews and size limits. Only `botInsights` calls this.

use serde_json::{Value, json};

/// Marker that starts a diff step (DIFF_STEP_MARKER).
pub const DIFF_STEP_MARKER: &str = "?";

/// Largest diff kept in memory, in bytes (DIFF_MAX_BYTES).
pub const DIFF_MAX_BYTES: u64 = 200 * 1024;

/// Largest diff kept in memory, in lines (DIFF_MAX_LINES).
pub const DIFF_MAX_LINES: usize = 4_000;

/// Kinds of a diff line (DiffKind).
pub const DIFF_KINDS: [&str; 3] = ["context", "added", "removed"];

/// Tools that produce an editable file diff.
pub const DIFF_TOOLS: [&str; 4] = ["Write", "Edit", "NotebookEdit", "MultiEdit"];

/// File name from a path, on either separator (fileName).
pub fn file_name(path: &str) -> &str {
    let cleaned = path.trim_end_matches(['/', '\\']);
    let index = cleaned.rfind(['/', '\\']);
    index.map_or(cleaned, |index| &cleaned[index + 1..])
}

/// UTF-8 length of a string (utf8Length).
pub fn utf8_length(text: &str) -> u64 {
    text.len() as u64
}

/// Lines of a text, normalizing CRLF (splitLines).
pub fn split_lines(text: &str) -> Vec<&str> {
    text.replace("\r\n", "\n").split('\n').map(str::trim_end).collect()
}

/// Whether a diff is too big to keep (tooLarge).
pub fn too_large(text: &str, other: Option<&str>) -> bool {
    let bytes = utf8_length(text) + other.map_or(0, utf8_length);
    bytes > DIFF_MAX_BYTES || split_lines(text).len() + other.map_or(0, |text| split_lines(text).len()) > DIFF_MAX_LINES
}

/// A diff of a newly written file (fromNew).
pub fn from_new(content: &str, path: &str) -> Value {
    let lines = split_lines(content);
    json!({
        "path": path,
        "added": lines.len(),
        "removed": 0,
        "tooLarge": too_large(content, None),
        "isNewFile": true,
    })
}

/// Count line changes when the full diff doesn't fit (countFallback).
pub fn count_fallback(old_text: &str, new_text: &str, path: &str) -> Value {
    json!({
        "path": path,
        "added": split_lines(new_text).len(),
        "removed": split_lines(old_text).len(),
        "tooLarge": true,
        "isNewFile": false,
    })
}

/// A diff built from an edit, falling back to counts when too big (fromEdit).
pub fn from_edit(old_text: &str, new_text: &str, path: &str) -> Value {
    if too_large(old_text, Some(new_text)) {
        return count_fallback(old_text, new_text, path);
    }
    let old_lines = split_lines(old_text);
    let new_lines = split_lines(new_text);
    let mut added = 0;
    let mut removed = 0;
    for line in &new_lines {
        if !old_lines.contains(line) {
            added += 1;
        }
    }
    for line in &old_lines {
        if !new_lines.contains(line) {
            removed += 1;
        }
    }
    json!({
        "path": path,
        "added": added,
        "removed": removed,
        "tooLarge": false,
        "isNewFile": false,
    })
}

/// Build a diff from a tool call's arguments (buildFileDiff).
pub fn build_file_diff(tool: &str, input: Option<&Value>) -> Value {
    let Some(input) = input.filter(|input| input.is_object()) else {
        return Value::Null;
    };
    let value = |key: &str| input[key].as_str();
    let path = value("file_path");
    match tool {
        "Write" => path.map(|path| from_new(value("content").unwrap_or(""), path)).unwrap_or(Value::Null),
        "Edit" => match (path, value("old_string"), value("new_string")) {
            (Some(path), Some(old), Some(new)) if !old.is_empty() || !new.is_empty() => from_edit(old, new, path),
            _ => Value::Null,
        },
        _ => Value::Null,
    }
}

/// A one-line preview of a text, with paragraphs joined (toOneLine).
pub fn to_one_line(text: &str, max_chars: usize) -> String {
    let mut paragraphs: Vec<String> = Vec::new();
    let mut current: Vec<String> = Vec::new();
    for raw in text.replace("\r\n", "\n").split('\n') {
        let trimmed = raw.trim_matches([' ', '\t']);
        let horizontal_rule = trimmed.len() >= 3
            && (trimmed.chars().all(|c| c == '-')
                || trimmed.chars().all(|c| c == '*')
                || trimmed.chars().all(|c| c == '_'));
        if trimmed.is_empty() || horizontal_rule || trimmed.starts_with('|') {
            if !current.is_empty() {
                paragraphs.push(current.join(" "));
                current.clear();
            }
        } else {
            current.push(trimmed.to_owned());
        }
    }
    if !current.is_empty() {
        paragraphs.push(current.join(" "));
    }
    let mut out = paragraphs.join("\n");
    if out.chars().count() > max_chars {
        out = out.chars().take(max_chars.saturating_sub(1)).collect();
        out.push('…');
    }
    out
}

/// Whether a step is a diff step (isDiffStep).
pub fn is_diff_step(step: &str) -> bool {
    step.starts_with(DIFF_STEP_MARKER)
}

/// Encode a step for a file's diff (makeDiffStep).
pub fn make_diff_step(filename: &str, added: i64, removed: i64, diff_id: i64) -> String {
    format!("{DIFF_STEP_MARKER}{filename}\t{added}:{removed}:{diff_id}")
}

/// Parse a diff step, or null when it isn't one (parseDiffStep).
pub fn parse_diff_step(step: &str) -> Value {
    if !is_diff_step(step) {
        return Value::Null;
    }
    let body = &step[DIFF_STEP_MARKER.len()..];
    let Some(tab) = body.find('\t') else {
        return Value::Null;
    };
    let parts: Vec<&str> = body[tab + 1..].split(':').collect();
    if parts.len() != 3 || !parts.iter().all(|part| part.parse::<i64>().is_ok()) {
        return Value::Null;
    }
    let numbers: Vec<i64> = parts.iter().filter_map(|part| part.parse().ok()).collect();
    json!({"filename": &body[..tab], "added": numbers[0], "removed": numbers[1], "diffId": numbers[2]})
}

/// Last non-diff step of a sequence (lastTextStep).
pub fn last_text_step(steps: &[String]) -> Option<&str> {
    steps.iter().rev().find(|step| !is_diff_step(step)).map(String::as_str)
}

/// One line describing a diff (diffLine).
pub fn diff_line(path: &str, added: u64, removed: u64) -> String {
    format!("{} · +{added} −{removed}", file_name(path))
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(kinds: usize, tools: usize, lines: usize, large: bool) -> Vec<(&'static str, String)> {
    vec![
        ("kinds", count_label(kinds)),
        ("tools", count_label(tools)),
        ("lines", count_label(lines)),
        ("large", if large { "yes" } else { "no" }.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_and_paths() {
        assert_eq!(DIFF_STEP_MARKER, "?");
        assert_eq!(DIFF_MAX_BYTES, 200 * 1024);
        assert_eq!(DIFF_MAX_LINES, 4_000);
        assert_eq!(DIFF_KINDS.len(), 3);
        assert_eq!(DIFF_TOOLS.len(), 4);
        assert_eq!(file_name("a/b/c.rs"), "c.rs");
        assert_eq!(file_name("a\\b\\c.rs"), "c.rs");
        assert_eq!(file_name("c.rs/"), "c.rs");
        assert_eq!(utf8_length("é"), 2);
        assert_eq!(split_lines("a\r\nb").len(), 2);
    }

    #[test]
    fn diffs() {
        assert!(!too_large("a\nb", None));
        assert!(too_large(&"x".repeat(DIFF_MAX_BYTES as usize), None));
        assert_eq!(from_new("a\nb", "new.rs")["added"], 2);
        assert_eq!(from_new("a\nb", "new.rs")["isNewFile"], true);
        assert_eq!(count_fallback("a\nb", "a\nb\nc", "x.rs")["removed"], 2);
        let edit = from_edit("a\nb", "a\nc", "x.rs");
        assert_eq!(edit["added"], 1);
        assert_eq!(edit["removed"], 1);
        let built = build_file_diff("Write", Some(&json!({"file_path":"x.rs","content":"hi"})));
        assert_eq!(built["added"], 1);
        assert_eq!(
            build_file_diff("Edit", Some(&json!({"file_path":"x.rs","old_string":"","new_string":""}))),
            Value::Null
        );
        assert_eq!(build_file_diff("Other", None), Value::Null);
    }

    #[test]
    fn previews() {
        assert_eq!(to_one_line("a\nb", 200), "a\nb");
        assert_eq!(to_one_line("a\n---\nb", 200), "a\nb");
        assert_eq!(to_one_line("abcde", 3), "ab…");
        assert!(to_one_line("a\n| x |\nb", 200).contains("a\nb"));
    }

    #[test]
    fn steps_and_presentation() {
        assert!(is_diff_step("?x.rs\t1:2:3"));
        assert!(!is_diff_step("plain"));
        let step = make_diff_step("x.rs", 1, 2, 3);
        let parsed = parse_diff_step(&step);
        assert_eq!(parsed["filename"], "x.rs");
        assert_eq!(parsed["added"], 1);
        assert_eq!(parsed["diffId"], 3);
        assert_eq!(parse_diff_step("?x.rs\tbad"), Value::Null);
        assert_eq!(parse_diff_step("plain"), Value::Null);
        let steps = vec!["hello".to_owned(), make_diff_step("x.rs", 1, 2, 3)];
        assert_eq!(last_text_step(&steps), Some("hello"));
        assert_eq!(diff_line("a/b/x.rs", 3, 1), "x.rs · +3 −1");
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(3, 4, 100, false).len(), 4);
    }
}
