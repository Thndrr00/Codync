//! Tool output limits shared by the turn guard and the context pruner.
//! Repeated failures are handled by the per-turn guard in `bot::stall`.
//! - Ideas 32–33: page and image pruning lives in `super::context_pruner`.
//! - Idea 34: Tool output cap (`MAX_TOOL_RESULT_CHARS = 12_000`).

pub const MAX_TOOL_RESULT_CHARS: usize = 12_000;
pub const KEEP_PAGE_RESULTS: usize = 3;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cap_keeps_a_char_boundary_and_reports_what_was_omitted() {
        let mut text = "a".repeat(MAX_TOOL_RESULT_CHARS);
        text.push('é');
        text.push_str("the rest");
        let capped = cap_tool_output(&text);
        assert!(capped.contains("truncated"), "{capped}");
        assert!(capped.is_char_boundary(capped.len()));
    }
}
