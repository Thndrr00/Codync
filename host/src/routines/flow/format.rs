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

use super::merge::Entries;
use crate::store::Store;
use anyhow::Result;
use serde_json::Value;
use std::collections::BTreeSet;

/// The workflow format this host writes. Bump when step kinds or settings are added.
pub const FORMAT_VERSION: u32 = 1;
/// kv prefix of a bot's stored workflow format version, followed by its id.
pub const VERSION_PREFIX: &str = "routineflow.format.";

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
}
