#![allow(clippy::unwrap_used)]
use super::*;
use crate::remote::identity::Identity;
use crate::store::tests::temp_store;
use crate::store::{MissionStatus, Store};

fn hub() -> (std::sync::Arc<Hub>, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("codync-mission-tools-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let store: Store = temp_store();
    (Hub::new(store, "test".into(), Identity::load_or_create(&dir).unwrap(), "token".into(), 0), dir)
}

fn bot(hub: &Hub, id: &str) {
    hub.store.save_bot(&serde_json::from_value(json!({"id": id, "name": id})).unwrap()).unwrap();
}

#[test]
fn the_tools_are_advertised_with_their_arguments() {
    let list = tools();
    assert_eq!(list.len(), TOOLS.len());
    for tool in &list {
        assert!(TOOLS.contains(&tool["name"].as_str().unwrap()));
        assert!(tool["inputSchema"]["properties"].is_object(), "every tool names what it takes");
    }
}

#[test]
fn a_bot_lists_only_its_own_missions() {
    let (hub, dir) = hub();
    bot(&hub, "b1");
    bot(&hub, "b2");
    let mine = hub.store.create_mission("b1", "mine", "", &[]).unwrap();
    let other = hub.store.create_mission("b2", "theirs", "", &[]).unwrap();

    let answer = call(&hub, "b1", "missions", &json!({})).unwrap();
    assert_eq!(answer["missions"].as_array().unwrap().len(), 1);
    assert_eq!(answer["missions"][0]["id"], mine.id);
    let error = call(&hub, "b1", "missions", &json!({"missionId": other.id})).unwrap_err();
    assert!(error.to_string().contains("another bot"), "{error}");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn done_missions_come_back_only_when_asked() {
    let (hub, dir) = hub();
    bot(&hub, "b1");
    let done = hub.store.create_mission("b1", "old", "", &[]).unwrap();
    hub.store.set_mission_status(&done.id, MissionStatus::Active, None).unwrap();
    hub.store.set_mission_status(&done.id, MissionStatus::Done, None).unwrap();
    hub.store.create_mission("b1", "open", "", &[]).unwrap();

    assert_eq!(call(&hub, "b1", "missions", &json!({})).unwrap()["missions"].as_array().unwrap().len(), 1);
    assert_eq!(call(&hub, "b1", "missions", &json!({"all": true})).unwrap()["missions"].as_array().unwrap().len(), 2);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn moving_a_mission_records_the_checkpoint_and_the_steps() {
    let (hub, dir) = hub();
    bot(&hub, "b1");
    let mission = hub.store.create_mission("b1", "long one", "", &["first".to_owned()]).unwrap();
    let step = mission.steps[0].id.clone();

    let answer = call(
        &hub,
        "b1",
        "update_mission",
        &json!({"missionId": mission.id, "status": "active", "checkpoint": "first is half done",
                "stepId": step, "stepStatus": "active"}),
    )
    .unwrap();
    assert_eq!(answer["mission"]["status"], "active");
    assert_eq!(answer["mission"]["checkpoint"], "first is half done");
    assert_eq!(answer["mission"]["steps"][0]["status"], "active");
    assert!(answer["note"].as_str().unwrap().contains("checkpoint"));

    let answer = call(&hub, "b1", "update_mission", &json!({"missionId": mission.id, "text": "then"})).unwrap();
    assert_eq!(answer["mission"]["steps"].as_array().unwrap().len(), 2);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn another_bots_mission_cannot_be_moved() {
    let (hub, dir) = hub();
    bot(&hub, "b1");
    bot(&hub, "b2");
    let other = hub.store.create_mission("b2", "theirs", "", &[]).unwrap();
    let error = call(&hub, "b1", "update_mission", &json!({"missionId": other.id, "status": "active"})).unwrap_err();
    assert!(error.to_string().contains("another bot"), "{error}");
    assert_eq!(hub.store.mission(&other.id).unwrap().unwrap().status, MissionStatus::Proposed);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_step_recording_needs_the_step_and_its_status() {
    let (hub, dir) = hub();
    bot(&hub, "b1");
    let mission = hub.store.create_mission("b1", "t", "", &["first".to_owned()]).unwrap();
    let step = mission.steps[0].id.clone();
    let error = call(&hub, "b1", "update_mission", &json!({"missionId": mission.id, "stepId": step})).unwrap_err();
    assert!(error.to_string().contains("stepStatus"), "{error}");
    let error =
        call(&hub, "b1", "update_mission", &json!({"missionId": mission.id, "stepId": step, "stepStatus": "nearly"}))
            .unwrap_err();
    assert!(error.to_string().contains("nearly"), "{error}");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_move_broadcasts_the_change() {
    let (hub, dir) = hub();
    bot(&hub, "b1");
    let mission = hub.store.create_mission("b1", "t", "", &[]).unwrap();
    let mut events = hub.events.subscribe();
    call(&hub, "b1", "update_mission", &json!({"missionId": mission.id, "status": "active"})).unwrap();
    let event = events.try_recv().unwrap();
    assert_eq!(event["type"], "mission");
    assert_eq!(event["mission"]["status"], "active");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_missing_mission_id_is_named() {
    let (hub, dir) = hub();
    bot(&hub, "b1");
    let error = call(&hub, "b1", "update_mission", &json!({"status": "active"})).unwrap_err();
    assert!(error.to_string().contains("missionId"), "{error}");
    let error = call(&hub, "b1", "update_mission", &json!({"missionId": "nope"})).unwrap_err();
    assert!(error.to_string().contains("nope"), "{error}");
    let _ = std::fs::remove_dir_all(dir);
}
