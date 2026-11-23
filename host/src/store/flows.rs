//! Routine flows: the links that hand a routine's reply from bot to bot, the node positions of
//! each bot's canvas and the bots placed on it (`routines::flow` holds the rules, `flow_steps`
//! the steps of each run).
//!
//! Routines live in the kv blob (`routines.v1`), not in a table, so the cascade on deleting a
//! routine or a bot is explicit and runs in one transaction. Every change stamps a global `rev`
//! and records it for each bot whose canvas it touches (kv `routineflow.rev.<bot>`), which is
//! what a client's catch-up compares against.

use super::{Store, next_rev};
use anyhow::{Context, Result, bail};
use rusqlite::{Connection, Transaction, params};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

/// The coordinate limit of a node; far beyond any canvas a person would build.
pub const MAX_COORDINATE: f64 = 1_000_000.0;
/// kv prefix of the per-bot canvas revision.
pub const REV_PREFIX: &str = "routineflow.rev.";

/// The caps a merged document's shapes must respect, mirroring the document's own caps. Each write
/// is checked again here so a store call can never grow a table past what a canvas can hold.
const MERGE_LINKS_CAP: usize = 1000;
const MERGE_NODES_CAP: usize = 1000;
const MERGE_MEDIA_CAP: usize = 500;
const MERGE_INSTRUCTION_CAP: usize = 8_000;
/// The media columns, in the order `row_media` of `flow_media` reads them.
const MEDIA_COLUMNS: &str = "id, kind, model, prompt, aspect, duration, resolution, transparent, voice, created_at, routine_id, owner_bot_id, name, batch";

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowLink {
    pub id: String,
    pub routine_id: String,
    pub from_bot_id: String,
    pub to_bot_id: String,
    /// A name the person gave this step, shown on the card and to agents reading the flow. Empty
    /// means "no name": a client falls back to the step's type. Old flows have no column value.
    #[serde(default)]
    pub name: String,
    pub instruction: String,
    /// The model this step runs on, when the link says one instead of the target bot's own.
    pub model: Option<String>,
    /// The model's options (effort…) that go with `model`, or on their own.
    pub model_options: BTreeMap<String, String>,
    pub created_at: i64,
    /// The bot that owns the routine, whose run starts the flow.
    #[serde(skip)]
    pub owner_bot_id: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowPosition {
    pub node_key: String,
    pub x: f64,
    pub y: f64,
}

/// What a mutation touched: the rev it stamped and the bots whose canvas changed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FlowChange {
    pub rev: i64,
    pub bots: Vec<String>,
}

pub(super) fn open(c: &Connection) -> Result<()> {
    c.execute_batch(
        "CREATE TABLE IF NOT EXISTS routine_flow_links(
            id TEXT PRIMARY KEY, routine_id TEXT NOT NULL, owner_bot_id TEXT NOT NULL,
            from_bot_id TEXT NOT NULL, to_bot_id TEXT NOT NULL, instruction TEXT NOT NULL DEFAULT '',
            model TEXT, model_options TEXT NOT NULL DEFAULT '{}',
            name TEXT NOT NULL DEFAULT '',
            created_at INTEGER NOT NULL, UNIQUE(routine_id, from_bot_id, to_bot_id));
         CREATE INDEX IF NOT EXISTS routine_flow_links_routine ON routine_flow_links(routine_id);
         CREATE INDEX IF NOT EXISTS routine_flow_links_owner ON routine_flow_links(owner_bot_id);
         CREATE INDEX IF NOT EXISTS routine_flow_links_to ON routine_flow_links(to_bot_id);
         CREATE TABLE IF NOT EXISTS routine_flow_steps(
            id TEXT PRIMARY KEY, run_id TEXT NOT NULL, routine_id TEXT NOT NULL, owner_bot_id TEXT NOT NULL,
            bot_id TEXT NOT NULL, delivery_id TEXT, input TEXT NOT NULL, output TEXT,
            status TEXT NOT NULL CHECK(status IN ('running','succeeded','failed','skipped','cancelled')),
            error TEXT, files TEXT, settings TEXT, run_at INTEGER NOT NULL, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL,
            UNIQUE(run_id, bot_id));
         CREATE INDEX IF NOT EXISTS routine_flow_steps_routine ON routine_flow_steps(routine_id, run_at DESC);
         CREATE INDEX IF NOT EXISTS routine_flow_steps_status ON routine_flow_steps(status);
         CREATE INDEX IF NOT EXISTS routine_flow_steps_bot ON routine_flow_steps(bot_id);
         CREATE TABLE IF NOT EXISTS routine_flow_positions(
            canvas_bot_id TEXT NOT NULL, node_key TEXT NOT NULL, x REAL NOT NULL, y REAL NOT NULL,
            PRIMARY KEY(canvas_bot_id, node_key));
         CREATE INDEX IF NOT EXISTS routine_flow_positions_node ON routine_flow_positions(node_key);
         CREATE TABLE IF NOT EXISTS routine_flow_placed(
            canvas_bot_id TEXT NOT NULL, bot_id TEXT NOT NULL, PRIMARY KEY(canvas_bot_id, bot_id));
         CREATE INDEX IF NOT EXISTS routine_flow_placed_bot ON routine_flow_placed(bot_id);
         CREATE TABLE IF NOT EXISTS routine_flow_media(
            id TEXT PRIMARY KEY, kind TEXT NOT NULL CHECK(kind IN ('image','video')),
            model TEXT NOT NULL, prompt TEXT NOT NULL DEFAULT '', aspect TEXT NOT NULL DEFAULT '1:1',
            duration INTEGER NOT NULL DEFAULT 6, resolution TEXT NOT NULL DEFAULT '480p',
            transparent INTEGER NOT NULL DEFAULT 0, voice TEXT, created_at INTEGER NOT NULL,
            routine_id TEXT NOT NULL, owner_bot_id TEXT NOT NULL, name TEXT NOT NULL DEFAULT '',
            batch INTEGER NOT NULL DEFAULT 1);
         CREATE INDEX IF NOT EXISTS routine_flow_media_routine ON routine_flow_media(routine_id);",
    )?;
    // A database built before a step kept its settings has the table but not the column. `open` runs
    // on every start, so this keeps older databases readable without a schema version bump here.
    let has_settings: bool = c.query_row(
        "SELECT count(*) > 0 FROM pragma_table_info('routine_flow_steps') WHERE name = 'settings'",
        [],
        |row| row.get(0),
    )?;
    if !has_settings {
        c.execute_batch("ALTER TABLE routine_flow_steps ADD COLUMN settings TEXT;")?;
    }
    // Likewise, a step's name was added after links and media steps already had tables.
    for table in ["routine_flow_links", "routine_flow_media"] {
        let has_name: bool = c.query_row(
            &format!("SELECT count(*) > 0 FROM pragma_table_info('{table}') WHERE name = 'name'"),
            [],
            |row| row.get(0),
        )?;
        if !has_name {
            c.execute_batch(&format!("ALTER TABLE {table} ADD COLUMN name TEXT NOT NULL DEFAULT '';"))?;
        }
    }
    // A media step added after batches existed still keeps its old database readable.
    let has_batch: bool = c.query_row(
        "SELECT count(*) > 0 FROM pragma_table_info('routine_flow_media') WHERE name = 'batch'",
        [],
        |row| row.get(0),
    )?;
    if !has_batch {
        c.execute_batch("ALTER TABLE routine_flow_media ADD COLUMN batch INTEGER NOT NULL DEFAULT 1;")?;
    }
    Ok(())
}

fn row_link(r: &rusqlite::Row) -> rusqlite::Result<FlowLink> {
    Ok(FlowLink {
        id: r.get(0)?,
        routine_id: r.get(1)?,
        owner_bot_id: r.get(2)?,
        from_bot_id: r.get(3)?,
        to_bot_id: r.get(4)?,
        instruction: r.get(5)?,
        model: r.get(6)?,
        model_options: serde_json::from_str(r.get::<_, String>(7)?.as_str()).unwrap_or_default(),
        created_at: r.get(8)?,
        name: r.get(9)?,
    })
}

const LINK_COLUMNS: &str =
    "id, routine_id, owner_bot_id, from_bot_id, to_bot_id, instruction, model, model_options, created_at, name";

fn options_json(options: &BTreeMap<String, String>) -> String {
    serde_json::to_string(options).unwrap_or_else(|_| "{}".into())
}

fn links_of(c: &Connection, routine_id: &str) -> Result<Vec<FlowLink>> {
    let mut st = c.prepare(&format!(
        "SELECT {LINK_COLUMNS} FROM routine_flow_links WHERE routine_id = ?1 ORDER BY created_at, id"
    ))?;
    Ok(st.query_map([routine_id], row_link)?.collect::<rusqlite::Result<_>>()?)
}

/// Stamps one rev for `bots` and records it as each one's canvas revision.
pub(super) fn touch(tx: &Transaction, bots: impl IntoIterator<Item = String>) -> Result<Option<FlowChange>> {
    touch_at(tx, next_rev(tx)?, bots)
}

/// Records an already-stamped `rev` (the one a merged document's groups were saved at) on every bot
/// whose canvas shows the result, so groups and flow tables share one rev.
fn touch_at(tx: &Transaction, rev: i64, bots: impl IntoIterator<Item = String>) -> Result<Option<FlowChange>> {
    let bots: Vec<String> = bots.into_iter().collect::<BTreeSet<_>>().into_iter().collect();
    if bots.is_empty() {
        return Ok(None);
    }
    for bot in &bots {
        tx.execute(
            "INSERT INTO kv(k, v) VALUES(?1, ?2) ON CONFLICT(k) DO UPDATE SET v = ?2",
            params![format!("{REV_PREFIX}{bot}"), rev.to_string()],
        )?;
    }
    Ok(Some(FlowChange { rev, bots }))
}

/// The owner and every bot a link of `routine_id` names: whoever's canvas shows the routine. Media
/// node keys are left out, since only bots hold a canvas.
pub(super) fn audience(c: &Connection, routine_id: &str, owner: &str) -> Result<BTreeSet<String>> {
    let mut bots = BTreeSet::from([owner.to_owned()]);
    for link in links_of(c, routine_id)? {
        for end in [link.from_bot_id, link.to_bot_id] {
            if !end.starts_with(super::flow_media::PREFIX) {
                bots.insert(end);
            }
        }
    }
    Ok(bots)
}

/// The most of a step's name that is kept. A name is a label, not a description.
pub const MAX_NAME_CHARS: usize = 200;

pub(crate) fn check_name(name: &str) -> Result<()> {
    if name.chars().count() > MAX_NAME_CHARS || name.chars().any(char::is_control) {
        bail!("a step name must be at most {MAX_NAME_CHARS} characters");
    }
    Ok(())
}

pub(crate) fn check_node_key(key: &str) -> Result<()> {
    let ok = key
        .split_once(':')
        .is_some_and(|(kind, id)| matches!(kind, "routine" | "bot" | "media") && !id.is_empty() && !id.contains(':'));
    if !ok || key.len() > 200 {
        bail!("nodeKey must be routine:<id>, bot:<id> or media:<id>");
    }
    Ok(())
}

pub(crate) fn check_coordinate(v: f64) -> Result<()> {
    if !v.is_finite() || v.abs() > MAX_COORDINATE {
        bail!("coordinates must be numbers within ±{MAX_COORDINATE}");
    }
    Ok(())
}

impl Store {
    pub fn flow_links(&self, routine_id: &str) -> Result<Vec<FlowLink>> {
        self.with_connection(|c| links_of(c, routine_id))
    }

    pub fn flow_link(&self, id: &str) -> Result<Option<FlowLink>> {
        self.with_connection(|c| {
            let mut st = c.prepare(&format!("SELECT {LINK_COLUMNS} FROM routine_flow_links WHERE id = ?1"))?;
            Ok(st.query_map([id], row_link)?.next().transpose()?)
        })
    }

    /// The settings and inputs a step actually ran with. Runs saved before steps kept them have no
    /// row here, which a client reads as "no saved settings" rather than an error.
    pub fn flow_step_settings(&self, step_id: &str) -> Result<Option<serde_json::Value>> {
        self.with_connection(|c| {
            let raw: Option<String> =
                c.query_row("SELECT settings FROM routine_flow_steps WHERE id = ?1", [step_id], |row| row.get(0))?;
            Ok(raw.and_then(|raw| serde_json::from_str(&raw).ok()))
        })
    }

    /// Saves what a step ran with. A null (older run, or no settings) clears the column.
    pub fn flow_set_step_settings(&self, step_id: &str, settings: Option<serde_json::Value>) -> Result<()> {
        let value = settings.map(|value| value.to_string());
        self.with_connection(|c| {
            c.execute("UPDATE routine_flow_steps SET settings = ?2 WHERE id = ?1", params![step_id, value])?;
            Ok(())
        })
    }

    /// Saves a media step's progress and stamps the canvas rev, so a client can show which results
    /// of a batch are done while the others are still being made.
    pub fn flow_progress_step(&self, step_id: &str, settings: &serde_json::Value) -> Result<Option<FlowChange>> {
        let value = settings.to_string();
        self.with_connection(|c| {
            let tx = c.unchecked_transaction()?;
            let (routine, owner): (String, String) = tx.query_row(
                "SELECT routine_id, owner_bot_id FROM routine_flow_steps WHERE id = ?1",
                [step_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;
            tx.execute("UPDATE routine_flow_steps SET settings = ?2 WHERE id = ?1", params![step_id, value])?;
            let change = touch(&tx, audience(&tx, &routine, &owner)?)?;
            tx.commit()?;
            Ok(change)
        })
    }

    /// The ids of the routines a bot's canvas shows besides its own: those a link of which names it.
    pub fn flow_routines_touching(&self, bot: &str) -> Result<Vec<String>> {
        self.with_connection(|c| {
            let mut st = c.prepare(
                "SELECT DISTINCT routine_id FROM routine_flow_links
                 WHERE owner_bot_id = ?1 OR from_bot_id = ?1 OR to_bot_id = ?1 ORDER BY routine_id",
            )?;
            Ok(st.query_map([bot], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?)
        })
    }

    pub fn flow_links_for(&self, routine_ids: &[String]) -> Result<Vec<FlowLink>> {
        let mut all = vec![];
        for id in routine_ids {
            all.extend(self.flow_links(id)?);
        }
        Ok(all)
    }

    /// Adds a link after `validate` accepts it against the routine's current links, in one
    /// transaction so two connects can't together make a cycle.
    pub fn flow_add_link(
        &self,
        link: &FlowLink,
        validate: impl FnOnce(&[FlowLink]) -> Result<()>,
    ) -> Result<FlowChange> {
        self.with_connection(|c| {
            let tx = c.unchecked_transaction()?;
            validate(&links_of(&tx, &link.routine_id)?)?;
            tx.execute(
                &format!(
                    "INSERT INTO routine_flow_links({LINK_COLUMNS}) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"
                ),
                params![
                    link.id,
                    link.routine_id,
                    link.owner_bot_id,
                    link.from_bot_id,
                    link.to_bot_id,
                    link.instruction,
                    link.model,
                    options_json(&link.model_options),
                    link.created_at,
                    link.name
                ],
            )?;
            let bots = audience(&tx, &link.routine_id, &link.owner_bot_id)?;
            let change = touch(&tx, bots)?.expect("the owner is always in the audience");
            tx.commit()?;
            Ok(change)
        })
    }

    /// Rewrites a link's instruction, model and model options (an empty `model` clears it).
    pub fn flow_update_link(
        &self,
        id: &str,
        instruction: &str,
        model: Option<&str>,
        model_options: &BTreeMap<String, String>,
    ) -> Result<Option<(FlowLink, FlowChange)>> {
        self.with_connection(|c| {
            let tx = c.unchecked_transaction()?;
            let gone = tx.execute(
                "UPDATE routine_flow_links SET instruction = ?2, model = ?3, model_options = ?4 WHERE id = ?1",
                params![id, instruction, model, options_json(model_options)],
            )?;
            if gone == 0 {
                return Ok(None);
            }
            let mut st = tx.prepare(&format!("SELECT {LINK_COLUMNS} FROM routine_flow_links WHERE id = ?1"))?;
            let link = st.query_row([id], row_link)?;
            drop(st);
            let bots = audience(&tx, &link.routine_id, &link.owner_bot_id)?;
            let change = touch(&tx, bots)?.expect("the owner is always in the audience");
            tx.commit()?;
            Ok(Some((link, change)))
        })
    }

    /// Gives the step this link reaches a name; an empty one clears it. The instruction and model
    /// the step runs with are left alone, and old links read as unnamed.
    pub fn flow_rename_link(&self, id: &str, name: &str) -> Result<Option<FlowChange>> {
        check_name(name)?;
        self.with_connection(|c| {
            let tx = c.unchecked_transaction()?;
            let gone = tx.execute("UPDATE routine_flow_links SET name = ?2 WHERE id = ?1", params![id, name])?;
            if gone == 0 {
                return Ok(None);
            }
            let mut st = tx.prepare(&format!("SELECT {LINK_COLUMNS} FROM routine_flow_links WHERE id = ?1"))?;
            let link = st.query_row([id], row_link)?;
            drop(st);
            let bots = audience(&tx, &link.routine_id, &link.owner_bot_id)?;
            let change = touch(&tx, bots)?;
            tx.commit()?;
            Ok(change)
        })
    }

    pub fn flow_remove_link(&self, id: &str) -> Result<Option<FlowChange>> {
        self.with_connection(|c| {
            let tx = c.unchecked_transaction()?;
            let mut st = tx.prepare(&format!("SELECT {LINK_COLUMNS} FROM routine_flow_links WHERE id = ?1"))?;
            let Some(link) = st.query_map([id], row_link)?.next().transpose()? else { return Ok(None) };
            drop(st);
            // The audience is read first: the removed link's bot may leave the canvas.
            let bots = audience(&tx, &link.routine_id, &link.owner_bot_id)?;
            tx.execute("DELETE FROM routine_flow_links WHERE id = ?1", [id])?;
            let change = touch(&tx, bots)?;
            tx.commit()?;
            Ok(change)
        })
    }

    pub fn flow_positions(&self, canvas_bot: &str) -> Result<Vec<FlowPosition>> {
        self.with_connection(|c| {
            let mut st = c.prepare(
                "SELECT node_key, x, y FROM routine_flow_positions WHERE canvas_bot_id = ?1 ORDER BY node_key",
            )?;
            let rows =
                st.query_map([canvas_bot], |r| Ok(FlowPosition { node_key: r.get(0)?, x: r.get(1)?, y: r.get(2)? }))?;
            Ok(rows.collect::<rusqlite::Result<_>>()?)
        })
    }

    pub fn flow_placed_bots(&self, canvas_bot: &str) -> Result<Vec<String>> {
        self.with_connection(|c| {
            let mut st =
                c.prepare("SELECT bot_id FROM routine_flow_placed WHERE canvas_bot_id = ?1 ORDER BY bot_id")?;
            Ok(st.query_map([canvas_bot], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?)
        })
    }

    pub fn flow_save_position(&self, canvas_bot: &str, key: &str, x: f64, y: f64) -> Result<FlowChange> {
        check_node_key(key)?;
        check_coordinate(x)?;
        check_coordinate(y)?;
        self.with_connection(|c| {
            let tx = c.unchecked_transaction()?;
            tx.execute(
                "INSERT INTO routine_flow_positions(canvas_bot_id, node_key, x, y) VALUES(?1, ?2, ?3, ?4)
                 ON CONFLICT(canvas_bot_id, node_key) DO UPDATE SET x = ?3, y = ?4",
                params![canvas_bot, key, x, y],
            )?;
            let change = touch(&tx, [canvas_bot.to_owned()])?.expect("one bot is given");
            tx.commit()?;
            Ok(change)
        })
    }

    /// Puts a bot on the canvas before any link reaches it, at `x`, `y`.
    pub fn flow_place_bot(&self, canvas_bot: &str, bot: &str, x: f64, y: f64) -> Result<FlowChange> {
        check_coordinate(x)?;
        check_coordinate(y)?;
        self.with_connection(|c| {
            let tx = c.unchecked_transaction()?;
            tx.execute(
                "INSERT OR IGNORE INTO routine_flow_placed(canvas_bot_id, bot_id) VALUES(?1, ?2)",
                params![canvas_bot, bot],
            )?;
            tx.execute(
                "INSERT INTO routine_flow_positions(canvas_bot_id, node_key, x, y) VALUES(?1, ?2, ?3, ?4)
                 ON CONFLICT(canvas_bot_id, node_key) DO UPDATE SET x = ?3, y = ?4",
                params![canvas_bot, format!("bot:{bot}"), x, y],
            )?;
            let change = touch(&tx, [canvas_bot.to_owned()])?.expect("one bot is given");
            tx.commit()?;
            Ok(change)
        })
    }

    /// Takes a node off the canvas; a `bot:` node also stops being placed. Nothing there is no change.
    pub fn flow_remove_position(&self, canvas_bot: &str, key: &str) -> Result<Option<FlowChange>> {
        check_node_key(key)?;
        self.with_connection(|c| {
            let tx = c.unchecked_transaction()?;
            let mut gone = tx.execute(
                "DELETE FROM routine_flow_positions WHERE canvas_bot_id = ?1 AND node_key = ?2",
                params![canvas_bot, key],
            )?;
            if let Some(bot) = key.strip_prefix("bot:") {
                gone += tx.execute(
                    "DELETE FROM routine_flow_placed WHERE canvas_bot_id = ?1 AND bot_id = ?2",
                    params![canvas_bot, bot],
                )?;
            }
            let change = if gone == 0 { None } else { touch(&tx, [canvas_bot.to_owned()])? };
            tx.commit()?;
            Ok(change)
        })
    }

    /// Replaces the runnable flow state of a merged document under `rev`, the revision its groups
    /// were saved at. Links of the routines on the canvas, the canvas's nodes and placed bots, and
    /// those routines' media steps are all rewritten in one transaction so a reload can never show
    /// half a merge. The owner and timestamp of a row already in the store are kept; a malformed
    /// entry fails the whole write instead of being dropped.
    pub fn flow_apply_merged(
        &self,
        rev: i64,
        canvas_bot: &str,
        routine_ids: &[String],
        links: &[FlowLink],
        positions: &[FlowPosition],
        placed: &[String],
        media: &[super::FlowMedia],
    ) -> Result<FlowChange> {
        if links.len() > MERGE_LINKS_CAP {
            bail!("a merged document has at most {MERGE_LINKS_CAP} links");
        }
        if positions.len() > MERGE_NODES_CAP || placed.len() > MERGE_NODES_CAP {
            bail!("a canvas holds at most {MERGE_NODES_CAP} nodes");
        }
        if media.len() > MERGE_MEDIA_CAP {
            bail!("a merged document has at most {MERGE_MEDIA_CAP} media steps");
        }
        for routine in routine_ids {
            if routine.is_empty() {
                bail!("a merged routine id is empty");
            }
        }
        self.with_connection(|c| {
            let tx = c.unchecked_transaction()?;
            let mut bots = BTreeSet::from([canvas_bot.to_owned()]);

            // Links: keep the owner and creation time of a row we already know, drop the ones the
            // merge removed, and write every kept link the way `flow_add_link` does.
            let mut existing: BTreeMap<String, (String, String, i64)> = BTreeMap::new();
            for routine in routine_ids {
                let mut st = tx.prepare(
                    "SELECT id, routine_id, owner_bot_id, created_at FROM routine_flow_links WHERE routine_id = ?1",
                )?;
                let rows = st.query_map([routine.as_str()], |r| {
                    Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, i64>(3)?))
                })?;
                for row in rows {
                    let (id, routine, owner, at) = row?;
                    existing.insert(id, (routine, owner, at));
                }
            }
            let mut new_ids = BTreeSet::new();
            let mut pairs = BTreeSet::new();
            for link in links {
                if link.id.is_empty()
                    || link.routine_id.is_empty()
                    || link.from_bot_id.is_empty()
                    || link.to_bot_id.is_empty()
                {
                    bail!("a merged link needs an id, a routine and both ends");
                }
                if link.instruction.len() > MERGE_INSTRUCTION_CAP {
                    bail!("merged link {} has too long an instruction", link.id);
                }
                check_name(&link.name).with_context(|| format!("merged link {}", link.id))?;
                if !routine_ids.contains(&link.routine_id) {
                    bail!("merged link {} names a routine that is not on this canvas", link.id);
                }
                if !new_ids.insert(link.id.as_str()) {
                    bail!("two merged links have the id {}", link.id);
                }
                if !pairs.insert((link.routine_id.as_str(), link.from_bot_id.as_str(), link.to_bot_id.as_str())) {
                    bail!("two merged links connect the same bots in routine {}", link.routine_id);
                }
                let (routine, owner, at) =
                    existing.get(&link.id).map(|(r, o, at)| (r.as_str(), o.as_str(), *at)).unwrap_or((
                        link.routine_id.as_str(),
                        link.owner_bot_id.as_str(),
                        link.created_at,
                    ));
                if routine != link.routine_id {
                    bail!("merged link {} moved to another routine", link.id);
                }
                tx.execute(
                    &format!(
                        "INSERT INTO routine_flow_links({LINK_COLUMNS}) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                         ON CONFLICT(id) DO UPDATE SET routine_id = ?2, from_bot_id = ?4, to_bot_id = ?5,
                         instruction = ?6, model = ?7, model_options = ?8, created_at = ?9, name = ?10"
                    ),
                    params![
                        link.id,
                        routine,
                        owner,
                        link.from_bot_id,
                        link.to_bot_id,
                        link.instruction,
                        link.model,
                        options_json(&link.model_options),
                        at,
                        link.name
                    ],
                )?;
                bots.insert(owner.to_owned());
                for end in [&link.from_bot_id, &link.to_bot_id] {
                    if !end.starts_with(super::flow_media::PREFIX) {
                        bots.insert(end.clone());
                    }
                }
            }
            for (id, (routine, _, _)) in &existing {
                if routine_ids.contains(routine) && !new_ids.contains(id.as_str()) {
                    tx.execute("DELETE FROM routine_flow_links WHERE id = ?1", [id.as_str()])?;
                }
            }

            // Positions and placements belong to this canvas alone, so replace them outright.
            tx.execute("DELETE FROM routine_flow_positions WHERE canvas_bot_id = ?1", [canvas_bot])?;
            tx.execute("DELETE FROM routine_flow_placed WHERE canvas_bot_id = ?1", [canvas_bot])?;
            for position in positions {
                check_node_key(&position.node_key)?;
                check_coordinate(position.x)?;
                check_coordinate(position.y)?;
                tx.execute(
                    "INSERT INTO routine_flow_positions(canvas_bot_id, node_key, x, y) VALUES(?1, ?2, ?3, ?4)",
                    params![canvas_bot, position.node_key, position.x, position.y],
                )?;
            }
            let mut placed_ids = BTreeSet::new();
            for bot in placed {
                if bot.is_empty() || !placed_ids.insert(bot.as_str()) {
                    bail!("a merged placed bot needs a unique id");
                }
                tx.execute(
                    "INSERT OR IGNORE INTO routine_flow_placed(canvas_bot_id, bot_id) VALUES(?1, ?2)",
                    params![canvas_bot, bot],
                )?;
            }

            // Media: drop steps the merge removed and write every kept step the way `flow_add_media`
            // does. Links were replaced above (one routine can only link to its own media steps), so
            // here only the step's place on other canvases needs clearing, as `flow_remove_media` does.
            let mut current: BTreeMap<String, (String, i64)> = BTreeMap::new();
            for routine in routine_ids {
                let mut st =
                    tx.prepare("SELECT id, owner_bot_id, created_at FROM routine_flow_media WHERE routine_id = ?1")?;
                let rows = st.query_map([routine.as_str()], |r| {
                    Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)?))
                })?;
                for row in rows {
                    let (id, owner, at) = row?;
                    current.insert(id, (owner, at));
                }
            }
            let mut media_ids = BTreeSet::new();
            for step in media {
                if step.id.is_empty() || step.routine_id.is_empty() || step.model.is_empty() {
                    bail!("a merged media step needs an id, a routine and a model");
                }
                if step.prompt.len() > super::flow_media::MAX_PROMPT_BYTES {
                    bail!("merged media step {} has too long a description", step.id);
                }
                check_name(&step.name).with_context(|| format!("merged media step {}", step.id))?;
                if !(1..=crate::media::MAX_BATCH).contains(&step.batch) {
                    bail!("merged media step {} has a batch outside 1 to 4", step.id);
                }
                if !routine_ids.contains(&step.routine_id) {
                    bail!("merged media step {} names a routine that is not on this canvas", step.id);
                }
                if !media_ids.insert(step.id.as_str()) {
                    bail!("two merged media steps have the id {}", step.id);
                }
            }
            for id in current.keys() {
                if !media_ids.contains(id.as_str()) {
                    let key = super::flow_media::key(id);
                    tx.execute("DELETE FROM routine_flow_positions WHERE node_key = ?1", [key.as_str()])?;
                    tx.execute("DELETE FROM routine_flow_media WHERE id = ?1", [id.as_str()])?;
                }
            }
            for step in media {
                let (owner, at) =
                    current.get(&step.id).map(|(o, at)| (o.as_str(), *at)).unwrap_or((step.owner_bot_id.as_str(), step.created_at));
                tx.execute(
                    &format!(
                        "INSERT INTO routine_flow_media({MEDIA_COLUMNS}) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
                         ON CONFLICT(id) DO UPDATE SET kind = ?2, model = ?3, prompt = ?4, aspect = ?5,
                         duration = ?6, resolution = ?7, transparent = ?8, voice = ?9, created_at = ?10,
                         routine_id = ?11, name = ?13, batch = ?14"
                    ),
                    params![
                        step.id,
                        step.kind,
                        step.model,
                        step.prompt,
                        step.aspect,
                        step.duration,
                        step.resolution,
                        step.transparent,
                        step.voice,
                        at,
                        step.routine_id,
                        owner,
                        step.name,
                        step.batch
                    ],
                )?;
                bots.insert(owner.to_owned());
            }

            let change = touch_at(&tx, rev, bots)?.expect("the canvas bot is always touched");
            tx.commit()?;
            Ok(change)
        })
    }

    /// A deleted routine leaves its links, steps and node positions.
    pub fn flow_forget_routine(&self, routine_id: &str, owner: &str) -> Result<Option<FlowChange>> {
        self.with_connection(|c| {
            let tx = c.unchecked_transaction()?;
            let bots = audience(&tx, routine_id, owner)?;
            let mut gone = tx.execute("DELETE FROM routine_flow_links WHERE routine_id = ?1", [routine_id])?;
            gone += super::flow_media::forget_routine(&tx, routine_id)?;
            gone += tx.execute("DELETE FROM routine_flow_steps WHERE routine_id = ?1", [routine_id])?;
            gone += tx
                .execute("DELETE FROM routine_flow_positions WHERE node_key = ?1", [format!("routine:{routine_id}")])?;
            let change = if gone == 0 { None } else { touch(&tx, bots)? };
            tx.commit()?;
            Ok(change)
        })
    }

    /// A deleted bot leaves every flow: its own routines' flows, the links that name it, its steps,
    /// its canvas and its place on others.
    pub fn flow_forget_bot(&self, bot: &str) -> Result<Option<FlowChange>> {
        self.with_connection(|c| {
            let tx = c.unchecked_transaction()?;
            let routines: Vec<(String, String)> = {
                let mut st = tx.prepare(
                    "SELECT DISTINCT routine_id, owner_bot_id FROM routine_flow_links
                     WHERE owner_bot_id = ?1 OR from_bot_id = ?1 OR to_bot_id = ?1",
                )?;
                st.query_map([bot], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<rusqlite::Result<_>>()?
            };
            let mut bots = BTreeSet::new();
            for (routine, owner) in &routines {
                bots.extend(audience(&tx, routine, owner)?);
            }
            let node = format!("bot:{bot}");
            let mut gone = tx.execute(
                "DELETE FROM routine_flow_links WHERE owner_bot_id = ?1 OR from_bot_id = ?1 OR to_bot_id = ?1",
                [bot],
            )?;
            gone += super::flow_media::forget_owner(&tx, bot)?;
            gone += tx.execute("DELETE FROM routine_flow_steps WHERE owner_bot_id = ?1 OR bot_id = ?1", [bot])?;
            gone += tx.execute(
                "DELETE FROM routine_flow_positions WHERE canvas_bot_id = ?1 OR node_key = ?2",
                params![bot, node],
            )?;
            gone += tx.execute("DELETE FROM routine_flow_placed WHERE canvas_bot_id = ?1 OR bot_id = ?1", [bot])?;
            tx.execute("DELETE FROM kv WHERE k = ?1", [format!("{REV_PREFIX}{bot}")])?;
            bots.remove(bot);
            let change = if gone == 0 { None } else { touch(&tx, bots)? };
            tx.commit()?;
            Ok(change)
        })
    }

    /// Canvas revisions after `since`: each bot whose flow changed since, with the rev.
    pub fn flow_revs_since(&self, since: i64) -> Vec<(String, i64)> {
        self.kv_prefix(REV_PREFIX)
            .into_iter()
            .filter_map(|key| {
                let rev = self.kv_get(&key)?.parse::<i64>().ok()?;
                let bot = key.strip_prefix(REV_PREFIX)?.to_owned();
                (rev > since).then_some((bot, rev))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Store;

    fn named_link(name: &str) -> FlowLink {
        FlowLink {
            id: "l1".into(),
            routine_id: "r".into(),
            owner_bot_id: "a".into(),
            from_bot_id: "a".into(),
            to_bot_id: "b".into(),
            name: name.into(),
            instruction: "Translate".into(),
            model: None,
            model_options: BTreeMap::new(),
            created_at: 1,
        }
    }

    #[test]
    fn a_step_name_is_saved_and_can_be_cleared() {
        let store = Store::open(std::path::Path::new(":memory:")).expect("a store");
        store.flow_add_link(&named_link(""), |_| Ok(())).expect("the link is added");
        assert_eq!(store.flow_link("l1").unwrap().unwrap().name, "", "an old link is unnamed");
        store.flow_rename_link("l1", "Translate").unwrap();
        assert_eq!(store.flow_link("l1").unwrap().unwrap().name, "Translate");
        assert_eq!(store.flow_link("l1").unwrap().unwrap().instruction, "Translate", "the task is untouched");
        store.flow_rename_link("l1", "").unwrap();
        assert_eq!(store.flow_link("l1").unwrap().unwrap().name, "", "an empty name clears it");
    }

    #[test]
    fn a_step_name_is_bounded() {
        assert!(check_name("Ada's step").is_ok());
        assert!(check_name(&"x".repeat(MAX_NAME_CHARS + 1)).is_err());
        assert!(check_name("one\ntwo").is_err(), "control characters are refused");
    }
}
