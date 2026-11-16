//! An A2A-style agent for offline tests: an agent card and a `message/send` that completes a task.

use axum::Json;
use axum::Router;
use axum::http::{HeaderMap, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use serde_json::{Value, json};

/// What every task's artifact says.
pub const ANSWER: &str = "Mock A2A answer.";

pub(super) fn router() -> Router {
    Router::new()
        .route("/.well-known/agent-card.json", get(card))
        .route("/a2a/.well-known/agent-card.json", get(card))
        .route("/a2a", post(message))
}

/// The card names this server, so a host can discover it the way A2A clients do.
async fn card(headers: HeaderMap) -> Json<Value> {
    let host = headers.get(header::HOST).and_then(|value| value.to_str().ok()).unwrap_or("127.0.0.1");
    Json(json!({
        "name": "Codync Mock Agent",
        "description": "An offline A2A agent for Codync host tests.",
        "url": format!("http://{host}/a2a"),
        "version": env!("CARGO_PKG_VERSION"),
        "protocolVersion": "0.2.6",
        "capabilities": {"streaming": false, "pushNotifications": false},
        "defaultInputModes": ["text/plain"],
        "defaultOutputModes": ["text/plain"],
        "skills": [{
            "id": "echo",
            "name": "Echo",
            "description": "Complete a task with a fixed answer.",
            "tags": ["mock"],
        }],
    }))
}

async fn message(Json(body): Json<Value>) -> Response {
    let Some(id) = body.get("id").cloned() else { return Json(Value::Null).into_response() };
    if body["method"] != "message/send" {
        return failure(id, -32601, "Only `message/send` is served");
    }
    let text = body["params"]["message"]["parts"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|part| part["kind"] == "text")
        .and_then(|part| part["text"].as_str())
        .unwrap_or_default();
    let answer = if text.is_empty() { ANSWER.to_owned() } else { format!("{ANSWER} You said: {text}") };
    Json(json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "kind": "task",
            "id": "mock-task",
            "contextId": "mock-context",
            "status": {"state": "completed"},
            "artifacts": [{
                "artifactId": "mock-artifact",
                "name": "answer",
                "parts": [{"kind": "text", "text": answer}],
            }],
        },
    }))
    .into_response()
}

fn failure(id: Value, code: i64, message: &str) -> Response {
    Json(json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})).into_response()
}
