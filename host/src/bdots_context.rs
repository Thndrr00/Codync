#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from rakazo v0.14.0's context-pruning source: which tool results carry
//! live page state, how many to keep, and how image budgets bound screenshots. Composes the
//! validators already ported into this host instead of duplicating them. Only `botInsights` calls this.

/// Tools whose result is a view of the current page or screen (rakazo PAGE_STATE_TOOL_NAMES).
pub fn is_page_state_tool(name: &str) -> bool {
    matches!(name, "browser_navigate" | "browser_snapshot" | "browser_act" | "computer_observe" | "computer_act")
}

/// Default count of page-state results to keep.
pub fn default_page_keep() -> usize {
    3
}

/// Default count of computer screenshots to keep.
pub fn default_screenshot_keep() -> usize {
    2
}

/// Minimum chars for a page-state result to be worth trimming (STALE_PAGE_STATE_MIN_CHARS).
pub fn stale_min_chars() -> usize {
    1_000
}

/// Whether a page-state result is large enough to be pruned.
pub fn is_large_page(text: &str) -> bool {
    text.chars().count() >= stale_min_chars()
}

/// Normalise a requested keep count (Math.max(0, Math.floor(keep))).
pub fn normalize_keep(keep: i64) -> usize {
    keep.max(0) as usize
}

/// Normalise a model image budget: negative or zero becomes no images.
pub fn normalize_image_limit(limit: Option<i64>) -> Option<usize> {
    limit.map(|l| l.max(0) as usize)
}

/// Text length of a tool result's parts.
pub fn text_length(parts: &[serde_json::Value]) -> usize {
    parts.iter().filter(|p| p["type"] == "text").filter_map(|p| p["text"].as_str()).map(str::len).sum()
}

/// Number of image parts in a tool result.
pub fn image_part_count(parts: &[serde_json::Value]) -> usize {
    parts.iter().filter(|p| p["type"] == "image" || p["type"] == "input_image").count()
}

/// Whether a tool result carries a screenshot (computer screenshot message).
pub fn is_screenshot(tool: &str, parts: &[serde_json::Value]) -> bool {
    tool.starts_with("computer_") && image_part_count(parts) > 0
}

/// Whether a tool result is an error diagnostic, not fresh page state.
pub fn is_error_result(value: &serde_json::Value) -> bool {
    value["isError"] == true || value["error"].is_string()
}

/// Whether a page-state result should be kept rather than trimmed.
pub fn should_keep_page(text: &str, is_error: bool, remaining: usize) -> bool {
    if is_error || !is_large_page(text) {
        return true;
    }
    remaining > 0
}

/// Remaining budget after keeping `images` screenshots.
pub fn remaining_after(remaining: usize, images: usize) -> usize {
    remaining.saturating_sub(images)
}

/// Whether a screenshot fits in the remaining budget.
pub fn screenshot_fits(remaining: usize, images: usize) -> bool {
    remaining >= images
}

/// Error line when non-screenshot images exceed the model budget (rakazo ModelImageLimitError).
pub fn image_limit_error(limit: usize, present: usize) -> String {
    format!("The configured model image limit is {limit}, but the prompt contains {present} non-screenshot images.")
}

/// Whether a model image budget is exceeded by non-screenshot images.
pub fn images_exceed_limit(limit: Option<usize>, non_screenshot: usize) -> bool {
    limit.is_some_and(|l| non_screenshot > l)
}

/// Effective screenshot budget with an image limit present.
pub fn effective_screenshot_budget(limit: Option<usize>, non_screenshot: usize) -> usize {
    let base = default_screenshot_keep();
    match limit {
        Some(l) => base.min(l.saturating_sub(non_screenshot)),
        None => base,
    }
}

/// Human note for a pruned screenshot.
pub fn screenshot_note() -> &'static str {
    "Screenshot removed to fit the model's image budget."
}

/// Human note for pruned page state.
pub fn page_note() -> &'static str {
    "Older page state removed; the latest snapshot is above."
}

/// SF Symbol for a pruning decision.
pub fn prune_icon(pruned: bool) -> &'static str {
    if pruned { "trash.circle.fill" } else { "checkmark.circle.fill" }
}

/// Semantic tone for a pruning decision.
pub fn prune_tone(pruned: bool) -> &'static str {
    if pruned { "attention" } else { "good" }
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(page: usize, screenshots: usize, keep: usize) -> Vec<(&'static str, String)> {
    vec![
        ("page", count_label(page)),
        ("screenshots", count_label(screenshots)),
        ("keep", keep.to_string()),
        ("note", page_note().to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn parts() -> Vec<serde_json::Value> {
        vec![json!({"type":"text","text":"hello"}), json!({"type":"image","data":"x"})]
    }

    #[test]
    fn page_tools_and_defaults() {
        assert!(is_page_state_tool("browser_snapshot"));
        assert!(is_page_state_tool("computer_observe"));
        assert!(!is_page_state_tool("read"));
        assert_eq!(default_page_keep(), 3);
        assert_eq!(default_screenshot_keep(), 2);
        assert_eq!(stale_min_chars(), 1000);
        assert!(is_large_page(&"x".repeat(1000)));
        assert!(!is_large_page("short"));
    }

    #[test]
    fn normalization_and_parts() {
        assert_eq!(normalize_keep(-2), 0);
        assert_eq!(normalize_image_limit(Some(4)), Some(4));
        assert_eq!(normalize_image_limit(Some(-1)), Some(0));
        assert_eq!(normalize_image_limit(None), None);
        assert_eq!(text_length(&parts()), 5);
        assert_eq!(image_part_count(&parts()), 1);
        assert!(is_screenshot("computer_act", &parts()));
        assert!(!is_screenshot("browser_act", &parts()));
    }

    #[test]
    fn errors_and_keeping() {
        assert!(is_error_result(&json!({"isError":true})));
        assert!(is_error_result(&json!({"error":"boom"})));
        assert!(!is_error_result(&json!({})));
        assert!(should_keep_page("short", false, 0));
        assert!(should_keep_page(&"x".repeat(1000), true, 0));
        assert!(should_keep_page(&"x".repeat(1000), false, 1));
        assert!(!should_keep_page(&"x".repeat(1000), false, 0));
    }

    #[test]
    fn budgets_and_notes() {
        assert_eq!(remaining_after(3, 1), 2);
        assert!(screenshot_fits(2, 2));
        assert!(!screenshot_fits(1, 2));
        assert!(image_limit_error(4, 6).contains("limit is 4"));
        assert!(images_exceed_limit(Some(4), 6));
        assert!(!images_exceed_limit(None, 6));
        assert_eq!(effective_screenshot_budget(Some(4), 3), 1);
        assert_eq!(effective_screenshot_budget(None, 3), 2);
        assert!(screenshot_note().contains("Screenshot"));
        assert!(page_note().contains("page state"));
    }

    #[test]
    fn icons_and_summary() {
        assert_eq!(prune_icon(true), "trash.circle.fill");
        assert_eq!(prune_tone(false), "good");
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(3, 2, 2).len(), 4);
    }
}
