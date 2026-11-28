#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from hermes-agent's `history_commentary.py` and `message_content.py`:
//! display-only projection of Codex commentary, think stripping and message content flattening.
//! Composes helpers already ported into this host. Only `botInsights` calls this.

use serde_json::Value;

/// Display-metadata flag marking a held turn whose commentary was never transformed.
pub const COMMENTARY_WITHHELD: &str = "commentary_withheld";

/// Commentary text safe to display: no think blocks and no API keys (visibleCommentary).
pub fn visible_commentary(text: &str) -> String {
    let stripped = crate::bdots_stream::strip_think_blocks(text);
    crate::bdots_mailtext::redact_keys(stripped.trim())
}

/// Whether a Codex item is an assistant message (_phaseMessageItems).
pub fn is_message_item(item: &Value) -> bool {
    item["type"] == "message" && item["role"] == "assistant"
}

/// Normalized phase of an assistant message, empty when unphased.
pub fn item_phase(item: &Value) -> String {
    item["phase"].as_str().map(|phase| phase.trim().to_ascii_lowercase()).unwrap_or_default()
}

/// Text of an assistant item's `output_text` parts.
pub fn item_text(item: &Value) -> String {
    item["content"]
        .as_array()
        .map(|parts| {
            parts
                .iter()
                .filter(|part| part["type"] == "output_text")
                .filter_map(|part| part["text"].as_str())
                .filter(|text| !text.trim().is_empty())
                .copied()
                .collect::<String>()
        })
        .unwrap_or_default()
        .trim()
        .to_owned()
}

/// Items of one or more phases from a stored message (_phaseMessageItems).
pub fn phase_items(message: &Value, phases: &[&str]) -> Vec<String> {
    let raw = message.get("codex_message_items");
    let items: Vec<Value> = match raw {
        Some(Value::String(text)) => serde_json::from_str(text).unwrap_or_default(),
        Some(Value::Array(array)) => array.clone(),
        _ => Vec::new(),
    };
    items
        .iter()
        .filter(|item| is_message_item(item))
        .filter(|item| phases.contains(&item_phase(item).as_str()))
        .map(item_text)
        .filter(|text| !text.is_empty())
        .collect()
}

/// Public commentary items of a message (commentaryItems).
pub fn commentary_items(message: &Value) -> Vec<String> {
    phase_items(message, &["commentary"])
}

/// Final-answer items of a message, including unphased ones (_finalItems).
pub fn final_items(message: &Value) -> Vec<String> {
    phase_items(message, &["final", "final_answer", ""])
}

/// Remove exact public commentary lines from a display copy of reasoning (_withoutFlattenedCommentary).
pub fn without_flattened_commentary(reasoning: &str, commentary: &[String]) -> String {
    reasoning.lines().filter(|line| !commentary.iter().any(|item| item == line)).collect::<Vec<_>>().join("\n")
}

/// Whether a message carries the held-turn commentary flag.
pub fn has_withheld_flag(message: &Value) -> bool {
    message["display_metadata"][COMMENTARY_WITHHELD] == true
}

/// A display-only projection of one message (_projectOne).
pub fn project_message(message: &Value, enabled: bool) -> Value {
    let commentary = commentary_items(message);
    let withheld = has_withheld_flag(message);
    let content = if enabled && !withheld { final_items(message).join("\n") } else { String::new() };
    serde_json::json!({
        "content": content,
        "commentary": commentary,
        "withheld": withheld,
    })
}

/// A display-only projection of a history (_projectHistory).
pub fn project_history(messages: &[Value], enabled: bool) -> Vec<Value> {
    messages.iter().map(|message| project_message(message, enabled)).collect()
}

/// Text of one content part (_textFromPart).
pub fn text_from_part(part: &Value) -> String {
    if part["type"] == "text" || part["type"] == "input_text" || part["type"] == "output_text" {
        part["text"].as_str().unwrap_or("").to_owned()
    } else {
        String::new()
    }
}

/// Flatten message content parts with a separator (flattenMessageText).
pub fn flatten_with_sep(parts: &[Value], sep: &str) -> String {
    parts.iter().map(text_from_part).filter(|text| !text.is_empty()).collect::<Vec<_>>().join(sep)
}

/// Flatten a message's `content` field, whether a string or an array of parts.
pub fn flatten_message_text(content: &Value) -> String {
    match content {
        Value::String(text) => text.clone(),
        Value::Array(parts) => flatten_with_sep(parts, "\n"),
        Value::Object(_) => content["text"].as_str().unwrap_or("").to_owned(),
        _ => String::new(),
    }
}

/// Number of visible characters after think blocks are stripped.
pub fn visible_length(text: &str) -> usize {
    crate::bdots_stream::strip_think_blocks(text).trim().chars().count()
}

/// Number of public commentary items in a message.
pub fn commentary_count(message: &Value) -> usize {
    commentary_items(message).len()
}

/// Number of final items in a message.
pub fn final_count(message: &Value) -> usize {
    final_items(message).len()
}

/// Display kind of a projected message.
pub fn display_kind(projected: &Value) -> &'static str {
    if projected["withheld"] == true { "hidden" } else { "visible" }
}

/// Semantic tone of a projected message's display kind.
pub fn display_tone(projected: &Value) -> &'static str {
    if projected["withheld"] == true { "secondary" } else { "good" }
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(messages: usize, commentary: usize, finals: usize, withheld: usize) -> Vec<(&'static str, String)> {
    vec![
        ("messages", count_label(messages)),
        ("commentary", count_label(commentary)),
        ("finals", count_label(finals)),
        ("withheld", count_label(withheld)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn message() -> Value {
        json!({
            "codex_message_items": [
                {"type":"message","role":"assistant","phase":"commentary","content":[{"type":"output_text","text":"Looking at files"}]},
                {"type":"message","role":"assistant","phase":"final","content":[{"type":"output_text","text":"Done."}]},
                {"type":"message","role":"user","phase":"final","content":[{"type":"output_text","text":"ignored"}]}
            ],
            "display_metadata": {"commentary_withheld": true}
        })
    }

    #[test]
    fn commentary_projection() {
        let msg = message();
        assert_eq!(commentary_items(&msg), vec!["Looking at files"]);
        assert_eq!(final_items(&msg), vec!["Done."]);
        assert_eq!(commentary_count(&msg), 1);
        assert_eq!(final_count(&msg), 1);
        assert!(has_withheld_flag(&msg));
    }

    #[test]
    fn visible_and_reasoning() {
        assert_eq!(visible_commentary(" <think>x</think> hello "), "hello");
        assert_eq!(visible_length(" <think>secret</think> hi "), 2);
        assert_eq!(
            without_flattened_commentary("intro\nLooking at files\noutro", &["Looking at files".to_owned()]),
            "intro\noutro"
        );
    }

    #[test]
    fn items_and_phases() {
        let item = json!({"type":"message","role":"assistant","phase":" Commentary ","content":[{"type":"output_text","text":"a"},{"type":"output_text","text":"b"}]});
        assert!(is_message_item(&item));
        assert_eq!(item_phase(&item), "commentary");
        assert_eq!(item_text(&item), "ab");
        assert!(!is_message_item(&json!({"type":"message","role":"user"})));
        assert_eq!(phase_items(&json!({"codex_message_items":"not json"}), &["final"]).len(), 0);
    }

    #[test]
    fn projecting_and_flattening() {
        let msg = message();
        let projected = project_message(&msg, true);
        assert_eq!(projected["content"], "");
        assert_eq!(projected["commentary"][0], "Looking at files");
        assert_eq!(display_kind(&projected), "hidden");
        assert_eq!(display_tone(&projected), "secondary");
        assert_eq!(project_history(&[msg.clone()], false).len(), 1);
        let parts = vec![json!({"type":"text","text":"a"}), json!({"type":"input_text","text":"b"})];
        assert_eq!(flatten_with_sep(&parts, " "), "a b");
        assert_eq!(flatten_message_text(&json!("plain")), "plain");
        assert_eq!(flatten_message_text(&parts), "a\nb");
    }

    #[test]
    fn labels_and_summary() {
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(4, 1, 2, 1).len(), 4);
    }
}
