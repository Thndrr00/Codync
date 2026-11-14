//! The host-side ACP adapter for a remote AG-UI agent: `codync-host agui-acp`.
//!
//! A bot's actor talks ACP to this process over stdio; this process posts the bot's turn to the AG-UI endpoint
//! and streams its events back as `session/update`s. Approvals go the other way: an AG-UI `CUSTOM` approval
//! becomes an ACP `session/request_permission`, and its answer goes back to the endpoint as a tool message in
//! a follow-up run of the same turn.
//!
//! `docs/features/agui.md` documents the mapping and what is not covered.

use super::{endpoint, events, remote};
use crate::LockExt;
use crate::agent::acp::content_text;
use anyhow::{Context, Result, anyhow};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::sync::{mpsc, oneshot};

/// Approvals one turn may ask for before it is stopped.
const MAX_APPROVALS: usize = 8;

pub async fn run() -> Result<()> {
    let (out, mut writes) = mpsc::unbounded_channel::<Value>();
    let writer = tokio::spawn(async move {
        let mut stdout = BufWriter::new(tokio::io::stdout());
        while let Some(message) = writes.recv().await {
            let Ok(line) = serde_json::to_string(&message) else { continue };
            if stdout.write_all(line.as_bytes()).await.is_err() || stdout.write_all(b"\n").await.is_err() {
                break;
            }
            let _ = stdout.flush().await;
        }
    });
    let peer = Peer { out, pending: Arc::default(), next: Arc::new(AtomicI64::new(1)), sessions: Arc::default() };
    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    while let Some(line) = lines.next_line().await.context("reading ACP from the host")? {
        let Ok(message) = serde_json::from_str::<Value>(&line) else { continue };
        peer.handle(message);
    }
    drop(peer);
    // A prompt still waiting on the endpoint mustn't keep this process alive after the host is gone.
    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), writer).await;
    Ok(())
}

#[derive(Clone)]
struct Peer {
    out: mpsc::UnboundedSender<Value>,
    pending: Arc<std::sync::Mutex<HashMap<i64, oneshot::Sender<Value>>>>,
    next: Arc<AtomicI64>,
    sessions: Arc<std::sync::Mutex<HashMap<String, Arc<std::sync::Mutex<Session>>>>>,
}

struct Session {
    messages: Vec<Value>,
    canceled: Arc<AtomicBool>,
}

impl Session {
    fn new() -> Self {
        Self { messages: Vec::new(), canceled: Arc::new(AtomicBool::new(false)) }
    }
}

struct Rpc {
    code: i64,
    message: String,
}

impl Rpc {
    fn unsupported(method: &str) -> Self {
        Self { code: -32601, message: format!("{method} is not supported by this AG-UI agent") }
    }
}

impl From<anyhow::Error> for Rpc {
    fn from(error: anyhow::Error) -> Self {
        Self { code: -32603, message: format!("{error:#}") }
    }
}

impl Peer {
    fn handle(&self, message: Value) {
        let Some(method) = message["method"].as_str() else {
            self.answer(message);
            return;
        };
        let method = method.to_owned();
        let params = message["params"].clone();
        match message.get("id").filter(|id| !id.is_null()) {
            Some(id) => {
                let peer = self.clone();
                tokio::spawn(async move {
                    match peer.dispatch(&method, params).await {
                        Ok(result) => peer.reply(json!({"jsonrpc": "2.0", "id": id, "result": result})),
                        Err(error) => peer.reply(json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "error": {"code": error.code, "message": error.message},
                        })),
                    }
                });
            }
            None => self.notification(&method, params),
        }
    }

    fn answer(&self, message: Value) {
        let Some(id) = message["id"].as_i64() else { return };
        if let Some(waiting) = self.pending.locked().remove(&id) {
            let _ = waiting.send(message);
        }
    }

    /// Notifications from the client: only a cancel matters here.
    fn notification(&self, method: &str, params: Value) {
        if method != "session/cancel" {
            return;
        }
        let Some(session) = params["sessionId"].as_str().and_then(|id| self.sessions.locked().get(id).cloned()) else {
            return;
        };
        session.locked().canceled.store(true, Ordering::Release);
    }

    async fn dispatch(&self, method: &str, params: Value) -> std::result::Result<Value, Rpc> {
        match method {
            "initialize" => Ok(initialize()),
            "session/new" => {
                let id = format!("agui-{}", uuid::Uuid::new_v4());
                self.sessions.locked().insert(id.clone(), Arc::new(std::sync::Mutex::new(Session::new())));
                Ok(json!({"sessionId": id, "configOptions": []}))
            }
            "session/prompt" => self.prompt(params).await,
            "session/set_config_option" => Ok(json!({"configOptions": []})),
            "session/set_model" | "session/close" => Ok(json!({})),
            "authenticate" => Ok(json!({})),
            other => Err(Rpc::unsupported(other)),
        }
    }

    async fn prompt(&self, params: Value) -> std::result::Result<Value, Rpc> {
        let session_id = params["sessionId"].as_str().unwrap_or_default().to_owned();
        let Some(session) = self.sessions.locked().get(&session_id).cloned() else {
            return Err(Rpc::from(anyhow!("unknown session")));
        };
        let text = content_text(&params["prompt"]).trim().to_owned();
        if text.is_empty() {
            return Ok(json!({"stopReason": "end_turn"}));
        }
        session.locked().canceled.store(false, Ordering::Release);
        session.locked().messages.push(json!({
            "id": uuid::Uuid::new_v4().to_string(),
            "role": "user",
            "content": text,
        }));
        let (endpoint, token) = endpoint::from_env().map_err(Rpc::from)?;
        for _ in 0..=MAX_APPROVALS {
            let messages = session.locked().messages.clone();
            let input = remote::run_input(&session_id, &uuid::Uuid::new_v4().to_string(), &messages, &json!({}));
            let mut run = remote::Run::start(&endpoint, token.as_deref(), &input).await.map_err(Rpc::from)?;
            match self.consume(&session_id, &session, &mut run).await? {
                Outcome::Finished => return Ok(json!({"stopReason": "end_turn"})),
                Outcome::Cancelled => return Ok(json!({"stopReason": "cancelled"})),
                Outcome::Resume => continue,
            }
        }
        Err(Rpc::from(anyhow!("The AG-UI agent asked for too many approvals in one turn.")))
    }

    async fn consume(
        &self,
        session_id: &str,
        session: &Arc<std::sync::Mutex<Session>>,
        run: &mut remote::Run,
    ) -> std::result::Result<Outcome, Rpc> {
        let mut text = String::new();
        let mut tools: Vec<Value> = Vec::new();
        let mut args: HashMap<String, String> = HashMap::new();
        while let Some(event) = run.next().await.map_err(Rpc::from)? {
            if session.locked().canceled.load(Ordering::Acquire) {
                return Ok(Outcome::Cancelled);
            }
            match event["type"].as_str().unwrap_or_default() {
                "TEXT_MESSAGE_CONTENT" => {
                    if let Some(delta) = event["delta"].as_str().filter(|delta| !delta.is_empty()) {
                        text.push_str(delta);
                        self.chunk(session_id, "agent_message_chunk", delta)?;
                    }
                }
                "REASONING_MESSAGE_CONTENT" | "THINKING_TEXT_MESSAGE_CONTENT" => {
                    if let Some(delta) = event["delta"].as_str().filter(|delta| !delta.is_empty()) {
                        self.chunk(session_id, "agent_thought_chunk", delta)?;
                    }
                }
                events::TOOL_CALL_START => {
                    let Some(id) = event["toolCallId"].as_str() else { continue };
                    let name = event["toolCallName"].as_str().unwrap_or("Tool");
                    tools.push(json!({
                        "id": id,
                        "type": "function",
                        "function": {"name": name, "arguments": ""},
                    }));
                    self.update(
                        session_id,
                        json!({"sessionUpdate": "tool_call", "toolCallId": id, "title": name,
                            "kind": "other", "status": "pending", "rawInput": Value::Null}),
                    )?;
                }
                events::TOOL_CALL_ARGS => {
                    if let Some(id) = event["toolCallId"].as_str() {
                        args.entry(id.to_owned()).or_default().push_str(event["delta"].as_str().unwrap_or_default());
                    }
                }
                events::TOOL_CALL_END => {
                    let Some(id) = event["toolCallId"].as_str() else { continue };
                    let collected = args.remove(id).unwrap_or_default();
                    if let Some(call) = tools.iter_mut().find(|call| call["id"] == id) {
                        call["function"]["arguments"] = collected.clone().into();
                    }
                    let input = serde_json::from_str::<Value>(&collected).unwrap_or_else(|_| json!({"raw": collected}));
                    self.update(
                        session_id,
                        json!({"sessionUpdate": "tool_call_update", "toolCallId": id,
                            "status": "in_progress", "rawInput": input}),
                    )?;
                }
                events::TOOL_CALL_RESULT => {
                    let Some(id) = event["toolCallId"].as_str() else { continue };
                    let content = match &event["content"] {
                        Value::String(text) => text.clone(),
                        other => other.to_string(),
                    };
                    self.update(
                        session_id,
                        json!({"sessionUpdate": "tool_call_update", "toolCallId": id, "status": "completed",
                            "content": [{"type": "text", "text": content}]}),
                    )?;
                }
                events::CUSTOM if is_approval(&event) => {
                    let result = self.ask(session_id, &event).await?;
                    session.locked().messages.push(assistant_message(&text, &tools));
                    session.locked().messages.push(result);
                    return Ok(Outcome::Resume);
                }
                events::RUN_FINISHED => {
                    session.locked().messages.push(assistant_message(&text, &tools));
                    return Ok(Outcome::Finished);
                }
                events::RUN_ERROR => {
                    let message = event["message"].as_str().unwrap_or("The AG-UI agent failed.");
                    return Err(Rpc::from(anyhow!("{message}")));
                }
                _ => {}
            }
        }
        Err(Rpc::from(anyhow!("The AG-UI endpoint closed the stream before RUN_FINISHED.")))
    }

    /// A `CUSTOM` approval as an ACP permission request, and the person's answer as a tool message for the
    /// endpoint's next run.
    async fn ask(&self, session_id: &str, event: &Value) -> std::result::Result<Value, Rpc> {
        let value = &event["value"];
        let tool_call_id = value["toolCallId"]
            .as_str()
            .filter(|id| !id.is_empty())
            .map_or_else(|| format!("approval-{}", uuid::Uuid::new_v4()), str::to_owned);
        let title = value["title"].as_str().unwrap_or("Approve this action");
        let options = approval_options(value);
        let reply = self
            .request(
                "session/request_permission",
                json!({
                    "sessionId": session_id,
                    "toolCall": {
                        "toolCallId": tool_call_id,
                        "title": title,
                        "kind": "other",
                        "status": "pending",
                        "rawInput": value,
                    },
                    "options": options,
                }),
            )
            .await?;
        let outcome = &reply["result"]["outcome"];
        let option_id = outcome["optionId"].as_str().map(str::to_owned);
        let approved = outcome["outcome"] == "selected"
            && option_id.as_deref().is_some_and(|id| {
                options.iter().any(|option| {
                    option["optionId"] == id && option["kind"].as_str().is_none_or(|kind| kind.starts_with("allow"))
                })
            });
        Ok(json!({
            "id": uuid::Uuid::new_v4().to_string(),
            "role": "tool",
            "toolCallId": tool_call_id,
            "content": json!({"approved": approved, "optionId": option_id}).to_string(),
        }))
    }

    fn chunk(&self, session_id: &str, kind: &str, text: &str) -> std::result::Result<(), Rpc> {
        self.update(session_id, json!({"sessionUpdate": kind, "content": {"type": "text", "text": text}}))
    }

    fn update(&self, session_id: &str, update: Value) -> std::result::Result<(), Rpc> {
        self.notify("session/update", json!({"sessionId": session_id, "update": update}))
    }

    fn notify(&self, method: &str, params: Value) -> std::result::Result<(), Rpc> {
        self.reply(json!({"jsonrpc": "2.0", "method": method, "params": params}));
        Ok(())
    }

    async fn request(&self, method: &str, params: Value) -> std::result::Result<Value, Rpc> {
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();
        self.pending.locked().insert(id, tx);
        self.reply(json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
        rx.await.map_err(|_| Rpc::from(anyhow!("the host stopped answering")))
    }

    fn reply(&self, message: Value) {
        let _ = self.out.send(message);
    }
}

enum Outcome {
    Finished,
    Cancelled,
    Resume,
}

fn initialize() -> Value {
    json!({
        "protocolVersion": 1,
        "agentCapabilities": {
            "loadSession": false,
            "promptCapabilities": {"image": false, "audio": false, "embeddedContext": false},
        },
        "agentInfo": {"name": "Codync AG-UI", "version": env!("CARGO_PKG_VERSION")},
        "authMethods": [{
            "id": "endpoint",
            "name": "AG-UI endpoint",
            "description": "The remote agent's URL, and an access token if it takes one.",
            "type": "env_var",
            "vars": [
                {"name": endpoint::URL_ENV, "label": "Endpoint URL", "secret": false, "optional": false},
                {"name": endpoint::TOKEN_ENV, "label": "Access token", "secret": true, "optional": true},
            ],
        }],
    })
}

fn is_approval(event: &Value) -> bool {
    event["name"].as_str().is_some_and(|name| name == events::APPROVAL || name.to_lowercase().contains("approval"))
}

/// The endpoint's options, or Allow/Deny when it named none.
fn approval_options(value: &Value) -> Vec<Value> {
    let options: Vec<Value> = value["options"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|option| {
            let id = option["optionId"].as_str().or(option["id"].as_str()).or(option["value"].as_str())?;
            let name = option["name"].as_str().or(option["label"].as_str()).unwrap_or(id);
            let kind = option["kind"]
                .as_str()
                .unwrap_or_else(|| if option["approved"] == true { "allow_once" } else { "reject_once" });
            Some(json!({"optionId": id, "name": name, "kind": kind}))
        })
        .collect();
    if options.is_empty() {
        return vec![
            json!({"optionId": "allow", "name": "Allow", "kind": "allow_once"}),
            json!({"optionId": "deny", "name": "Deny", "kind": "reject_once"}),
        ];
    }
    options
}

fn assistant_message(text: &str, tools: &[Value]) -> Value {
    let mut message = json!({
        "id": uuid::Uuid::new_v4().to_string(),
        "role": "assistant",
        "content": text,
    });
    if !tools.is_empty() {
        message["toolCalls"] = json!(tools);
    }
    message
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_endpoint_is_a_plain_env_var_sign_in() {
        let init = initialize();
        assert_eq!(init["authMethods"][0]["type"], "env_var");
        let names: Vec<&str> =
            init["authMethods"][0]["vars"].as_array().unwrap().iter().filter_map(|var| var["name"].as_str()).collect();
        assert_eq!(names, [endpoint::URL_ENV, endpoint::TOKEN_ENV]);
        assert_eq!(init["authMethods"][0]["vars"][1]["secret"], true);
    }

    #[test]
    fn approvals_default_to_allow_and_deny() {
        let options = approval_options(&json!({}));
        assert_eq!(options[0]["kind"], "allow_once");
        assert_eq!(options[1]["kind"], "reject_once");
    }

    #[test]
    fn an_endpoints_own_approval_options_are_kept() {
        let options = approval_options(&json!({"options": [
            {"id": "yes", "label": "Yes", "approved": true},
            {"id": "no", "label": "No"},
        ]}));
        assert_eq!(options[0]["optionId"], "yes");
        assert_eq!(options[0]["name"], "Yes");
        assert_eq!(options[0]["kind"], "allow_once");
        assert_eq!(options[1]["kind"], "reject_once");
    }

    #[test]
    fn a_matching_name_is_an_approval() {
        assert!(is_approval(&json!({"name": "codync.approval"})));
        assert!(is_approval(&json!({"name": "approval_required"})));
        assert!(!is_approval(&json!({"name": "codync.notice"})));
    }
}
