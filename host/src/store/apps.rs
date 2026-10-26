//! Persistence for bot-built mini-apps (`bot_apps`), capability share links (`bot_app_links`),
//! and per-app JSON collections (`bot_app_data`).

use super::{Store, now_ms};
use crate::LockExt;
use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const MAX_ITEM_BYTES: usize = 64 * 1024;
const MAX_ITEMS_PER_COLLECTION: i64 = 2000;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BotApp {
    pub slug: String,
    pub bot_id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub entry: String,
    pub version: i64,
    pub created_at: i64,
    pub updated_at: i64,
    /// Owner capability token for viewing/editing inside Codync.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_token: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AppLinkMode {
    #[default]
    View,
    Edit,
}

impl AppLinkMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::View => "view",
            Self::Edit => "edit",
        }
    }

    pub fn parse(raw: &str) -> Self {
        if raw.eq_ignore_ascii_case("edit") { Self::Edit } else { Self::View }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppLink {
    pub token: String,
    pub bot_id: String,
    pub slug: String,
    pub label: String,
    pub mode: AppLinkMode,
    pub created_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_opened_at: Option<i64>,
    pub opens: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppDataItem {
    pub id: String,
    pub value: Value,
    pub updated_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_by: Option<String>,
}

pub(super) fn open(c: &Connection) -> Result<()> {
    c.execute_batch(
        "CREATE TABLE IF NOT EXISTS bot_apps (
            bot_id TEXT NOT NULL,
            slug TEXT NOT NULL,
            title TEXT NOT NULL,
            description TEXT,
            entry TEXT NOT NULL DEFAULT 'index.html',
            version INTEGER NOT NULL DEFAULT 1,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            PRIMARY KEY (bot_id, slug)
        );
        CREATE TABLE IF NOT EXISTS bot_app_links (
            token TEXT PRIMARY KEY,
            bot_id TEXT NOT NULL,
            slug TEXT NOT NULL,
            label TEXT NOT NULL,
            mode TEXT NOT NULL DEFAULT 'view',
            is_owner INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL,
            last_opened_at INTEGER,
            opens INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_bot_app_links_app ON bot_app_links(bot_id, slug);
        CREATE TABLE IF NOT EXISTS bot_app_data (
            bot_id TEXT NOT NULL,
            slug TEXT NOT NULL,
            collection TEXT NOT NULL,
            id TEXT NOT NULL,
            value TEXT NOT NULL,
            updated_at INTEGER NOT NULL,
            updated_by TEXT,
            PRIMARY KEY (bot_id, slug, collection, id)
        );
        CREATE INDEX IF NOT EXISTS idx_bot_app_data_col ON bot_app_data(bot_id, slug, collection, updated_at DESC);",
    )?;
    Ok(())
}

pub fn check_slug(raw: &str) -> Result<String> {
    let s: String = raw
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect();
    let s = s.trim_matches('-').to_owned();
    if s.is_empty() || s.len() > 64 {
        bail!("app slug must be 1–64 letters, digits, hyphens or underscores");
    }
    Ok(s)
}

pub fn check_collection(raw: &str) -> Result<String> {
    let s: String = raw
        .trim()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(48)
        .collect();
    if s.is_empty() {
        bail!("collection name is required");
    }
    Ok(s)
}

fn new_token() -> String {
    format!("tok_{}", uuid::Uuid::new_v4().simple())
}

fn ensure_owner_link(c: &Connection, bot_id: &str, slug: &str) -> Result<String> {
    if let Some(tok) = c
        .query_row(
            "SELECT token FROM bot_app_links WHERE bot_id = ?1 AND slug = ?2 AND is_owner = 1 LIMIT 1",
            params![bot_id, slug],
            |r| r.get::<_, String>(0),
        )
        .optional()?
    {
        return Ok(tok);
    }
    let token = new_token();
    let now = now_ms();
    c.execute(
        "INSERT INTO bot_app_links(token, bot_id, slug, label, mode, is_owner, created_at, opens)
         VALUES(?1, ?2, ?3, 'Owner', 'edit', 1, ?4, 0)",
        params![token, bot_id, slug, now],
    )?;
    Ok(token)
}

impl Store {
    pub fn list_bot_apps(&self, bot_id: &str) -> Result<Vec<BotApp>> {
        let c = self.db.locked();
        let mut st = c.prepare(
            "SELECT slug, bot_id, title, description, entry, version, created_at, updated_at
             FROM bot_apps WHERE bot_id = ?1 ORDER BY updated_at DESC",
        )?;
        let rows: Vec<BotApp> = st
            .query_map([bot_id], |r| {
                Ok(BotApp {
                    slug: r.get(0)?,
                    bot_id: r.get(1)?,
                    title: r.get(2)?,
                    description: r.get(3)?,
                    entry: r.get(4)?,
                    version: r.get(5)?,
                    created_at: r.get(6)?,
                    updated_at: r.get(7)?,
                    owner_token: None,
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        let mut out = Vec::with_capacity(rows.len());
        for mut app in rows {
            app.owner_token = Some(ensure_owner_link(&c, &app.bot_id, &app.slug)?);
            out.push(app);
        }
        Ok(out)
    }

    pub fn get_bot_app(&self, bot_id: &str, slug: &str) -> Result<Option<BotApp>> {
        let c = self.db.locked();
        let app = c
            .query_row(
                "SELECT slug, bot_id, title, description, entry, version, created_at, updated_at
                 FROM bot_apps WHERE bot_id = ?1 AND slug = ?2",
                params![bot_id, slug],
                |r| {
                    Ok(BotApp {
                        slug: r.get(0)?,
                        bot_id: r.get(1)?,
                        title: r.get(2)?,
                        description: r.get(3)?,
                        entry: r.get(4)?,
                        version: r.get(5)?,
                        created_at: r.get(6)?,
                        updated_at: r.get(7)?,
                        owner_token: None,
                    })
                },
            )
            .optional()?;
        match app {
            Some(mut a) => {
                a.owner_token = Some(ensure_owner_link(&c, &a.bot_id, &a.slug)?);
                Ok(Some(a))
            }
            None => Ok(None),
        }
    }

    pub fn save_bot_app(
        &self,
        bot_id: &str,
        slug: &str,
        title: &str,
        description: Option<&str>,
        entry: Option<&str>,
    ) -> Result<BotApp> {
        let slug = check_slug(slug)?;
        let title = title.trim();
        if title.is_empty() {
            bail!("app title is required");
        }
        let entry = entry.unwrap_or("index.html").trim().trim_start_matches('/');
        if entry.is_empty() || entry.contains("..") {
            bail!("invalid entry path");
        }
        let now = now_ms();
        {
            let c = self.db.locked();
            c.execute(
                "INSERT INTO bot_apps(bot_id, slug, title, description, entry, version, created_at, updated_at)
                 VALUES(?1, ?2, ?3, ?4, ?5, 1, ?6, ?6)
                 ON CONFLICT(bot_id, slug) DO UPDATE SET
                   title = excluded.title,
                   description = COALESCE(excluded.description, bot_apps.description),
                   entry = excluded.entry,
                   version = bot_apps.version + 1,
                   updated_at = excluded.updated_at",
                params![bot_id, slug, title, description, entry, now],
            )?;
            let _ = ensure_owner_link(&c, bot_id, &slug)?;
        }
        self.get_bot_app(bot_id, &slug)?.context("saved app missing")
    }

    pub fn delete_bot_app(&self, bot_id: &str, slug: &str) -> Result<bool> {
        let c = self.db.locked();
        let removed = c.execute("DELETE FROM bot_apps WHERE bot_id = ?1 AND slug = ?2", params![bot_id, slug])?;
        c.execute("DELETE FROM bot_app_links WHERE bot_id = ?1 AND slug = ?2", params![bot_id, slug])?;
        c.execute("DELETE FROM bot_app_data WHERE bot_id = ?1 AND slug = ?2", params![bot_id, slug])?;
        Ok(removed > 0)
    }

    pub fn list_app_links(&self, bot_id: &str, slug: &str) -> Result<Vec<AppLink>> {
        let c = self.db.locked();
        let mut st = c.prepare(
            "SELECT token, bot_id, slug, label, mode, created_at, last_opened_at, opens
             FROM bot_app_links WHERE bot_id = ?1 AND slug = ?2 AND is_owner = 0
             ORDER BY created_at DESC",
        )?;
        Ok(st
            .query_map(params![bot_id, slug], |r| {
                Ok(AppLink {
                    token: r.get(0)?,
                    bot_id: r.get(1)?,
                    slug: r.get(2)?,
                    label: r.get(3)?,
                    mode: AppLinkMode::parse(&r.get::<_, String>(4)?),
                    created_at: r.get(5)?,
                    last_opened_at: r.get(6)?,
                    opens: r.get(7)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?)
    }

    pub fn create_app_link(&self, bot_id: &str, slug: &str, label: &str, mode: AppLinkMode) -> Result<AppLink> {
        let c = self.db.locked();
        let token = new_token();
        let now = now_ms();
        let label = if label.trim().is_empty() { "Shared link" } else { label.trim() };
        c.execute(
            "INSERT INTO bot_app_links(token, bot_id, slug, label, mode, is_owner, created_at, opens)
             VALUES(?1, ?2, ?3, ?4, ?5, 0, ?6, 0)",
            params![token, bot_id, slug, label, mode.as_str(), now],
        )?;
        Ok(AppLink {
            token,
            bot_id: bot_id.to_owned(),
            slug: slug.to_owned(),
            label: label.to_owned(),
            mode,
            created_at: now,
            last_opened_at: None,
            opens: 0,
        })
    }

    pub fn revoke_app_link(&self, bot_id: &str, slug: &str, token: &str) -> Result<bool> {
        let c = self.db.locked();
        let n = c.execute(
            "DELETE FROM bot_app_links WHERE bot_id = ?1 AND slug = ?2 AND token = ?3 AND is_owner = 0",
            params![bot_id, slug, token],
        )?;
        Ok(n > 0)
    }

    pub fn resolve_app_link(&self, bot_id: &str, slug: &str, token: &str, touch: bool) -> Result<Option<AppLink>> {
        let c = self.db.locked();
        let link = c
            .query_row(
                "SELECT token, bot_id, slug, label, mode, created_at, last_opened_at, opens
                 FROM bot_app_links WHERE bot_id = ?1 AND slug = ?2 AND token = ?3",
                params![bot_id, slug, token],
                |r| {
                    Ok(AppLink {
                        token: r.get(0)?,
                        bot_id: r.get(1)?,
                        slug: r.get(2)?,
                        label: r.get(3)?,
                        mode: AppLinkMode::parse(&r.get::<_, String>(4)?),
                        created_at: r.get(5)?,
                        last_opened_at: r.get(6)?,
                        opens: r.get(7)?,
                    })
                },
            )
            .optional()?;
        if touch && link.is_some() {
            let now = now_ms();
            let _ = c.execute(
                "UPDATE bot_app_links SET opens = opens + 1, last_opened_at = ?2 WHERE token = ?1",
                params![token, now],
            );
        }
        Ok(link)
    }

    pub fn app_collections(&self, bot_id: &str, slug: &str) -> Result<Vec<Value>> {
        let c = self.db.locked();
        let mut st = c.prepare(
            "SELECT collection, COUNT(*) AS count, MAX(updated_at) AS updated_at
             FROM bot_app_data WHERE bot_id = ?1 AND slug = ?2
             GROUP BY collection ORDER BY collection",
        )?;
        Ok(st
            .query_map(params![bot_id, slug], |r| {
                Ok(json!({
                    "collection": r.get::<_, String>(0)?,
                    "count": r.get::<_, i64>(1)?,
                    "updatedAt": r.get::<_, i64>(2)?,
                }))
            })?
            .collect::<rusqlite::Result<_>>()?)
    }

    pub fn app_data_list(&self, bot_id: &str, slug: &str, collection: &str, limit: i64) -> Result<Vec<AppDataItem>> {
        let col = check_collection(collection)?;
        let limit = limit.clamp(1, 500);
        let c = self.db.locked();
        let mut st = c.prepare(
            "SELECT id, value, updated_at, updated_by
             FROM bot_app_data WHERE bot_id = ?1 AND slug = ?2 AND collection = ?3
             ORDER BY updated_at DESC LIMIT ?4",
        )?;
        Ok(st
            .query_map(params![bot_id, slug, col, limit], |r| {
                let raw: String = r.get(1)?;
                Ok(AppDataItem {
                    id: r.get(0)?,
                    value: serde_json::from_str(&raw).unwrap_or(Value::Null),
                    updated_at: r.get(2)?,
                    updated_by: r.get(3)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?)
    }

    pub fn app_data_get(&self, bot_id: &str, slug: &str, collection: &str, id: &str) -> Result<Option<AppDataItem>> {
        let col = check_collection(collection)?;
        let c = self.db.locked();
        Ok(c.query_row(
            "SELECT id, value, updated_at, updated_by
             FROM bot_app_data WHERE bot_id = ?1 AND slug = ?2 AND collection = ?3 AND id = ?4",
            params![bot_id, slug, col, id],
            |r| {
                let raw: String = r.get(1)?;
                Ok(AppDataItem {
                    id: r.get(0)?,
                    value: serde_json::from_str(&raw).unwrap_or(Value::Null),
                    updated_at: r.get(2)?,
                    updated_by: r.get(3)?,
                })
            },
        )
        .optional()?)
    }

    pub fn app_data_put(
        &self,
        bot_id: &str,
        slug: &str,
        collection: &str,
        id: Option<&str>,
        value: &Value,
        by: Option<&str>,
    ) -> Result<AppDataItem> {
        let col = check_collection(collection)?;
        let id = id
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map_or_else(|| format!("it_{}", uuid::Uuid::new_v4().simple()), str::to_owned);
        let encoded = serde_json::to_string(value)?;
        if encoded.len() > MAX_ITEM_BYTES {
            bail!("item exceeds 64 KB limit");
        }
        let now = now_ms();
        let c = self.db.locked();
        let count: i64 = c.query_row(
            "SELECT COUNT(*) FROM bot_app_data WHERE bot_id = ?1 AND slug = ?2 AND collection = ?3",
            params![bot_id, slug, col],
            |r| r.get(0),
        )?;
        if count >= MAX_ITEMS_PER_COLLECTION {
            bail!("collection `{col}` reached {MAX_ITEMS_PER_COLLECTION} items");
        }
        c.execute(
            "INSERT INTO bot_app_data(bot_id, slug, collection, id, value, updated_at, updated_by)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(bot_id, slug, collection, id) DO UPDATE SET
               value = excluded.value,
               updated_at = excluded.updated_at,
               updated_by = excluded.updated_by",
            params![bot_id, slug, col, id, encoded, now, by],
        )?;
        Ok(AppDataItem { id, value: value.clone(), updated_at: now, updated_by: by.map(str::to_owned) })
    }

    pub fn app_data_delete(&self, bot_id: &str, slug: &str, collection: &str, id: Option<&str>) -> Result<usize> {
        let col = check_collection(collection)?;
        let c = self.db.locked();
        let removed = match id {
            Some(id) => c.execute(
                "DELETE FROM bot_app_data WHERE bot_id = ?1 AND slug = ?2 AND collection = ?3 AND id = ?4",
                params![bot_id, slug, col, id],
            )?,
            None => c.execute(
                "DELETE FROM bot_app_data WHERE bot_id = ?1 AND slug = ?2 AND collection = ?3",
                params![bot_id, slug, col],
            )?,
        };
        Ok(removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::tests::temp_store;

    #[test]
    fn bot_apps_links_and_collections_round_trip() {
        let s = temp_store();
        let app1 = s.save_bot_app("bot1", "Split Bill!", "Split Bill", Some("Split expenses"), None).unwrap();
        assert_eq!(app1.slug, "split-bill");
        assert_eq!(app1.version, 1);
        assert!(app1.owner_token.as_deref().unwrap_or_default().starts_with("tok_"));

        let app2 = s.save_bot_app("bot1", "split-bill", "Split Bill v2", None, None).unwrap();
        assert_eq!(app2.version, 2);
        assert_eq!(app2.description.as_deref(), Some("Split expenses"));

        let link = s.create_app_link("bot1", "split-bill", "Friends", AppLinkMode::View).unwrap();
        assert_eq!(s.list_app_links("bot1", "split-bill").unwrap().len(), 1);
        let resolved = s.resolve_app_link("bot1", "split-bill", &link.token, true).unwrap().unwrap();
        assert_eq!(resolved.mode, AppLinkMode::View);

        let item = s
            .app_data_put("bot1", "split-bill", "expenses", Some("e1"), &json!({"amount": 42}), Some("Owner"))
            .unwrap();
        assert_eq!(item.id, "e1");
        assert_eq!(s.app_data_list("bot1", "split-bill", "expenses", 10).unwrap().len(), 1);
        assert_eq!(s.app_collections("bot1", "split-bill").unwrap().len(), 1);
        assert_eq!(s.app_data_delete("bot1", "split-bill", "expenses", Some("e1")).unwrap(), 1);
        assert!(s.delete_bot_app("bot1", "split-bill").unwrap());
        assert!(s.list_bot_apps("bot1").unwrap().is_empty());
    }
}
