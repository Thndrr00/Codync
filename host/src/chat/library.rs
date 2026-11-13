//! A chat's Library, Grok Bot's way: the artifacts its bot published and the files shared in it, by the bot
//! (`send_file`) or the user (attachments), newest first. The apps open each one the way the chat does: an artifact with
//! `readArtifact`, a bot's file with `readFile`, an attachment with `readUpload`.

use crate::chat::artifacts;
use crate::hub::Hub;
use crate::store::EntryKind;
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};

/// Messages with files it looks through, newest first.
const MAX_FILE_MESSAGES: i64 = 500;

/// `library {botId}` → `{items: [{kind: artifact | file | attachment, at, …}]}`.
pub fn list(hub: &Hub, bot: &str) -> Result<Value> {
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
    Ok(json!({"items": items}))
}

/// `fileVersions {botId, name}` → `{versions: [{at, entryId, file}]}`, newest first: every snapshot a
/// bot published under that name. Each one is immutable, so an older version is read the same way as
/// the newest (`readFile` with its `entryId` and `fileId`).
pub fn versions(hub: &Hub, b: &Value) -> Result<Value> {
    let bot = b["botId"].as_str().context("`botId` is required")?;
    let name = b["name"].as_str().context("`name` is required")?;
    ensure!(hub.store.bot(bot)?.is_some_and(|row| !row.deleted), "unknown bot");
    let mut versions: Vec<Value> = Vec::new();
    for e in hub.store.file_messages(bot, MAX_FILE_MESSAGES)? {
        let files = if e.kind == EntryKind::User.as_str() { &e.data["attachments"] } else { &e.data["files"] };
        for file in files.as_array().into_iter().flatten() {
            if file["name"].as_str() == Some(name) {
                versions.push(json!({"at": e.created_at, "entryId": e.id, "file": file}));
            }
        }
    }
    versions.sort_by_key(|item| std::cmp::Reverse(item["at"].as_i64().unwrap_or_default()));
    Ok(json!({"versions": versions}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::identity::Identity;
    use crate::store::{Lane, Store};
    use std::sync::Arc;

    fn hub() -> (Arc<Hub>, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("codync-library-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let identity = Identity::load_or_create(&dir).unwrap();
        (Hub::new(store, "h".into(), identity, "t".into(), 0), dir)
    }

    #[test]
    fn every_snapshot_of_a_name_is_a_version() {
        let (hub, dir) = hub();
        let config = serde_json::from_value(json!({"id": "b1", "name": "B", "backend": "claude"})).unwrap();
        hub.store.save_bot(&config).unwrap();
        let lane = Lane::main("b1");
        for (file_id, other) in [("f1", "notes.md"), ("f2", "notes.md"), ("f3", "other.md")] {
            let data = json!({
                "text": "here",
                "final": true,
                "files": [{"id": file_id, "name": other, "size": 3, "sha256": "abc"}],
            });
            hub.store.insert_entry(&lane, EntryKind::Agent, 1, &data).unwrap();
        }
        let found = versions(&hub, &json!({"botId": "b1", "name": "notes.md"})).unwrap();
        let versions = found["versions"].as_array().unwrap();
        assert_eq!(versions.len(), 2, "the other name is not a version of this one");
        let ids: Vec<&str> = versions.iter().filter_map(|v| v["file"]["id"].as_str()).collect();
        assert!(ids.contains(&"f1") && ids.contains(&"f2"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
