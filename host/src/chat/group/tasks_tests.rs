//! A request's sub-tasks: the tree the room consults, and the lead's drop.

use super::*;
use crate::store::{GroupTask, TaskKind, TaskState};
use serde_json::json;

/// A group with members `a` (its lead) and `b`, and a user request in its lane.
fn fixture() -> (Arc<Hub>, std::path::PathBuf, BotConfig, Lane, Entry) {
    let dir = std::env::temp_dir().join(format!("codync-group-tasks-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let store = Store::open(&dir.join("test.db")).unwrap();
    for id in ["a", "b"] {
        let cfg: BotConfig = serde_json::from_value(json!({
            "id": id, "name": id, "backend": "fixture", "cwd": dir.join(id),
        }))
        .unwrap();
        store.save_bot(&cfg).unwrap();
    }
    let group: BotConfig = serde_json::from_value(json!({
        "id": "5f0c7a52-0000-4000-8000-0000000000cc", "kind": "group", "name": "Ship room",
        "members": ["a", "b"], "chiefBotId": "a",
    }))
    .unwrap();
    store.save_bot(&group).unwrap();
    let lane = Lane::main(&group.id);
    let request = store
        .insert_entry_once("request", &lane, crate::store::EntryKind::User, 1, &json!({"text": "Ship it"}))
        .unwrap()
        .0;
    let hub = Hub::new(
        store,
        "test".into(),
        crate::remote::identity::Identity::load_or_create(&dir).unwrap(),
        "token".into(),
        0,
    );
    (hub, dir, group, lane, request)
}

fn tree(hub: &Hub, group: &str, lane: &Lane) -> Vec<GroupTask> {
    hub.store.group_tasks(group, &lane.key()).unwrap()
}

#[test]
fn a_request_opens_a_tree_its_lead_owns_and_its_round_plans_each_member() {
    let (hub, dir, group, lane, request) = fixture();
    let run = tasks::open(&hub, &group, &lane, &request, true, false).unwrap();
    let rows = tree(&hub, &group.id, &lane);
    assert_eq!(rows.len(), 1, "the request alone");
    assert_eq!(rows[0].kind, TaskKind::Request);
    assert_eq!(rows[0].state, TaskState::Running);
    assert_eq!(rows[0].owner_id.as_deref(), Some("a"), "the lead owns the request");
    assert_eq!(rows[0].instruction, "Ship it");

    tasks::plan(&hub, &run, &[("a".into(), None), ("b".into(), None)], 0).unwrap();
    let rows = tree(&hub, &group.id, &lane);
    assert_eq!(rows.len(), 3, "the request and one row per member");
    assert!(rows[1..].iter().all(|row| row.kind == TaskKind::Assignment && row.state == TaskState::Queued));

    let start = tasks::next(&hub, &run, "b", None, 0).unwrap();
    let tasks::Gate::Start(task) = start else { panic!("b starts") };
    assert_eq!(tree(&hub, &group.id, &lane)[2].state, TaskState::Running);
    tasks::finish(&hub, &group.id, &task, tasks::Outcome::Passed).unwrap();
    assert_eq!(tree(&hub, &group.id, &lane)[2].state, TaskState::Completed);
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn stop_pauses_every_open_row_and_a_paused_root_gates_its_members() {
    let (hub, dir, group, lane, request) = fixture();
    let run = tasks::open(&hub, &group, &lane, &request, true, false).unwrap();
    tasks::plan(&hub, &run, &[("a".into(), None), ("b".into(), None)], 0).unwrap();
    tasks::pause_all(&hub, &group.id);
    let rows = tree(&hub, &group.id, &lane);
    assert!(rows.iter().all(|row| row.state == TaskState::Paused), "{rows:?}");
    assert!(matches!(tasks::next(&hub, &run, "b", None, 0).unwrap(), tasks::Gate::Paused));
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_newer_request_cancels_the_earlier_open_rows() {
    let (hub, dir, group, lane, request) = fixture();
    let run = tasks::open(&hub, &group, &lane, &request, true, false).unwrap();
    tasks::plan(&hub, &run, &[("a".into(), None)], 0).unwrap();
    // The user writes again: the room opens the newer request and nothing of the old one may speak.
    let newer = hub
        .store
        .insert_entry_once("request-2", &lane, crate::store::EntryKind::User, 2, &json!({"text": "Actually this"}))
        .unwrap()
        .0;
    let next = tasks::open(&hub, &group, &lane, &newer, true, false).unwrap();
    assert_ne!(run.id, next.id);
    let rows = tree(&hub, &group.id, &lane);
    assert_eq!(rows.len(), 1, "only the newest request's root");
    assert!(hub.store.group_task(&run.id).unwrap().is_none(), "the replaced tree is gone");
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn drop_cancels_the_whole_run_and_keeps_a_notice_naming_who_asked() {
    let (hub, dir, group, lane, request) = fixture();
    let run = tasks::open(&hub, &group, &lane, &request, true, false).unwrap();
    tasks::plan(&hub, &run, &[("a".into(), None), ("b".into(), None)], 0).unwrap();
    let dropped = tasks::drop_run(&hub, &group, "Ada").unwrap();
    assert_eq!(dropped, 3, "the running and queued sub-tasks");
    let rows = tree(&hub, &group.id, &lane);
    assert!(rows.iter().all(|row| row.state == TaskState::Cancelled), "{rows:?}");
    let notice = hub
        .store
        .messages_after(&lane, 0, 10)
        .unwrap()
        .into_iter()
        .find(|entry| entry.kind == "notice")
        .expect("the channel says who asked");
    let text = notice.data["text"].as_str().unwrap_or_default();
    assert!(text.contains("Ada asked to drop this request"), "{text}");
    std::fs::remove_dir_all(dir).ok();
}
