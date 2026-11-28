#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from agentes-chat's `engine/context.ts`: no-reply detection, folded repeats,
//! conversation/turn blocks and prompt-cache-friendly ordering. Composes helpers already ported into
//! this host. Only `botInsights` calls this.

use serde_json::{Value, json};

/// Word a routine sends when it has nothing to say (ROUTINE_NO_REPLY).
pub const ROUTINE_NO_REPLY: &str = "NO_REPLY";

/// Longest fixed part of "# This turn" (THIS_TURN_MAX_CHARS).
pub const THIS_TURN_MAX_CHARS: usize = 2_000;

/// How many routine messages are delivered as history (ROUTINE_DELIVER_HISTORY).
pub const ROUTINE_DELIVER_HISTORY: usize = 12;

/// Minutes after a slot when a scheduled run is late (LATE_AFTER_MINUTES).
pub const LATE_AFTER_MINUTES: u64 = 5;

/// Intro of the per-turn prompt block (THIS_TURN_INTRO).
pub const THIS_TURN_INTRO: &str = "What is true now, for this turn (it replaces what earlier turns said here):";

/// Tools that never appear as work to recall (QUIET_TOOLS).
pub const QUIET_TOOLS: [&str; 3] = ["ask_user", "react", "send_gif"];

/// Whether a message is the no-reply marker (isNoReply).
pub fn is_no_reply(text: &str) -> bool {
    text.trim().lines().last().unwrap_or("").trim().trim_matches(|c| c == '`' || c == '*').trim_end_matches('.')
        == ROUTINE_NO_REPLY
}

/// Whether a text only restates a reaction (restatesReaction).
pub fn restates_reaction(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() || is_no_reply(trimmed) {
        return true;
    }
    if !trimmed.chars().any(|c| c.is_alphanumeric()) {
        return true;
    }
    let lower = trimmed.to_ascii_lowercase();
    let reacted = ["reagi", "reacted", "reaction", "reaccion", "reagito", "reagiert"];
    trimmed.chars().count() <= 120 && reacted.iter().any(|word| lower.contains(word)) && !has_second_sentence(trimmed)
}

/// Whether a text has a second sentence after punctuation.
fn has_second_sentence(text: &str) -> bool {
    let mut after = false;
    for ch in text.chars() {
        if after && ch.is_alphanumeric() {
            return true;
        }
        if matches!(ch, '\n' | '.' | '!' | '?') {
            after = true;
        }
    }
    false
}

/// Fold repeated lines by key, keeping the newest `max` (collapseRepeats).
pub fn collapse_repeats(entries: &[Value], max: usize) -> Vec<String> {
    let mut latest: Vec<(String, String, usize, usize)> = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        let key = entry["key"].as_str().unwrap_or("").to_owned();
        let line = entry["line"].as_str().unwrap_or("").to_owned();
        if let Some(found) = latest.iter_mut().find(|(existing, _, _, _)| *existing == key) {
            found.1 = line;
            found.2 += 1;
            found.3 = index;
        } else {
            latest.push((key, line, 1, index));
        }
    }
    latest.sort_by_key(|(_, _, _, at)| *at);
    latest
        .into_iter()
        .rev()
        .take(max)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|(_, line, count, _)| if count > 1 { format!("{line} ({count} times)") } else { line })
        .collect()
}

/// Human age of a timestamp (ago).
pub fn ago(at_ms: i64, now_ms: i64) -> String {
    let minutes = (now_ms.saturating_sub(at_ms) / 60_000).max(0);
    if minutes < 60 {
        return format!("{minutes} min ago");
    }
    let hours = minutes / 60;
    if hours < 48 {
        return format!("{hours} h ago");
    }
    format!("{} days ago", hours / 24)
}

/// Whether a block is a conversation block.
pub fn is_conversation_block(block: &Value) -> bool {
    block["conversation"] == true
}

/// Whether a block changes from turn to turn.
pub fn is_turn_block(block: &Value) -> bool {
    block["turn"] == true
}

/// Whether a block is read at the top of a turn.
pub fn is_computer_turn_block(block: &Value) -> bool {
    is_turn_block(block) || block["found"] == true
}

/// Conversation section of a prompt (conversationText).
pub fn conversation_text(blocks: &[Value]) -> String {
    let parts: Vec<String> = blocks
        .iter()
        .filter(|block| is_conversation_block(block) && !is_computer_turn_block(block))
        .filter_map(|block| block["text"].as_str().filter(|text| !text.trim().is_empty()))
        .map(level_heading)
        .collect();
    if parts.is_empty() { String::new() } else { format!("# This conversation\n\n{}", parts.join("\n\n")) }
}

/// Demote a block heading one level (levelHeading).
pub fn level_heading(text: &str) -> String {
    text.strip_prefix("# ").map_or_else(|| text.to_owned(), |rest| format!("## {rest}"))
}

/// Stable blocks first, turn blocks last (stableFirst).
pub fn stable_first(blocks: &[Value]) -> Vec<Value> {
    blocks
        .iter()
        .filter(|block| !is_turn_block(block))
        .chain(blocks.iter().filter(|block| is_turn_block(block)))
        .cloned()
        .collect()
}

/// The per-turn prompt in two parts: clipped fixed text and found text (thisTurnParts).
pub fn this_turn_parts(blocks: &[Value], now_ms: i64, include_found: bool) -> Value {
    let turn_blocks: Vec<&Value> = blocks.iter().filter(|block| is_computer_turn_block(block)).collect();
    let fixed: Vec<&Value> = turn_blocks.iter().copied().filter(|block| block["found"] != true).collect();
    let found: Vec<&Value> = if include_found {
        turn_blocks.iter().copied().filter(|block| block["found"] == true).collect()
    } else {
        Vec::new()
    };
    if fixed.is_empty() && found.is_empty() {
        return json!({"head": "", "found": ""});
    }
    let mut ordered = fixed.clone();
    ordered.sort_by_key(|block| if block["now"] == true { 0 } else { 1 });
    let head_parts: Vec<String> = ordered
        .into_iter()
        .map(|block| {
            if block["now"] == true {
                let zone = block["now"]["zone"].as_str().unwrap_or("UTC");
                let who = block["now"]["who"].as_str().unwrap_or("you");
                level_heading(&now_line(zone, who, now_ms))
            } else {
                level_heading(block["text"].as_str().unwrap_or(""))
            }
        })
        .collect();
    let head = clip_text(&head_parts.join("\n\n"), THIS_TURN_MAX_CHARS);
    let found_text = found
        .into_iter()
        .filter_map(|block| block["text"].as_str().map(level_heading))
        .collect::<Vec<_>>()
        .join("\n\n");
    json!({"head": head, "found": found_text})
}

/// The per-turn prompt text (thisTurnText).
pub fn this_turn_text(blocks: &[Value], now_ms: i64, include_found: bool) -> String {
    let parts = this_turn_parts(blocks, now_ms, include_found);
    [parts["head"].as_str().unwrap_or(""), parts["found"].as_str().unwrap_or("")]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// The "now" block line.
pub fn now_line(zone: &str, who: &str, now_ms: i64) -> String {
    format!("# Now · {zone} · {who} · {now_ms}")
}

/// Clip text to `max` characters, appending an ellipsis when cut.
pub fn clip_text(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_owned();
    }
    let mut out: String = text.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// Key of an agent's sticker (stickerNameKey).
pub fn sticker_name_key(agent_id: &str) -> String {
    format!("sticker:{agent_id}")
}

/// Quoted line from message meta (quotedLine).
pub fn quoted_line(meta: Option<&Value>) -> String {
    meta.and_then(|value| value["quote"].as_str())
        .map(|text| format!("> {}", text.replace('\n', "\n> ")))
        .unwrap_or_default()
}

/// Description of attached media (mediaLine).
pub fn media_line(media: &[Value], names: &Value) -> String {
    let labels: Vec<String> = media
        .iter()
        .filter_map(|item| {
            let kind = item["type"].as_str().unwrap_or("file");
            let name = item["id"].as_str().and_then(|id| names[id].as_str()).unwrap_or("file");
            Some(format!("{kind}: {name}"))
        })
        .collect();
    labels.join(", ")
}

/// Line about attached files (attachedLine).
pub fn attached_line(meta: Option<&Value>) -> String {
    let count = meta.and_then(|value| value.get("attachments")).and_then(Value::as_array).map_or(0, Vec::len);
    if count == 0 { String::new() } else { format!("Attached: {count} file{}", if count == 1 { "" } else { "s" }) }
}

/// Note about files the bot itself wrote (ownFilesNote).
pub fn own_files_note(meta: Option<&Value>) -> String {
    let files = meta
        .and_then(|value| value.get("ownFiles"))
        .and_then(Value::as_array)
        .map(|values| values.iter().filter_map(Value::as_str).collect::<Vec<_>>())
        .unwrap_or_default();
    if files.is_empty() { String::new() } else { format!("Wrote: {}", files.join(", ")) }
}

/// Whether a tool is quiet and not worth recalling (isQuietTool).
pub fn is_quiet_tool(name: &str) -> bool {
    QUIET_TOOLS.contains(&name)
}

/// Whether a scheduled run is late (isLateRun).
pub fn is_late_run(minutes_after: u64) -> bool {
    minutes_after > LATE_AFTER_MINUTES
}

/// One line describing a context (contextLine).
pub fn context_line(blocks: usize, turns: usize, found: usize, stable: bool) -> String {
    format!("{blocks} blocks · {turns} turn · {found} found · {}", if stable { "stable" } else { "mixed" })
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(blocks: usize, turns: usize, found: usize, quiet: bool) -> Vec<(&'static str, String)> {
    vec![
        ("blocks", count_label(blocks)),
        ("turns", count_label(turns)),
        ("found", count_label(found)),
        ("quiet", if quiet { "yes" } else { "no" }.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_reply_and_reactions() {
        assert!(is_no_reply("NO_REPLY"));
        assert!(is_no_reply("`NO_REPLY.`"));
        assert!(is_no_reply("hello\nNO_REPLY"));
        assert!(!is_no_reply("hello"));
        assert!(restates_reaction(""));
        assert!(restates_reaction("NO_REPLY"));
        assert!(restates_reaction("👍!"));
        assert!(restates_reaction("Reagi com 👍 à mensagem do Caio"));
        assert!(!restates_reaction("Contei a reação e escrevi mais"));
        assert!(!restates_reaction("Uma resposta real com conteúdo"));
    }

    #[test]
    fn repeats_and_age() {
        let entries =
            vec![json!({"key":"a","line":"read"}), json!({"key":"a","line":"read"}), json!({"key":"b","line":"write"})];
        let folded = collapse_repeats(&entries, 2);
        assert_eq!(folded.len(), 2);
        assert!(folded.iter().any(|line| line == "read (2 times)"));
        assert_eq!(ago(0, 120_000), "2 min ago");
        assert_eq!(ago(0, 3_600_000), "1 h ago");
        assert_eq!(ago(0, 172_800_000), "2 days ago");
    }

    #[test]
    fn blocks_and_prompt() {
        let blocks = vec![
            json!({"text":"# Stable","conversation":true}),
            json!({"text":"# Turn","turn":true}),
            json!({"text":"# Found","found":true}),
        ];
        assert!(is_conversation_block(&blocks[0]));
        assert!(is_turn_block(&blocks[1]));
        assert!(is_computer_turn_block(&blocks[2]));
        assert_eq!(level_heading("# Title"), "## Title");
        assert_eq!(stable_first(&blocks)[0]["text"], "# Stable");
        let conversation = conversation_text(&blocks);
        assert!(conversation.contains("## Stable"));
        let parts = this_turn_parts(&blocks, 1_000, true);
        assert!(parts["found"].as_str().unwrap_or("").contains("Found"));
        let text = this_turn_text(&blocks, 1_000, false);
        assert!(!text.is_empty());
        assert_eq!(clip_text("abcdef", 4), "abc…");
    }

    #[test]
    fn lines_and_summary() {
        assert_eq!(sticker_name_key("a1"), "sticker:a1");
        assert_eq!(quoted_line(Some(&json!({"quote":"hi\nthere"}))), "> hi\n> there");
        assert_eq!(media_line(&[json!({"type":"image","id":"m1"})], &json!({"m1":"shot.png"})), "image: shot.png");
        assert_eq!(attached_line(Some(&json!({"attachments":[1,2]}))), "Attached: 2 files");
        assert_eq!(own_files_note(Some(&json!({"ownFiles":["a.md"]}))), "Wrote: a.md");
        assert!(is_quiet_tool("react"));
        assert!(is_late_run(6));
        assert!(!is_late_run(5));
        assert!(context_line(4, 2, 1, true).contains("blocks"));
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(4, 2, 1, false).len(), 4);
    }
}
