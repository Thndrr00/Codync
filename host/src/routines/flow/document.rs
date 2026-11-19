//! One bot's stamped flow document, kept in the kv blob beside its groups and canvas revision.
//!
//! Each save appends a snapshot stamped with the canvas revision, so a client that started from an
//! earlier revision gives the merge its common base. The history is short: a document whose base
//! fell out of it is read as one with no base (a plain save). A conflict waits here as a pending
//! document until the person chooses a version for every field; the groups of the final document
//! are written through `groups::save` so the canvas shows what was merged, with a single rev stamp
//! shared by the groups, the document and the per-bot canvas revision.

use super::groups::{self, FlowGroup};
use super::merge::{FlowConflict, FlowDocument};
use crate::store::{FlowChange, Store};
use anyhow::{Context, Result};
use rusqlite::params;
use serde::{Deserialize, Serialize};

/// kv prefix of one bot's document history, followed by its id.
const DOCS_PREFIX: &str = "routineflow.docs.";
/// kv prefix of a conflict waiting for the person's choices.
const PENDING_PREFIX: &str = "routineflow.pending.";
/// How many revisions of one canvas can serve as a merge base.
const MAX_SNAPSHOTS: usize = 8;

/// A document at the revision it was saved with.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub rev: i64,
    pub document: FlowDocument,
}

/// A merge paused on the fields both people changed.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pending {
    pub base_rev: i64,
    pub document: FlowDocument,
    pub conflicts: Vec<FlowConflict>,
}

fn key(bot: &str) -> String {
    format!("{DOCS_PREFIX}{bot}")
}

fn pending_key(bot: &str) -> String {
    format!("{PENDING_PREFIX}{bot}")
}

fn put(store: &Store, name: String, value: &str) -> Result<()> {
    store.with_connection(|c| {
        c.execute("INSERT INTO kv(k, v) VALUES(?1, ?2) ON CONFLICT(k) DO UPDATE SET v = ?2", params![name, value])?;
        Ok(())
    })
}

/// The recent stamped documents of `bot`, oldest first.
pub fn history(store: &Store, bot: &str) -> Vec<Snapshot> {
    store.kv_get(&key(bot)).and_then(|raw| serde_json::from_str(&raw).ok()).unwrap_or_default()
}

/// The document at `rev`, when it is still in the history.
pub fn base_at(snapshots: &[Snapshot], rev: i64) -> Option<&FlowDocument> {
    snapshots.iter().find(|snapshot| snapshot.rev == rev).map(|snapshot| &snapshot.document)
}

/// The conflict waiting for this bot, if a save found one.
pub fn pending(store: &Store, bot: &str) -> Option<Pending> {
    store.kv_get(&pending_key(bot)).and_then(|raw| serde_json::from_str(&raw).ok())
}

pub fn save_pending(store: &Store, bot: &str, waiting: &Pending) -> Result<()> {
    let value = serde_json::to_string(waiting).context("could not store the flow conflict")?;
    put(store, pending_key(bot), &value)
}

pub fn clear_pending(store: &Store, bot: &str) -> Result<()> {
    store.with_connection(|c| {
        c.execute("DELETE FROM kv WHERE k = ?1", [pending_key(bot)])?;
        Ok(())
    })
}

/// Saves a resolved document and its groups with one rev stamp, and drops any pending conflict.
pub fn save(store: &Store, bot: &str, document: &FlowDocument, groups: &[FlowGroup]) -> Result<FlowChange> {
    // Groups own the canvas rev stamp; the snapshot reuses it so base revisions line up.
    let change = groups::save(store, bot, groups)?;
    let mut snapshots = history(store, bot);
    snapshots.push(Snapshot { rev: change.rev, document: document.clone() });
    if snapshots.len() > MAX_SNAPSHOTS {
        let extra = snapshots.len() - MAX_SNAPSHOTS;
        snapshots.drain(..extra);
    }
    let value = serde_json::to_string(&snapshots).context("could not store the flow document")?;
    put(store, key(bot), &value)?;
    clear_pending(store, bot)?;
    Ok(change)
}
