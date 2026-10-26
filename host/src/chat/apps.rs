//! Built-in `chat` MCP tools for bot-built mini-apps (`save_app`, `list_apps`, `delete_app`,
//! `app_data_list`, `app_data_put`, `app_data_delete`) and host API methods for managing apps
//! and share links from the UI (Ideas 1–7, 47, 48).

use crate::api::apps_runtime::emit_app_live;
use crate::hub::Hub;
use crate::store::{AppLinkMode, EntryKind, Lane};
use anyhow::{Context, Result, anyhow, bail};
use serde_json::{Value, json};
use std::path::PathBuf;

pub const TOOLS: [&str; 6] = [
    "save_app",
    "list_apps",
    "delete_app",
    "app_data_list",
    "app_data_put",
    "app_data_delete",
];

pub fn tools() -> [Value; 6] {
    [
        json!({
            "name": "save_app",
            "description": "Register or update an interactive web mini-app you built in apps/<slug>/ (HTML/CSS/JS). Creates a live card in the chat and reloads any open viewer tabs.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "slug": {"type": "string", "description": "Folder name under apps/, e.g. split-bill."},
                    "title": {"type": "string", "description": "Display title of the mini-app."},
                    "description": {"type": "string", "description": "Short summary of what the app does."},
                    "entry": {"type": "string", "description": "Entry HTML file relative to apps/<slug>/ (default index.html)."}
                },
                "required": ["slug", "title"]
            }
        }),
        json!({
            "name": "list_apps",
            "description": "List all mini-apps you have built in this workspace with their versions, URLs and collections.",
            "inputSchema": {"type": "object", "properties": {}},
            "annotations": {"readOnlyHint": true}
        }),
        json!({
            "name": "delete_app",
            "description": "Delete a mini-app registration, its share links and its collections.",
            "inputSchema": {
                "type": "object",
                "properties": {"slug": {"type": "string"}},
                "required": ["slug"]
            }
        }),
        json!({
            "name": "app_data_list",
            "description": "Read items from a mini-app's SQLite JSON collection (or omit collection to list collections).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "slug": {"type": "string"},
                    "collection": {"type": "string"},
                    "limit": {"type": "integer"}
                },
                "required": ["slug"]
            },
            "annotations": {"readOnlyHint": true}
        }),
        json!({
            "name": "app_data_put",
            "description": "Insert or update a JSON item in a mini-app's collection and broadcast a live update to open viewers.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "slug": {"type": "string"},
                    "collection": {"type": "string"},
                    "id": {"type": "string"},
                    "value": {"description": "JSON object or value to store."}
                },
                "required": ["slug", "collection", "value"]
            }
        }),
        json!({
            "name": "app_data_delete",
            "description": "Delete one item (by id) or an entire collection from a mini-app.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "slug": {"type": "string"},
                    "collection": {"type": "string"},
                    "id": {"type": "string"}
                },
                "required": ["slug", "collection"]
            }
        }),
    ]
}

fn app_url(hub: &Hub, bot_id: &str, slug: &str, token: &str) -> String {
    format!("http://127.0.0.1:{}/apps/{bot_id}/{slug}/{token}/", hub.port)
}

pub fn call(hub: &Hub, bot_id: &str, name: &str, args: &Value) -> Result<Value> {
    let row = hub.store.bot(bot_id)?.filter(|r| !r.deleted).ok_or_else(|| anyhow!("unknown bot"))?;
    match name {
        "save_app" => {
            let raw_slug = args["slug"].as_str().context("slug is required")?;
            let slug = crate::store::apps::check_slug(raw_slug)?;
            let title = args["title"].as_str().context("title is required")?;
            let description = args["description"].as_str();
            let entry = args["entry"].as_str().unwrap_or("index.html");
            let entry_path = PathBuf::from(&row.config.cwd).join("apps").join(&slug).join(entry);
            if !entry_path.is_file() {
                bail!(
                    "Entry file apps/{slug}/{entry} does not exist yet. Write the HTML file inside apps/{slug}/ first, then call save_app."
                );
            }
            let saved = hub.store.save_bot_app(bot_id, &slug, title, description, Some(entry))?;
            let token = saved.owner_token.clone().unwrap_or_default();
            let url = app_url(hub, bot_id, &saved.slug, &token);
            emit_app_live(hub, bot_id, &saved.slug, json!({"type": "reload", "version": saved.version}));
            let notice = json!({
                "text": format!("App ready: {} (v{})", saved.title, saved.version),
                "botApp": {
                    "slug": saved.slug,
                    "title": saved.title,
                    "description": saved.description,
                    "entry": saved.entry,
                    "version": saved.version,
                    "ownerToken": token,
                    "url": url,
                }
            });
            hub.add_entry(&Lane::main(bot_id), EntryKind::Notice, hub.store.max_turn(bot_id), &notice);
            Ok(json!({
                "app": saved,
                "url": url,
                "helper": "Include _codync.js (auto-injected) to use window.codync.{list,get,put,update,remove,tell,subscribe}."
            }))
        }
        "list_apps" => {
            let apps: Vec<Value> = hub
                .store
                .list_bot_apps(bot_id)?
                .into_iter()
                .map(|app| {
                    let token = app.owner_token.clone().unwrap_or_default();
                    let collections = hub.store.app_collections(bot_id, &app.slug).unwrap_or_default();
                    json!({
                        "slug": app.slug,
                        "title": app.title,
                        "description": app.description,
                        "entry": app.entry,
                        "version": app.version,
                        "updatedAt": app.updated_at,
                        "url": app_url(hub, bot_id, &app.slug, &token),
                        "collections": collections,
                    })
                })
                .collect();
            Ok(json!({"apps": apps}))
        }
        "delete_app" => {
            let slug = crate::store::apps::check_slug(args["slug"].as_str().context("slug is required")?)?;
            let removed = hub.store.delete_bot_app(bot_id, &slug)?;
            Ok(json!({"deleted": removed, "slug": slug}))
        }
        "app_data_list" => {
            let slug = crate::store::apps::check_slug(args["slug"].as_str().context("slug is required")?)?;
            if let Some(col) = args["collection"].as_str().filter(|s| !s.trim().is_empty()) {
                let limit = args["limit"].as_i64().unwrap_or(100);
                let items = hub.store.app_data_list(bot_id, &slug, col, limit)?;
                Ok(json!({"items": items}))
            } else {
                let collections = hub.store.app_collections(bot_id, &slug)?;
                Ok(json!({"collections": collections}))
            }
        }
        "app_data_put" => {
            let slug = crate::store::apps::check_slug(args["slug"].as_str().context("slug is required")?)?;
            let col = args["collection"].as_str().context("collection is required")?;
            let id = args["id"].as_str();
            let val = args.get("value").context("value is required")?;
            let item = hub.store.app_data_put(bot_id, &slug, col, id, val, Some("bot"))?;
            emit_app_live(hub, bot_id, &slug, json!({"type": "change", "collection": col, "id": item.id}));
            Ok(json!({"item": item}))
        }
        "app_data_delete" => {
            let slug = crate::store::apps::check_slug(args["slug"].as_str().context("slug is required")?)?;
            let col = args["collection"].as_str().context("collection is required")?;
            let id = args["id"].as_str();
            let removed = hub.store.app_data_delete(bot_id, &slug, col, id)?;
            emit_app_live(hub, bot_id, &slug, json!({"type": "change", "collection": col, "id": id}));
            Ok(json!({"removed": removed}))
        }
        _ => bail!("unknown app tool: {name}"),
    }
}

/// Host API methods for UI clients (`listBotApps`, `createAppLink`, `revokeAppLink`, `deleteBotApp`).
pub fn api_call(hub: &Hub, method: &str, b: &Value) -> Result<Value> {
    let bot_id = b["botId"].as_str().context("`botId` is required")?;
    match method {
        "listBotApps" => {
            let apps: Vec<Value> = hub
                .store
                .list_bot_apps(bot_id)?
                .into_iter()
                .map(|app| {
                    let token = app.owner_token.clone().unwrap_or_default();
                    let links = hub.store.list_app_links(bot_id, &app.slug).unwrap_or_default();
                    let collections = hub.store.app_collections(bot_id, &app.slug).unwrap_or_default();
                    json!({
                        "slug": app.slug,
                        "botId": app.bot_id,
                        "title": app.title,
                        "description": app.description,
                        "entry": app.entry,
                        "version": app.version,
                        "createdAt": app.created_at,
                        "updatedAt": app.updated_at,
                        "ownerToken": token,
                        "url": app_url(hub, bot_id, &app.slug, &token),
                        "links": links,
                        "collections": collections,
                    })
                })
                .collect();
            Ok(json!({"apps": apps}))
        }
        "createAppLink" => {
            let slug = b["slug"].as_str().context("`slug` is required")?;
            let label = b["label"].as_str().unwrap_or("Shared link");
            let mode = AppLinkMode::parse(b["mode"].as_str().unwrap_or("view"));
            let link = hub.store.create_app_link(bot_id, slug, label, mode)?;
            let url = app_url(hub, bot_id, slug, &link.token);
            Ok(json!({"link": link, "url": url}))
        }
        "revokeAppLink" => {
            let slug = b["slug"].as_str().context("`slug` is required")?;
            let token = b["token"].as_str().context("`token` is required")?;
            let revoked = hub.store.revoke_app_link(bot_id, slug, token)?;
            Ok(json!({"revoked": revoked}))
        }
        "deleteBotApp" => {
            let slug = b["slug"].as_str().context("`slug` is required")?;
            let deleted = hub.store.delete_bot_app(bot_id, slug)?;
            Ok(json!({"deleted": deleted}))
        }
        _ => bail!("unknown app api method: {method}"),
    }
}
