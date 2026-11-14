//! View ids and delta application (card K4).

use super::*;
use serde_json::json;

#[test]
fn view_ids_name_a_bot_and_a_kind() {
    let chat = ViewId::parse("chat:b1").unwrap();
    assert_eq!(chat.kind, Kind::Chat);
    assert_eq!(chat.as_str(), "chat:b1");
    let routine = ViewId::parse("routine:b1").unwrap();
    assert_eq!(routine.kind, Kind::Routine);
    for bad in ["b1", "board:b1", "chat:", "chat:a\nb"] {
        assert!(ViewId::parse(bad).is_err(), "{bad}");
    }
}

#[test]
fn deltas_set_remove_and_append() {
    let mut state = json!({"open": true, "selected": {"kind": "routine", "id": "r1"}});
    apply(
        &mut state,
        &json!([
            {"op": "set", "path": "/thread", "value": "root-1"},
            {"op": "set", "path": "/selected/id", "value": "r2"},
            {"op": "remove", "path": "/open"},
            {"op": "set", "path": "/recent", "value": ["a"]},
            {"op": "append", "path": "/recent", "value": "b"}
        ]),
    )
    .unwrap();
    assert_eq!(state, json!({"selected": {"kind": "routine", "id": "r2"}, "thread": "root-1", "recent": ["a", "b"]}));
}

#[test]
fn deltas_refuse_bad_paths_unknown_ops_and_unknown_props() {
    for ops in [
        json!([{"op": "set", "path": "thread", "value": 1}]),
        json!([{"op": "set", "path": "/a/b/c/d/e", "value": 1}]),
        json!([{"op": "replace", "path": "/a", "value": 1}]),
        json!([{"op": "set", "path": "/a"}]),
        json!([{"op": "append", "path": "/a", "value": 1}]),
        json!([{"op": "set", "path": "/a", "value": 1, "extra": 2}]),
        json!([]),
    ] {
        let mut state = json!({});
        assert!(apply(&mut state, &ops).is_err(), "{ops}");
    }
    let mut state = json!({"a": 1});
    assert!(apply(&mut state, &json!([{"op": "set", "path": "/a", "value": 2}])).is_ok());
}
