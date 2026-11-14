//! The `RunAgentInput` a client posts to `POST /agui/{bot}`: which conversation, which run, and what the user said.

use anyhow::{Context, Result, bail};
use serde_json::Value;

/// A request's own ids and the newest thing the user said.
pub struct RunInput {
    pub thread_id: String,
    pub run_id: String,
    /// The shared state the client sent: emitted back as the run's first snapshot.
    pub state: Value,
    pub text: String,
}

impl RunInput {
    pub fn parse(body: &Value) -> Result<Self> {
        let messages = body["messages"].as_array().context("`messages` is required")?;
        let text = messages
            .iter()
            .rev()
            .find(|message| message["role"] == "user")
            .map(message_text)
            .map(|text| text.trim().to_owned())
            .filter(|text| !text.is_empty())
            .context("the run has no user message")?;
        if text.len() > 256 * 1024 {
            bail!("the message is too long");
        }
        Ok(Self {
            thread_id: id(body["threadId"].as_str(), "thread"),
            run_id: id(body["runId"].as_str(), "run"),
            state: body.get("state").cloned().unwrap_or_else(|| serde_json::json!({})),
            text,
        })
    }
}

/// A client's id, or a fresh one when it sent none (AG-UI requires both).
fn id(sent: Option<&str>, prefix: &str) -> String {
    match sent.map(str::trim).filter(|id| !id.is_empty()) {
        Some(id) => id.to_owned(),
        None => format!("{prefix}-{}", uuid::Uuid::new_v4()),
    }
}

/// A message's text: a string, or the `text` parts of a content array.
fn message_text(message: &Value) -> String {
    match &message["content"] {
        Value::String(text) => text.clone(),
        Value::Array(parts) => parts
            .iter()
            .filter(|part| part["type"] == "text")
            .filter_map(|part| part["text"].as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_newest_user_message_is_the_turn() {
        let input = RunInput::parse(&json!({
            "threadId": "t",
            "runId": "r",
            "messages": [
                {"id": "1", "role": "user", "content": "first"},
                {"id": "2", "role": "assistant", "content": "answer"},
                {"id": "3", "role": "user", "content": "  second  "},
            ],
            "state": {"view": "chat"},
        }))
        .unwrap();
        assert_eq!(input.text, "second");
        assert_eq!(input.thread_id, "t");
        assert_eq!(input.run_id, "r");
        assert_eq!(input.state["view"], "chat");
    }

    #[test]
    fn content_parts_read_only_their_text() {
        let input = RunInput::parse(&json!({
            "messages": [{"role": "user", "content": [
                {"type": "text", "text": "look"},
                {"type": "image", "url": "data:..."},
                {"type": "text", "text": "here"},
            ]}],
        }))
        .unwrap();
        assert_eq!(input.text, "look\nhere");
        assert!(input.thread_id.starts_with("thread-"));
        assert!(input.run_id.starts_with("run-"));
        assert_eq!(input.state, json!({}));
    }

    #[test]
    fn a_run_without_user_text_is_refused() {
        assert!(RunInput::parse(&json!({"messages": []})).is_err());
        assert!(RunInput::parse(&json!({"messages": [{"role": "assistant", "content": "hi"}]})).is_err());
        assert!(RunInput::parse(&json!({"messages": [{"role": "user", "content": "   "}]})).is_err());
        assert!(RunInput::parse(&json!({})).is_err());
    }
}
