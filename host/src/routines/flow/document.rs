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
use super::format;
use super::groups::{self, FlowGroup};
use super::merge::{FlowConflict, FlowDocument};
use crate::store::{FlowChange, Store, now_ms};
use anyhow::{Context, Result};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

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
/// OpenBot `WORKFLOW_AUTO_VERSION_INTERVAL_MS`: a save records a user version at most this often.
pub const AUTO_VERSION_MS: i64 = 5 * 60 * 1000;
/// OpenBot `WORKFLOW_MAX_VERSIONS`: how many user-visible versions one canvas keeps.
pub const MAX_VERSIONS: usize = 50;
/// kv prefix of a bot's user-visible version list, followed by its id. This is kept apart from the
/// short merge-snapshot history: restoring a version must never drop a merge base.
const VERSIONS_PREFIX: &str = "routineflow.versions.";

/// The optional width, height and font of a canvas node. Every side is validated in `format`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeSize {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font: Option<f64>,
}

/// The sizes of the nodes that name one, for a canvas to draw them.
pub fn node_sizes(document: &FlowDocument) -> BTreeMap<String, NodeSize> {
    document
        .nodes
        .iter()
        .filter_map(|(key, value)| format::node_size(value).map(|size| (key.clone(), size)))
        .filter(|(_, size)| *size != NodeSize::default())
        .collect()
}

/// The nodes a client must not change: locked placeholders of newer kinds.
pub fn locked_nodes(document: &FlowDocument) -> BTreeSet<String> {
    document.nodes.iter().filter(|(_, value)| format::node_locked(value)).map(|(key, _)| key.clone()).collect()
}

/// The drawing strokes of a document that are well formed. Bad ones are refused on save, so this
/// only skips ones written by an older host that did not validate them.
pub fn strokes(document: &FlowDocument) -> Vec<Value> {
    document.strokes.values().filter(|stroke| format::stroke_valid(stroke)).cloned().collect()
}

/// Who made a user-visible version. `run` freezes the graph a run started from, `restore` the one a
/// person went back to, and `auto` the periodic save.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum UserVersionKind {
    Auto,
    Restore,
    Run,
}

impl UserVersionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Restore => "restore",
            Self::Run => "run",
        }
    }
}

/// A version a person can list, inspect and restore. It keeps the whole document so the graph shown
/// for a version is exactly the one that was saved, with no reconstruction from newer tables.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserVersion {
    pub id: String,
    pub at: i64,
    pub kind: UserVersionKind,
    pub rev: i64,
    pub document: FlowDocument,
}

/// Pure autosave decision: record a version when the window has passed and the list has room.
pub fn version_due(elapsed_ms: i64, count: usize, max: usize) -> bool {
    count < max && elapsed_ms >= AUTO_VERSION_MS
}

/// What the version list shows: no document bytes, only the id, time, kind and canvas revision.
pub fn version_summary(version: &UserVersion) -> Value {
    json!({"id": version.id, "at": version.at, "kind": version.kind.as_str(), "rev": version.rev})
}

/// The saved graph of one version.
pub fn version_graph(version: &UserVersion) -> &FlowDocument {
    &version.document
}

/// The document a restore writes: the version's graph, with a fresh canvas revision set on save.
pub fn restored(version: &UserVersion) -> FlowDocument {
    version.document.clone()
}

/// Finds a version by id. Ids are opaque, so an unknown one is simply absent.
pub fn find_version<'a>(versions: &'a [UserVersion], id: &str) -> Option<&'a UserVersion> {
    versions.iter().find(|version| version.id == id)
}

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

fn versions_key(bot: &str) -> String {
    format!("{VERSIONS_PREFIX}{bot}")
}

/// The user-visible versions of `bot`, oldest first. Stored separately from the merge snapshots so
/// a restore or a cap on versions cannot take away the short merge base history.
pub fn versions(store: &Store, bot: &str) -> Vec<UserVersion> {
    store.kv_get(&versions_key(bot)).and_then(|raw| serde_json::from_str(&raw).ok()).unwrap_or_default()
}

fn write_versions(store: &Store, bot: &str, list: &[UserVersion]) -> Result<()> {
    let value = serde_json::to_string(list).context("could not store the flow versions")?;
    put(store, versions_key(bot), &value)
}

/// Appends a version of `kind` and caps the list at `MAX_VERSIONS`, oldest first.
pub fn append_version(
    store: &Store,
    bot: &str,
    document: &FlowDocument,
    rev: i64,
    kind: UserVersionKind,
) -> Result<()> {
    let mut list = versions(store, bot);
    list.push(UserVersion {
        id: uuid::Uuid::new_v4().to_string(),
        at: now_ms(),
        kind,
        rev,
        document: document.clone(),
    });
    if list.len() > MAX_VERSIONS {
        let extra = list.len() - MAX_VERSIONS;
        list.drain(..extra);
    }
    write_versions(store, bot, &list)
}

/// Records a save as an automatic version only when the last one is old enough. A first save with no
/// version yet is always due; a clock that went backwards reads as not due rather than as a new one.
fn maybe_auto_version(store: &Store, bot: &str, document: &FlowDocument, rev: i64) -> Result<()> {
    let list = versions(store, bot);
    let last = list.iter().filter(|version| version.kind == UserVersionKind::Auto).map(|version| version.at).max();
    let elapsed = last.map_or(i64::MAX, |at| now_ms().saturating_sub(at));
    if version_due(elapsed, list.len(), MAX_VERSIONS) {
        append_version(store, bot, document, rev, UserVersionKind::Auto)?;
    }
    Ok(())
}

/// Freezes the current document before a run, so a person can go back to the graph that run used.
pub fn record_run_version(store: &Store, bot: &str) -> Result<bool> {
    let Some(latest) = history(store, bot).last() else { return Ok(false) };
    append_version(store, bot, &latest.document, latest.rev, UserVersionKind::Run)?;
    Ok(true)
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
    // Strokes are drawing data: validate them, but never hand them to the runnable flow tables.
    format::check_strokes(&document.strokes)?;
    // Groups own the canvas rev stamp; the snapshot reuses it so base revisions line up.
    let change = groups::save(store, bot, groups)?;
    stamp(store, bot, document, change.rev)?;
    maybe_auto_version(store, bot, document, change.rev)?;
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
        &state.nodes,
    )?;
    stamp(store, bot, document, change.rev)?;
    maybe_auto_version(store, bot, document, change.rev)?;
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

    #[test]
    fn a_version_is_due_every_five_minutes_until_the_list_is_full() {
        assert!(version_due(AUTO_VERSION_MS, 0, MAX_VERSIONS), "a first save has no older version");
        assert!(!version_due(AUTO_VERSION_MS - 1, 1, MAX_VERSIONS));
        assert!(version_due(AUTO_VERSION_MS, 1, MAX_VERSIONS));
        assert!(version_due(i64::MAX, MAX_VERSIONS - 1, MAX_VERSIONS));
        assert!(!version_due(i64::MAX, MAX_VERSIONS, MAX_VERSIONS), "no room for another");
        assert!(!version_due(-1, 1, MAX_VERSIONS), "a clock that went backwards is not due");
    }

    #[test]
    fn versions_have_a_summary_a_graph_and_a_restore_without_touching_snapshots() {
        let store = Store::open(std::path::Path::new(":memory:")).expect("a store");
        let document = FlowDocument { rev: 7, ..FlowDocument::default() };
        append_version(&store, "b", &document, 7, UserVersionKind::Run).expect("a run version");
        append_version(&store, "b", &document, 7, UserVersionKind::Restore).expect("a restore version");
        let list = versions(&store, "b");
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].kind, UserVersionKind::Run);
        assert_eq!(list[0].document.rev, 7);
        assert_eq!(version_summary(&list[0])["kind"], "run");
        assert_eq!(version_graph(&list[0]).rev, 7);
        assert_eq!(restored(&list[1]), document);
        assert!(find_version(&list, &list[0].id).is_some());
        assert!(find_version(&list, "not-a-version").is_none());
        assert!(record_run_version(&store, "b").expect("no snapshot yet") == false);

        // The user list is independent of the merge snapshots.
        stamp(&store, "b", &document, 7).expect("a snapshot");
        assert!(record_run_version(&store, "b").expect("a run version"));
        assert_eq!(versions(&store, "b").len(), 3);
        assert_eq!(history(&store, "b").len(), 1);
    }

    #[test]
    fn node_sizes_locked_nodes_and_strokes_are_read_from_a_document() {
        let document: FlowDocument = serde_json::from_value(json!({
            "rev": 1,
            "nodes": {
                "bot:a": {"x": 1.0, "y": 2.0, "size": {"width": 180.0, "font": 14.0}},
                "future:1": {"x": 3.0, "y": 4.0, "locked": true}
            },
            "strokes": {
                "s1": {"id": "s1", "brush": "pen", "color": "red", "width": 2.0, "points": [{"x": 1, "y": 2}]},
                "bad": {"id": "bad", "brush": "pen", "color": "red", "width": -2.0, "points": []}
            }
        }))
        .expect("a document");
        let sizes = node_sizes(&document);
        assert_eq!(sizes["bot:a"].width, Some(180.0));
        assert_eq!(sizes["bot:a"].font, Some(14.0));
        assert!(!sizes.contains_key("future:1"));
        assert_eq!(locked_nodes(&document), BTreeSet::from(["future:1".to_owned()]));
        assert_eq!(strokes(&document).len(), 1, "a malformed stroke is not shown");
    }
}
