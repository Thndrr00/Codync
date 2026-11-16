//! Things the person is waiting on: a reply, a review, a delivery. They stay on this computer and
//! appear in the daily digest, so a bot can answer "what am I waiting on?".

use crate::hub::Hub;
use anyhow::{Result, bail, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const METHODS: &[&str] = &["waitingItems", "saveWaitingItem", "setWaitingDone", "deleteWaitingItem"];
const KEY: &str = "waiting.v1";
const LIMIT: usize = 200;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Waiting {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub counterpart: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub note: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_at: Option<i64>,
    #[serde(default)]
    pub done: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

fn read(store: &crate::store::Store) -> Vec<Waiting> {
    store.kv_read(KEY).ok().flatten().and_then(|text| serde_json::from_str(&text).ok()).unwrap_or_default()
}

fn write(store: &crate::store::Store, items: &[Waiting]) -> Result<()> {
    store.kv_set(KEY, &serde_json::to_string(items)?)
}

/// Open items, the soonest due first, for the digest.
pub fn open(store: &crate::store::Store) -> Vec<Waiting> {
    let mut items: Vec<Waiting> = read(store).into_iter().filter(|item| !item.done).collect();
    items.sort_by_key(|item| (item.due_at.unwrap_or(i64::MAX), item.created_at));
    items
}

pub fn call(hub: &Hub, method: &str, b: &Value) -> Result<Value> {
    match method {
        "waitingItems" => {
            let mut items = read(&hub.store);
            items.sort_by_key(|item| (item.done, item.due_at.unwrap_or(i64::MAX), item.created_at));
            Ok(json!({"items": items}))
        }
        "saveWaitingItem" => {
            let title = b["title"].as_str().unwrap_or_default().trim();
            ensure!(!title.is_empty(), "what are you waiting on?");
            ensure!(title.chars().count() <= 300, "that title is too long");
            let now = crate::store::now_ms();
            let mut items = read(&hub.store);
            let id = b["id"].as_str().filter(|id| !id.is_empty());
            let new_counterpart = b["counterpart"].as_str().map(str::to_owned);
            let new_url = b["url"].as_str().map(str::to_owned);
            let new_note = b["note"].as_str().map(str::to_owned);
            let item = match id.and_then(|id| items.iter().position(|item| item.id == id)) {
                Some(index) => {
                    let item = &mut items[index];
                    title.clone_into(&mut item.title);
                    if let Some(value) = new_counterpart {
                        item.counterpart = value;
                    }
                    if let Some(value) = new_url {
                        item.url = value;
                    }
                    if let Some(value) = new_note {
                        item.note = value;
                    }
                    item.due_at = b["dueAt"].as_i64().or(item.due_at);
                    item.updated_at = now;
                    item.clone()
                }
                None => {
                    ensure!(items.len() < LIMIT, "at most {LIMIT} waiting items");
                    let item = Waiting {
                        id: uuid::Uuid::new_v4().to_string(),
                        title: title.to_owned(),
                        counterpart: b["counterpart"].as_str().unwrap_or_default().chars().take(200).collect(),
                        url: b["url"].as_str().unwrap_or_default().chars().take(2_000).collect(),
                        note: b["note"].as_str().unwrap_or_default().chars().take(2_000).collect(),
                        due_at: b["dueAt"].as_i64(),
                        done: false,
                        created_at: now,
                        updated_at: now,
                    };
                    items.push(item.clone());
                    item
                }
            };
            write(&hub.store, &items)?;
            Ok(json!({"item": item}))
        }
        "setWaitingDone" => {
            let id = str_arg(b, "id")?;
            let done = b["done"].as_bool().unwrap_or(true);
            let mut items = read(&hub.store);
            let item = items.iter_mut().find(|item| item.id == id).ok_or_else(|| anyhow::anyhow!("unknown item"))?;
            item.done = done;
            item.updated_at = crate::store::now_ms();
            let changed = item.clone();
            write(&hub.store, &items)?;
            Ok(json!({"item": changed}))
        }
        "deleteWaitingItem" => {
            let id = str_arg(b, "id")?;
            let mut items = read(&hub.store);
            let before = items.len();
            items.retain(|item| item.id != id);
            write(&hub.store, &items)?;
            Ok(json!({"deleted": items.len() != before}))
        }
        other => bail!("unknown waiting method {other}"),
    }
}

fn str_arg<'a>(b: &'a Value, key: &str) -> Result<&'a str> {
    b[key].as_str().filter(|s| !s.trim().is_empty()).ok_or_else(|| anyhow::anyhow!("`{key}` is required"))
}

pub fn tools() -> Value {
    json!([{
        "name": "waiting_on",
        "description": "List what the user is waiting on (replies, reviews, deliveries) as kept on this computer. Read-only.",
        "inputSchema": {"type": "object", "properties": {}},
        "annotations": {"readOnlyHint": true, "idempotentHint": true},
    }])
}

pub fn tool_call(store: &crate::store::Store, name: &str) -> Result<Value> {
    ensure!(name == "waiting_on", "unknown waiting tool: {name}");
    let items = open(store);
    if items.is_empty() {
        return Ok(json!({"text": "Nothing is marked as waiting."}));
    }
    let lines: Vec<String> = items
        .iter()
        .map(|item| {
            let who = if item.counterpart.is_empty() { String::new() } else { format!(" — {}", item.counterpart) };
            let due = item
                .due_at
                .and_then(chrono::DateTime::<chrono::Utc>::from_timestamp_millis)
                .map(|at| format!(" (due {})", at.format("%d %b")))
                .unwrap_or_default();
            format!("- {}{who}{due}", item.title)
        })
        .collect();
    Ok(json!({"text": lines.join("\n")}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_items_skip_done_ones() {
        let store = crate::store::Store::open(std::path::Path::new(":memory:")).unwrap();
        let now = crate::store::now_ms();
        let items = vec![
            Waiting {
                id: "a".into(),
                title: "Reply".into(),
                counterpart: String::new(),
                url: String::new(),
                note: String::new(),
                due_at: None,
                done: false,
                created_at: now,
                updated_at: now,
            },
            Waiting {
                id: "b".into(),
                title: "Done".into(),
                counterpart: String::new(),
                url: String::new(),
                note: String::new(),
                due_at: None,
                done: true,
                created_at: now,
                updated_at: now,
            },
        ];
        write(&store, &items).unwrap();
        assert_eq!(open(&store).iter().map(|item| item.id.as_str()).collect::<Vec<_>>(), ["a"]);
    }
}
