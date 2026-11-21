//! Auto-approve preference per agent backend. One host setting, stored in the existing KV table,
//! maps a backend (`claude`, `codex`, `hermes`…) to a bool: a person may let only the agents they
//! trust skip ordinary approval cards. The guard below is pure, and the cards it names are always
//! manual, so no setting can auto-approve a deletion, a credential or computer use. No second store.

use crate::store::Store;
use anyhow::{Result, bail};
use serde_json::Value;
use std::collections::BTreeMap;

/// The one KV row holding the whole map, as JSON.
pub const KEY: &str = "approvalAutomation";

/// Reads the map from the host's KV row. Bad or missing data is an empty preference: never a reason
/// to auto-approve more than the person chose. `parse` is the pure part, tested on its own.
pub fn load(store: &Store) -> BTreeMap<String, bool> {
    parse(store.kv_get(KEY).as_deref())
}

pub fn parse(raw: Option<&str>) -> BTreeMap<String, bool> {
    raw.and_then(|v| serde_json::from_str::<BTreeMap<String, bool>>(v).ok()).unwrap_or_default()
}

/// Turns one backend on or off and returns the whole map. Only `true` entries are kept, so the
/// stored form says exactly which agents are opted in.
pub fn save(store: &Store, backend: &str, enabled: bool) -> Result<BTreeMap<String, bool>> {
    let backend = backend.trim();
    if backend.is_empty() {
        bail!("backend is required");
    }
    let mut all = load(store);
    if enabled {
        all.insert(backend.to_owned(), true);
    } else {
        all.remove(backend);
    }
    store.kv_set(KEY, &serde_json::to_string(&all)?)?;
    Ok(all)
}

/// Whether this backend was opted in.
pub fn enabled(map: &BTreeMap<String, bool>, backend: &str) -> bool {
    map.get(backend).copied().unwrap_or(false)
}

/// The cards no automation may approve. Reuses the signals the existing approval code already
/// classifies on: `kind: "deleteAgents"` and the "Delete agent(s)" titles of
/// `market::bot_deletion`; `secret`/`vault` as `agent::bot::turn_rules::Question::secret` does;
/// and the `computer` tool kind of the computer server. A public post (`market::feedback`) also
/// asks its own card and is never automatic, whatever a backend's preference says.
pub fn always_manual(card: &Value) -> bool {
    let kind = card["kind"].as_str().unwrap_or_default();
    let title = card["title"].as_str().unwrap_or_default();
    let tool_kind = card["toolKind"].as_str().unwrap_or_default();
    kind == "deleteAgents"
        || matches!(title, "Delete an agent" | "Delete agents")
        || card["secret"].as_bool() == Some(true)
        || card["vault"].is_object()
        || tool_kind == "computer"
        || title == "Send a public bug report or idea?"
}

/// The whole decision: an opted-in backend still gets a card for anything in `always_manual`.
pub fn should_auto_approve(map: &BTreeMap<String, bool>, backend: &str, card: &Value) -> bool {
    enabled(map, backend) && !always_manual(card)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn only_opted_in_backends_auto_approve() {
        let map = BTreeMap::from([("codex".to_owned(), true)]);
        let ordinary = json!({"title": "Edit src/main.rs", "toolKind": "edit"});
        assert!(should_auto_approve(&map, "codex", &ordinary));
        assert!(!should_auto_approve(&map, "claude", &ordinary), "a backend not chosen still asks");
        assert!(!should_auto_approve(&BTreeMap::new(), "codex", &ordinary));
        assert!(!enabled(&map, "hermes"));
    }

    #[test]
    fn deletion_credential_and_computer_cards_are_always_manual() {
        let map = BTreeMap::from([("codex".to_owned(), true), ("claude".to_owned(), true)]);
        let deletion = json!({"kind": "deleteAgents", "title": "Delete agents"});
        assert!(always_manual(&deletion));
        assert!(always_manual(&json!({"title": "Delete an agent"})));
        assert!(!should_auto_approve(&map, "codex", &deletion));

        assert!(always_manual(&json!({"title": "Save a login", "secret": true})));
        assert!(always_manual(&json!({"title": "Fill a field", "vault": {}})));
        assert!(!should_auto_approve(&map, "claude", &json!({"title": "Save a login", "secret": true})));

        let computer = json!({"title": "Click the button", "toolKind": "computer"});
        assert!(always_manual(&computer));
        assert!(!should_auto_approve(&map, "codex", &computer));

        assert!(always_manual(&json!({"title": "Send a public bug report or idea?"})));
        assert!(!should_auto_approve(&map, "codex", &json!({"title": "Send a public bug report or idea?"})));
    }

    #[test]
    fn an_ordinary_card_is_auto_approved_only_when_its_backend_is_on() {
        let map = BTreeMap::new();
        let edit = json!({"title": "Edit a.txt", "toolKind": "edit"});
        assert!(!always_manual(&edit), "an edit is not a sensitive card");
        assert!(!should_auto_approve(&map, "claude", &edit));
        let map = BTreeMap::from([("claude".to_owned(), true)]);
        assert!(should_auto_approve(&map, "claude", &edit));
    }

    #[test]
    fn a_missing_or_malformed_preference_defaults_to_none() {
        assert!(parse(None).is_empty());
        assert!(parse(Some("{not json")).is_empty());
        assert!(parse(Some("[]")).is_empty());
        let map = parse(Some(r#"{"codex":true,"claude":false}"#));
        assert!(enabled(&map, "codex"));
        assert!(!enabled(&map, "claude"), "a stored false is not an opt-in");
    }

    #[test]
    fn the_public_shape_is_a_backend_to_bool_map() {
        let map = BTreeMap::from([("codex".to_owned(), true)]);
        assert_eq!(json!(map), json!({"codex": true}));
        assert_eq!(KEY, "approvalAutomation");
    }
}
