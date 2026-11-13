//! A chat's Library, Grok Bot's way: the artifacts its bot published and the files shared in it, by the bot
//! (`send_file`) or the user (attachments), newest first. The apps open each one the way the chat does: an artifact with
//! `readArtifact`, a bot's file with `readFile`, an attachment with `readUpload`.

use crate::chat::artifacts;
use crate::hub::Hub;
use crate::store::EntryKind;
use anyhow::{Result, ensure};
use serde_json::{Value, json};

/// Messages with files it looks through, newest first.
const MAX_FILE_MESSAGES: i64 = 500;

/// What the server's Library shows: everything, only images, or only other files.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Filter {
    All,
    Images,
    Files,
}

impl Filter {
    fn parse(value: Option<&str>) -> Self {
        match value {
            Some("images") => Self::Images,
            Some("files") => Self::Files,
            _ => Self::All,
        }
    }
}

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

/// `serverLibrary {filter}` → every artifact and file of the server's bots, newest first. It reads the
/// same per-chat stores as `library`, so nothing on the computer's disk is scanned.
pub fn server(hub: &Hub, filter: Option<&str>) -> Result<Value> {
    let filter = Filter::parse(filter);
    let mut items: Vec<Value> = Vec::new();
    for row in hub.store.bots()?.into_iter().filter(|row| !row.deleted) {
        let config = &row.config;
        let listed = list(hub, &config.id)?;
        for mut item in listed["items"].as_array().cloned().unwrap_or_default() {
            item["botId"] = config.id.clone().into();
            item["botName"] = config.name.clone().into();
            item["image"] = is_image(&item).into();
            items.push(item);
        }
    }
    Ok(json!({"items": merge(items, filter)}))
}

/// A file the Library shows as a picture: the extensions the apps preview.
fn is_image(item: &Value) -> bool {
    let name = item["file"]["name"].as_str().unwrap_or_default().to_ascii_lowercase();
    matches!(
        name.rsplit('.').next().unwrap_or_default(),
        "png" | "jpg" | "jpeg" | "heic" | "gif" | "webp" | "tiff" | "bmp"
    )
}

/// Newest first, one row per artifact or file, and only what `filter` asks for. Two bots that were
/// sent the same file share its id, so it shows once.
fn merge(mut items: Vec<Value>, filter: Filter) -> Vec<Value> {
    items.sort_by_key(|item| std::cmp::Reverse(item["at"].as_i64().unwrap_or_default()));
    let mut seen = std::collections::HashSet::new();
    items.retain(|item| {
        let id = item["artifact"]["id"].as_str().or_else(|| item["file"]["id"].as_str()).unwrap_or_default();
        let key = format!("{}:{id}", item["kind"].as_str().unwrap_or_default());
        if !seen.insert(key) {
            return false;
        }
        match filter {
            Filter::All => true,
            Filter::Images => item["image"].as_bool() == Some(true),
            Filter::Files => item["image"].as_bool() != Some(true),
        }
    });
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    fn artifact(id: &str, at: i64) -> Value {
        json!({"kind": "artifact", "at": at, "artifact": {"id": id}, "image": false})
    }

    fn file(id: &str, name: &str, at: i64) -> Value {
        let item = json!({"kind": "file", "at": at, "file": {"id": id, "name": name}});
        json!({"kind": "file", "at": at, "file": {"id": id, "name": name}, "image": is_image(&item)})
    }

    #[test]
    fn the_server_list_is_newest_first_and_one_row_per_item() {
        let merged = merge(
            vec![artifact("a", 10), file("f", "notes.txt", 30), file("f", "notes.txt", 30), artifact("a", 10)],
            Filter::All,
        );
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0]["file"]["id"], "f");
        assert_eq!(merged[1]["artifact"]["id"], "a");
    }

    #[test]
    fn images_and_files_filter_by_the_extension() {
        let items = vec![artifact("a", 3), file("photo", "Trip.PNG", 2), file("doc", "plan.md", 1)];
        let images = merge(items.clone(), Filter::Images);
        assert_eq!(images.len(), 1);
        assert_eq!(images[0]["file"]["name"], "Trip.PNG");
        let files = merge(items, Filter::Files);
        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|item| item["kind"] == "artifact"));
    }

    #[test]
    fn an_unknown_filter_shows_everything() {
        assert_eq!(Filter::parse(None), Filter::All);
        assert_eq!(Filter::parse(Some("images")), Filter::Images);
        assert_eq!(Filter::parse(Some("files")), Filter::Files);
        assert_eq!(Filter::parse(Some("videos")), Filter::All);
    }
}
