//! Persistence for the component gallery: the cards the playground saved (`component_instances`)
//! and, per bot, which catalogue components it may use (`component_access`).
//!
//! Nothing here stores markup: an instance is a component name, its version and checked props. The
//! catalogue itself lives in `components::catalogue`, so a host update changes the cards every
//! client draws without touching a row.

use super::{Store, now_ms};
use crate::LockExt;
use anyhow::Result;
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One card the playground saved: a catalogue component, its props, and whether the gallery shows it.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ComponentInstance {
    pub id: String,
    /// The catalogue component (`metric`, `table`, …).
    pub name: String,
    /// The catalogue version it was checked against.
    pub version: i64,
    pub title: String,
    pub props: Value,
    /// The data functions the card was saved with, as `{function, limit?}` requests.
    pub data: Value,
    /// The bot that saved it, when a bot did; none for the playground's own drafts.
    pub author: Option<String>,
    /// In the gallery for everyone; a draft is the playground's alone.
    pub published: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

/// One bot's say over one component: a row that isn't there means the bot may use it.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ComponentAccess {
    pub bot: String,
    pub component: String,
    pub published: bool,
    pub updated_at: i64,
}

/// A JSON column, or the row's failure to read as JSON.
fn json_column(text: String, column: usize) -> rusqlite::Result<Value> {
    serde_json::from_str(&text).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(column, rusqlite::types::Type::Text, Box::new(error))
    })
}

pub(super) fn open(c: &rusqlite::Connection) -> Result<()> {
    c.execute_batch(
        "CREATE TABLE IF NOT EXISTS component_instances (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            version INTEGER NOT NULL,
            title TEXT NOT NULL DEFAULT '',
            props TEXT NOT NULL,
            data TEXT NOT NULL DEFAULT '[]',
            author TEXT,
            published INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_component_instances_name ON component_instances(name, published);
        CREATE TABLE IF NOT EXISTS component_access (
            bot_id TEXT NOT NULL,
            component TEXT NOT NULL,
            published INTEGER NOT NULL DEFAULT 1,
            updated_at INTEGER NOT NULL,
            PRIMARY KEY (bot_id, component)
        );",
    )?;
    Ok(())
}

const INSTANCES: &str = "SELECT id, name, version, title, props, data, author, published, created_at, updated_at FROM component_instances ORDER BY created_at DESC";
const INSTANCE: &str = "SELECT id, name, version, title, props, data, author, published, created_at, updated_at FROM component_instances WHERE id = ?1";

/// A true/false column.
fn flag(on: bool) -> i64 {
    if on { 1 } else { 0 }
}

fn instance_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<ComponentInstance> {
    Ok(ComponentInstance {
        id: r.get(0)?,
        name: r.get(1)?,
        version: r.get(2)?,
        title: r.get(3)?,
        props: json_column(r.get(4)?, 4)?,
        data: json_column(r.get(5)?, 5)?,
        author: r.get(6)?,
        published: r.get::<_, i64>(7)? != 0,
        created_at: r.get(8)?,
        updated_at: r.get(9)?,
    })
}

impl Store {
    /// Every saved card, newest first, drafts included: the gallery keeps the published ones and the
    /// playground shows the rest.
    pub fn component_instances(&self) -> Result<Vec<ComponentInstance>> {
        let c = self.db.locked();
        let mut st = c.prepare(INSTANCES)?;
        let rows = st.query_map([], |r| instance_row(r))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn component_instance(&self, id: &str) -> Result<Option<ComponentInstance>> {
        let c = self.db.locked();
        let row = c.query_row(INSTANCE, [id], |r| instance_row(r)).optional()?;
        Ok(row)
    }

    /// Saves a card, stamped: a new one keeps its `created_at`, an update moves `updated_at`.
    pub fn save_component_instance(&self, instance: &mut ComponentInstance) -> Result<()> {
        let now = now_ms();
        instance.updated_at = now;
        if instance.created_at == 0 {
            instance.created_at = now;
        }
        let c = self.db.locked();
        c.execute(
            "INSERT INTO component_instances(id, name, version, title, props, data, author, published, created_at, updated_at)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(id) DO UPDATE SET name = ?2, version = ?3, title = ?4, props = ?5, data = ?6,
                                           author = ?7, published = ?8, updated_at = ?10",
            params![
                instance.id,
                instance.name,
                instance.version,
                instance.title,
                instance.props.to_string(),
                instance.data.to_string(),
                instance.author,
                flag(instance.published),
                instance.created_at,
                instance.updated_at,
            ],
        )?;
        Ok(())
    }

    /// Shows or hides a saved card in the gallery.
    pub fn set_component_published(&self, id: &str, published: bool) -> Result<()> {
        let c = self.db.locked();
        let changed = c.execute(
            "UPDATE component_instances SET published = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, flag(published), now_ms()],
        )?;
        if changed == 0 {
            anyhow::bail!("unknown component instance");
        }
        Ok(())
    }

    pub fn delete_component_instance(&self, id: &str) -> Result<()> {
        let c = self.db.locked();
        c.execute("DELETE FROM component_instances WHERE id = ?1", [id])?;
        Ok(())
    }

    /// Every bot's say over every component, as saved.
    pub fn component_access(&self) -> Result<Vec<ComponentAccess>> {
        let c = self.db.locked();
        let mut st = c.prepare(
            "SELECT bot_id, component, published, updated_at FROM component_access ORDER BY component, bot_id",
        )?;
        let rows = st.query_map([], |r| {
            Ok(ComponentAccess {
                bot: r.get(0)?,
                component: r.get(1)?,
                published: r.get::<_, i64>(2)? != 0,
                updated_at: r.get(3)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Lets a bot use a component, or withholds it. Withholding keeps the row so the gallery can show
    /// who is left out; a bot with no row may use every component.
    pub fn set_component_access(&self, bot: &str, component: &str, published: bool) -> Result<()> {
        let c = self.db.locked();
        c.execute(
            "INSERT INTO component_access(bot_id, component, published, updated_at) VALUES(?1, ?2, ?3, ?4)
             ON CONFLICT(bot_id, component) DO UPDATE SET published = ?3, updated_at = ?4",
            params![bot, component, flag(published), now_ms()],
        )?;
        Ok(())
    }

    /// Whether a bot may show a component: withheld only when a saved row says so.
    pub fn component_available(&self, bot: &str, component: &str) -> Result<bool> {
        let c = self.db.locked();
        let saved: Option<i64> = c
            .query_row(
                "SELECT published FROM component_access WHERE bot_id = ?1 AND component = ?2",
                params![bot, component],
                |r| r.get(0),
            )
            .optional()?;
        Ok(saved.unwrap_or(1) != 0)
    }
}
