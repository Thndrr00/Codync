//! Turn-level safeguards and context pruning (Ideas 31–34 from `bdots`):
//! - Idea 31: `LoopGuard` with deterministic `stable_json` and `MAX_IDENTICAL_CALLS = 5`.
//! - Idea 32: Adaptive page snapshot pruning (`KEEP_PAGE_RESULTS = 3`).
//! - Idea 33: Screenshot / image payload pruning (`KEEP_IMAGES = 2`).
//! - Idea 34: Tool output cap (`MAX_TOOL_RESULT_CHARS = 12_000`).

use serde_json::{Value, json};
use std::collections::HashMap;

pub const MAX_IDENTICAL_CALLS: usize = 5;
pub const MAX_TOOL_RESULT_CHARS: usize = 12_000;
#[allow(dead_code)]
pub const KEEP_PAGE_RESULTS: usize = 3;
#[allow(dead_code)]
pub const KEEP_IMAGES: usize = 2;

/// Serializes JSON with object keys sorted recursively so semantically identical tool arguments
/// produce the exact same fingerprint regardless of key insertion order.
pub fn stable_json(value: &Value) -> String {
    match value {
        Value::Array(items) => {
            let parts: Vec<String> = items.iter().map(stable_json).collect();
            format!("[{}]", parts.join(","))
        }
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let parts: Vec<String> = keys
                .into_iter()
                .map(|k| {
                    let k_json = serde_json::to_string(k).unwrap_or_default();
                    format!("{k_json}:{}", stable_json(&map[k]))
                })
                .collect();
            format!("{{{}}}", parts.join(","))
        }
        _ => serde_json::to_string(value).unwrap_or_default(),
    }
}

/// Detects when a model gets stuck calling the exact same tool with the exact same arguments
/// repeatedly within a single turn (`MAX_IDENTICAL_CALLS = 5`).
#[derive(Default, Debug, Clone)]
pub struct LoopGuard {
    counts: HashMap<String, usize>,
}

impl LoopGuard {
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a tool call and returns `Some(warning)` once the identical `(tool_name, args)` pair
    /// reaches `MAX_IDENTICAL_CALLS`.
    pub fn check(&mut self, tool_name: &str, args: &Value) -> Option<String> {
        let key = format!("{tool_name}:{}", stable_json(args));
        let count = self.counts.entry(key).and_modify(|n| *n += 1).or_insert(1);
        if *count >= MAX_IDENTICAL_CALLS {
            Some(format!(
                "Stopped: {tool_name} was called {count} times with identical arguments. \
Summarize what worked and what failed instead of retrying the same call."
            ))
        } else {
            None
        }
    }
}

/// Caps a tool output string to `MAX_TOOL_RESULT_CHARS` (`12_000`), preserving char boundaries.
pub fn cap_tool_output(text: &str) -> String {
    if text.len() <= MAX_TOOL_RESULT_CHARS {
        return text.to_owned();
    }
    let mut end = MAX_TOOL_RESULT_CHARS;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    let omitted = text.len() - end;
    format!("{}\n…[truncated {omitted} chars]", &text[..end])
}

/// Prunes older page snapshots (`KEEP_PAGE_RESULTS = 3`), older screenshots (`KEEP_IMAGES = 2`),
/// and truncates oversized tool outputs (`MAX_TOOL_RESULT_CHARS = 12_000`) across a turn's tool messages.
#[allow(dead_code)]
pub fn shape_tool_results(entries: &mut [Value]) {
    let mut page_indices = Vec::new();
    let mut image_indices = Vec::new();

    for (idx, entry) in entries.iter().enumerate() {
        let tool = entry["toolName"].as_str().or_else(|| entry["name"].as_str()).unwrap_or_default();
        if matches!(tool, "browser_page" | "browse_page" | "get_window_state") {
            page_indices.push(idx);
        }
        if entry.get("image").is_some()
            || entry["content"].as_array().is_some_and(|arr| arr.iter().any(|c| c["type"] == "image"))
        {
            image_indices.push(idx);
        }
    }

    let drop_pages = page_indices.len().saturating_sub(KEEP_PAGE_RESULTS);
    for &idx in &page_indices[..drop_pages] {
        entries[idx]["output"] = json!("[earlier page snapshot omitted — call browse_page again if needed]");
    }

    let drop_images = image_indices.len().saturating_sub(KEEP_IMAGES);
    for &idx in &image_indices[..drop_images] {
        if entries[idx].get("image").is_some() {
            entries[idx]["image"] = Value::Null;
            entries[idx]["output"] = json!("[earlier screenshot omitted]");
        }
        if let Some(arr) = entries[idx]["content"].as_array_mut() {
            for item in arr.iter_mut() {
                if item["type"] == "image" {
                    *item = json!({"type": "text", "text": "[earlier screenshot omitted]"});
                }
            }
        }
    }

    for entry in entries.iter_mut() {
        if let Some(s) = entry["output"].as_str().map(str::to_owned)
            && s.len() > MAX_TOOL_RESULT_CHARS
        {
            entry["output"] = Value::String(cap_tool_output(&s));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_json_sorts_keys_and_loop_guard_trips_at_five() {
        let a = json!({"b": 1, "a": {"y": 2, "x": 3}});
        let b = json!({"a": {"x": 3, "y": 2}, "b": 1});
        assert_eq!(stable_json(&a), stable_json(&b));

        let mut guard = LoopGuard::new();
        for _ in 0..4 {
            assert!(guard.check("read_file", &a).is_none());
        }
        let fifth = guard.check("read_file", &b);
        assert!(fifth.is_some());
        assert!(fifth.unwrap().contains("called 5 times"));
    }

    #[test]
    fn shape_tool_results_prunes_old_pages_images_and_long_text() {
        let mut items = vec![
            json!({"toolName": "browse_page", "output": "page 1", "image": "base64_1"}),
            json!({"toolName": "browse_page", "output": "page 2", "image": "base64_2"}),
            json!({"toolName": "browse_page", "output": "page 3", "image": "base64_3"}),
            json!({"toolName": "browse_page", "output": "page 4"}),
            json!({"toolName": "shell", "output": "x".repeat(MAX_TOOL_RESULT_CHARS + 500)}),
        ];
        shape_tool_results(&mut items);
        assert_eq!(items[0]["output"], "[earlier screenshot omitted]");
        assert!(items[0]["image"].is_null());
        assert_eq!(items[1]["output"], "page 2");
        assert_eq!(items[2]["output"], "page 3");
        assert_eq!(items[3]["output"], "page 4");
        assert!(items[4]["output"].as_str().unwrap().contains("truncated 500 chars"));
    }
}
