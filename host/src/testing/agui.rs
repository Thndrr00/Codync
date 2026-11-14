//! An AG-UI endpoint for offline tests: one run as `text/event-stream`, using the host's own event names.
//!
//! The stream is a tool call, a text answer, an approval (`[mock:approval]`) and the run's end, or just
//! `RUN_ERROR` when the last user message says `[mock:error]`.

use crate::agui::events;
use axum::Json;
use axum::Router;
use axum::body::Body;
use axum::extract::Path;
use axum::http::{HeaderValue, header};
use axum::response::Response;
use axum::routing::post;
use serde_json::{Value, json};

pub(super) fn router() -> Router {
    Router::new().route("/agui/{bot}", post(run))
}

async fn run(Path(_bot): Path<String>, Json(body): Json<Value>) -> Response {
    let thread = body["threadId"].as_str().unwrap_or("mock-thread");
    let run_id = body["runId"].as_str().unwrap_or("mock-run");
    let text = last_user_text(&body);
    let mut frames = String::new();
    let mut push = |value: Value| frames.push_str(&events::frame(&value));
    push(events::run_started(thread, run_id));
    if text.contains("[mock:error]") {
        push(events::run_error("The mock AG-UI run failed.", "mock_error"));
        return sse(&frames);
    }
    push(events::event(events::STATE_SNAPSHOT, json!({"snapshot": {"mock": true}})));
    push(events::event(
        events::TOOL_CALL_START,
        json!({"toolCallId": "call_mock", "toolCallName": "echo", "parentMessageId": Value::Null}),
    ));
    push(events::event(events::TOOL_CALL_ARGS, json!({"toolCallId": "call_mock", "delta": "{\"text\":\"codync\"}"})));
    push(events::event(events::TOOL_CALL_END, json!({"toolCallId": "call_mock"})));
    push(events::event(
        events::TOOL_CALL_RESULT,
        json!({"messageId": "mock-tool", "toolCallId": "call_mock", "content": "codync", "role": "tool"}),
    ));
    push(events::text_start("mock-message"));
    push(events::text_content("mock-message", "Mock AG-UI answer."));
    push(events::text_end("mock-message"));
    if text.contains("[mock:approval]") {
        push(events::custom(
            events::APPROVAL,
            json!({
                "entryId": "mock-approval",
                "title": "Mock approval",
                "status": "pending",
                "options": [{"optionId": "allow", "name": "Allow"}, {"optionId": "deny", "name": "Deny"}],
                "answer": {"method": "respondPermission"},
            }),
        ));
    }
    push(events::run_finished(thread, run_id, Some("success")));
    sse(&frames)
}

fn last_user_text(body: &Value) -> String {
    body["messages"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|message| message["role"] == "user")
        .last()
        .map(|message| match &message["content"] {
            Value::String(text) => text.clone(),
            Value::Array(parts) => parts.iter().filter_map(|part| part["text"].as_str()).collect::<Vec<_>>().join("\n"),
            _ => String::new(),
        })
        .unwrap_or_default()
}

fn sse(frames: &str) -> Response {
    let mut response = Response::new(Body::from(frames.to_owned()));
    response.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_static("text/event-stream"));
    response
}
