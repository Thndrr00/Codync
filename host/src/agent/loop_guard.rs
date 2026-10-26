//! Tool output limits and context pruning.
//! Repeated failures are handled by the per-turn guard in `bot::stall`.
//! - Idea 32: Adaptive page snapshot pruning (`KEEP_PAGE_RESULTS = 3`).
//! - Idea 33: Screenshot / image payload pruning (`KEEP_IMAGES = 2`).
//! - Idea 34: Tool output cap (`MAX_TOOL_RESULT_CHARS = 12_000`).

use serde_json::{Value, json};

pub const MAX_TOOL_RESULT_CHARS: usize = 12_000;
#[allow(dead_code)]
pub const KEEP_PAGE_RESULTS: usize = 3;
#[allow(dead_code)]
pub const KEEP_IMAGES: usize = 2;

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
