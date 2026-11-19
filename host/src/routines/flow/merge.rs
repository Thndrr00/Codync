//! Field-level three-way merge of one bot's routine-flow document.
//!
//! The document is the canvas's editable drawing state: `nodes` (positions and placed bots),
//! `links`, `settings` (the picture and clip steps) and `groups`, each a map from an id to its
//! version. Two people can edit the same canvas: one saves, the other saves on top with the
//! revision they started from. The host keeps a bounded history of stamped documents, so it can
//! find the common base and fold the two edits together here, with no database or Hub.
//!
//! The rules:
//! - a field one side did not touch keeps the other side's version, so a node only one person
//!   moved is never lost;
//! - both sides changing one entry the same way is not a conflict;
//! - two different changes to one entry, or a change against a removal, is a conflict: the
//!   person's own version stays in the document until they choose one. A missing version in a
//!   conflict means that side removed the entry;
//! - with no base (an older canvas, or a snapshot that fell out of the history) the local save
//!   wins outright, the same as an ordinary incoming save;
//! - maps are bounded and keyed in order, so the result is deterministic and cannot grow past the
//!   caps.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// The most entries of each field one document may hold. A canvas is drawing state, far smaller
/// than any of these; the caps keep a hostile or broken document from growing without limit.
pub const MAX_NODES: usize = 1000;
pub const MAX_LINKS: usize = 1000;
pub const MAX_SETTINGS: usize = 500;
pub const MAX_GROUPS: usize = 100;

/// One field of a flow document: an id (node key, link id, media id, group id) to its version.
pub type Entries = BTreeMap<String, Value>;

/// One bot's flow document, the four editable fields together with the canvas revision it was
/// saved at. Every field is optional on the wire so an older canvas that sends fewer fields reads
/// as an empty one instead of failing.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowDocument {
    #[serde(default)]
    pub rev: i64,
    #[serde(default)]
    pub nodes: Entries,
    #[serde(default)]
    pub links: Entries,
    #[serde(default)]
    pub settings: Entries,
    #[serde(default)]
    pub groups: Entries,
}

/// One entry both people changed differently. The person has to pick one of the two versions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowConflict {
    /// `nodes`, `links`, `settings` or `groups`.
    pub field: String,
    pub key: String,
    /// The version in the document the person saved; `None` when that side removed the entry.
    pub local: Option<Value>,
    /// The version already on the host; `None` when that side removed the entry.
    pub incoming: Option<Value>,
}

/// Which version of a conflict to keep.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Version {
    Local,
    Incoming,
}

/// The outcome of a merge: the document to save, the choices still needed, and whether an edit
/// from someone else was folded in quietly (so the client can say the flow changed).
#[derive(Clone, Debug, PartialEq)]
pub struct MergeResult {
    pub document: FlowDocument,
    pub conflicts: Vec<FlowConflict>,
    pub merged: bool,
}

/// Drops null (removed) entries and caps the map, in key order so the choice is deterministic.
fn bounded(entries: &Entries, cap: usize) -> Entries {
    entries
        .iter()
        .filter(|(_, value)| **value != Value::Null)
        .take(cap)
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

/// Caps every field of a document without merging it (the no-base and incoming-save paths).
pub fn bound(document: &FlowDocument) -> FlowDocument {
    FlowDocument {
        rev: document.rev,
        nodes: bounded(&document.nodes, MAX_NODES),
        links: bounded(&document.links, MAX_LINKS),
        settings: bounded(&document.settings, MAX_SETTINGS),
        groups: bounded(&document.groups, MAX_GROUPS),
    }
}

/// Folds one field together. `changed_by_other` reports that the host's version was taken, which
/// is the merge the person needs to know about when there was no conflict.
fn merge_entries(
    field: &str,
    cap: usize,
    base: &Entries,
    local: &Entries,
    incoming: &Entries,
    conflicts: &mut Vec<FlowConflict>,
) -> (Entries, bool) {
    let (base, local, incoming) = (bounded(base, cap), bounded(local, cap), bounded(incoming, cap));
    let keys: BTreeSet<&str> = base.keys().chain(local.keys()).chain(incoming.keys()).map(String::as_str).collect();
    let mut merged = Entries::new();
    let mut changed_by_other = false;
    for key in keys {
        let (b, l, i) = (base.get(key), local.get(key), incoming.get(key));
        let keep = if l == i {
            // Both agree (including both removing it): nothing to decide.
            l.cloned()
        } else if l == b {
            // Only the host's side changed this entry; take its version.
            changed_by_other = true;
            i.cloned()
        } else if i == b {
            // Only the person's side changed it; their edit survives.
            l.cloned()
        } else {
            // Both changed it differently (or one removed it). Keep the person's version for now,
            // and ask which one to keep.
            conflicts.push(FlowConflict {
                field: field.to_owned(),
                key: key.to_owned(),
                local: l.cloned(),
                incoming: i.cloned(),
            });
            l.cloned()
        };
        if let Some(value) = keep {
            merged.insert(key.to_owned(), value);
        }
        if merged.len() >= cap {
            break;
        }
    }
    (merged, changed_by_other)
}

/// Three-way merge of `local` (the person's save) and `incoming` (the host's stored document)
/// over `base` (the revision they both started from). With no base, the local save simply wins.
pub fn merge(base: Option<&FlowDocument>, local: &FlowDocument, incoming: &FlowDocument) -> MergeResult {
    let Some(base) = base else {
        return MergeResult { document: bound(local), conflicts: Vec::new(), merged: false };
    };
    let mut conflicts = Vec::new();
    let (nodes, n) = merge_entries("nodes", MAX_NODES, &base.nodes, &local.nodes, &incoming.nodes, &mut conflicts);
    let (links, l) = merge_entries("links", MAX_LINKS, &base.links, &local.links, &incoming.links, &mut conflicts);
    let (settings, s) =
        merge_entries("settings", MAX_SETTINGS, &base.settings, &local.settings, &incoming.settings, &mut conflicts);
    let (groups, g) =
        merge_entries("groups", MAX_GROUPS, &base.groups, &local.groups, &incoming.groups, &mut conflicts);
    MergeResult {
        document: FlowDocument { rev: base.rev.max(local.rev).max(incoming.rev), nodes, links, settings, groups },
        conflicts,
        merged: n || l || s || g,
    }
}

fn entries_mut<'a>(document: &'a mut FlowDocument, field: &str) -> Option<&'a mut Entries> {
    match field {
        "nodes" => Some(&mut document.nodes),
        "links" => Some(&mut document.links),
        "settings" => Some(&mut document.settings),
        "groups" => Some(&mut document.groups),
        _ => None,
    }
}

/// Applies one resolved choice to the merged document: `None` removes the entry again.
pub fn resolve(document: &mut FlowDocument, conflict: &FlowConflict, version: Version) {
    let Some(entries) = entries_mut(document, &conflict.field) else { return };
    let chosen = match version {
        Version::Local => &conflict.local,
        Version::Incoming => &conflict.incoming,
    };
    match chosen {
        Some(value) => {
            entries.insert(conflict.key.clone(), value.clone());
        }
        None => {
            entries.remove(&conflict.key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn document(value: Value) -> FlowDocument {
        serde_json::from_value(value).expect("a well-formed test document")
    }

    fn sample() -> FlowDocument {
        document(json!({
            "rev": 4,
            "nodes": {"bot:a": {"x": 1.0, "y": 2.0}, "bot:b": {"x": 10.0, "y": 20.0}},
            "links": {"l1": {"instruction": "Translate"}},
            "settings": {"m1": {"prompt": "a picture"}},
            "groups": {"g1": {"name": "Pictures", "members": ["media:m1"]}}
        }))
    }

    #[test]
    fn edits_on_different_fields_both_survive() {
        let base = sample();
        let local = FlowDocument {
            nodes: document(json!({"bot:a": {"x": 9.0, "y": 2.0}, "bot:b": {"x": 10.0, "y": 20.0}})).nodes,
            ..base.clone()
        };
        let incoming = FlowDocument {
            groups: document(json!({"g1": {"name": "Clips", "members": ["media:m1"]}})).groups,
            ..base.clone()
        };
        let result = merge(Some(&base), &local, &incoming);
        assert!(result.conflicts.is_empty());
        assert!(result.merged, "a group came from the other editor");
        assert_eq!(result.document.nodes["bot:a"]["x"], 9.0, "our move is kept");
        assert_eq!(result.document.groups["g1"]["name"], "Clips", "their rename is kept");
    }

    #[test]
    fn different_nodes_changed_by_each_side_keep_both() {
        let base = sample();
        let local = FlowDocument {
            nodes: document(json!({"bot:a": {"x": 9.0, "y": 2.0}, "bot:b": {"x": 10.0, "y": 20.0}})).nodes,
            ..base.clone()
        };
        let incoming = FlowDocument {
            nodes: document(json!({"bot:a": {"x": 1.0, "y": 2.0}, "bot:b": {"x": 11.0, "y": 21.0}})).nodes,
            ..base.clone()
        };
        let result = merge(Some(&base), &local, &incoming);
        assert!(result.conflicts.is_empty());
        assert_eq!(result.document.nodes["bot:a"]["x"], 9.0);
        assert_eq!(result.document.nodes["bot:b"]["x"], 11.0);
        assert_eq!(result.document.nodes.len(), 2, "neither node was lost");
    }

    #[test]
    fn one_field_changed_both_ways_is_a_conflict_with_both_versions() {
        let base = sample();
        let local = FlowDocument { links: document(json!({"l1": {"instruction": "Summarize"}})).links, ..base.clone() };
        let incoming =
            FlowDocument { links: document(json!({"l1": {"instruction": "Translate it"}})).links, ..base.clone() };
        let result = merge(Some(&base), &local, &incoming);
        assert_eq!(result.document.links["l1"]["instruction"], "Summarize", "our version is provisional");
        assert_eq!(result.conflicts.len(), 1);
        let conflict = &result.conflicts[0];
        assert_eq!((conflict.field.as_str(), conflict.key.as_str()), ("links", "l1"));
        assert_eq!(conflict.local.as_ref().and_then(|v| v["instruction"].as_str()), Some("Summarize"));
        assert_eq!(conflict.incoming.as_ref().and_then(|v| v["instruction"].as_str()), Some("Translate it"));

        let mut resolved = result.document.clone();
        resolve(&mut resolved, conflict, Version::Incoming);
        assert_eq!(resolved.links["l1"]["instruction"], "Translate it");
    }

    #[test]
    fn no_base_is_a_plain_local_save() {
        let local = sample();
        let incoming = FlowDocument {
            groups: document(json!({"g1": {"name": "Someone else", "members": []}})).groups,
            ..local.clone()
        };
        let result = merge(None, &local, &incoming);
        assert_eq!(result.document, bound(&local));
        assert!(result.conflicts.is_empty());
        assert!(!result.merged);
    }

    #[test]
    fn a_removal_by_one_side_is_kept_without_a_conflict() {
        let base = sample();
        let local = FlowDocument { nodes: Entries::new(), ..base.clone() };
        let result = merge(Some(&base), &local, &base.clone());
        assert!(result.conflicts.is_empty());
        assert!(result.document.nodes.is_empty());
        assert_eq!(result.document.links.len(), 1, "other fields are untouched");
    }

    #[test]
    fn removal_against_a_change_is_a_conflict_with_a_missing_version() {
        let base = sample();
        let local = FlowDocument { nodes: Entries::new(), ..base.clone() };
        let incoming = FlowDocument { nodes: document(json!({"bot:a": {"x": 7.0, "y": 2.0}})).nodes, ..base.clone() };
        let result = merge(Some(&base), &local, &incoming);
        assert_eq!(result.conflicts.len(), 1);
        assert_eq!(result.conflicts[0].local, None);
        assert!(result.conflicts[0].incoming.is_some());
        let mut resolved = result.document.clone();
        resolve(&mut resolved, &result.conflicts[0], Version::Incoming);
        assert_eq!(resolved.nodes["bot:a"]["x"], 7.0);
    }

    #[test]
    fn entries_are_capped_in_key_order() {
        let mut many = Entries::new();
        for index in 0..(MAX_NODES + 10) {
            many.insert(format!("bot:{index:04}"), json!({"x": index}));
        }
        let document = FlowDocument { nodes: many, ..FlowDocument::default() };
        let result = merge(None, &document, &FlowDocument::default());
        assert_eq!(result.document.nodes.len(), MAX_NODES);
        assert!(result.document.nodes.contains_key("bot:0000"));
        assert!(!result.document.nodes.contains_key(&format!("bot:{MAX_NODES:04}")));
    }
}
