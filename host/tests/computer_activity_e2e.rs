//! Metadata-only activity through a real host, including restart and bot deletion.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[allow(dead_code)]
mod common;

use common::host::Host;
use serde_json::json;

#[tokio::test]
async fn activity_is_private_durable_and_scoped_to_the_bot() {
    let mut host = Host::start().await;
    let bot = host.bot("Computer activity", "ask").await;
    let other = host.bot("Other bot", "ask").await;
    let error = host
        .refused(
            "computerCall",
            json!({
                "botId": bot, "name": "type_text", "arguments": {"text": "audit-secret-password"}
            }),
        )
        .await;
    assert!(error.contains("turned off"));
    let response = host.call("listComputerActivity", json!({"botId": bot})).await;
    assert_eq!(response["items"].as_array().unwrap().len(), 1);
    let row = &response["items"][0];
    assert_eq!(row["action"], "type_text");
    assert_eq!(row["actorKind"], "bot");
    assert_eq!(row["actorId"], bot);
    assert_eq!(row["result"], "failed");
    assert!(!response.to_string().contains("audit-secret-password"));
    assert!(!response.to_string().contains("turned off"));
    assert_eq!(host.call("listComputerActivity", json!({"botId": other})).await["items"], json!([]));
    let export = host.call("exportComputerActivity", json!({"botId": bot})).await;
    let csv = export["csv"].as_str().unwrap();
    assert!(csv.starts_with("id,botId,action,actorKind,actorId,result,createdAt\r\n"));
    assert!(csv.contains("type_text"));
    assert!(!csv.contains("audit-secret-password"));
    host.restart(|_| {}).await;
    assert_eq!(host.call("listComputerActivity", json!({"botId": bot})).await, response);
    assert_eq!(host.call("exportComputerActivity", json!({"botId": bot})).await, export);
    host.call("deleteBot", json!({"botId": bot})).await;
    assert!(host.refused("listComputerActivity", json!({"botId": bot})).await.contains("unknown bot"));
    assert!(host.refused("exportComputerActivity", json!({"botId": bot})).await.contains("unknown bot"));
}
