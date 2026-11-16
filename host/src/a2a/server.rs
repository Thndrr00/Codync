//! The local A2A endpoint: `GET /a2a/{bot}/.well-known/agent-card.json` and `POST /a2a/{bot}` with
//! `message/send` and `message/stream` (Server-Sent Events).
//!
//! The route is authenticated like the rest of the local API (loopback + bearer token), so it is
//! for this computer's apps and helpers. Another computer reaches the same bot through
//! codync-network (`a2a::client`), where the network's identity, rules and the bot's approvals
//! apply; no A2A port is opened to the internet.

use super::{card, protocol};
use crate::agent::bot::Cmd;
use crate::chat::messages;
use crate::hub::Hub;
use crate::store::{BotConfig, Lane};
use anyhow::{Context, Result, anyhow, bail};
use axum::Json;
use axum::body::{Body, Bytes};
use axum::http::header;
use axum::response::Response;
use futures::StreamExt;
use serde_json::{Value, json};
use std::convert::Infallible;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

/// One A2A request body: a message and its parts.
pub const MAX_BODY: usize = 1024 * 1024;
/// One task's turn may keep the connection open this long.
const RUN_TIMEOUT: Duration = Duration::from_mins(10);

fn resolve(hub: &Hub, name: &str) -> Result<BotConfig> {
    let row = if let Some(row) = hub.store.bot(name)? {
        row
    } else {
        hub.store
            .bots()?
            .into_iter()
            .find(|row| !row.deleted && !row.config.is_group() && row.config.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| anyhow!("unknown A2A agent `{name}`"))?
    };
    anyhow::ensure!(!row.deleted && !row.config.is_group(), "unknown A2A agent `{name}`");
    anyhow::ensure!(
        crate::a2a::agent_available(&row.config),
        "`{name}` isn't offered to the network, so it isn't an A2A agent"
    );
    Ok(row.config)
}

/// `GET /a2a/{bot}/.well-known/agent-card.json`.
pub(crate) async fn agent_card(hub: Arc<Hub>, bot: &str) -> Result<Json<Value>> {
    let cfg = resolve(&hub, bot)?;
    let handle = hub.network.handle_of(&cfg.id).unwrap_or_default();
    let computer = hub.network.my_name().unwrap_or_default();
    let url = format!("http://127.0.0.1:{}/a2a/{}", hub.port, cfg.id);
    Ok(Json(card::agent_card(&cfg, &handle, &computer, &url)))
}

/// `POST /a2a/{bot}`.
pub(crate) async fn serve(hub: Arc<Hub>, bot: &str, body: Bytes) -> Result<Response> {
    let cfg = resolve(&hub, bot)?;
    let body: Value = serde_json::from_slice(&body).context("invalid JSON")?;
    let request = match protocol::Request::parse(&body) {
        Ok(request) => request,
        Err(error) => {
            let id = body.get("id").cloned().unwrap_or(Value::Null);
            return json_response(protocol::error(&id, -32602, &format!("{error:#}")));
        }
    };
    match request.method {
        protocol::Method::Send => {
            let text = turn(hub, cfg, &request, None).await;
            let result = match text {
                Ok(text) => protocol::task(&request, protocol::State::Completed, Some(&text)),
                Err(error) => {
                    return json_response(protocol::error(&request.id, -32000, &format!("{error:#}")));
                }
            };
            json_response(protocol::success(&request.id, result))
        }
        protocol::Method::Stream => {
            let (tx, rx) = mpsc::channel(64);
            let done = Arc::new(AtomicBool::new(false));
            let bot_id = cfg.id.clone();
            let guard = StopOnDrop { hub: hub.clone(), bot: bot_id, done: done.clone() };
            tokio::spawn(async move {
                let working = protocol::status_update(&request, protocol::State::Working, false);
                let _ = tx.send(protocol::success(&request.id, working)).await;
                let text = turn(hub, cfg, &request, Some(&tx)).await;
                match text {
                    Ok(text) => {
                        let artifact = protocol::artifact_update(&request, &text, false, true);
                        let _ = tx.send(protocol::success(&request.id, artifact)).await;
                        let end = protocol::status_update(&request, protocol::State::Completed, true);
                        let _ = tx.send(protocol::success(&request.id, end)).await;
                    }
                    Err(error) => {
                        let failed = protocol::status_update(&request, protocol::State::Failed, true);
                        let _ = tx.send(protocol::success(&request.id, failed)).await;
                        tracing::warn!(error = format!("{error:#}"), "A2A stream ended in a failure");
                    }
                }
                done.store(true, Ordering::Release);
            });
            let stream = ReceiverStream::new(rx).map(move |value| {
                let _keep = &guard;
                Ok::<Bytes, Infallible>(Bytes::from(protocol::frame(&value)))
            });
            Response::builder()
                .header(header::CONTENT_TYPE, "text/event-stream")
                .header(header::CACHE_CONTROL, "no-cache")
                .header(header::CONNECTION, "keep-alive")
                .header("x-accel-buffering", "no")
                .body(Body::from_stream(stream))
                .context("building the A2A event stream")
        }
    }
}

fn json_response(value: Value) -> Result<Response> {
    Ok(Json(value).into_response())
}

use axum::response::IntoResponse;

/// Runs one turn of the bot with the given text and returns its final text. With `tx`, agent
/// segments are also streamed as artifact updates.
async fn turn(
    hub: Arc<Hub>,
    cfg: BotConfig,
    request: &protocol::Request,
    tx: Option<&mpsc::Sender<Value>>,
) -> Result<String> {
    let mut events = hub.events.subscribe();
    let entry = messages::post_in_lane(
        &hub,
        &cfg,
        Lane::main(&cfg.id),
        // Where it came from, for a policy's `initiator.kind` (`agent::bot::context`).
        json!({"text": request.text.clone(), "clientNonce": request.message_id.clone(), "origin": "network"}),
    )?;
    let mut sent = String::new();
    if let Some(result) = entry.data.get("turnResult")
        && let Some(text) = result["text"].as_str().filter(|text| !text.is_empty())
    {
        return Ok(text.to_owned());
    }
    let deadline = tokio::time::Instant::now() + RUN_TIMEOUT;
    loop {
        let event = match tokio::time::timeout_at(deadline, events.recv()).await {
            Ok(Ok(event)) => event,
            Ok(Err(_)) => {
                if let Some(result) = hub.store.entry(&entry.id).and_then(|entry| entry.data.get("turnResult").cloned())
                {
                    return finish(result);
                }
                continue;
            }
            Err(_) => bail!("the A2A task didn't finish in time"),
        };
        if event["type"] != "entry" || event["entry"]["botId"] != cfg.id {
            continue;
        }
        let live = &event["entry"];
        if live["threadId"].is_string() || live["turn"].as_i64() != Some(entry.turn) {
            continue;
        }
        if live["kind"] == "agent" {
            let text = live["data"]["text"].as_str().unwrap_or_default();
            if let Some(tx) = tx
                && text.len() > sent.len()
                && text.starts_with(&sent)
            {
                let chunk = &text[sent.len()..];
                let update = protocol::artifact_update(request, chunk, !sent.is_empty(), false);
                if tx.send(protocol::success(&request.id, update)).await.is_err() {
                    return Ok(text.to_owned());
                }
                sent = text.to_owned();
            }
        }
        if live["id"] == entry.id.as_str()
            && let Some(result) = live["data"].get("turnResult")
        {
            return finish(result);
        }
    }
}

fn finish(result: &Value) -> Result<String> {
    match result["state"].as_str() {
        Some("done") => Ok(result["text"].as_str().unwrap_or_default().to_owned()),
        Some("paused") | Some("truncated") | Some("stopped") => {
            Ok(result["text"].as_str().unwrap_or_default().to_owned())
        }
        Some(other) => bail!("the bot's turn ended {other}"),
        None => bail!("the bot's turn ended without a result"),
    }
}

/// Stops the turn when the client disconnects before it ended.
struct StopOnDrop {
    hub: Arc<Hub>,
    bot: String,
    done: Arc<AtomicBool>,
}

impl Drop for StopOnDrop {
    fn drop(&mut self) {
        if !self.done.load(Ordering::Acquire) {
            let _ = self.hub.send_cmd(&self.bot, Cmd::Stop);
        }
    }
}
