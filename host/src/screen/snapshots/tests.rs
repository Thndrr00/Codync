use super::SnapshotState;
use serde_json::json;

fn snapshot(id: &str) -> serde_json::Value {
    json!({"structuredContent": {"snapshot_id": id}})
}

#[test]
fn epoch_zero_preserves_legacy_arguments() {
    let state = SnapshotState::default();
    assert_eq!(state.epoch(), 0);
    assert!(state.validate("bot", &json!({"element_token": "legacy"})).is_ok());
}

#[test]
fn handoff_requires_new_capture_and_rejects_old_window_tokens() {
    let mut state = SnapshotState::default();
    state.record("bot", 0, &snapshot("old-window"));
    state.invalidate();
    assert!(state.validate("bot", &json!({"x": 10, "y": 20})).is_err());
    state.record("bot", state.epoch(), &snapshot("other-window"));
    assert!(state.validate("bot", &json!({"element_token": "old-window:1"})).is_err());
    assert!(state.validate("bot", &json!({"element_token": "other-window:1"})).is_ok());
}

#[test]
fn freshness_and_identifiers_are_isolated_per_bot() {
    let mut state = SnapshotState::default();
    state.invalidate();
    state.record("one", state.epoch(), &snapshot("one-window"));
    assert!(state.validate("two", &json!({})).is_err());
    state.record("two", state.epoch(), &snapshot("two-window"));
    assert!(state.validate("two", &json!({"element_token": "one-window:0"})).is_err());
    state.forget("one");
    assert!(state.validate("one", &json!({})).is_err());
    assert!(state.validate("two", &json!({})).is_ok());
}

#[test]
fn outstanding_captures_cannot_restore_freshness_after_invalidation() {
    let mut state = SnapshotState::default();
    let first = state.epoch();
    state.invalidate();
    let second = state.epoch();
    state.invalidate();
    assert_eq!(state.epoch(), 2);
    state.record("bot", first, &snapshot("first"));
    state.record("bot", second, &snapshot("second"));
    assert!(state.validate("bot", &json!({})).is_err());
    state.record("bot", state.epoch(), &snapshot("current"));
    assert!(state.validate("bot", &json!({"element_token": "current:0"})).is_ok());
}

#[test]
fn invalidated_driver_ids_are_removed_from_every_bot() {
    let mut state = SnapshotState::default();
    state.invalidate();
    state.record("one", state.epoch(), &snapshot("closed"));
    state.record("two", state.epoch(), &snapshot("closed"));
    state.record(
        "one",
        state.epoch(),
        &json!({"structuredContent": {
            "snapshot_id": "open", "invalidated_snapshot_ids": ["closed"]
        }}),
    );
    assert!(state.validate("one", &json!({"element_token": "closed:0"})).is_err());
    assert!(state.validate("one", &json!({"element_token": "open:0"})).is_ok());
    assert!(state.validate("two", &json!({"capture_id": "closed"})).is_err());
}

#[test]
fn bounded_retention_evicts_oldest_ids_and_refreshes_duplicates() {
    let mut state = SnapshotState::default();
    state.invalidate();
    for index in 0..64 {
        state.record("bot", state.epoch(), &snapshot(&format!("s{index}")));
    }
    state.record("bot", state.epoch(), &snapshot("s0"));
    state.record("bot", state.epoch(), &snapshot("s64"));
    assert_eq!(state.bots["bot"].ids.len(), 64);
    assert!(state.validate("bot", &json!({"capture_id": "s1"})).is_err());
    assert!(state.validate("bot", &json!({"capture_id": "s0"})).is_ok());
    assert!(state.validate("bot", &json!({"capture_id": "s64"})).is_ok());
}

#[test]
fn plain_capture_refreshes_pixels_but_error_does_not() {
    let mut state = SnapshotState::default();
    state.invalidate();
    state.record("bot", state.epoch(), &json!({"isError": true, "structuredContent": {"capture_id": 9}}));
    assert!(state.validate("bot", &json!({})).is_err());
    state.record("bot", state.epoch(), &json!({"content": [{"type": "image", "data": "ignored"}]}));
    assert!(state.validate("bot", &json!({"x": 5})).is_ok());
    assert!(state.validate("bot", &json!({"capture_id": 9})).is_err());
}

#[test]
fn numeric_capture_ids_and_colons_in_snapshot_ids_are_supported() {
    let mut state = SnapshotState::default();
    state.invalidate();
    state.record(
        "bot",
        state.epoch(),
        &json!({"structuredContent": {
            "snapshot_id": "window:part", "capture_id": 17, "screenshot_capture_id": "shot"
        }}),
    );
    for args in [json!({"element_token": "window:part:2"}), json!({"capture_id": 17}), json!({"capture_id": "shot"})] {
        assert!(state.validate("bot", &args).is_ok());
    }
    for token in ["window:part:", "window:part:-1", "window:part:abc", "unknown:2"] {
        assert!(state.validate("bot", &json!({"element_token": token})).is_err());
    }
    assert!(state.validate("bot", &json!({"element_token": 2})).is_err());
    assert!(state.validate("bot", &json!({"capture_id": null})).is_err());
}
