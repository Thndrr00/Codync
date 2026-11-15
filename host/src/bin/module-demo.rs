//! A sample module: the reference for what a module is, and the fixture the host's tests run.
//!
//! It speaks the module contract on stdio: one JSON object per line in, one out. `codync-host module-demo` isn't
//! meant to be installed by anyone; a real module declares the same methods and answers with its own data.

use serde_json::{Value, json};
use std::io::{BufRead, Write};

fn main() -> std::io::Result<()> {
    let stdin = std::io::stdin();
    let mut out = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line?;
        let Ok(msg) = serde_json::from_str::<Value>(&line) else { continue };
        let method = msg["method"].as_str().unwrap_or_default();
        let params = &msg["params"];
        // Notifications have no id and take no answer.
        let Some(id) = msg.get("id").filter(|id| !id.is_null()).cloned() else {
            continue;
        };
        let reply = match method {
            "initialize" => json!({
                "tools": ["forecast", "announce", "hang"],
                "views": ["today"],
                "botFolder": params["botFolder"],
            }),
            "tool" => match params["name"].as_str().unwrap_or_default() {
                "forecast" => json!({"content": [{"type": "text", "text": "Sunny, 24 °C."}]}),
                // The host must stop a module that never answers, and say so.
                "hang" => continue,
                "announce" => {
                    let note = json!({
                        "jsonrpc": "2.0",
                        "method": "event",
                        "params": {"name": "messageAdded", "body": {"from": "module-demo"}},
                    });
                    writeln!(out, "{note}")?;
                    out.flush()?;
                    json!({"content": [{"type": "text", "text": "announced"}]})
                }
                other => {
                    json!({"content": [{"type": "text", "text": format!("no tool called {other}")}], "isError": true})
                }
            },
            "view" => json!({
                "title": "Today",
                "sections": [{
                    "title": "Weather",
                    "rows": [{"label": "Sky", "value": "Clear"}, {"label": "High", "value": "24 °C"}],
                }],
            }),
            "shutdown" => {
                writeln!(out, "{}", json!({"jsonrpc": "2.0", "id": id, "result": {}}))?;
                out.flush()?;
                return Ok(());
            }
            other => json!({"error": {"code": -32601, "message": format!("unknown method {other}")}}),
        };
        let reply = if reply.get("error").is_some() {
            json!({"jsonrpc": "2.0", "id": id, "error": reply["error"]})
        } else {
            json!({"jsonrpc": "2.0", "id": id, "result": reply})
        };
        writeln!(out, "{reply}")?;
        out.flush()?;
    }
    Ok(())
}
