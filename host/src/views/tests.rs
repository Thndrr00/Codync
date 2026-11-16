//! The view API and named actions (card K4), through the same dispatch the apps use.

use super::*;
use crate::api::devices::Caller;
use crate::hub::Hub;
use crate::store::{BotConfig, Store};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::Arc;

/// A host with one bot `a`, no agent started.
struct Fixture {
    hub: Arc<Hub>,
    dir: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("codync-views-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::open(&dir.join("test.db")).unwrap();
        for id in ["a", "b"] {
            let cwd = dir.join(id);
            std::fs::create_dir_all(&cwd).unwrap();
            store
                .save_bot(
                    &serde_json::from_value::<BotConfig>(json!({
                        "id": id, "name": id.to_uppercase(), "backend": "fixture", "cwd": cwd, "notify": false,
                    }))
                    .unwrap(),
                )
                .unwrap();
        }
        let identity = crate::remote::identity::Identity::load_or_create(&dir).unwrap();
        let hub = Hub::new(store, "test-host".into(), identity, "test-token".into(), 19223);
        Self { hub, dir }
    }

    async fn call(&self, method: &str, body: Value) -> anyhow::Result<Value> {
        crate::api::dispatch(&self.hub, &Caller::Local, method, body).await
    }

    fn tool(&self, args: Value) -> anyhow::Result<Value> {
        call(&self.hub, "a", "view", &args)
    }

    fn shutdown(self) {
        // Close the database before removing its folder (Windows keeps an open file locked).
        drop(self.hub);
        let _ = std::fs::remove_dir_all(self.dir);
    }
}

#[tokio::test]
async fn a_delta_applies_and_stamps_a_new_version() {
    let f = Fixture::new();
    let first = f
        .call(
            "patchViewState",
            json!({"view": "chat:a", "baseRev": 0, "ops": [
                {"op": "set", "path": "/thread", "value": "root-1"},
                {"op": "set", "path": "/open", "value": true}
            ]}),
        )
        .await
        .unwrap();
    assert_eq!(first["rev"], 1);
    assert_eq!(first["state"], json!({"thread": "root-1", "open": true}));

    let read = f.call("readViewState", json!({"view": "chat:a"})).await.unwrap();
    assert_eq!(read["rev"], 1);
    assert_eq!(read["state"]["thread"], "root-1");

    // The next delta from the same version lands on rev 2.
    let second = f
        .call("patchViewState", json!({"view": "chat:a", "baseRev": 1, "ops": [{"op": "remove", "path": "/thread"}]}))
        .await
        .unwrap();
    assert_eq!(second["rev"], 2);
    assert!(second["state"].get("thread").is_none());

    // A stale client is told the current version and nothing changes.
    let stale = f
        .call(
            "patchViewState",
            json!({"view": "chat:a", "baseRev": 1, "ops": [{"op": "set", "path": "/thread", "value": "old"}]}),
        )
        .await
        .unwrap();
    assert_eq!(stale["conflict"], true);
    assert_eq!(stale["rev"], 2);
    assert!(stale["state"].get("thread").is_none());
    f.shutdown();
}

#[tokio::test]
async fn unknown_views_and_actions_are_refused() {
    let f = Fixture::new();
    let unknown_view = f.call("readViewState", json!({"view": "routine:missing"})).await.unwrap_err();
    assert!(unknown_view.to_string().contains("unknown bot"), "{unknown_view:#}");

    let unknown_kind = f.call("readViewState", json!({"view": "canvas:a"})).await.unwrap_err();
    assert!(unknown_kind.to_string().contains("kind of view"), "{unknown_kind:#}");

    let chat_action = f
        .tool(json!({"op": "act", "view": "chat:a", "action": "create_routine", "arguments": {"name": "x"}}))
        .unwrap_err();
    assert!(chat_action.to_string().contains("it declares: open_thread"), "{chat_action:#}");

    let another_bot = f.tool(json!({"op": "read", "view": "chat:b"})).unwrap_err();
    assert!(another_bot.to_string().contains("isn't your view"), "{another_bot:#}");

    let missing_schedule = f
        .tool(json!({"op": "act", "view": "routine:a", "action": "create_routine",
                     "arguments": {"name": "Weekly review", "instruction": "Write it"}}))
        .unwrap_err();
    assert!(missing_schedule.to_string().contains("give a `schedule`"), "{missing_schedule:#}");
    f.shutdown();
}

#[tokio::test]
async fn the_agent_creates_a_routine_through_the_view_action() {
    let f = Fixture::new();
    let answer = f
        .tool(json!({
            "op": "act",
            "view": "routine:a",
            "action": "create_routine",
            "arguments": {
                "name": "Monday review",
                "instruction": "Write the weekly review",
                "schedule": {"kind": "cron", "calendarStyle": "weekly", "weekday": 1, "hour": 8, "minute": 0,
                             "zone": "Europe/Madrid"}
            }
        }))
        .unwrap();
    let id = answer["result"]["routineId"].as_str().unwrap();
    assert_eq!(answer["state"]["routineId"], id);
    assert_eq!(answer["rev"], 1);

    // It went through the routine API: the routine is saved, enabled and owned by the bot.
    let routines = f.hub.routines.list("a", |_| false);
    let saved = routines["routines"].as_array().unwrap().iter().find(|r| r["id"] == id).expect("saved");
    assert_eq!(saved["name"], "Monday review");
    assert_eq!(saved["enabled"], true);

    // And the view says the canvas shows it: a new client reading the view gets the same routine.
    let read = f.tool(json!({"op": "read", "view": "routine:a"})).unwrap();
    assert_eq!(read["state"]["routineId"], id);
    assert!(read["actions"].as_array().unwrap().iter().any(|action| action.as_str() == Some("create_routine")));
    f.shutdown();
}

#[tokio::test]
async fn two_agents_lanes_merge_and_survive_a_conflict() {
    let f = Fixture::new();
    let routine = "shared".to_owned();
    let path = |bot: &str| format!("/lanes/{routine}/{bot}");

    // Each agent writes its own lane; the second write sees the first one's lane.
    let first = f
        .call(
            "patchViewState",
            json!({"view": "routine:a", "baseRev": 0, "ops": [{"op": "set", "path": path("a"), "value": {"seen": true}}]}),
        )
        .await
        .unwrap();
    let second = f
        .call(
            "patchViewState",
            json!({"view": "routine:a", "baseRev": first["rev"], "ops": [{"op": "set", "path": path("b"), "value": {"seen": true}}]}),
        )
        .await
        .unwrap();
    assert_eq!(second["state"]["lanes"][routine.as_str()]["a"], json!({"seen": true}));
    assert_eq!(second["state"]["lanes"][routine.as_str()]["b"], json!({"seen": true}));

    // A stale writer is told the version and keeps both lanes; re-sending its delta adds nothing.
    let stale = f
        .call(
            "patchViewState",
            json!({"view": "routine:a", "baseRev": 0, "ops": [{"op": "set", "path": path("a"), "value": {"seen": false}}]}),
        )
        .await
        .unwrap();
    assert_eq!(stale["conflict"], true);
    assert_eq!(stale["state"]["lanes"][routine.as_str()]["a"], json!({"seen": true}));
    assert_eq!(stale["state"]["lanes"][routine.as_str()]["b"], json!({"seen": true}));

    // A new client reads the same two lanes.
    let read = f.call("readViewState", json!({"view": "routine:a"})).await.unwrap();
    assert_eq!(read["rev"], second["rev"]);
    assert_eq!(read["state"]["lanes"][routine.as_str()]["b"], json!({"seen": true}));
    f.shutdown();
}

#[tokio::test]
async fn an_agent_focuses_its_lane_and_not_another_cards() {
    let f = Fixture::new();
    let routine = f
        .tool(json!({
            "op": "act", "view": "routine:a", "action": "create_routine",
            "arguments": {"name": "Shared review", "instruction": "Write it",
                "triggers": [{"type": "interval", "seconds": 86400}]}
        }))
        .unwrap()["result"]["routineId"]
        .as_str()
        .unwrap()
        .to_owned();
    f.hub
        .store
        .flow_add_link(
            &crate::store::FlowLink {
                id: uuid::Uuid::new_v4().to_string(),
                routine_id: routine.clone(),
                owner_bot_id: "a".into(),
                from_bot_id: "a".into(),
                to_bot_id: "b".into(),
                instruction: String::new(),
                model: None,
                model_options: std::collections::BTreeMap::new(),
                created_at: 1,
            },
            |_| Ok(()),
        )
        .unwrap();

    let answer = f
        .tool(json!({"op": "act", "view": "routine:a", "action": "focus_lane",
                     "arguments": {"routineId": routine, "botId": "b"}}))
        .unwrap();
    assert_eq!(answer["state"]["selected"], json!({"kind": "lane", "routineId": routine, "botId": "b"}));
    assert_eq!(answer["result"]["botId"], "b");

    // A bot that doesn't work the card is refused, and so is a routine off the canvas.
    let wrong_lane = f
        .tool(json!({"op": "act", "view": "routine:a", "action": "focus_lane",
                     "arguments": {"routineId": routine, "botId": "missing"}}))
        .unwrap_err();
    assert!(wrong_lane.to_string().contains("its lanes are a, b"), "{wrong_lane:#}");
    let off_canvas = f
        .tool(json!({"op": "act", "view": "routine:a", "action": "focus_lane",
                     "arguments": {"routineId": "missing", "botId": "a"}}))
        .unwrap_err();
    assert!(off_canvas.to_string().contains("isn't on this canvas"), "{off_canvas:#}");
    f.shutdown();
}
