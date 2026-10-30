//! A real host and a scripted agent: creating, testing and deleting an approval rule changes what a real approval
//! does, and the tester says what the approval then does.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::host::Host;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

async fn bot(host: &Host) -> String {
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/command_agent.py");
    let spec = json!({"name": "Runner", "backend": "custom", "command": common::python_agent(&script),
        "connectors": [], "notify": false, "permission": "ask"});
    host.call("createBot", spec).await["bot"]["id"].as_str().unwrap().to_owned()
}

async fn test_rule(host: &Host, bot: &str, command: &str) -> Value {
    host.call("testApprovalRule", json!({"botId": bot, "command": command})).await["evaluation"].clone()
}

async fn entries(host: &Host, bot: &str) -> Vec<Value> {
    host.call("history", json!({"botId": bot})).await["entries"].as_array().unwrap().clone()
}

/// The text of the agent's next final reply after `known` replies.
async fn next_reply(host: &Host, bot: &str, known: usize) -> String {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let finals: Vec<Value> = entries(host, bot)
            .await
            .into_iter()
            .filter(|e| e["kind"] == "agent" && e["data"]["final"] == true)
            .collect();
        if finals.len() > known {
            return finals[known]["data"]["text"].as_str().unwrap().to_owned();
        }
        assert!(Instant::now() < deadline, "no reply");
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

async fn pending_card(host: &Host, bot: &str) -> Value {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        if let Some(card) =
            entries(host, bot).await.into_iter().find(|e| e["kind"] == "permission" && e["data"]["status"] == "pending")
        {
            return card;
        }
        assert!(Instant::now() < deadline, "no approval card");
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

async fn run(host: &Host, bot: &str, command: &str) {
    host.call(
        "send",
        json!({"botId": bot, "text": format!("run:{command}"), "clientNonce": uuid::Uuid::new_v4().to_string()}),
    )
    .await;
}

#[tokio::test]
async fn a_rule_created_tested_and_deleted_changes_a_real_approval() {
    if Command::new(common::python()).arg("--version").output().is_err() {
        eprintln!("skipping: Python not available");
        return;
    }
    let host = Host::start().await;
    let bot = bot(&host).await;

    // No rule: the bot's setting asks.
    let before = test_rule(&host, &bot, "echo hi").await;
    assert_eq!((before["outcome"].as_str(), before["permission"].as_str()), (Some("setting"), Some("ask")), "{before}");

    // An allow rule: the tester says so, and a real request is approved without a card.
    host.call("updateBot", json!({"id": bot, "approvalRules": [{"pattern": "echo *", "decision": "allow"}]})).await;
    let allowed = test_rule(&host, &bot, "echo hi").await;
    assert_eq!(
        (allowed["outcome"].as_str(), allowed["decision"].as_str(), allowed["index"].as_i64()),
        (Some("rule"), Some("allow"), Some(0))
    );
    run(&host, &bot, "echo hi").await;
    assert!(next_reply(&host, &bot, 0).await.contains("\"yes\""), "the rule approves it");

    // A chained command is not approved by an allow rule, and the tester says why.
    let chained = test_rule(&host, &bot, "echo hi && ls").await;
    assert_eq!(
        (chained["outcome"].as_str(), chained["chained"].as_bool(), chained["allowPassedOver"].as_i64()),
        (Some("setting"), Some(true), Some(0)),
        "{chained}"
    );

    // An unsaved list can be tested too.
    let draft = host
        .call(
            "testApprovalRule",
            json!({"botId": bot, "command": "echo hi", "rules": [{"pattern": "echo *", "decision": "deny"}]}),
        )
        .await;
    assert_eq!(draft["evaluation"]["decision"], "deny");

    // Deleted: the same request needs a person again.
    host.call("updateBot", json!({"id": bot, "approvalRules": []})).await;
    assert_eq!(test_rule(&host, &bot, "echo hi").await["outcome"], "setting");
    run(&host, &bot, "echo again").await;
    let card = pending_card(&host, &bot).await;
    host.call("respondPermission", json!({"entryId": card["id"], "optionId": "no"})).await;
    assert!(next_reply(&host, &bot, 1).await.contains("\"no\""));

    // A rule can refuse too, and a forbidden command is refused whatever the rules say.
    host.call("updateBot", json!({"id": bot, "approvalRules": [{"pattern": "*", "decision": "allow"}]})).await;
    assert_eq!(test_rule(&host, &bot, "rm -rf /").await["outcome"], "forbidden");
    run(&host, &bot, "rm -rf /").await;
    assert!(next_reply(&host, &bot, 2).await.contains("\"no\""), "forbidden before the allow-everything rule");
}
