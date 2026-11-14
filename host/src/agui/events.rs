//! AG-UI wire names and event helpers.
//!
//! The protocol's events are plain JSON objects with a `type` (its uppercase name) and the fields that event
//! carries. Everything here builds those objects and frames them for `text/event-stream`; no AG-UI package is used.

use serde_json::{Value, json};

pub const RUN_STARTED: &str = "RUN_STARTED";
pub const RUN_FINISHED: &str = "RUN_FINISHED";
pub const RUN_ERROR: &str = "RUN_ERROR";
pub const TEXT_MESSAGE_START: &str = "TEXT_MESSAGE_START";
pub const TEXT_MESSAGE_CONTENT: &str = "TEXT_MESSAGE_CONTENT";
pub const TEXT_MESSAGE_END: &str = "TEXT_MESSAGE_END";
pub const TOOL_CALL_START: &str = "TOOL_CALL_START";
pub const TOOL_CALL_ARGS: &str = "TOOL_CALL_ARGS";
pub const TOOL_CALL_END: &str = "TOOL_CALL_END";
pub const TOOL_CALL_RESULT: &str = "TOOL_CALL_RESULT";
pub const STATE_SNAPSHOT: &str = "STATE_SNAPSHOT";
pub const STATE_DELTA: &str = "STATE_DELTA";
pub const CUSTOM: &str = "CUSTOM";

/// A CUSTOM event naming a Codync approval card.
pub const APPROVAL: &str = "codync.approval";
/// A CUSTOM event naming a Codync notice.
pub const NOTICE: &str = "codync.notice";
/// A CUSTOM event carrying a plan's entries.
pub const PLAN: &str = "codync.plan";
/// The event the host answers with when its event bus lagged and it couldn't replay the gap.
pub const RESYNC: &str = "codync.resync";

/// The largest text chunk one event carries, so a single SSE line stays bounded.
pub const MAX_DELTA: usize = 8 * 1024;
/// The largest tool output or argument payload one event carries.
pub const MAX_PAYLOAD: usize = 16 * 1024;

/// `value` with its AG-UI `type`.
pub fn event(kind: &str, mut value: Value) -> Value {
    value["type"] = kind.into();
    value
}

/// An SSE frame: one `data:` line and a blank line. The JSON is compact, so it never adds a line of its own.
pub fn frame(value: &Value) -> String {
    let json = serde_json::to_string(value).unwrap_or_else(|_| "{}".into());
    format!("data: {json}\n\n")
}

/// `text` in at most [`MAX_DELTA`]-byte chunks, split on character boundaries.
pub fn chunks(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
        let mut end = rest.len().min(MAX_DELTA);
        while end > 0 && !rest.is_char_boundary(end) {
            end -= 1;
        }
        // A single character can't exceed MAX_DELTA bytes, so `end` is always > 0 here.
        out.push(rest[..end].to_owned());
        rest = &rest[end..];
    }
    out
}

/// `text` shortened to [`MAX_PAYLOAD`] bytes on a character boundary.
pub fn payload(text: &str) -> String {
    if text.len() <= MAX_PAYLOAD {
        return text.to_owned();
    }
    let mut end = MAX_PAYLOAD;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &text[..end])
}

/// A text message start with its role.
pub fn text_start(message_id: &str) -> Value {
    event(TEXT_MESSAGE_START, json!({"messageId": message_id, "role": "assistant"}))
}

/// A text delta, already split into bounded chunks by the caller.
pub fn text_content(message_id: &str, delta: &str) -> Value {
    event(TEXT_MESSAGE_CONTENT, json!({"messageId": message_id, "delta": delta}))
}

pub fn text_end(message_id: &str) -> Value {
    event(TEXT_MESSAGE_END, json!({"messageId": message_id}))
}

pub fn run_started(thread_id: &str, run_id: &str) -> Value {
    event(RUN_STARTED, json!({"threadId": thread_id, "runId": run_id}))
}

pub fn run_finished(thread_id: &str, run_id: &str, result: Option<&str>) -> Value {
    event(RUN_FINISHED, json!({"threadId": thread_id, "runId": run_id, "result": result}))
}

pub fn run_error(message: &str, code: &str) -> Value {
    event(RUN_ERROR, json!({"message": payload(message), "code": code}))
}

/// An RFC 6902 patch to the shared state (the shape K4's view state would use).
pub fn state_delta(delta: Value) -> Value {
    event(STATE_DELTA, json!({"delta": delta}))
}

pub fn custom(name: &str, value: Value) -> Value {
    event(CUSTOM, json!({"name": name, "value": value}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_keep_their_ag_ui_type_and_fields() {
        let started = run_started("thread", "run");
        assert_eq!(started["type"], "RUN_STARTED");
        assert_eq!(started["threadId"], "thread");
        assert_eq!(started["runId"], "run");
    }

    #[test]
    fn a_frame_is_one_data_line_and_a_blank_line() {
        let frame = frame(&text_content("m", "hola\nmundo"));
        assert!(frame.starts_with("data: {"));
        assert!(frame.ends_with("\n\n"));
        assert_eq!(frame.matches('\n').count(), 2, "the text's own newline is escaped by JSON");
        let body = frame.strip_prefix("data: ").unwrap().strip_suffix("\n\n").unwrap();
        assert_eq!(serde_json::from_str::<Value>(body).unwrap()["delta"], "hola\nmundo");
    }

    #[test]
    fn chunks_split_on_character_boundaries_and_stay_bounded() {
        let text = "ñ".repeat(MAX_DELTA);
        let chunks = chunks(&text);
        assert!(chunks.iter().all(|chunk| chunk.len() <= MAX_DELTA));
        assert_eq!(chunks.concat(), text);
        assert_eq!(chunks.len(), 2, "a two-byte character at the edge moves whole to the next chunk");
        assert!(chunks("").is_empty());
    }

    #[test]
    fn payloads_are_capped() {
        let long = "x".repeat(MAX_PAYLOAD + 10);
        assert_eq!(payload(&long).len(), MAX_PAYLOAD + "…".len());
        assert_eq!(payload("short"), "short");
    }
}
