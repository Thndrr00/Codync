//! Read-only roster queries. Authorization is the dispatcher's Control scope.

use super::str_arg;
use crate::hub::Hub;
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::sync::Arc;

fn visible_bot(hub: &Hub, id: &str) -> Result<()> {
    ensure!(hub.store.bot(id)?.is_some_and(|b| !b.deleted && !b.config.hidden), "bot is not available in the roster");
    Ok(())
}

pub(super) async fn search(hub: &Arc<Hub>, body: &Value) -> Result<Value> {
    let query = str_arg(body, "query")?.trim();
    let terms = crate::store::search_terms(query)?;
    let scope = serde_json::from_value(body.get("scope").cloned().unwrap_or(json!("all")))?;
    let bots: Vec<String> = serde_json::from_value(body["botIds"].clone()).context("botIds is required")?;
    ensure!(!bots.is_empty(), "select at least one bot");
    let limit = body["limit"].as_i64().unwrap_or(30).clamp(1, 50);
    let before = body["beforeSeq"].as_i64().unwrap_or(i64::MAX);
    let hub = hub.clone();
    tokio::task::spawn_blocking(move || {
        let mut rows = if scope == crate::store::SearchScope::All {
            hub.store.roster_search(&bots, &terms, before, limit + 1)?
        } else {
            hub.store.roster_search_scoped(&bots, &terms, before, limit + 1, scope)?
        };
        let more = rows.len() > usize::try_from(limit)?;
        rows.truncate(usize::try_from(limit)?);
        let next = if more { rows.last().map(|e| e.seq) } else { None };
        let results: Vec<Value> = rows
            .iter()
            .map(|e| {
                json!({
                    "id": e.id, "botId": e.bot_id, "threadId": e.thread_id, "seq": e.seq,
                    "text": e.data["text"].as_str().unwrap_or_default().chars().take(400).collect::<String>(),
                    "createdAt": e.created_at,
                })
            })
            .collect();
        Ok(json!({"results":results,"next":next}))
    })
    .await?
}

pub(super) async fn read(hub: &Arc<Hub>, method: &str, body: &Value) -> Result<Value> {
    let bot = str_arg(body, "botId")?.to_owned();
    visible_bot(hub, &bot)?;
    let entry = if method == "messageContext" {
        Some(
            hub.store
                .entry(str_arg(body, "entryId")?)
                .filter(|e| e.bot_id == bot)
                .context("message is no longer available")?,
        )
    } else {
        None
    };
    let hub = hub.clone();
    tokio::task::spawn_blocking(move || {
        visible_bot(&hub, &bot)?;
        let entries = match entry {
            Some(e) => hub.store.message_context(&e)?,
            None => hub.store.roster_preview(&bot)?,
        };
        Ok(json!({"entries":entries}))
    })
    .await?
}
