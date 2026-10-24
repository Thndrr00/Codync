use super::*;

#[test]
fn observed_events_carry_no_content() {
    let b = json!({"botId": "b1", "text": "secret plan", "threadId": "e1", "attachments": ["u1", "u2"]});
    let (event, p) = observe(&Caller::Local, "send", &b).expect("send is tracked");
    assert_eq!(event, Event::MessageSent);
    assert_eq!(Value::Object(p.clone()), json!({"in_thread": true, "attachments": 2, "client": "local"}));
    assert!(!Value::Object(p).to_string().contains("secret"));

    let custom = json!({"name": "mine", "command": "/Users/me/bin/server"});
    let (_, p) = observe(&Caller::Local, "installConnector", &custom).expect("tracked");
    assert_eq!(p["connector"], "custom");
    assert!(observe(&Caller::Local, "history", &json!({})).is_none());
}

#[tokio::test]
async fn a_resent_message_counts_once_and_names_its_agent() {
    let dir = std::env::temp_dir().join(format!("codync-analytics-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let store = Store::open(std::path::Path::new(":memory:")).unwrap();
    let hub =
        Hub::new(store, "h".into(), crate::remote::identity::Identity::load_or_create(&dir).unwrap(), "t".into(), 0);
    let cfg = serde_json::from_value(json!({"id": "b1", "name": "b", "backend": "codex", "cwd": "/tmp"})).unwrap();
    hub.store.save_bot(&cfg).unwrap();
    let send = json!({"botId": "b1", "text": "hi", "clientNonce": "n1"});

    let (_, p) = observe_call(&hub, &Caller::Local, "send", &send).expect("a new message is tracked");
    assert_eq!((p["agent"].as_str(), p["kind"].as_str()), (Some("codex"), Some("bot")));
    let lane = crate::store::Lane::main("b1");
    hub.store
        .insert_entry(&lane, crate::store::EntryKind::User, 1, &json!({"text": "hi", "clientNonce": "n1"}))
        .unwrap();
    assert!(observe_call(&hub, &Caller::Local, "send", &send).is_none(), "a replay is the same message");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn on_until_the_owner_turns_it_off() {
    let store = Store::open(std::path::Path::new(":memory:")).unwrap();
    assert!(Analytics::load(&store).state());
    Analytics::load(&store).set_enabled(&store, false).unwrap();
    assert!(!Analytics::load(&store).state(), "an off stays off");
}

#[test]
fn computer_tag_matches_the_iphone() {
    // Same vector as the iPhone's AnalyticsTests.
    assert_eq!(computer_tag("AAECAwQFBgcICQoLDA0ODw"), "2d6878bee20aba04");
}

#[test]
fn a_failed_turn_names_its_kind_not_its_text() {
    use crate::agent::acp::{Failure, FailureKind, RpcError};
    use anyhow::Context as _;
    let code = |e: &anyhow::Error| {
        let (kind, rpc) = failure_kind(e);
        (kind, rpc.map(|r| r.code))
    };
    let setup = Failure::error(FailureKind::NotSetUp, "Codex isn't set up on this computer.");
    assert_eq!(code(&setup), ("not_installed", None));
    let hung = Err::<(), _>(Failure::error(FailureKind::Timeout, "no answer")).context("starting");
    assert_eq!(code(&hung.expect_err("failed")), ("timeout", None));
    let missing = std::io::Error::new(std::io::ErrorKind::NotFound, "/Users/me/bin/agent");
    let spawn = Err::<(), _>(missing).context("starting `/Users/me/bin/agent`");
    assert_eq!(code(&spawn.expect_err("failed")), ("not_installed", None));
    let auth = anyhow::Error::new(RpcError { code: -32000, message: "Authentication required".into(), kind: None });
    assert_eq!(code(&auth), ("sign_in", None));
    let rpc = anyhow::Error::new(RpcError { code: -32603, message: "Internal error: /Users/me".into(), kind: None });
    assert_eq!(code(&rpc), ("agent_error", Some(-32603)));
    let limited = anyhow::Error::new(RpcError { code: -32603, message: "x".into(), kind: Some("rate_limit".into()) });
    assert_eq!(failure_kind(&limited).1.and_then(|r| r.kind.as_deref()), Some("rate_limit"));
    assert_eq!(code(&anyhow::anyhow!("something else")), ("other", None));
}

#[test]
fn only_app_events_can_be_tracked_directly() {
    assert!(Event::deserialize(&json!("app_opened")).is_ok_and(Event::client_reported));
    assert!(!Event::deserialize(&json!("message_sent")).is_ok_and(Event::client_reported));
    assert!(Event::deserialize(&json!("anything_else")).is_err());
}
