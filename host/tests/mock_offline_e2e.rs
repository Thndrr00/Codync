//! Offline agent tests against `codync-host mock`: one real host turn through a mocked provider, a provider
//! error and an approval, plus the mock endpoints an A2A, MCP or AG-UI client would call.
//!
//! No network and no keys: the mock listens on loopback, and the fixture agent talks only to it.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::host::Host;
use serde_json::{Value, json};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// `codync-host mock` on one loopback port. Debug builds only; a release build skips these tests.
struct Mock {
    child: Child,
    base: String,
}

impl Mock {
    async fn start() -> Option<Self> {
        let mut child = Command::new(env!("CARGO_BIN_EXE_codync-host"))
            .arg("mock")
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        let stdout = child.stdout.take()?;
        let mut line = String::new();
        BufReader::new(stdout).read_line(&mut line).ok()?;
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        };
        let port = value["port"].as_u64()?;
        let base = format!("http://127.0.0.1:{port}");
        let deadline = Instant::now() + Duration::from_secs(10);
        while reqwest::get(format!("{base}/health")).await.is_err() {
            if Instant::now() >= deadline {
                let _ = child.kill();
                return None;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        Some(Self { child, base })
    }
}

impl Drop for Mock {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

async fn entries(host: &Host, bot: &str) -> Vec<Value> {
    host.call("history", json!({"botId": bot})).await["entries"].as_array().cloned().unwrap_or_default()
}

async fn next_reply(host: &Host, bot: &str, known: usize) -> String {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let finals: Vec<Value> = entries(host, bot)
            .await
            .into_iter()
            .filter(|entry| entry["kind"] == "agent" && entry["data"]["final"] == true)
            .collect();
        if finals.len() > known {
            return finals[known]["data"]["text"].as_str().unwrap_or_default().to_owned();
        }
        assert!(Instant::now() < deadline, "the bot did not answer the mock turn");
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

async fn pending_card(host: &Host, bot: &str) -> Value {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if let Some(card) = entries(host, bot)
            .await
            .into_iter()
            .find(|entry| entry["kind"] == "permission" && entry["data"]["status"] == "pending")
        {
            return card;
        }
        assert!(Instant::now() < deadline, "the approval card did not arrive");
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

async fn mock_agent(host: &Host, mock: &Mock) -> String {
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mock_provider_agent.py");
    let command = format!("{} {}", common::python_agent(&script), mock.base);
    let spec = json!({"name": "Mocked", "backend": "custom", "command": command,
        "connectors": [], "notify": false, "permission": "ask"});
    let bot = host.call("createBot", spec).await;
    bot["bot"]["id"].as_str().unwrap_or_default().to_owned()
}

#[tokio::test]
async fn one_turn_runs_against_the_mock_provider_and_agent() {
    if common::python_missing() {
        return;
    }
    let Some(mock) = Mock::start().await else {
        eprintln!("skipping: this binary has no `mock` command (a release build)");
        return;
    };
    let host = Host::start().await;
    let bot = mock_agent(&host, &mock).await;

    // A tool call goes out through the mock provider and its result comes back.
    host.call("send", json!({"botId": bot, "text": "tool: look up codync", "clientNonce": "mock-tool"})).await;
    let tool = next_reply(&host, &bot, 0).await;
    assert!(tool.contains("The mock tool answered."), "{tool}");

    // A provider error reaches the agent as an HTTP error, which the scripted agent reports.
    host.call("send", json!({"botId": bot, "text": "error: offline", "clientNonce": "mock-error"})).await;
    let failed = next_reply(&host, &bot, 1).await;
    assert!(failed.contains("provider error: 500"), "{failed}");
    assert!(failed.contains("mock provider refused"), "{failed}");

    // An approval waits for an answer and the turn finishes with the choice.
    host.call("send", json!({"botId": bot, "text": "approval: cargo test", "clientNonce": "mock-approval"})).await;
    let card = pending_card(&host, &bot).await;
    assert!(card["data"]["title"].as_str().unwrap_or_default().contains("cargo test"), "{card}");
    host.call("respondPermission", json!({"entryId": card["id"], "optionId": "yes"})).await;
    let answered = next_reply(&host, &bot, 2).await;
    assert!(answered.contains("approval outcome: yes"), "{answered}");
}

#[tokio::test]
async fn every_mock_endpoint_answers_without_network_or_keys() {
    let Some(mock) = Mock::start().await else {
        eprintln!("skipping: this binary has no `mock` command (a release build)");
        return;
    };
    let client = reqwest::Client::new();
    let chat = |body: Value| {
        let client = client.clone();
        let url = format!("{}/v1/chat/completions", mock.base);
        async move { client.post(url).json(&body).send().await.unwrap() }
    };

    // A fixed answer, a tool call, the answer after the tool result, and the error mode.
    let answer: Value = chat(json!({"model": "mock-model", "messages": [{"role": "user", "content": "hi"}]}))
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(answer["choices"][0]["message"]["content"], "Mock answer.");
    let tools = json!([{"type": "function", "function": {"name": "echo", "parameters": {"type": "object"}}}]);
    let call: Value = chat(json!({"model": "mock-model", "tools": tools,
        "messages": [{"role": "user", "content": "[mock:tool]"}]}))
    .await
    .json()
    .await
    .unwrap();
    assert_eq!(call["choices"][0]["message"]["tool_calls"][0]["function"]["name"], "echo");
    let result: Value = chat(json!({"model": "mock-model", "tools": tools, "messages": [
        {"role": "user", "content": "[mock:tool]"},
        {"role": "assistant", "content": Value::Null, "tool_calls": [{"id": "call_mock_1", "type": "function",
            "function": {"name": "echo", "arguments": "{}"}}]},
        {"role": "tool", "tool_call_id": "call_mock_1", "content": "codync"},
    ]}))
    .await
    .json()
    .await
    .unwrap();
    assert_eq!(result["choices"][0]["message"]["content"], "The mock tool answered.");
    let refused = chat(json!({"model": "mock-model", "messages": [{"role": "user", "content": "[mock:error]"}]})).await;
    assert_eq!(refused.status(), 500);
    let refused: Value = refused.json().await.unwrap();
    assert_eq!(refused["error"]["code"], "mock_error");

    // Streaming ends with `[DONE]`.
    let streamed = chat(json!({"model": "mock-model", "stream": true,
        "messages": [{"role": "user", "content": "hi"}]}))
    .await
    .text()
    .await
    .unwrap();
    assert!(streamed.contains("\"content\":\"Mock answer.\""));
    assert!(streamed.ends_with("data: [DONE]\n\n"));

    // Embeddings are fixed-length and deterministic.
    let embeddings: Value = client
        .post(format!("{}/v1/embeddings", mock.base))
        .json(&json!({"model": "mock-embedding", "input": ["codync", "openbot"]}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(embeddings["data"].as_array().unwrap().len(), 2);
    assert_eq!(embeddings["data"][0]["embedding"].as_array().unwrap().len(), 8);

    // The MCP echo server answers a tool call.
    let echoed: Value = client
        .post(format!("{}/mcp", mock.base))
        .json(&json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": "echo", "arguments": {"text": "hola"}}}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(echoed["result"]["content"][0]["text"], "hola");

    // The A2A agent completes a task, and its card describes it.
    let card: Value =
        client.get(format!("{}/.well-known/agent-card.json", mock.base)).send().await.unwrap().json().await.unwrap();
    assert_eq!(card["name"], "Codync Mock Agent");
    let task: Value = client
        .post(format!("{}/a2a", mock.base))
        .json(&json!({"jsonrpc": "2.0", "id": "a1", "method": "message/send",
            "params": {"message": {"role": "user", "parts": [{"kind": "text", "text": "hola"}]}}}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(task["result"]["status"]["state"], "completed");
    assert!(task["result"]["artifacts"][0]["parts"][0]["text"].as_str().unwrap().contains("Mock A2A answer."));

    // The AG-UI endpoint streams the host's own event names, approval included.
    let run = client
        .post(format!("{}/agui/demo", mock.base))
        .json(&json!({"threadId": "t", "runId": "r",
            "messages": [{"role": "user", "content": "[mock:approval]"}]}))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    for expected in [
        "RUN_STARTED",
        "TOOL_CALL_START",
        "TOOL_CALL_RESULT",
        "TEXT_MESSAGE_CONTENT",
        "codync.approval",
        "RUN_FINISHED",
    ] {
        assert!(run.contains(expected), "the AG-UI stream has no {expected}: {run}");
    }
}
