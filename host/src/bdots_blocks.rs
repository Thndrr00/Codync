#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers from bdots' message-parts and guards sources: shaping message blocks into a
//! transcript view. Composes `bdots_text`, `bdots_more`, `bdots_activity` and `bdots_desk`
//! instead of duplicating them. Only `botInsights` calls this.

use serde_json::Value;

/// One display block in a message transcript.
#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    pub kind: &'static str,
    pub text: String,
    pub icon: &'static str,
    pub tone: &'static str,
    pub says: bool,
}

fn block(kind: &'static str, text: String, icon: &'static str, tone: &'static str, says: bool) -> Block {
    Block { kind, text, icon, tone, says }
}

/// SF Symbol for a message block kind.
pub fn block_icon(kind: &str) -> &'static str {
    match kind {
        "text" => "bubble.left",
        "show" => "rectangle.on.rectangle",
        "steps" => "wrench",
        "event" => "bolt",
        "approval" => "hand.raised",
        _ => "circle",
    }
}

/// Semantic tone for a message block kind.
pub fn block_tone(kind: &str) -> &'static str {
    match kind {
        "text" | "show" => "good",
        "steps" => "secondary",
        "event" => "accent",
        "approval" => "attention",
        _ => "tertiary",
    }
}

/// Whether a block counts as something the bot said.
pub fn says(kind: &str) -> bool {
    matches!(kind, "text" | "show")
}

/// Text of a message from its parts, joined with newlines (messageText).
pub fn message_text(value: &Value) -> String {
    let parts = value.get("parts").and_then(Value::as_array);
    let text: Vec<&str> = parts
        .map(|a| a.iter().filter(|p| p["type"] == "text").filter_map(|p| p["text"].as_str()).collect())
        .unwrap_or_default();
    let joined = text.join("\n");
    crate::bdots_more::shorten(joined.trim(), 400)
}

/// Origin from a message's metadata (originOf).
pub fn origin_of(value: &Value) -> String {
    value.get("metadata").and_then(|m| m.get("origin")).and_then(Value::as_str).unwrap_or("").to_owned()
}

/// Whether a message is a quiet background reply (isQuietReply).
pub fn is_quiet_message(value: &Value) -> bool {
    let text = message_text(value);
    text.trim().is_empty() || text.trim() == "(no update)" || text.trim() == "NO_UPDATE"
}

/// Human label for an origin.
pub fn origin_label(origin: &str) -> &'static str {
    match origin {
        "email" => "Email",
        "routine" => "Routine",
        "check-in" => "Check-in",
        "mention" => "Mention",
        "background" => "Background",
        _ => "Chat",
    }
}

/// SF Symbol for an origin.
pub fn origin_icon(origin: &str) -> &'static str {
    match origin {
        "email" => "envelope",
        "routine" => "calendar.badge.clock",
        "check-in" => "bell.badge",
        "mention" => "at",
        "background" => "moon",
        _ => "bubble.left",
    }
}

/// Build a text block from a message part.
pub fn text_block(text: &str) -> Block {
    block("text", crate::bdots_more::shorten(text, 400), block_icon("text"), block_tone("text"), true)
}

/// Build a show block from a tool result's summary.
pub fn show_block(text: &str) -> Block {
    block("show", crate::bdots_more::shorten(text, 200), block_icon("show"), block_tone("show"), true)
}

/// Build a steps block from a count, reusing the shared group label.
pub fn steps_block(count: usize, has_write: bool) -> Block {
    let label = crate::bdots_more::describe_group("files", &vec![String::new(); count], has_write);
    block("steps", label, block_icon("steps"), block_tone("steps"), false)
}

/// Build an event block from a label.
pub fn event_block(label: &str) -> Block {
    block("event", crate::bdots_more::shorten(label, 120), block_icon("event"), block_tone("event"), false)
}

/// Build an approval block from a title.
pub fn approval_block(title: &str) -> Block {
    block("approval", crate::bdots_more::shorten(title, 120), block_icon("approval"), block_tone("approval"), false)
}

/// Blocks from a raw part value: text, show, steps, event or approval.
pub fn block_from_part(part: &Value) -> Option<Block> {
    match part["type"].as_str()? {
        "text" => part["text"].as_str().map(text_block),
        "show" => part["summary"].as_str().map(show_block),
        "steps" => Some(steps_block(part["calls"].as_array().map_or(0, Vec::len), part["write"] == true)),
        "event" => part["label"].as_str().map(event_block),
        "approval" => part["title"].as_str().map(approval_block),
        _ => None,
    }
}

/// All blocks from a message's parts.
pub fn message_blocks(value: &Value) -> Vec<Block> {
    value
        .get("parts")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(block_from_part).collect())
        .unwrap_or_default()
}

/// Number of blocks that count as speech.
pub fn said_count(blocks: &[Block]) -> usize {
    blocks.iter().filter(|b| b.says).count()
}

/// Number of non-speech blocks (work, events, approvals).
pub fn work_count(blocks: &[Block]) -> usize {
    blocks.len() - said_count(blocks)
}

/// Whether the message is all work and no words.
pub fn work_only(blocks: &[Block]) -> bool {
    !blocks.is_empty() && said_count(blocks) == 0
}

/// The last spoken block, or `None`.
pub fn reply_block(blocks: &[Block]) -> Option<&Block> {
    blocks.iter().rev().find(|b| b.says)
}

/// A one-line transcript preview from blocks.
pub fn transcript_line(blocks: &[Block]) -> String {
    let said = said_count(blocks);
    let work = work_count(blocks);
    if blocks.is_empty() {
        return "Empty message".to_owned();
    }
    format!("{} said · {} work", said, work)
}

/// Compact block count label.
pub fn count_label(blocks: &[Block]) -> String {
    crate::bdots_desk::compact_count(blocks.len() as u64)
}

/// Summary rows for the card.
pub fn summary(blocks: &[Block], origin: &str) -> Vec<(&'static str, String)> {
    vec![
        ("blocks", count_label(blocks)),
        ("said", said_count(blocks).to_string()),
        ("origin", origin_label(origin).to_owned()),
        ("line", transcript_line(blocks)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn message() -> Value {
        json!({
            "id":"m1",
            "metadata":{"origin":"routine"},
            "parts":[
                {"type":"steps","calls":[1,2],"write":true},
                {"type":"text","text":"Done"},
                {"type":"show","summary":"A table"},
                {"type":"event","label":"Started"},
                {"type":"approval","title":"Delete file?"}
            ]
        })
    }

    #[test]
    fn icons_tones_and_says() {
        assert_eq!(block_icon("text"), "bubble.left");
        assert_eq!(block_icon("approval"), "hand.raised");
        assert_eq!(block_tone("event"), "accent");
        assert!(says("text"));
        assert!(says("show"));
        assert!(!says("steps"));
    }

    #[test]
    fn text_origin_and_quiet() {
        assert!(message_text(&message()).contains("Done"));
        assert_eq!(origin_of(&message()), "routine");
        assert!(!is_quiet_message(&message()));
        assert!(is_quiet_message(&json!({"parts":[{"type":"text","text":"(no update)"}]})));
        assert_eq!(origin_label("email"), "Email");
        assert_eq!(origin_icon("mention"), "at");
    }

    #[test]
    fn blocks_from_parts() {
        let blocks = message_blocks(&message());
        assert_eq!(blocks.len(), 5);
        assert_eq!(blocks[0].kind, "steps");
        assert_eq!(blocks[1].kind, "text");
        assert_eq!(blocks[4].kind, "approval");
        assert_eq!(said_count(&blocks), 2);
        assert_eq!(work_count(&blocks), 3);
        assert!(!work_only(&blocks));
        assert_eq!(reply_block(&blocks).map(|b| b.kind), Some("show"));
        assert!(transcript_line(&blocks).contains("2 said"));
    }

    #[test]
    fn work_only_and_empty() {
        let blocks = vec![steps_block(1, false)];
        assert!(work_only(&blocks));
        assert!(transcript_line(&[]).contains("Empty"));
        assert_eq!(count_label(&blocks), "1");
    }

    #[test]
    fn build_helpers_and_summary() {
        assert_eq!(text_block("hello").kind, "text");
        assert_eq!(show_block("x").kind, "show");
        assert_eq!(event_block("y").kind, "event");
        assert_eq!(approval_block("z").kind, "approval");
        let blocks = message_blocks(&message());
        let rows = summary(&blocks, "routine");
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[2].1, "Routine");
    }
}
