//! The A2A JSON-RPC messages this adapter speaks: `message/send` and `message/stream`, the task
//! and artifact shapes, and the Server-Sent Events frame. Only text parts are carried; a bot's own
//! tools and approvals stay in Codync.

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

/// The state of the task a request becomes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Submitted,
    Working,
    Completed,
    Failed,
}

impl State {
    fn as_str(self) -> &'static str {
        match self {
            Self::Submitted => "submitted",
            Self::Working => "working",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }
}

/// One A2A request, after the parts were read.
#[derive(Clone, Debug)]
pub struct Request {
    /// The JSON-RPC id to answer under.
    pub id: Value,
    pub method: Method,
    pub text: String,
    pub message_id: String,
    pub context_id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    Send,
    Stream,
}

impl Request {
    /// Parses a body the local endpoint received.
    pub fn parse(body: &Value) -> Result<Self> {
        if body["jsonrpc"].as_str() != Some("2.0") {
            bail!("Not a JSON-RPC 2.0 request.");
        }
        let method = match body["method"].as_str() {
            Some("message/send") => Method::Send,
            Some("message/stream") => Method::Stream,
            Some(other) => bail!("A2A method `{other}` is not served; use message/send or message/stream."),
            None => bail!("An A2A request names no method."),
        };
        let message = &body["params"]["message"];
        let text = text_of(message).context("The A2A message has no text part.")?;
        Ok(Self {
            id: body.get("id").cloned().unwrap_or(Value::Null),
            method,
            text,
            message_id: message["messageId"]
                .as_str()
                .filter(|id| !id.is_empty())
                .map_or_else(|| uuid::Uuid::new_v4().to_string(), str::to_owned),
            context_id: message["contextId"]
                .as_str()
                .or_else(|| body["params"]["contextId"].as_str())
                .filter(|id| !id.is_empty())
                .map_or_else(|| uuid::Uuid::new_v4().to_string(), str::to_owned),
        })
    }
}

/// The text parts of an A2A message, joined in order.
pub fn text_of(message: &Value) -> Option<String> {
    let mut parts = Vec::new();
    for part in message["parts"].as_array().into_iter().flatten() {
        if part["kind"].as_str().is_some_and(|kind| kind == "text" || kind == "TextPart")
            && let Some(text) = part["text"].as_str().filter(|text| !text.trim().is_empty())
        {
            parts.push(text.trim().to_owned());
        }
    }
    (!parts.is_empty()).then(|| parts.join("\n\n"))
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn task_id(message_id: &str) -> String {
    format!("task-{message_id}")
}

/// The `result` of a task in `state`. A completed one carries the answer as its artifact.
pub fn task(request: &Request, state: State, text: Option<&str>) -> Value {
    let mut result = json!({
        "kind": "task",
        "id": task_id(&request.message_id),
        "contextId": request.context_id.clone(),
        "status": {"state": state.as_str(), "timestamp": now()},
    });
    if let Some(text) = text {
        result["artifacts"] = json!([{
            "artifactId": format!("artifact-{}", request.message_id),
            "name": "answer",
            "parts": [{"kind": "text", "text": text}],
        }]);
    }
    result
}

/// A status event for `message/stream`.
pub fn status_update(request: &Request, state: State, final_event: bool) -> Value {
    json!({
        "kind": "status-update",
        "taskId": task_id(&request.message_id),
        "contextId": request.context_id.clone(),
        "status": {"state": state.as_str(), "timestamp": now()},
        "final": final_event,
    })
}

/// An answer chunk for `message/stream`: `append` continues the same artifact.
pub fn artifact_update(request: &Request, text: &str, append: bool, last_chunk: bool) -> Value {
    json!({
        "kind": "artifact-update",
        "taskId": task_id(&request.message_id),
        "contextId": request.context_id.clone(),
        "append": append,
        "lastChunk": last_chunk,
        "artifact": {
            "artifactId": format!("artifact-{}", request.message_id),
            "name": "answer",
            "parts": [{"kind": "text", "text": text}],
        },
    })
}

/// A JSON-RPC success answer.
pub fn success(id: &Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

/// A JSON-RPC error answer.
pub fn error(id: &Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

/// One SSE event carrying `value`.
pub fn frame(value: &Value) -> String {
    format!("data: {value}\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(method: &str) -> Value {
        json!({
            "jsonrpc": "2.0", "id": "7", "method": method,
            "params": {"message": {"role": "user", "messageId": "m1", "contextId": "c1",
                "parts": [{"kind": "text", "text": "review this"}, {"kind": "data", "data": {}},
                          {"kind": "text", "text": "then answer"}]}},
        })
    }

    #[test]
    fn a_request_reads_every_text_part() {
        let request = Request::parse(&body("message/send")).expect("a request");
        assert_eq!(request.id, json!("7"));
        assert_eq!(request.method, Method::Send);
        assert_eq!(request.text, "review this\n\nthen answer");
        assert_eq!(request.message_id, "m1");
        assert_eq!(request.context_id, "c1");
        assert_eq!(Request::parse(&body("message/stream")).expect("stream").method, Method::Stream);
    }

    #[test]
    fn an_unknown_method_or_an_empty_message_is_refused() {
        let mut unknown = body("tasks/get");
        assert!(Request::parse(&unknown).is_err());
        unknown["method"] = json!("message/send");
        unknown["params"]["message"]["parts"] = json!([]);
        assert!(Request::parse(&unknown).is_err());
    }

    #[test]
    fn a_completed_task_carries_the_answer_artifact() {
        let request = Request::parse(&body("message/send")).expect("a request");
        let task = task(&request, State::Completed, Some("done"));
        assert_eq!(task["kind"], "task");
        assert_eq!(task["id"], "task-m1");
        assert_eq!(task["status"]["state"], "completed");
        assert_eq!(task["artifacts"][0]["parts"][0]["text"], "done");
        assert!(task(&request, State::Working, None).get("artifacts").is_none());
    }

    #[test]
    fn a_stream_status_and_artifact_are_framed_for_sse() {
        let request = Request::parse(&body("message/stream")).expect("a request");
        assert_eq!(status_update(&request, State::Working, false)["final"], false);
        let artifact = artifact_update(&request, "done", true, true);
        assert_eq!(artifact["append"], true);
        assert_eq!(artifact["artifact"]["parts"][0]["text"], "done");
        let frame = frame(&success(&json!("7"), artifact));
        assert!(frame.starts_with("data: {"));
        assert!(frame.ends_with("\n\n"));
    }
}
