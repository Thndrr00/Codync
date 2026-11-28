#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from hermes-agent's `stream_delivery.py`: held-row withholding, interim
//! text dedup, Codex commentary visibility, stream writer ownership and delta fan-out decisions.
//! Composes helpers already ported into this host. Only `botInsights` calls this.

/// Mark a held turn's row as hidden when it carries visible text (withholdHeldRow).
pub fn withhold_held_row(display_kind: Option<&str>, held: bool, has_visible_text: bool) -> Option<&'static str> {
    if held && display_kind.is_none() && has_visible_text { Some("hidden") } else { display_kind }
}

/// Result of publishing a held final row: whether to clear `hidden` and whether to stitch.
pub fn publish_held_final_row(held: bool, display_kind: Option<&str>, joined: bool, differs: bool) -> (bool, bool) {
    if !held {
        return (false, false);
    }
    (display_kind == Some("hidden"), joined && differs)
}

/// Whether a stream delta may leave while a turn is held (deliverToStreamCallbacks).
pub fn should_deliver_delta(held: bool, has_consumer: bool) -> bool {
    !held && has_consumer
}

/// Remove inline `<think>…</think>` blocks from stream text.
pub fn strip_think_blocks(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("<think>") {
        out.push_str(&rest[..start]);
        rest = &rest[start + "<think>".len()..];
        match rest.find("</think>") {
            Some(end) => rest = &rest[end + "</think>".len()..],
            None => return out,
        }
    }
    out.push_str(rest);
    out
}

/// Normalise interim text: no think blocks, whitespace collapsed (normalizeInterimVisibleText).
pub fn normalize_interim_text(text: &str) -> String {
    strip_think_blocks(text).split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Text visible after removing withheld commentary (interimAssistantVisibleText).
pub fn visible_text(streamed: &str, interim: &str) -> String {
    let visible = normalize_interim_text(&strip_think_blocks(interim));
    let mut out: Vec<&str> = Vec::new();
    let mut remaining = streamed.trim();
    for word in visible.split_whitespace() {
        if !remaining.is_empty() && remaining.starts_with(word) {
            remaining = remaining[word.len()..].trim_start();
        } else {
            out.push(word);
        }
    }
    out.join(" ")
}

/// Whether all of an interim message was already streamed (interimContentFullyStreamed).
pub fn is_fully_streamed(streamed: &str, interim: &str) -> bool {
    let text = normalize_interim_text(interim);
    !text.is_empty() && streamed.contains(&text)
}

/// Whether some but not all of an interim message was streamed.
pub fn is_partially_streamed(streamed: &str, interim: &str) -> bool {
    if is_fully_streamed(streamed, interim) {
        return false;
    }
    normalize_interim_text(interim).split_whitespace().any(|word| streamed.contains(word))
}

/// Only the part of `next` not already delivered as `previous` (interim dedup).
pub fn dedup_interim(previous: &str, next: &str) -> String {
    let previous = previous.trim();
    let next = next.trim();
    if next.starts_with(previous) {
        next[previous.len()..].trim().to_owned()
    } else if previous.starts_with(next) {
        String::new()
    } else {
        next.to_owned()
    }
}

/// Whether an interim is identical to what was already delivered.
pub fn is_duplicate_interim(previous: &str, next: &str) -> bool {
    !next.is_empty() && dedup_interim(previous, next).is_empty()
}

/// Visible parts of a Codex message, without its commentary (extractCodexInterimVisibleParts).
pub fn codex_visible_parts(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    loop {
        let Some(start) = rest.find("<commentary>") else {
            let tail = normalize_interim_text(rest);
            if !tail.is_empty() {
                out.push(tail);
            }
            break;
        };
        let before = normalize_interim_text(&rest[..start]);
        if !before.is_empty() {
            out.push(before);
        }
        match rest[start..].find("</commentary>") {
            Some(end) => rest = &rest[start + "</commentary>".len() + end..],
            None => break,
        }
    }
    out
}

/// Whether a message carries withheld Codex commentary.
pub fn has_commentary(text: &str) -> bool {
    text.contains("<commentary>") || text.contains("</commentary>")
}

/// Claim the stream writer when no other writer owns it (claimStreamWriter).
pub fn claim_stream_writer(current: Option<&str>, writer: &str) -> bool {
    current.is_none_or(|owner| owner == writer)
}

/// Whether a writer still owns the stream (streamWriterIsCurrent).
pub fn is_writer_current(current: Option<&str>, writer: &str) -> bool {
    current == Some(writer)
}

/// Whether a writer has been superseded by another one.
pub fn is_writer_superseded(current: Option<&str>, writer: &str) -> bool {
    current.is_some_and(|owner| owner != writer)
}

/// Human label for a stream event (emitStreamStart / emitStreamEnd).
pub fn stream_event(event: &str, ending: bool) -> String {
    let label = if ending { "stream.end" } else { "stream.start" };
    format!("{label}:{event}")
}

/// Whether any stream consumer is attached (hasStreamConsumers).
pub fn has_consumers(delta: bool, reasoning: bool, hooks: bool) -> bool {
    delta || reasoning || hooks
}

/// Human label for a delta kind.
pub fn delta_kind_label(kind: &str) -> &'static str {
    match kind.to_ascii_lowercase().as_str() {
        "delta" => "Text",
        "reasoning" => "Reasoning",
        "tool_gen" => "Tool generation",
        "interim" => "Interim",
        _ => "Other",
    }
}

/// Split text into chunks of at most `max` chars, breaking on whitespace.
pub fn chunk_text(text: &str, max: usize) -> Vec<String> {
    let limit = max.max(1);
    let mut chunks = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.is_empty() {
            current.push_str(word);
        } else if current.chars().count() + 1 + word.chars().count() <= limit {
            current.push(' ');
            current.push_str(word);
        } else {
            chunks.push(std::mem::take(&mut current));
            current.push_str(word);
        }
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

/// Number of chunks a text would produce.
pub fn chunk_count(text: &str, max: usize) -> usize {
    chunk_text(text, max).len()
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(chunks: usize, held: bool, consumers: bool) -> Vec<(&'static str, String)> {
    vec![
        ("chunks", count_label(chunks)),
        ("held", if held { "yes" } else { "no" }.to_owned()),
        ("consumers", if consumers { "live" } else { "none" }.to_owned()),
        ("state", if held { "withheld" } else { "live" }.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn held_rows() {
        assert_eq!(withhold_held_row(None, true, true), Some("hidden"));
        assert_eq!(withhold_held_row(Some("visible"), true, true), Some("visible"));
        assert_eq!(withhold_held_row(None, false, true), None);
        assert_eq!(publish_held_final_row(true, Some("hidden"), true, true), (true, true));
        assert_eq!(publish_held_final_row(false, Some("hidden"), true, true), (false, false));
        assert!(should_deliver_delta(false, true));
        assert!(!should_deliver_delta(true, true));
    }

    #[test]
    fn interim_text() {
        assert_eq!(strip_think_blocks("a<think>secret</think>b"), "ab");
        assert_eq!(strip_think_blocks("a<think>open"), "a");
        assert_eq!(normalize_interim_text(" a <think>x</think> b "), "a b");
        assert_eq!(visible_text("a b", "a b"), "");
        assert_eq!(visible_text("a", "a b"), "b");
        assert!(is_fully_streamed("hello world", "hello world"));
        assert!(!is_fully_streamed("hello", "hello world"));
        assert!(is_partially_streamed("hello", "hello world"));
    }

    #[test]
    fn dedup_and_commentary() {
        assert_eq!(dedup_interim("hello", "hello world"), "world");
        assert_eq!(dedup_interim("hello world", "hello"), "");
        assert_eq!(dedup_interim("hi", "bye"), "bye");
        assert!(is_duplicate_interim("hello", "hello"));
        assert_eq!(codex_visible_parts("a<commentary>hidden</commentary>b"), vec!["a", "b"]);
        assert!(has_commentary("<commentary>x</commentary>"));
        assert!(!has_commentary("plain"));
    }

    #[test]
    fn writers_and_events() {
        assert!(claim_stream_writer(None, "w1"));
        assert!(claim_stream_writer(Some("w1"), "w1"));
        assert!(!claim_stream_writer(Some("w2"), "w1"));
        assert!(is_writer_current(Some("w1"), "w1"));
        assert!(is_writer_superseded(Some("w2"), "w1"));
        assert_eq!(stream_event("reply", false), "stream.start:reply");
        assert_eq!(stream_event("reply", true), "stream.end:reply");
        assert!(has_consumers(false, false, true));
        assert_eq!(delta_kind_label("reasoning"), "Reasoning");
    }

    #[test]
    fn chunks_and_summary() {
        assert_eq!(chunk_text("one two three", 7), vec!["one two", "three"]);
        assert_eq!(chunk_count("one two three", 7), 2);
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(3, true, false).len(), 4);
    }
}
