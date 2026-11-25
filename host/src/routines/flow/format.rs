//! Workflow format versions, ported from OpenBot nightly's workflow format rule.
//!
//! A canvas and a `openbot.workflow` file carry the version of the format they were written with.
//! An older client must treat steps and settings it doesn't know as read-only: it sends them back
//! untouched, so a save from that client cannot silently drop them (the server puts them back). A
//! server that is asked to save a setting it does not know refuses with an update message instead
//! of guessing what to do with it. Saved settings also survive a transfer: the ones an older
//! editor leaves out of its save are restored from what is already stored.
//!
//! The version itself lives in one kv slot per bot beside the canvas, so an older host reads a file
//! written by a newer one as a plain value and a newer host keeps the highest version it has seen.
//! Everything here except the two kv wrappers is pure and tested.

use super::document::NodeSize;
use super::merge::Entries;
use crate::store::Store;
use anyhow::{Result, bail};
use serde_json::{Value, json};
use std::collections::BTreeSet;

/// The workflow format this host writes. Bump when step kinds or settings are added. Version 2 adds
/// freehand strokes, per-node size and locking, and typed ports on links.
pub const FORMAT_VERSION: u32 = 2;
/// kv prefix of a bot's stored workflow format version, followed by its id.
pub const VERSION_PREFIX: &str = "routineflow.format.";

/// The longest id, brush or color name a stroke may carry (OpenBot `INPUT_LIMITS.identifier`).
pub const MAX_ID_CHARS: usize = 128;
/// The largest side a node size may name (OpenBot `MAX_SIZE`).
pub const MAX_SIZE: f64 = 100_000.0;
/// OpenBot `INPUT_LIMITS.workflowStrokePoints`.
pub const MAX_STROKE_POINTS: usize = 2_000;
/// OpenBot `INPUT_LIMITS.workflowStrokes`.
pub const MAX_STROKES: usize = 500;

fn bounded_id(value: Option<&Value>) -> bool {
    value
        .and_then(Value::as_str)
        .is_some_and(|id| !id.is_empty() && id.chars().count() <= MAX_ID_CHARS && !id.chars().any(char::is_control))
}

fn bounded_number(value: Option<&Value>) -> bool {
    value.and_then(Value::as_f64).is_some_and(|number| number.is_finite() && number > 0.0 && number <= MAX_SIZE)
}

fn bounded_coordinate(value: Option<&Value>) -> bool {
    value
        .and_then(Value::as_f64)
        .is_some_and(|number| number.is_finite() && number.abs() <= crate::store::flows::MAX_COORDINATE)
}

/// A freehand stroke: an id, a brush and color, a positive width and bounded points. Strokes are
/// drawing data: they are saved and merged but never executed. Kept pure so a save can refuse a bad
/// one before anything is written.
pub fn stroke_valid(value: &Value) -> bool {
    let Some(entry) = value.as_object() else { return false };
    let Some(points) = entry.get("points").and_then(Value::as_array) else { return false };
    bounded_id(entry.get("id"))
        && bounded_id(entry.get("brush"))
        && bounded_id(entry.get("color"))
        && bounded_number(entry.get("width"))
        && points.len() <= MAX_STROKE_POINTS
        && points.iter().all(|point| {
            point.as_object().is_some_and(|p| bounded_coordinate(p.get("x")) && bounded_coordinate(p.get("y")))
        })
}

/// Refuses a stroke that is malformed or whose id does not match its key, before any write.
pub fn check_strokes(entries: &Entries) -> Result<()> {
    for (id, stroke) in entries {
        if !stroke_valid(stroke) {
            bail!("stroke {id} is invalid");
        }
        if stroke.get("id").and_then(Value::as_str) != Some(id.as_str()) {
            bail!("stroke {id} does not match its id");
        }
    }
    Ok(())
}

/// A node's optional width, height and font. `None` means the field is absent or malformed; a node
/// with no size reads as the default one. Every named side must be a positive, finite, bounded number.
pub fn node_size(value: &Value) -> Option<NodeSize> {
    let Some(size) = value.as_object()?.get("size") else { return Some(NodeSize::default()) };
    let object = size.as_object()?;
    let side = |key: &str| -> Option<Option<f64>> {
        match object.get(key) {
            None | Some(Value::Null) => Some(None),
            Some(number) => bounded_number(Some(number)).then_some(number.as_f64()),
        }
    };
    let (width, height, font) = (side("width")?, side("height")?, side("font")?);
    Some(NodeSize { width: width.flatten(), height: height.flatten(), font: font.flatten() })
}

/// A locked node is read-only in the model: its place cannot be moved and it is not applied to the
/// runnable flow tables. Locked note nodes stand in for kinds this host does not know yet.
pub fn node_locked(value: &Value) -> bool {
    value.as_object().and_then(|entry| entry.get("locked")).and_then(Value::as_bool).unwrap_or(false)
}

/// The kinds this host can show. A node or setting naming anything else becomes a locked note.
pub const KNOWN_NODE_KINDS: &[&str] = &[
    "routine",
    "bot",
    "media",
    "note",
    "text",
    "prompt",
    "asset",
    "llm",
    "agent",
    "image",
    "trigger",
    "condition",
    "approval",
    "output",
    "shape",
    "check",
];

/// OpenBot `workflowGraphPlaceholders`: an unknown node kind becomes a locked note with a short
/// reason instead of making the whole workflow unreadable. The kind is truncated to a bounded id.
pub fn placeholder_for(kind: &str) -> Value {
    let short: String = kind.chars().take(MAX_ID_CHARS).collect();
    json!({"kind": "note", "locked": true, "note": format!("This step needs a newer app ({short}).")})
}

/// Projects the entries of a document, replacing unknown kinds with locked placeholders. The bool
/// is `needsNewerApp`: the canvas shows the placeholder and stays readable rather than refusing all.
pub fn project_unknown(entries: &Entries) -> (Entries, bool) {
    let mut projected = Entries::new();
    let mut newer = false;
    for (key, value) in entries {
        let kind = value
            .as_object()
            .and_then(|entry| entry.get("kind").and_then(Value::as_str).map(str::to_owned))
            .or_else(|| key.split_once(':').map(|(prefix, _)| prefix.to_owned()));
        match kind {
            Some(kind) if !KNOWN_NODE_KINDS.contains(&kind.as_str()) => {
                projected.insert(key.clone(), placeholder_for(&kind));
                newer = true;
            }
            _ => {
                projected.insert(key.clone(), value.clone());
            }
        }
    }
    (projected, newer)
}

/// Setting keys this host understands. A workflow setting with any other key came from a newer
/// editor and is read-only here (or refused on a save that introduces it).
pub const KNOWN_SETTING_KEYS: &[&str] = &[
    "id",
    "routineId",
    "kind",
    "model",
    "modelOptions",
    "name",
    "prompt",
    "aspect",
    "duration",
    "resolution",
    "transparent",
    "voice",
    "createdAt",
    "answer",
    "check",
    "timeZone",
];

/// The keys this host can read, as a set.
pub fn known_settings() -> BTreeSet<String> {
    KNOWN_SETTING_KEYS.iter().map(|key| (*key).to_owned()).collect()
}

/// An older client reads a newer workflow as read-only: it may show it, but may not change it.
pub fn client_read_only(client: u32, document: u32) -> bool {
    client < document
}

/// Unknown settings a save introduces that were not already saved on the host. A setting the host
/// already knows about (one an older client carries back unchanged) is read-only, not a refusal.
pub fn new_unknown_settings(local: &Entries, incoming: &Entries) -> Vec<String> {
    let known = known_settings();
    let saved: BTreeSet<&str> =
        incoming.values().filter_map(Value::as_object).flat_map(|object| object.keys().map(String::as_str)).collect();
    let mut unknown: Vec<String> = local
        .values()
        .filter_map(Value::as_object)
        .flat_map(|object| object.keys())
        .filter(|key| !known.contains(*key) && !saved.contains(key.as_str()))
        .cloned()
        .collect();
    unknown.sort();
    unknown.dedup();
    unknown
}

/// The message a server gives when a save introduces a setting it does not know.
pub fn refusal_text(unknown: &[String]) -> String {
    format!("An update is required: this workflow uses settings this version does not know ({}).", unknown.join(", "))
}

/// The saved steps or settings an older client left out of its save. An editor that reads a newer
/// document as read-only sends it back without changing those entries; restoring them here is what
/// keeps a newer step or setting alive through that save and through a server transfer.
pub fn preserved_entries(client: u32, document: u32, local: &Entries, incoming: &Entries) -> Vec<(String, Value)> {
    if !client_read_only(client, document) {
        return Vec::new();
    }
    incoming
        .iter()
        .filter(|(key, _)| !local.contains_key(*key))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

/// Settings already saved that an imported copy of the workflow left out. Importing over an
/// existing workflow must not drop a setting the person configured on this computer.
pub fn surviving_settings(imported: &Entries, saved: &Entries) -> Vec<(String, Value)> {
    saved
        .iter()
        .filter(|(key, _)| !imported.contains_key(*key))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

/// The version stored for `bot`, or the current one for a canvas written before versions existed.
pub fn version(store: &Store, bot: &str) -> u32 {
    store.kv_get(&format!("{VERSION_PREFIX}{bot}")).and_then(|raw| raw.parse().ok()).unwrap_or(FORMAT_VERSION)
}

/// Remembers the highest version seen for `bot`. An older client saving a newer document must not
/// lower it, or the next save would lose the read-only protection.
pub fn save_version(store: &Store, bot: &str, seen: u32) -> Result<()> {
    store.kv_set(&format!("{VERSION_PREFIX}{bot}"), &version(store, bot).max(seen).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn entries(settings: Value) -> Entries {
        serde_json::from_value(settings).expect("a settings map")
    }

    #[test]
    fn an_older_client_reads_a_newer_document_as_read_only() {
        assert!(client_read_only(0, 1));
        assert!(!client_read_only(1, 1));
        assert!(!client_read_only(2, 1), "a newer client understands an older document");
    }

    #[test]
    fn new_unknown_settings_are_named_once_in_sorted_order() {
        let local = entries(json!({
            "s1": {"prompt": "a picture", "futureSetting": 1},
            "s2": {"anotherSetting": true, "futureSetting": 2},
        }));
        assert_eq!(new_unknown_settings(&local, &Entries::new()), ["anotherSetting", "futureSetting"]);
        assert!(new_unknown_settings(&entries(json!({"s1": {"prompt": "known"}})), &Entries::new()).is_empty());
        assert!(refusal_text(&new_unknown_settings(&local, &Entries::new())).contains("update is required"));
    }

    #[test]
    fn only_a_new_unknown_setting_is_refused() {
        let local = entries(json!({"s1": {"futureSetting": 1}}));
        let saved = entries(json!({"s1": {"futureSetting": 1}}));
        assert!(new_unknown_settings(&local, &saved).is_empty(), "carrying a saved setting back is read-only");
        assert_eq!(new_unknown_settings(&local, &Entries::new()), ["futureSetting"]);
    }

    #[test]
    fn a_read_only_client_keeps_the_settings_it_did_not_send() {
        let local = entries(json!({"s1": {"prompt": "a picture"}}));
        let incoming = entries(json!({
            "s1": {"prompt": "a picture"},
            "s2": {"prompt": "from a newer editor", "futureSetting": 1},
        }));
        let kept = preserved_entries(0, 1, &local, &incoming);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].0, "s2");
        assert_eq!(kept[0].1["futureSetting"], 1);
        assert!(preserved_entries(1, 1, &local, &incoming).is_empty(), "a current client owns every setting");
    }

    #[test]
    fn importing_over_a_workflow_keeps_settings_the_file_left_out() {
        let imported = entries(json!({"s1": {"prompt": "in the file"}}));
        let saved = entries(json!({"s2": {"prompt": "saved here"}}));
        let kept = surviving_settings(&imported, &saved);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].0, "s2");
    }

    #[test]
    fn a_stored_version_never_goes_backwards() {
        let store = Store::open(std::path::Path::new(":memory:")).expect("a store");
        assert_eq!(version(&store, "b"), FORMAT_VERSION);
        save_version(&store, "b", 4).expect("a version save");
        assert_eq!(version(&store, "b"), 4);
        save_version(&store, "b", 2).expect("an older client's save");
        assert_eq!(version(&store, "b"), 4);
    }

    #[test]
    fn a_stroke_is_bounded_in_width_color_and_points() {
        let good = json!({
            "id": "s1", "brush": "pen", "color": "#123456", "width": 2.5,
            "points": [{"x": 1.0, "y": 2.0}, {"x": -3.0, "y": 4.0}]
        });
        assert!(stroke_valid(&good));
        let mut long = good.clone();
        long["points"] = json!(vec![json!({"x": 0, "y": 0}); MAX_STROKE_POINTS + 1]);
        assert!(!stroke_valid(&long), "too many points");
        let mut wide = good.clone();
        wide["width"] = json!(MAX_SIZE + 1.0);
        assert!(!stroke_valid(&wide), "an unbounded width");
        let mut color = good.clone();
        color["color"] = json!("x".repeat(MAX_ID_CHARS + 1));
        assert!(!stroke_valid(&color), "an unbounded color");
        let mut far = good.clone();
        far["points"] = json!([{"x": 1e9, "y": 0.0}]);
        assert!(!stroke_valid(&far), "a point outside the canvas");
        assert!(!stroke_valid(&json!({"id": "s", "brush": "pen", "color": "red", "width": 1})), "no points");
    }

    #[test]
    fn node_size_accepts_only_positive_bounded_sides() {
        assert_eq!(node_size(&json!({"x": 1.0})), Some(NodeSize::default()), "an old node has no size");
        let sized = json!({"size": {"width": 200.0, "font": 14.0}});
        assert_eq!(node_size(&sized), Some(NodeSize { width: Some(200.0), height: None, font: Some(14.0) }));
        assert_eq!(node_size(&json!({"size": {"width": 0.0}})), None, "a zero side is not a size");
        assert_eq!(node_size(&json!({"size": {"height": -1.0}})), None);
        assert_eq!(node_size(&json!({"size": {"font": "big"}})), None);
        assert_eq!(node_size(&json!({"size": {"width": MAX_SIZE + 1.0}})), None);
    }

    #[test]
    fn a_locked_node_is_named_only_by_a_true_flag() {
        assert!(node_locked(&json!({"locked": true})));
        assert!(!node_locked(&json!({"locked": false})));
        assert!(!node_locked(&json!({})));
        assert!(!node_locked(&json!({"locked": "true"})));
    }

    #[test]
    fn an_unknown_kind_becomes_a_locked_note_with_a_flag() {
        let placeholder = placeholder_for("future-step");
        assert_eq!(placeholder["kind"], "note");
        assert_eq!(placeholder["locked"], true);
        assert!(placeholder["note"].as_str().is_some_and(|text| text.contains("future-step")));
        assert!(
            placeholder_for(&"x".repeat(MAX_ID_CHARS + 10))["note"]
                .as_str()
                .is_some_and(|text| text.chars().count() < MAX_ID_CHARS * 2)
        );

        let entries = entries(json!({
            "bot:a": {"x": 1.0, "y": 2.0},
            "future:1": {"kind": "future-step", "x": 3.0},
            "bot:b": {"x": 4.0, "y": 5.0, "locked": true}
        }));
        let (projected, newer) = project_unknown(&entries);
        assert!(newer);
        assert_eq!(projected["bot:a"], entries["bot:a"]);
        assert_eq!(projected["bot:b"], entries["bot:b"], "a known locked node stays as it was");
        assert_eq!(projected["future:1"]["kind"], "note");
        assert_eq!(projected["future:1"]["locked"], true);

        let known = entries(json!({"bot:a": {"x": 1.0, "y": 2.0}}));
        let (same, newer) = project_unknown(&known);
        assert!(!newer && same == known);
    }
}
