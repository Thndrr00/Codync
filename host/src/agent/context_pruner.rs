//! Adaptive context pruning for tool results returned to the agent (Ideas 32–33).
//!
//! The MCP bridge is the one funnel through which every host tool result reaches the
//! model. Shaping them there keeps the live context small without dropping anything
//! from the durable chat: a trimmed page can be read again, and an omitted screenshot
//! can be captured again with another screen call.
//!
//! - Idea 32: only the last `KEEP_PAGE_RESULTS` page snapshots stay full; older ones are cut to 1,000 chars.
//! - Idea 33: only the last `KEEP_IMAGES` images stay; older image blocks become a text placeholder.

#[cfg(test)]
use super::loop_guard::MAX_TOOL_RESULT_CHARS;
use super::loop_guard::{KEEP_IMAGES, KEEP_PAGE_RESULTS, cap_tool_output};
use serde_json::{Value, json};
use std::sync::Mutex;

/// Page tools from bdots' current `guards.ts` plus Codync's own screen tools, so results that go
/// stale as the bot moves on are the ones that get shortened.
const PAGE_TOOLS: &[&str] = &[
    "browser_execute",
    "browser_markdown",
    "browser_extract",
    "browser_scrape",
    "browser_links",
    "browser_page",
    "browse_page",
    "get_window_state",
];
/// Length a trimmed page snapshot keeps, from Idea 32.
const PAGE_KEEP_CHARS: usize = 1_000;
const PAGE_NOTE: &str = "[earlier page snapshot trimmed — call the screen tool again for the full page]";
const IMAGE_NOTE: &str = "[earlier screenshot omitted — capture the screen again if you need it]";

#[derive(Debug, Default)]
struct State {
    pages: usize,
    images: usize,
}

/// Session-scoped pruning state for one MCP bridge (one bot process).
#[derive(Debug, Default)]
pub struct ContextPruner {
    state: Mutex<State>,
}

impl ContextPruner {
    pub fn new() -> Self {
        Self::default()
    }

    /// Shapes a tool result just before it goes back to the agent. Returns `true`
    /// when the result changed. The result uses MCP's `{"content": [...]}` shape;
    /// a plain string content is capped but otherwise left alone.
    pub fn shape(&self, tool: &str, result: &mut Value) -> bool {
        let mut state = self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let is_page = PAGE_TOOLS.contains(&tool);
        if is_page {
            state.pages += 1;
        }
        let trim_page = is_page && state.pages > KEEP_PAGE_RESULTS;
        let mut changed = false;

        if let Some(items) = result["content"].as_array_mut() {
            for item in items.iter_mut() {
                match item["type"].as_str().unwrap_or_default() {
                    "image" => {
                        state.images += 1;
                        if state.images > KEEP_IMAGES {
                            *item = json!({"type": "text", "text": IMAGE_NOTE});
                            changed = true;
                        }
                    }
                    "text" => {
                        if trim_page
                            && let Some(text) = item["text"].as_str()
                            && text.chars().count() > PAGE_KEEP_CHARS
                        {
                            let kept: String = text.chars().take(PAGE_KEEP_CHARS).collect();
                            item["text"] = json!(format!("{kept}\n{PAGE_NOTE}"));
                            changed = true;
                        }
                        if let Some(text) = item["text"].as_str().map(str::to_owned)
                            && let capped = cap_tool_output(&text)
                            && capped != text
                        {
                            item["text"] = Value::String(capped);
                            changed = true;
                        }
                    }
                    _ => {}
                }
            }
        } else if let Some(text) = result["content"].as_str().map(str::to_owned) {
            let capped = cap_tool_output(&text);
            if capped != text {
                result["content"] = Value::String(capped);
                changed = true;
            }
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(value: &str) -> Value {
        json!({"content": [{"type": "text", "text": value}]})
    }

    fn image() -> Value {
        json!({"content": [{"type": "image", "data": "base64"}]})
    }

    #[test]
    fn the_fourth_page_snapshot_is_trimmed_to_the_first_thousand_chars() {
        let pruner = ContextPruner::new();
        let page = "x".repeat(2_000);
        for i in 1..=3 {
            let mut result = text(&page);
            assert!(!pruner.shape("browse_page", &mut result), "page {i} stays full");
        }
        let mut fourth = text(&page);
        assert!(pruner.shape("browse_page", &mut fourth));
        let kept = fourth["content"][0]["text"].as_str().unwrap();
        assert!(kept.starts_with(&"x".repeat(PAGE_KEEP_CHARS)));
        assert!(kept.contains(PAGE_NOTE));
        assert!(!kept.contains(&"x".repeat(PAGE_KEEP_CHARS + 1)));
    }

    #[test]
    fn only_the_last_two_screenshots_stay_as_images() {
        let pruner = ContextPruner::new();
        assert!(!pruner.shape("get_window_state", &mut image()));
        assert!(!pruner.shape("get_window_state", &mut image()));
        let mut third = image();
        assert!(pruner.shape("get_window_state", &mut third));
        assert_eq!(third["content"][0]["type"], "text");
        assert_eq!(third["content"][0]["text"], IMAGE_NOTE);
    }

    #[test]
    fn an_unrelated_tool_never_trims_pages_but_still_caps_output() {
        let pruner = ContextPruner::new();
        for _ in 0..5 {
            assert!(!pruner.shape("shell", &mut text("short")));
        }
        let mut huge = text(&"y".repeat(MAX_TOOL_RESULT_CHARS + 500));
        assert!(pruner.shape("shell", &mut huge));
        assert!(huge["content"][0]["text"].as_str().unwrap().contains("truncated"));
    }

    #[test]
    fn a_plain_string_content_is_capped() {
        let pruner = ContextPruner::new();
        let mut result = json!({"content": "z".repeat(MAX_TOOL_RESULT_CHARS + 500)});
        assert!(pruner.shape("chat", &mut result));
        assert!(result["content"].as_str().unwrap().contains("truncated"));
    }
}
