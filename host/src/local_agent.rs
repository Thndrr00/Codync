//! A small ACP agent for a local Ollama model. It speaks JSON-RPC over stdin/stdout like the other
//! harnesses, so a bot can use it as a custom command:
//!
//!   <path to codync-host> local-agent --model llama3.2
//!
//! It answers one prompt at a time, without tools or sessions: the chat UI, memory and routines stay
//! Codync's. The model comes from `--model` or `CODYNC_LOCAL_MODEL`; Ollama from `CODYNC_OLLAMA_URL`.

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

const DEFAULT_MODEL: &str = "llama3.2";
const DEFAULT_URL: &str = "http://127.0.0.1:11434";

fn endpoint() -> String {
    std::env::var("CODYNC_OLLAMA_URL").unwrap_or_else(|_| DEFAULT_URL.into()).trim_end_matches('/').to_owned()
}

pub async fn run(model: Option<String>) -> Result<()> {
    let model = model
        .filter(|model| !model.trim().is_empty())
        .or_else(|| std::env::var("CODYNC_LOCAL_MODEL").ok().filter(|model| !model.trim().is_empty()))
        .unwrap_or_else(|| DEFAULT_MODEL.into());
    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    let mut out = tokio::io::stdout();
    while let Some(line) = lines.next_line().await? {
        let Ok(value) = serde_json::from_str::<Value>(&line) else { continue };
        // A notification has no id and needs no answer.
        let Some(id) = value.get("id").cloned() else { continue };
        let method = value["method"].as_str().unwrap_or_default();
        let params = &value["params"];
        let result = match method {
            "initialize" => Ok(json!({
                "protocolVersion": 1,
                "agentCapabilities": {"loadSession": false, "promptCapabilities": {"image": false, "audio": false, "embeddedContext": false}},
                "authMethods": [],
            })),
            "authenticate" => Ok(json!({})),
            "session/new" => Ok(json!({"sessionId": format!("local-{}", uuid::Uuid::new_v4())})),
            "session/load" => Err(anyhow::anyhow!("a local model has no saved sessions")),
            "session/cancel" => Ok(json!({})),
            "session/set_mode" | "session/set_model" | "unstable_setSessionModel" => Ok(json!({})),
            "session/prompt" => {
                let text = prompt_text(params);
                if text.trim().is_empty() {
                    Err(anyhow::anyhow!("the prompt has no text"))
                } else {
                    let session = params["sessionId"].as_str().unwrap_or_default().to_owned();
                    match chat(&model, &text).await {
                        Ok(reply) => {
                            write(
                                &mut out,
                                &json!({
                                    "jsonrpc": "2.0",
                                    "method": "session/update",
                                    "params": {
                                        "sessionId": session,
                                        "update": {
                                            "sessionUpdate": "agent_message_chunk",
                                            "content": {"type": "text", "text": reply},
                                        },
                                    },
                                }),
                            )
                            .await?;
                            Ok(json!({"stopReason": "end_turn"}))
                        }
                        Err(error) => {
                            write(
                                &mut out,
                                &json!({
                                    "jsonrpc": "2.0",
                                    "method": "session/update",
                                    "params": {
                                        "sessionId": session,
                                        "update": {
                                            "sessionUpdate": "agent_message_chunk",
                                            "content": {"type": "text", "text": format!("The local model couldn't answer: {error:#}")},
                                        },
                                    },
                                }),
                            )
                            .await?;
                            Ok(json!({"stopReason": "end_turn"}))
                        }
                    }
                }
            }
            other => Err(anyhow::anyhow!("unknown ACP method: {other}")),
        };
        let response = match result {
            Ok(result) => json!({"jsonrpc": "2.0", "id": id, "result": result}),
            Err(error) => {
                json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32603, "message": format!("{error:#}")}})
            }
        };
        write(&mut out, &response).await?;
    }
    Ok(())
}

async fn write(out: &mut tokio::io::Stdout, value: &Value) -> Result<()> {
    out.write_all(format!("{value}\n").as_bytes()).await?;
    out.flush().await?;
    Ok(())
}

fn prompt_text(params: &Value) -> String {
    params["prompt"]
        .as_array()
        .map(|parts| {
            parts
                .iter()
                .filter_map(|part| part["text"].as_str().or_else(|| part["content"].as_str()))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default()
}

async fn chat(model: &str, text: &str) -> Result<String> {
    let response = crate::http()
        .post(format!("{}/api/chat", endpoint()))
        .json(&json!({
            "model": model,
            "stream": false,
            "messages": [{"role": "user", "content": text}],
        }))
        .timeout(Duration::from_mins(3))
        .send()
        .await
        .context("Ollama did not answer")?;
    if !response.status().is_success() {
        bail!("Ollama answered {}", response.status());
    }
    let value: Value = response.json().await.context("Ollama answered with something else")?;
    value["message"]["content"].as_str().map(str::to_owned).context("Ollama returned no message")
}
