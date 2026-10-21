//! Authenticated, real-host roster queries on disposable data; no installed agent is required.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[allow(dead_code)]
mod common;

use common::host::Host;
use serde_json::json;

#[tokio::test]
async fn roster_search_paginates_opens_threads_and_previews_the_main_chat() {
    let host = Host::start().await;
    let id = host.bot("Roster test", "ask").await;
    let other = host.bot("Other chat", "ask").await;
    let root = host.call("send", json!({"botId":id,"text":"ÁRBOL first","clientNonce":"roster-root"})).await;
    let root_id = root["entry"]["id"].as_str().unwrap();
    let reply = host
        .call("send", json!({"botId":id,"threadId":root_id,"text":"árbol thread","clientNonce":"roster-thread"}))
        .await;
    let page = host.call("searchMessages", json!({"botIds":[id],"query":"ÁRBOL","limit":1})).await;
    assert_eq!(page["results"].as_array().unwrap().len(), 1);
    let large_scope = host.call("searchMessages", json!({"botIds":vec![id.clone(); 101],"query":"rbol"})).await;
    assert_eq!(large_scope["results"].as_array().unwrap().len(), 2);
    let second =
        host.call("searchMessages", json!({"botIds":[id],"query":"árbol","limit":1,"beforeSeq":page["next"]})).await;
    assert_ne!(page["results"][0]["id"], second["results"][0]["id"]);
    let context = host.call("messageContext", json!({"botId":id,"entryId":reply["entry"]["id"]})).await;
    assert!(context["entries"].as_array().unwrap().iter().any(|e| e["id"] == root_id));
    assert!(context["entries"].as_array().unwrap().iter().any(|e| e["threadId"] == root_id));
    assert_ne!(host.refused("messageContext", json!({"botId":other,"entryId":root_id})).await, "");
    let preview = host.call("rosterPreview", json!({"botId":id})).await;
    assert!(preview["entries"].as_array().unwrap().iter().all(|e| e["threadId"].is_null()));
    host.call("updateBot", json!({"id":id,"hidden":true})).await;
    assert_eq!(
        host.call("searchMessages", json!({"botIds":[id],"query":"árbol"})).await["results"].as_array().unwrap().len(),
        0
    );
}
