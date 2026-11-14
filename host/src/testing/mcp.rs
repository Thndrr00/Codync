//! An MCP server over HTTP for offline tests: `initialize`, `ping`, `tools/list` and an `echo` tool.

use axum::Json;
use axum::Router;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use serde_json::{Value, json};

pub(super) fn router() -> Router {
    Router::new().route("/mcp", post(rpc))
}

async fn rpc(Json(body): Json<Value>) -> Response {
    let Some(id) = body.get("id").cloned() else {
        // A notification: MCP expects no answer.
        return StatusCode::ACCEPTED.into_response();
    };
    let method = body["method"].as_str().unwrap_or_default();
    let result = match method {
        "initialize" => json!({
            "protocolVersion": "2025-06-18",
            "capabilities": {"tools": {"listChanged": false}},
            "serverInfo": {"name": "codync-mock-mcp", "version": env!("CARGO_PKG_VERSION")},
        }),
        "ping" => json!({}),
        "tools/list" => json!({"tools": [{
            "name": "echo",
            "description": "Echo the text it is given.",
            "inputSchema": {
                "type": "object",
                "properties": {"text": {"type": "string"}},
                "required": ["text"],
            },
        }]}),
        "tools/call" => {
            if body["params"]["name"] != "echo" {
                return failure(id, -32602, "Unknown tool");
            }
            let text = body["params"]["arguments"]["text"].as_str().unwrap_or_default();
            json!({"content": [{"type": "text", "text": text}], "isError": false})
        }
        other => return failure(id, -32601, &format!("Unknown method `{other}`")),
    };
    Json(json!({"jsonrpc": "2.0", "id": id, "result": result})).into_response()
}

fn failure(id: Value, code: i64, message: &str) -> Response {
    Json(json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})).into_response()
}
