//! WebMCP's allow list, its explicit grant and its host-origin gate (card K15).

use super::*;
use crate::store::{BotConfig, Store};
use serde_json::json;
use std::path::PathBuf;

struct Fixture {
    hub: Hub,
    dir: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("codync-webmcp-{}", uuid::Uuid::new_v4()));
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
        store.save_bot_app("a", "planner", "Planner", None, Some("index.html")).unwrap();
        let identity = crate::remote::identity::Identity::load_or_create(&dir).unwrap();
        let hub = Hub::new(store, "test-host".into(), identity, "test-token".into(), 19224);
        Self { hub, dir }
    }

    fn grant(&self, actions: &[&str]) -> anyhow::Result<Value> {
        let names: Vec<String> = actions.iter().map(|name| (*name).to_owned()).collect();
        set_granted(&self.hub, "a", "planner", &names).map(|actions| json!(actions))
    }

    fn shutdown(self) {
        drop(self.hub);
        let _ = std::fs::remove_dir_all(self.dir);
    }
}

#[test]
fn the_allow_list_is_small_and_every_action_is_described() {
    assert_eq!(ACTIONS, ["open_chat", "show_artifact", "start_call"]);
    let actions = actions();
    let list = actions.as_array().unwrap();
    assert_eq!(list.len(), ACTIONS.len());
    for (action, name) in list.iter().zip(ACTIONS) {
        assert_eq!(action["name"], name);
        assert!(action["description"].as_str().is_some_and(|text| !text.is_empty()));
        assert!(action["arguments"]["type"] == "object");
    }
}

#[test]
fn an_action_outside_the_list_is_refused_before_any_permission() {
    let f = Fixture::new();
    let error = invoke(&f.hub, "a", "planner", "deleteBot", &json!({"botId": "a"})).unwrap_err();
    assert!(error.to_string().contains("unknown action"), "{error:#}");
    let arbitrary = invoke(&f.hub, "a", "planner", "call", &json!({"method": "setVoiceKey"})).unwrap_err();
    assert!(arbitrary.to_string().contains("unknown action"), "{arbitrary:#}");
    f.shutdown();
}

#[test]
fn grants_start_empty_change_only_declared_actions_and_revoke() {
    let f = Fixture::new();
    assert_eq!(granted(&f.hub, "a", "planner").unwrap(), Vec::<String>::new());
    assert!(f.grant(&["deleteBot"]).is_err(), "an undeclared action can't be granted");
    assert_eq!(f.grant(&["open_chat", "open_chat"]).unwrap(), json!(["open_chat"]));
    assert_eq!(granted(&f.hub, "a", "planner").unwrap(), vec!["open_chat".to_owned()]);
    assert_eq!(f.grant(&[]).unwrap(), json!([]));
    assert!(granted(&f.hub, "a", "planner").unwrap().is_empty());
    f.shutdown();
}

#[test]
fn a_call_needs_its_grant_and_a_known_bot() {
    let f = Fixture::new();
    let denied = invoke(&f.hub, "a", "planner", "open_chat", &json!({"botId": "b"})).unwrap_err();
    assert!(denied.to_string().contains("isn't allowed"), "{denied:#}");

    f.grant(&["open_chat", "start_call", "show_artifact"]).unwrap();
    let event = invoke(&f.hub, "a", "planner", "open_chat", &json!({"botId": "b"})).unwrap();
    assert_eq!(event, json!({"botId": "b"}));
    let unknown = invoke(&f.hub, "a", "planner", "open_chat", &json!({"botId": "missing"})).unwrap_err();
    assert!(unknown.to_string().contains("unknown bot"), "{unknown:#}");
    let no_artifact = invoke(&f.hub, "a", "planner", "show_artifact", &json!({"artifactId": "x"})).unwrap_err();
    assert!(no_artifact.to_string().contains("isn't this bot's"), "{no_artifact:#}");
    f.shutdown();
}

#[test]
fn the_announced_action_carries_only_ids_and_no_credentials() {
    let f = Fixture::new();
    f.grant(&["open_chat"]).unwrap();
    let mut events = f.hub.events.subscribe();
    invoke(&f.hub, "a", "planner", "open_chat", &json!({"botId": "b"})).unwrap();
    let event = events.try_recv().unwrap();
    assert_eq!(event["type"], "webMcpAction");
    assert_eq!(event["action"], "open_chat");
    assert_eq!(event["arguments"], json!({"botId": "b"}));
    assert_eq!(event["botId"], "a");
    assert_eq!(event["slug"], "planner");
    let text = event.to_string();
    for secret in ["test-token", "token", "cwd", "apiKey", "authorization"] {
        assert!(!text.contains(secret), "the event leaked {secret}: {text}");
    }
    f.shutdown();
}

#[test]
fn only_pages_served_by_this_host_may_call() {
    assert!(host_origin(19224, None), "a same-origin fetch may omit the header");
    assert!(host_origin(19224, Some("http://127.0.0.1:19224")));
    assert!(host_origin(19224, Some("http://localhost:19224")));
    assert!(!host_origin(19224, Some("https://example.com")));
    assert!(!host_origin(19224, Some("http://127.0.0.1:19225")));
    assert!(host_origin(19224, Some("null")), "a sandboxed host-served page has an opaque origin");
}

#[test]
fn the_app_panel_reads_and_sets_the_grant_through_the_api() {
    let f = Fixture::new();
    let state = api(&f.hub, "webMcpState", &json!({"botId": "a", "slug": "planner"})).unwrap();
    let actions = state["actions"].as_array().unwrap();
    assert_eq!(actions.len(), ACTIONS.len());
    assert!(actions.iter().all(|action| action["granted"] == false));

    api(&f.hub, "setWebMcpGrant", &json!({"botId": "a", "slug": "planner", "actions": ["start_call"]})).unwrap();
    let state = api(&f.hub, "webMcpState", &json!({"botId": "a", "slug": "planner"})).unwrap();
    let granted: Vec<&str> = state["actions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|action| action["granted"] == true)
        .filter_map(|action| action["name"].as_str())
        .collect();
    assert_eq!(granted, ["start_call"]);

    assert!(api(&f.hub, "webMcpState", &json!({"botId": "a", "slug": "missing"})).is_err());
    assert!(api(&f.hub, "other", &json!({"botId": "a", "slug": "planner"})).is_err());
    f.shutdown();
}
