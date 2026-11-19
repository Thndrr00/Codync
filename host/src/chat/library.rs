//! A chat's Library, Grok Bot's way: the artifacts its bot published and the files shared in it, by the bot
//! (`send_file`) or the user (attachments), newest first. The apps open each one the way the chat does: an artifact with
//! `readArtifact`, a bot's file with `readFile`, an attachment with `readUpload`.
//!
//! A chat may also file its items in folders (`library/folders.rs`). The folder blob is separate from the items, so
//! a library with no folders — or one saved before folders existed — reads exactly as it always did.

pub mod agent_tools;
mod folders;
mod state;

pub use agent_tools::{TOOLS, tools};

use crate::chat::artifacts;
use crate::hub::Hub;
use crate::store::EntryKind;
use anyhow::{Context as _, Result, ensure};
use folders::FolderState;
use serde_json::{Value, json};
use std::collections::BTreeMap;

/// Messages with files it looks through, newest first.
const MAX_FILE_MESSAGES: i64 = 500;

/// The items of a chat's Library, newest first, before folder annotations.
pub(crate) fn items(hub: &Hub, bot: &str) -> Result<Vec<Value>> {
    ensure!(hub.store.bot(bot)?.is_some_and(|row| !row.deleted), "unknown bot");
    let mut items: Vec<Value> = artifacts::list(&hub.store, bot)
        .into_iter()
        .filter(artifacts::Artifact::is_public)
        .filter(|artifact| {
            artifact
                .entry_id
                .as_deref()
                .and_then(|id| hub.store.entry(id))
                .is_none_or(|entry| !crate::store::is_internal(&entry.data))
        })
        .map(|artifact| json!({"kind": "artifact", "at": artifact.updated_at, "artifact": artifact}))
        .collect();
    for e in hub.store.file_messages(bot, MAX_FILE_MESSAGES)? {
        let (kind, files) = if e.kind == EntryKind::User.as_str() {
            ("attachment", &e.data["attachments"])
        } else {
            ("file", &e.data["files"])
        };
        for file in files.as_array().into_iter().flatten() {
            items.push(json!({"kind": kind, "at": e.created_at, "entryId": e.id, "file": file}));
        }
    }
    items.sort_by_key(|item| std::cmp::Reverse(item["at"].as_i64().unwrap_or_default()));
    Ok(items)
}

/// `library {botId}` → `{items: [{kind: artifact | file | attachment, at, folderId?, …}]}`.
pub fn list(hub: &Hub, bot: &str) -> Result<Value> {
    let mut items = items(hub, bot)?;
    let folders = state::load(&hub.store, bot)?;
    for item in &mut items {
        if let Some(key) = state::item_key(item)
            && let Some(folder) = folders.folder_of(&key)
        {
            item["folderId"] = json!(folder);
        }
    }
    Ok(json!({"items": items}))
}

/// The item keys with the metadata a folder tile's newest list and an agent's filing call need.
fn item_index(items: &[Value]) -> BTreeMap<String, (i64, String, &'static str)> {
    items
        .iter()
        .filter_map(|item| {
            let key = state::item_key(item)?;
            let at = item["at"].as_i64().unwrap_or_default();
            let (name, kind) = match item["kind"].as_str()? {
                "artifact" => (item["artifact"]["title"].as_str().unwrap_or_default().to_owned(), "artifact"),
                _ => (item["file"]["name"].as_str().unwrap_or_default().to_owned(), "file"),
            };
            Some((key, (at, name, kind)))
        })
        .collect()
}

/// Folders in their saved order, each with its filed item keys newest first and the time of its newest item.
fn folder_views(folders: &FolderState, index: &BTreeMap<String, (i64, String, &'static str)>) -> Value {
    json!(
        folders
            .folders
            .iter()
            .map(|folder| {
                let mut filed: Vec<(i64, &String)> = folders
                    .items
                    .iter()
                    .filter(|(_, id)| id.as_str() == folder.id)
                    .filter_map(|(key, _)| index.get(key).map(|(at, _, _)| (*at, key)))
                    .collect();
                filed.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.cmp(a.1)));
                let at = filed.first().map_or(folder.created_at, |(at, _)| *at);
                json!({
                    "id": folder.id,
                    "name": folder.name,
                    "at": at,
                    "count": filed.len(),
                    "items": filed.into_iter().map(|(_, key)| key).collect::<Vec<_>>(),
                })
            })
            .collect::<Vec<_>>()
    )
}

fn item_views(index: &BTreeMap<String, (i64, String, &'static str)>) -> Value {
    let mut items: Vec<(&String, &(i64, String, &'static str))> = index.iter().collect();
    items.sort_by(|a, b| b.1.0.cmp(&a.1.0).then_with(|| b.0.cmp(a.0)));
    json!(
        items
            .into_iter()
            .map(|(key, (at, name, kind))| json!({"key": key, "at": at, "name": name, "kind": kind}))
            .collect::<Vec<_>>()
    )
}

fn bot_arg(b: &Value) -> Result<&str> {
    b["botId"].as_str().context("botId is required")
}

fn text_arg(b: &Value, key: &str) -> Result<&str> {
    b[key].as_str().with_context(|| format!("{key} is required"))
}

/// The desktop Library's folder methods, all scoped to one chat. Every change stamps a rev and returns the folder
/// list, so the caller can show the new order or, after a delete, where the items went.
pub fn call(hub: &Hub, method: &str, b: &Value) -> Result<Value> {
    let bot = bot_arg(b)?;
    ensure!(hub.store.bot(bot)?.is_some_and(|row| !row.deleted), "unknown bot");
    let mut folders = state::load(&hub.store, bot)?;
    match method {
        "libraryFolders" => {}
        "libraryFolderCreate" => {
            let id = uuid::Uuid::new_v4().to_string();
            folders.create(id, text_arg(b, "name")?, crate::store::now_ms())?;
            state::save(&hub.store, bot, &mut folders)?;
        }
        "libraryFolderRename" => {
            folders.rename(text_arg(b, "id")?, text_arg(b, "name")?)?;
            state::save(&hub.store, bot, &mut folders)?;
        }
        "libraryFolderMove" => {
            folders.move_folder(text_arg(b, "id")?, b["beforeId"].as_str())?;
            state::save(&hub.store, bot, &mut folders)?;
        }
        "libraryFolderDelete" => {
            folders.delete(text_arg(b, "id")?)?;
            state::save(&hub.store, bot, &mut folders)?;
        }
        "libraryFileItem" => {
            let key = text_arg(b, "item")?;
            FolderState::check_item(key)?;
            match b["folderId"].as_str() {
                Some(folder) => folders.file(key, folder)?,
                None => folders.unfile(key)?,
            }
            state::save(&hub.store, bot, &mut folders)?;
        }
        _ => anyhow::bail!("unknown Library method {method}"),
    }
    let items = items(hub, bot)?;
    let index = item_index(&items);
    Ok(json!({"folders": folder_views(&folders, &index), "items": item_views(&index)}))
}
