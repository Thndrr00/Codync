//! An OpenAI-compatible provider for offline tests: fixed answers, a tool call, and an error mode.
//!
//! `POST /v1/chat/completions` (plain or streamed), `POST /v1/embeddings` and `GET /v1/models`. A message that
//! says `[mock:error]` (or the model `mock-error`) gets a 500 in OpenAI's error shape; `[mock:tool]` (or any
//! request carrying `tools`) gets a tool call, and the turn after the tool result gets a fixed answer.

use axum::Json;
use axum::Router;
use axum::body::Body;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use serde_json::{Value, json};

/// The answer to a request with no tool involved.
pub const ANSWER: &str = "Mock answer.";
/// The answer once the mock tool has been called.
pub const TOOL_ANSWER: &str = "The mock tool answered.";
/// What the mock provider says when it is asked to fail.
pub const ERROR_MESSAGE: &str = "The mock provider refused this request.";

const ERROR_MARKER: &str = "[mock:error]";
const TOOL_MARKER: &str = "[mock:tool]";

pub(super) fn router() -> Router {
    Router::new()
        .route("/v1/models", get(models))
        .route("/v1/chat/completions", post(chat))
        .route("/v1/embeddings", post(embeddings))
}

async fn models() -> Json<Value> {
    Json(json!({"object": "list", "data": [
        {"id": "mock-model", "object": "model", "created": 0, "owned_by": "codync-mock"},
        {"id": "mock-tool", "object": "model", "created": 0, "owned_by": "codync-mock"},
    ]}))
}

async fn chat(Json(body): Json<Value>) -> Response {
    if body["model"] == "mock-error" || marker(&body, ERROR_MARKER) {
        return refused();
    }
    let model = body["model"].as_str().unwrap_or("mock-model").to_owned();
    let wants_tool = marker(&body, TOOL_MARKER) || body["tools"].as_array().is_some_and(|tools| !tools.is_empty());
    let (message, finish) = if wants_tool && !answered(&body) {
        (tool_call(&body), "tool_calls")
    } else if wants_tool {
        (text_message(TOOL_ANSWER), "stop")
    } else {
        (text_message(ANSWER), "stop")
    };
    if body["stream"] == true {
        return stream(&model, &message, finish);
    }
    Json(completion(&model, message, finish)).into_response()
}

/// A 500 in the shape an OpenAI client reads: `error.message`, `error.type`, `error.code`.
fn refused() -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({"error": {
            "message": ERROR_MESSAGE,
            "type": "server_error",
            "param": Value::Null,
            "code": "mock_error",
        }})),
    )
        .into_response()
}

fn marker(body: &Value, marker: &str) -> bool {
    body["messages"].as_array().into_iter().flatten().any(|message| match &message["content"] {
        Value::String(text) => text.contains(marker),
        Value::Array(parts) => parts.iter().any(|part| part["text"].as_str().is_some_and(|text| text.contains(marker))),
        _ => false,
    })
}

fn answered(body: &Value) -> bool {
    body["messages"].as_array().into_iter().flatten().any(|message| message["role"] == "tool")
}

fn text_message(text: &str) -> Value {
    json!({"role": "assistant", "content": text})
}

/// The first tool the request offered, called with a fixed argument.
fn tool_call(body: &Value) -> Value {
    let name = body["tools"][0]["function"]["name"].as_str().unwrap_or("echo");
    json!({
        "role": "assistant",
        "content": Value::Null,
        "tool_calls": [{
            "id": "call_mock_1",
            "type": "function",
            "function": {"name": name, "arguments": "{\"text\":\"codync\"}"},
        }],
    })
}

fn completion(model: &str, message: Value, finish: &str) -> Value {
    json!({
        "id": "chatcmpl-mock",
        "object": "chat.completion",
        "created": 0,
        "model": model,
        "choices": [{"index": 0, "message": message, "finish_reason": finish}],
        "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2},
    })
}

/// The same answer as `text/event-stream`, ending with `data: [DONE]`.
fn stream(model: &str, message: &Value, finish: &str) -> Response {
    let mut body = String::new();
    body.push_str(&chunk(model, json!({"role": "assistant"}), Value::Null));
    if let Some(calls) = message["tool_calls"].as_array().filter(|calls| !calls.is_empty()) {
        let indexed: Vec<Value> = calls
            .iter()
            .enumerate()
            .map(|(index, call)| {
                let mut call = call.clone();
                call["index"] = json!(index);
                call
            })
            .collect();
        body.push_str(&chunk(model, json!({"tool_calls": indexed}), Value::Null));
    } else {
        body.push_str(&chunk(model, json!({"content": message["content"]}), Value::Null));
    }
    body.push_str(&chunk(model, json!({}), json!(finish)));
    body.push_str("data: [DONE]\n\n");
    let mut response = Response::new(Body::from(body));
    response.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_static("text/event-stream"));
    response
}

fn chunk(model: &str, delta: Value, finish: Value) -> String {
    format!(
        "data: {}\n\n",
        json!({
            "id": "chatcmpl-mock",
            "object": "chat.completion.chunk",
            "created": 0,
            "model": model,
            "choices": [{"index": 0, "delta": delta, "finish_reason": finish}],
        })
    )
}

async fn embeddings(Json(body): Json<Value>) -> Json<Value> {
    let inputs: Vec<&str> = match &body["input"] {
        Value::String(text) => vec![text.as_str()],
        Value::Array(items) => items.iter().filter_map(Value::as_str).collect(),
        _ => Vec::new(),
    };
    let data: Vec<Value> = inputs
        .iter()
        .enumerate()
        .map(|(index, text)| json!({"object": "embedding", "index": index, "embedding": vector(text)}))
        .collect();
    let tokens: usize = inputs.iter().map(|text| text.split_whitespace().count()).sum();
    Json(json!({
        "object": "list",
        "data": data,
        "model": body["model"].as_str().unwrap_or("mock-embedding"),
        "usage": {"prompt_tokens": tokens, "total_tokens": tokens},
    }))
}

/// A deterministic 8-dimension vector, so a test can compare it with itself across runs.
fn vector(text: &str) -> Vec<f64> {
    let mut dimensions = [0.0_f64; 8];
    for (index, byte) in text.bytes().enumerate() {
        dimensions[index % 8] += f64::from(byte);
    }
    dimensions.iter().map(|value| (value % 97.0) / 97.0).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(content: &str) -> Value {
        json!({"model": "mock-model", "messages": [{"role": "user", "content": content}],
            "tools": [{"type": "function", "function": {"name": "echo", "parameters": {"type": "object"}}}]})
    }

    #[test]
    fn the_markers_pick_the_answer() {
        assert!(marker(&request("[mock:error]"), ERROR_MARKER));
        assert!(marker(&request("[mock:tool]"), TOOL_MARKER));
        assert!(!marker(&request("hello"), TOOL_MARKER));
        assert!(answered(&json!({"messages": [{"role": "tool", "content": "x"}]})));
    }

    #[test]
    fn embeddings_are_deterministic_vectors() {
        let first = vector("codync");
        assert_eq!(first.len(), 8);
        assert_eq!(first, vector("codync"));
        assert_ne!(first, vector("openbot"));
    }
}
