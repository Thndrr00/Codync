//! Queue intents are serialized by the actor; transcript deletion never cancels work.
use crate::agent::bot::Cmd;
use crate::hub::Hub;
use anyhow::{Context, Result};
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::oneshot;

pub(super) async fn read(hub: &Arc<Hub>, bot: &str) -> Result<Value> {
    hub.store.bot(bot)?.filter(|row| !row.deleted).context("Unknown bot")?;
    let (reply, receiver) = oneshot::channel();
    hub.send_cmd(bot, Cmd::QueuedMessages { reply })?;
    let ids = tokio::time::timeout(Duration::from_secs(5), receiver).await.context("Queue did not respond")??;
    let entries: Vec<Value> = ids
        .into_iter()
        .filter_map(|id| {
            let entry = hub.store.entry(&id)?;
            (entry.bot_id == bot && entry.data["status"] == "queued")
                .then(|| json!({"id": entry.id, "at": entry.created_at}))
        })
        .collect();
    Ok(json!({"entries": entries}))
}

pub(super) async fn cancel(hub: &Arc<Hub>, bot: &str, entry_id: &str) -> Result<Value> {
    hub.store.bot(bot)?.filter(|row| !row.deleted).context("Unknown bot")?;
    hub.store.entry(entry_id).filter(|entry| entry.bot_id == bot).context("Unknown message")?;
    let (reply, receiver) = oneshot::channel();
    hub.send_cmd(bot, Cmd::CancelQueued { entry_id: entry_id.to_owned(), reply })?;
    let cancelled = tokio::time::timeout(Duration::from_secs(5), receiver)
        .await
        .context("Cancellation did not respond; refresh queue before retrying")???;
    Ok(json!({"cancelled": cancelled}))
}
