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
        let cwd = dir.join("a");
        std::fs::create_dir_all(&cwd).unwrap();
        store
            .save_bot(
                &serde_json::from_value::<BotConfig>(json!({
                    "id": "a", "name": "A", "backend": "fixture", "cwd": cwd, "notify": false,
                }))
                .unwrap(),
            )
            .unwrap();
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
