//! Batch actions for the desktop Library's selected items, dispatched from `api/mod.rs`.
//! To wire it, `mod.rs` declares this module and routes the two methods before `bots::call`:
//! ```ignore
//! mod library_batch;
//! // … inside the dispatch chain, before `bots::call`:
//! if matches!(method, "libraryBatchPlan" | "libraryBatchDelete") {
//!     return library_batch::call(hub, method, b).await;
//! }
//! ```
//! `plan` names what each selected key reads; `delete` removes message-backed items and artifacts of
//! one chat. Both loop the same single-item reads and deletions the chat already uses.

use crate::chat::library;
use crate::hub::Hub;
use anyhow::{Result, anyhow, bail, ensure};
use serde_json::{Value, json};
use std::sync::Arc;

/// One action on many items is a mistake at this size, not a batch.
const MAX_ITEMS: usize = 100;

fn bot_arg(body: &Value) -> Result<&str> {
    body["botId"].as_str().ok_or_else(|| anyhow!("botId is required"))
}

/// The key the Library and its folder list use to name one item.
fn item_key(item: &Value) -> Option<String> {
    match item["kind"].as_str()? {
        "artifact" => Some(format!("artifact:{}", item["artifact"]["id"].as_str()?)),
        kind @ ("file" | "attachment") => {
            Some(format!("{}:{}:{}", kind, item["entryId"].as_str()?, item["file"]["id"].as_str()?))
        }
        _ => None,
    }
}

fn selected(hub: &Hub, body: &Value) -> Result<(String, Vec<String>, Vec<Value>)> {
    let bot = bot_arg(body)?.to_owned();
    ensure!(hub.store.bot(&bot)?.is_some_and(|row| !row.deleted), "unknown bot");
    let keys = body["keys"].as_array().ok_or_else(|| anyhow!("keys must be a list"))?;
    ensure!(keys.len() <= MAX_ITEMS, "too many items selected");
    let keys: Vec<String> = keys
        .iter()
        .map(|key| key.as_str().map(str::to_owned).ok_or_else(|| anyhow!("keys must be strings")))
        .collect::<Result<_>>()?;
    let items = library::items(hub, &bot)?;
    let mut targets = Vec::new();
    for key in &keys {
        let Some(item) = items.iter().find(|item| item_key(item).as_deref() == Some(key.as_str())) else {
            continue;
        };
        let kind = item["kind"].as_str().unwrap_or_default();
        targets.push(json!({
            "key": key,
            "kind": kind,
            "entryId": item["entryId"].as_str().unwrap_or_default(),
            "id": if kind == "artifact" { item["artifact"]["id"].as_str().unwrap_or_default() } else { item["file"]["id"].as_str().unwrap_or_default() },
            "name": if kind == "artifact" { item["artifact"]["title"].as_str().unwrap_or_default() } else { item["file"]["name"].as_str().unwrap_or_default() },
        }));
    }
    Ok((bot, keys, targets))
}

/// `libraryBatchPlan {botId, keys}` → the reads each item needs, so a client can loop its own save path.
pub fn plan(hub: &Hub, body: &Value) -> Result<Value> {
    let (_, keys, targets) = selected(hub, body)?;
    Ok(json!({
        "targets": targets,
        "missing": keys.len().saturating_sub(targets.len()),
    }))
}

/// `libraryBatchDelete {botId, keys}` → `{deleted}`, scoped to items of one live chat.
pub async fn delete(hub: &Arc<Hub>, body: &Value) -> Result<Value> {
    let (bot, _, targets) = selected(hub, body)?;
    let mut deleted = 0usize;
    for target in targets {
        let kind = target["kind"].as_str().unwrap_or_default();
        let entry = target["entryId"].as_str().unwrap_or_default();
        if !entry.is_empty() {
            hub.delete_entry(entry)?;
        }
        if kind == "artifact" {
            let id = target["id"].as_str().unwrap_or_default();
            if uuid::Uuid::parse_str(id).is_err() {
                continue;
            }
            hub.store.kv_delete(&format!("artifact.{bot}.{id}"))?;
            if let Ok(path) = crate::chat::artifacts::html_path(&bot, id) {
                let _ = std::fs::remove_file(path);
            }
        }
        deleted += 1;
    }
    Ok(json!({"deleted": deleted}))
}

pub async fn call(hub: &Arc<Hub>, method: &str, body: &Value) -> Result<Value> {
    match method {
        "libraryBatchPlan" => plan(hub, body),
        "libraryBatchDelete" => delete(hub, body).await,
        _ => bail!("unknown Library batch method {method}"),
    }
}
