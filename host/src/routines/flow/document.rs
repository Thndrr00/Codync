//! One bot's stamped flow document, kept in the kv blob beside its groups and canvas revision.
//!
//! Each save appends a snapshot stamped with the canvas revision, so a client that started from an
//! earlier revision gives the merge its common base. The history is short: a document whose base
//! fell out of it is read as one with no base (a plain save). A conflict waits here as a pending
//! document until the person chooses a version for every field; the groups of the final document
//! are written through `groups::save` so the canvas shows what was merged. On a finished merge the
//! links, nodes and media steps go back to the runnable flow tables too, under one rev stamp shared
//! by those tables, the groups, the document and the per-bot canvas revision. A conflict nobody chooses
//! within seven days is dropped on the next load, along with that bot's snapshots, so an abandoned merge
//! can't park a stale document forever.

use super::apply;
use super::groups::{self, FlowGroup};
use super::merge::{FlowConflict, FlowDocument};
use crate::store::{FlowChange, Store, now_ms};
use anyhow::{Context, Result};
use rusqlite::params;
use serde::{Deserialize, Serialize};

/// kv prefix of one bot's document history, followed by its id.
const DOCS_PREFIX: &str = "routineflow.docs.";
/// kv prefix of a conflict waiting for the person's choices.
const PENDING_PREFIX: &str = "routineflow.pending.";
/// When a conflict was parked, kept beside it so adding the timestamp doesn't change the pending blob
/// (older hosts write the blob without one and read as already expired).
const PENDING_AT_PREFIX: &str = "routineflow.pending.at.";
/// How many revisions of one canvas can serve as a merge base.
const MAX_SNAPSHOTS: usize = 8;
/// How long a conflict may wait for the person's choices. Past this it is dropped on load, with the
/// snapshots it was based on, so an abandoned merge doesn't pin a canvas forever.
pub(crate) const PENDING_TTL_MS: i64 = 7 * 24 * 60 * 60 * 1000;

/// Pure expiry decision: a pending merge older than the window is dropped. A clock that went backwards
/// reads as fresh; a pending written before the timestamp existed (`0`) reads as expired.
pub fn pending_expired(saved_at: i64, now: i64) -> bool {
    now.saturating_sub(saved_at) > PENDING_TTL_MS
}

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

fn pending_at_key(bot: &str) -> String {
    format!("{PENDING_AT_PREFIX}{bot}")
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

/// The conflict waiting for this bot, if a save found one and it hasn't aged out. Loading an expired one
/// drops it, its timestamp and the bot's snapshots: the snapshots were only a merge base for that pending
/// document, so keeping them would pin the canvas to a conflict nobody will resolve. A pending written by
/// an older host has no timestamp and reads as expired.
pub fn pending(store: &Store, bot: &str) -> Option<Pending> {
    let waiting: Pending = store.kv_get(&pending_key(bot)).and_then(|raw| serde_json::from_str(&raw).ok())?;
    let saved_at = store.kv_get(&pending_at_key(bot)).and_then(|raw| raw.parse::<i64>().ok());
    if saved_at.is_none_or(|at| pending_expired(at, now_ms())) {
        let _ = clear_pending(store, bot);
        if let Err(error) = store.kv_delete(&key(bot)) {
            tracing::warn!(bot, error = format!("{error:#}"), "couldn't clear the expired flow snapshots");
        }
        return None;
    }
    Some(waiting)
}

pub fn save_pending(store: &Store, bot: &str, waiting: &Pending) -> Result<()> {
    let value = serde_json::to_string(waiting).context("could not store the flow conflict")?;
    put(store, pending_key(bot), &value)?;
    // Each step of a resolution saves again, which keeps an actively worked conflict alive.
    put(store, pending_at_key(bot), &now_ms().to_string())
}

pub fn clear_pending(store: &Store, bot: &str) -> Result<()> {
    store.with_connection(|c| {
        c.execute("DELETE FROM kv WHERE k = ?1", [pending_key(bot)])?;
        c.execute("DELETE FROM kv WHERE k = ?1", [pending_at_key(bot)])?;
        Ok(())
    })
}

/// Appends `document` to `bot`'s history at `rev` and drops any pending conflict.
fn stamp(store: &Store, bot: &str, document: &FlowDocument, rev: i64) -> Result<()> {
    let mut snapshots = history(store, bot);
    snapshots.push(Snapshot { rev, document: document.clone() });
    if snapshots.len() > MAX_SNAPSHOTS {
        let extra = snapshots.len() - MAX_SNAPSHOTS;
        snapshots.drain(..extra);
    }
    let value = serde_json::to_string(&snapshots).context("could not store the flow document")?;
    put(store, key(bot), &value)?;
    clear_pending(store, bot)
}

/// Saves a resolved document and its groups with one rev stamp, and drops any pending conflict.
pub fn save(store: &Store, bot: &str, document: &FlowDocument, groups: &[FlowGroup]) -> Result<FlowChange> {
    // Groups own the canvas rev stamp; the snapshot reuses it so base revisions line up.
    let change = groups::save(store, bot, groups)?;
    stamp(store, bot, document, change.rev)?;
    Ok(change)
}

/// Saves a finished merge. On top of the groups and the snapshot, the merged nodes, links and media
/// steps are written to the runnable flow tables so a canvas reload shows all of the merge under the
/// same rev. The mapping runs first, so a bad entry fails before anything is written.
pub fn save_merged(
    store: &Store,
    bot: &str,
    document: &FlowDocument,
    groups: &[FlowGroup],
    routine_ids: &[String],
) -> Result<FlowChange> {
    let state = apply::runnable(bot, document).context("the merged flow document cannot be applied")?;
    let change = groups::save(store, bot, groups)?;
    store.flow_apply_merged(
        change.rev,
        bot,
        routine_ids,
        &state.links,
        &state.positions,
        &state.placed,
        &state.media,
    )?;
    stamp(store, bot, document, change.rev)?;
    Ok(change)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_conflict_expires_only_after_seven_days() {
        let now = 1_800_000_000_000;
        let saved = now - PENDING_TTL_MS;
        assert!(!pending_expired(saved, now), "exactly at the window is still live");
        assert!(!pending_expired(now, now), "a conflict just parked is live");
        assert!(pending_expired(saved - 1, now), "a moment past the window is dropped");
        assert!(pending_expired(0, now), "one with no timestamp reads as expired");
        assert!(!pending_expired(now + 1000, now), "a clock that went backwards is not expired");
    }

    fn waiting() -> Pending {
        Pending { base_rev: 1, document: FlowDocument::default(), conflicts: vec![] }
    }

    #[test]
    fn loading_an_expired_conflict_drops_it_with_its_snapshots() {
        let store = Store::open(std::path::Path::new(":memory:")).expect("a store");
        save_pending(&store, "b", &waiting()).expect("a pending conflict");
        put(&store, key("b"), "[]").expect("a snapshot");
        assert!(pending(&store, "b").is_some(), "a fresh conflict is kept");

        let old = now_ms() - PENDING_TTL_MS - 1000;
        put(&store, pending_at_key("b"), &old.to_string()).expect("an old timestamp");
        assert!(pending(&store, "b").is_none(), "the expired conflict is gone on load");
        assert!(store.kv_get(&pending_key("b")).is_none());
        assert!(store.kv_get(&pending_at_key("b")).is_none());
        assert!(store.kv_get(&key("b")).is_none(), "its stale snapshots go with it");
        assert!(history(&store, "b").is_empty());
    }
}
