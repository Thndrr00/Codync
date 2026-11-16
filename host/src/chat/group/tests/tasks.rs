//! A request's sub-tasks and the lead's drop: the tree the room consults, and its four controls.

use super::*;
use crate::api::devices::Caller;
use crate::hub::BotStatus;
use crate::store::{Lane, TaskKind, TaskState};
use serde_json::json;

/// `room`'s task rows in the chat's lane, the request first.
fn tree(room: &Room) -> Vec<crate::store::GroupTask> {
    room.hub.store.group_tasks(&room.group, &room.group).unwrap()
}

/// The sub-task actions a client calls (`groupTask`).
async fn group_task(room: &Room, task: &str, action: &str, owner: Option<&str>) {
    let mut body = json!({"botId": room.group, "taskId": task, "action": action});
    if let Some(owner) = owner {
        body["ownerId"] = owner.into();
    }
    crate::api::dispatch(&room.hub, &Caller::Local, "groupTask", body).await.unwrap();
}

#[tokio::test]
async fn stop_pauses_the_request_tree_and_the_next_speaker_does_not_start() {
    let room = Room::new();
    room.send("BLOCK everyone look", None).await;
    room.until(|| room.hub.runtime("a").status == BotStatus::Working, "a starts").await;
    room.until(|| tree(&room).len() == 3, "the request, a's row and b's row").await;
    let root = tree(&room)[0].id.clone();

    // Stop on the request pauses everything below it, members that had not started included.
    group_task(&room, &root, "stop", None).await;
    room.settle().await;
    assert!(tree(&room).iter().all(|row| row.state == TaskState::Paused), "{:?}", tree(&room));
    assert!(!room.asked("b"), "b never got a turn while the tree is stopped");

    // Continue asks the rows that waited again: a finishes, then b.
    std::fs::write(room.dir.join("a/release"), "").unwrap();
    group_task(&room, &root, "continue", None).await;
    room.until(|| room.asked("b"), "b is asked after Continue").await;
    room.settle().await;
    assert_eq!(
        tree(&room).iter().filter(|row| row.state == TaskState::Completed).count(),
        3,
        "the request and both assignments finished: {:?}",
        tree(&room)
    );
    room.shutdown().await;
}

#[tokio::test]
async fn reassign_gives_a_stopped_row_another_member_and_resumes_what_waited() {
    let room = Room::new();
    room.send("BLOCK everyone look", None).await;
    room.until(|| room.hub.runtime("a").status == BotStatus::Working, "a starts").await;
    room.until(|| tree(&room).len() == 3, "the tree").await;
    let rows = tree(&room);
    let (root, a_row) = (rows[0].id.clone(), rows[1].id.clone());
    group_task(&room, &root, "stop", None).await;
    room.settle().await;
    assert!(!room.asked("b"), "nothing runs while the tree is stopped");

    std::fs::write(room.dir.join("a/release"), "").unwrap();
    group_task(&room, &a_row, "reassign", Some("b")).await;
    room.until(|| room.asked("b"), "b takes the reassigned row").await;
    room.settle().await;
    let rows = tree(&room);
    assert_eq!(rows[1].owner_id.as_deref(), Some("b"), "the row belongs to b now");
    assert!(rows[1].state == TaskState::Completed, "{:?}", rows[1]);
    assert!(rows[2].state == TaskState::Completed, "the row b had waited too: {:?}", rows[2]);
    assert_eq!(
        room.replies(&Lane::main(&room.group)).iter().filter(|(who, _)| who == "b").count(),
        1,
        "only b answered"
    );
    room.shutdown().await;
}

#[tokio::test]
async fn cancel_ends_the_request_and_every_row_below_it_and_keeps_the_messages() {
    let room = Room::new();
    room.send("BLOCK everyone look", None).await;
    room.until(|| room.hub.runtime("a").status == BotStatus::Working, "a starts").await;
    room.until(|| tree(&room).len() == 3, "the tree").await;
    let a_row = tree(&room)[1].id.clone();
    // Cancel a row ends its whole run, from the request down: a parent that waits on a child can't hang.
    group_task(&room, &a_row, "cancel", None).await;
    room.settle().await;
    assert!(tree(&room).iter().all(|row| row.state == TaskState::Cancelled), "{:?}", tree(&room));
    assert!(!room.asked("b"), "no member turn starts after Cancel");
    let messages = room.hub.store.messages_after(&Lane::main(&room.group), 0, 100).unwrap();
    assert!(messages.iter().any(|e| e.kind == "user" && e.data["text"] == "BLOCK everyone look"), "the messages stay");
    room.shutdown().await;
}

#[tokio::test]
async fn dropping_a_request_cancels_its_sub_tasks_and_names_who_asked() {
    let room = Room::new();
    room.send("BLOCK everyone look", None).await;
    room.until(|| room.hub.runtime("a").status == BotStatus::Working, "a starts").await;
    room.until(|| tree(&room).len() == 3, "two sub-tasks under the request").await;
    let group = room.hub.store.bot(&room.group).unwrap().unwrap().config;
    let dropped = crate::chat::group::tasks::drop_run(&room.hub, &group, "Ada").unwrap();
    assert_eq!(dropped, 3, "the request and both assignments");
    room.settle().await;
    assert!(tree(&room).iter().all(|row| row.state == TaskState::Cancelled), "{:?}", tree(&room));
    assert!(
        room.replies(&Lane::main(&room.group)).iter().all(|(_, text)| text != "reply: group"),
        "both member turns are cancelled"
    );
    let messages = room.hub.store.messages_after(&Lane::main(&room.group), 0, 100).unwrap();
    let note = messages.iter().find(|e| e.kind == "notice").expect("the channel says who asked");
    assert_eq!(note.data["text"], "Ada asked to drop this request: BLOCK everyone look");
    assert!(messages.iter().any(|e| e.kind == "user"), "the messages stay");
    room.shutdown().await;
}

#[tokio::test]
async fn a_lead_drops_a_request_and_a_denied_call_changes_nothing() {
    let dir = std::env::temp_dir().join(format!("codync-channel-drop-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let hub = tool_hub(&dir);
    let channel = hub
        .create_bot(
            serde_json::from_value(
                json!({"id": "", "kind": "group", "name": "Crew", "members": ["lead", "dev", "mgr"],
                "chiefBotId": "lead"}),
            )
            .unwrap(),
        )
        .unwrap();
    let id = channel["id"].as_str().unwrap().to_owned();
    // A request with two sub-tasks, one running.
    let request = crate::store::new_task(
        &id,
        &id,
        "r",
        "",
        None,
        Some("lead"),
        TaskKind::Request,
        TaskState::Running,
        0,
        "Ship it",
    );
    let mut root = request;
    root.root_id.clone_from(&root.id);
    hub.store.save_group_task(&root).unwrap();
    for (member, state) in [("dev", TaskState::Running), ("mgr", TaskState::Queued)] {
        let row = crate::store::new_task(
            &id,
            &id,
            "r",
            &root.id,
            Some(&root.id),
            Some(member),
            TaskKind::Assignment,
            state,
            0,
            "Work",
        );
        hub.store.save_group_task(&row).unwrap();
    }
    let dropped = crate::chat::group::drop::call(&hub, "lead", "drop_request", &json!({"channelId": id})).await;
    assert!(dropped.is_ok(), "{dropped:?}");
    let rows = hub.store.group_tasks(&id, &id).unwrap();
    assert!(rows.iter().all(|row| row.state == TaskState::Cancelled), "{rows:?}");
    // Under Ask the same call waits on the card; an idle lead that nobody answers refuses it.
    hub.update_bot(&json!({"id": "lead", "permission": "ask"})).unwrap();
    let second = hub
        .create_bot(
            serde_json::from_value(json!({"id": "", "kind": "group", "name": "Second", "members": ["lead", "mgr"],
                "chiefBotId": "lead"}))
            .unwrap(),
        )
        .unwrap();
    let second = second["id"].as_str().unwrap().to_owned();
    let mut open = crate::store::new_task(
        &second,
        &second,
        "r2",
        "",
        None,
        Some("lead"),
        TaskKind::Request,
        TaskState::Running,
        0,
        "Keep it",
    );
    open.root_id.clone_from(&open.id);
    hub.store.save_group_task(&open).unwrap();
    let denied = crate::chat::group::drop::call(&hub, "lead", "drop_request", &json!({"channelId": second}))
        .await
        .unwrap_err()
        .to_string();
    assert!(denied.contains("didn't allow this change"), "{denied}");
    assert_eq!(hub.store.group_tasks(&second, &second).unwrap()[0].state, TaskState::Running, "nothing changed");
    hub.shutdown().await;
    std::fs::remove_dir_all(&dir).unwrap();
}
