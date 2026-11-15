#![allow(clippy::unwrap_used)]
use super::*;
use crate::remote::identity::Identity;
use crate::store::tests::temp_store;
use crate::store::{MissionStepStatus, Store};

fn hub() -> (std::sync::Arc<Hub>, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("codync-missions-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let store: Store = temp_store();
    (Hub::new(store, "test".into(), Identity::load_or_create(&dir).unwrap(), "token".into(), 0), dir)
}

fn create(hub: &Hub, bot: &str, title: &str) -> Value {
    call(hub, "createMission", &json!({"botId": bot, "title": title, "goal": "the long goal", "steps": ["one"]}))
        .unwrap()
}

fn id(value: &Value) -> String {
    value["mission"]["id"].as_str().unwrap().to_owned()
}

#[test]
fn a_mission_is_created_reached_by_id_and_broadcast() {
    let (hub, dir) = hub();
    let mut events = hub.events.subscribe();
    let answer = create(&hub, "b1", "  ship it  ");
    let mission = &answer["mission"];
    assert_eq!(mission["title"], "ship it");
    assert_eq!(mission["status"], "proposed");
    assert_eq!(mission["steps"].as_array().unwrap().len(), 1);
    let event = events.try_recv().unwrap();
    assert_eq!(event["type"], "mission", "a second client refreshes its list");

    let fetched = call(&hub, "mission", &json!({"id": id(&answer)})).unwrap();
    assert_eq!(fetched["mission"]["goal"], "the long goal");
    let listed = call(&hub, "missions", &json!({"botId": "b1"})).unwrap();
    assert_eq!(listed["missions"].as_array().unwrap().len(), 1);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn an_unknown_mission_is_named_in_the_error() {
    let (hub, dir) = hub();
    let error = call(&hub, "mission", &json!({"id": "nope"})).unwrap_err();
    assert!(error.to_string().contains("nope"), "{error}");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_status_checkpoint_and_step_are_recorded() {
    let (hub, dir) = hub();
    let answer = create(&hub, "b1", "t");
    let mission = id(&answer);
    let step = answer["mission"]["steps"][0]["id"].as_str().unwrap().to_owned();

    let moved = call(&hub, "updateMission", &json!({"id": mission, "status": "active"})).unwrap();
    assert_eq!(moved["mission"]["status"], "active");
    let moved = call(&hub, "updateMission", &json!({"id": mission, "checkpoint": "half way"})).unwrap();
    assert_eq!(moved["mission"]["checkpoint"], "half way");

    let recorded = call(&hub, "missionStep", &json!({"id": mission, "stepId": step, "status": "done"})).unwrap();
    assert_eq!(recorded["mission"]["steps"][0]["status"], "done");
    let added = call(&hub, "missionStep", &json!({"id": mission, "text": "and then"})).unwrap();
    assert_eq!(added["mission"]["steps"].as_array().unwrap().len(), 2);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn an_unknown_status_or_step_status_is_refused() {
    let (hub, dir) = hub();
    let answer = create(&hub, "b1", "t");
    let mission = id(&answer);
    let error = call(&hub, "updateMission", &json!({"id": mission, "status": "nearly"})).unwrap_err();
    assert!(error.to_string().contains("nearly"), "{error}");
    let error = call(&hub, "missionStep", &json!({"id": mission, "stepId": "x", "status": "done"})).unwrap_err();
    assert!(error.to_string().contains("step"), "{error}");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn an_update_with_nothing_to_change_says_so() {
    let (hub, dir) = hub();
    let mission = id(&create(&hub, "b1", "t"));
    let error = call(&hub, "updateMission", &json!({"id": mission})).unwrap_err();
    assert!(error.to_string().contains("give a status"), "{error}");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn abandoning_needs_a_reason_and_keeps_it() {
    let (hub, dir) = hub();
    let mission = id(&create(&hub, "b1", "t"));
    assert!(call(&hub, "updateMission", &json!({"id": mission, "status": "abandoned"})).is_err());
    let dropped =
        call(&hub, "updateMission", &json!({"id": mission, "status": "abandoned", "stopReason": "no longer asked"}))
            .unwrap();
    assert_eq!(dropped["mission"]["stopReason"], "no longer asked");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn every_move_the_store_knows_is_reachable_through_the_api() {
    let (hub, dir) = hub();
    let mission = id(&create(&hub, "b1", "t"));
    for status in ["active", "waiting", "done"] {
        let answer = call(&hub, "updateMission", &json!({"id": mission, "status": status})).unwrap();
        assert_eq!(answer["mission"]["status"], status);
    }
    let linked = call(&hub, "updateMission", &json!({"id": mission, "runId": "run-1"})).unwrap();
    assert_eq!(linked["mission"]["runId"], "run-1");
    let linked = call(&hub, "updateMission", &json!({"id": mission, "approvalId": "entry-1"})).unwrap();
    assert_eq!(linked["mission"]["approvalId"], "entry-1");
    let linked = call(&hub, "updateMission", &json!({"id": mission, "approvalId": null})).unwrap();
    assert!(linked["mission"]["approvalId"].is_null());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn finishing_broadcasts_the_finished_mission() {
    let (hub, dir) = hub();
    hub.store.save_bot(&serde_json::from_value(json!({"id": "b1", "name": "b1"})).unwrap()).unwrap();
    let mission = id(&create(&hub, "b1", "t"));
    call(&hub, "updateMission", &json!({"id": mission, "status": "active"})).unwrap();
    let mut events = hub.events.subscribe();
    let answer = call(&hub, "updateMission", &json!({"id": mission, "status": "done"})).unwrap();
    assert_eq!(answer["mission"]["status"], "done");
    let event = events.try_recv().unwrap();
    assert_eq!(event["mission"]["status"], "done");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn step_statuses_parse_the_names_the_clients_send() {
    for name in ["pending", "active", "done", "skipped"] {
        assert!(MissionStepStatus::parse(name).is_some());
    }
    assert!(MissionStepStatus::parse("nah").is_none());
}
