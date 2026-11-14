//! `POST /agui/{bot}`: one bot turn as an AG-UI run over Server-Sent Events.
//!
//! This is a view of the host's event bus, not a second agent loop: the newest user message goes through
//! the bot's normal `send` path, and everything the bot writes during that turn is mapped by [`Mapper`].

use super::events;
use super::input::RunInput;
use super::mapping::Mapper;
use crate::agent::bot::Cmd;
use crate::chat::messages;
use crate::hub::Hub;
use crate::store::{BotConfig, Lane};
use anyhow::{Context, Result, anyhow};
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

/// The largest request body accepted: a run's whole message history.
pub const MAX_BODY: usize = 1024 * 1024;
/// How long a run may keep the connection open without ending.
const RUN_TIMEOUT: Duration = Duration::from_mins(30);

/// Streams one run for `bot`, named by id or name.
pub async fn serve(hub: Arc<Hub>, bot: &str, body: Bytes) -> Result<Response> {
    let cfg = resolve(&hub, bot)?;
    let body: Value = serde_json::from_slice(&body).context("invalid JSON")?;
    let input = RunInput::parse(&body).context("invalid AG-UI run input")?;
    let (tx, rx) = mpsc::channel(64);
    let done = Arc::new(AtomicBool::new(false));
    let bot_id = cfg.id.clone();
    tokio::spawn(run(hub.clone(), cfg, input, tx, done.clone()));
    // The client went away: stop the turn, unless it already ended.
    let guard = StopOnDrop { hub, bot: bot_id, done };
    let stream = ReceiverStream::new(rx).map(move |event| {
        let _keep = &guard;
        Ok::<Bytes, Infallible>(Bytes::from(events::frame(&event)))
    });
    Response::builder()
        .header(header::CONTENT_TYPE, "text/event-stream")
        .header(header::CACHE_CONTROL, "no-cache")
        .header(header::CONNECTION, "keep-alive")
        .header("x-accel-buffering", "no")
        .body(Body::from_stream(stream))
        .context("building the event stream")
}

/// The bot `name` names, by id or by name.
fn resolve(hub: &Hub, name: &str) -> Result<BotConfig> {
    if let Some(row) = hub.store.bot(name)?
        && !row.deleted
        && !row.config.is_group()
    {
        return Ok(row.config);
    }
    hub.store
        .bots()?
        .into_iter()
        .find(|row| !row.deleted && !row.config.is_group() && row.config.name.eq_ignore_ascii_case(name))
        .map(|row| row.config)
        .ok_or_else(|| anyhow!("unknown bot `{name}`"))
}

async fn run(hub: Arc<Hub>, cfg: BotConfig, input: RunInput, tx: mpsc::Sender<Value>, done: Arc<AtomicBool>) {
    let mut events = hub.events.subscribe();
    if !emit(&tx, events::run_started(&input.thread_id, &input.run_id)).await {
        return;
    }
    let _ = emit(&tx, Mapper::new(&input.thread_id, &input.run_id).state_snapshot(&input.state)).await;
    let entry = match messages::post_in_lane(
        &hub,
        &cfg,
        Lane::main(&cfg.id),
        json!({"text": input.text, "clientNonce": format!("agui-{}", input.run_id)}),
    ) {
        Ok(entry) => entry,
        Err(error) => {
            finish(&tx, &done, &hub, &cfg.id, events::run_error(&format!("{error:#}"), "send_failed")).await;
            return;
        }
    };
    let mut mapper = Mapper::new(&input.thread_id, &input.run_id);
    let deadline = tokio::time::Instant::now() + RUN_TIMEOUT;
    loop {
        match tokio::time::timeout_at(deadline, events.recv()).await {
            Ok(Ok(event)) => {
                if event["type"] == "entryDeleted" && event["id"] == entry.id.as_str() {
                    finish(&tx, &done, &hub, &cfg.id, events::run_error("The message was deleted.", "deleted")).await;
                    return;
                }
                if !is_turn_entry(&event, &cfg.id, entry.turn) {
                    continue;
                }
                for mapped in mapper.entry(&event["entry"]) {
                    if !emit(&tx, mapped).await {
                        return;
                    }
                }
                if event["entry"]["id"] == entry.id.as_str()
                    && let Some(result) = event["entry"]["data"].get("turnResult")
                {
                    // The turn is over, whether or not the client is still there to hear it.
                    done.store(true, Ordering::Release);
                    for mapped in mapper.finish(result) {
                        let _ = emit(&tx, mapped).await;
                    }
                    return;
                }
            }
            // The event bus dropped events: the result may have been in the gap.
            Ok(Err(_)) => {
                if let Some(result) = hub.store.entry(&entry.id).and_then(|entry| entry.data.get("turnResult").cloned())
                {
                    done.store(true, Ordering::Release);
                    for mapped in mapper.finish(&result) {
                        let _ = emit(&tx, mapped).await;
                    }
                    return;
                }
                if !emit(&tx, events::custom(events::RESYNC, json!({}))).await {
                    return;
                }
            }
            Err(_) => {
                finish(&tx, &done, &hub, &cfg.id, events::run_error("The turn didn't finish in time.", "timeout"))
                    .await;
                return;
            }
        }
    }
}

/// A live entry of this turn in the bot's own chat (not a side conversation, not another turn).
fn is_turn_entry(event: &Value, bot: &str, turn: i64) -> bool {
    let entry = &event["entry"];
    event["type"] == "entry"
        && entry["botId"] == bot
        && entry["turn"].as_i64() == Some(turn)
        && entry.get("threadId").is_none()
}

async fn emit(tx: &mpsc::Sender<Value>, value: Value) -> bool {
    tx.send(value).await.is_ok()
}

/// Sends the ending event, then stops the bot's turn if it is still running.
async fn finish(tx: &mpsc::Sender<Value>, done: &AtomicBool, hub: &Hub, bot: &str, event: Value) {
    done.store(true, Ordering::Release);
    let _ = emit(tx, event).await;
    // A failed or timed-out run may still be working; deleting the message stops at once.
    let _ = hub.send_cmd(bot, Cmd::Stop);
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
