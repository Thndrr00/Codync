use super::*;
use crate::store::tests::temp_store;
use serde_json::json;

fn request() -> Value {
    json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"write","arguments":{"text":"exact original"}}})
}

#[test]
fn default_ask_blocks_until_approval_and_claims_only_the_saved_message_once() {
    let store = temp_store();
    let message = request();
    assert_eq!(store.mcp_policy("remote", "write").unwrap(), Mode::Ask);
    let id = store.create_mcp_call("bot", "remote", &message).unwrap();
    assert!(store.claim_mcp_call(&id).unwrap().is_none());
    store.decide_mcp_call(&id, true).unwrap();
    store.decide_mcp_call(&id, true).unwrap();
    assert_eq!(store.claim_mcp_call(&id).unwrap(), Some(message));
    assert!(store.claim_mcp_call(&id).unwrap().is_none());
}

#[test]
fn refusal_expiry_and_policy_changes_never_dispatch() {
    let store = temp_store();
    let id = store.create_mcp_call("bot", "remote", &request()).unwrap();
    store.decide_mcp_call(&id, false).unwrap();
    store.decide_mcp_call(&id, true).unwrap();
    assert!(store.claim_mcp_call(&id).unwrap().is_none());
    let id = store.create_mcp_call("bot", "remote", &request()).unwrap();
    store.db.locked().execute("UPDATE pending_tool_calls SET expires_at=0 WHERE id=?1", [&id]).unwrap();
    store.decide_mcp_call(&id, true).unwrap();
    assert!(store.claim_mcp_call(&id).unwrap().is_none());
    let id = store.create_mcp_call("bot", "remote", &request()).unwrap();
    store.decide_mcp_call(&id, true).unwrap();
    store.set_mcp_policy("remote", "write", Mode::Deny).unwrap();
    store.set_mcp_policy("remote", "write", Mode::Allow).unwrap();
    assert!(store.claim_mcp_call(&id).unwrap().is_none());
}

#[test]
fn allow_is_explicit_and_deny_does_not_create_a_pending_call() {
    let store = temp_store();
    store.set_mcp_policy("remote", "write", Mode::Deny).unwrap();
    assert!(store.create_mcp_call("bot", "remote", &request()).is_err());
    store.set_mcp_policy("remote", "write", Mode::Allow).unwrap();
    let id = store.create_mcp_call("bot", "remote", &request()).unwrap();
    assert_eq!(store.claim_mcp_call(&id).unwrap(), Some(request()));
    assert_eq!(store.mcp_policy("different", "write").unwrap(), Mode::Ask);
}

#[test]
fn policy_changes_return_revoked_card_ids_and_reset_returns_to_ask() {
    let store = temp_store();
    let id = store.create_mcp_call("bot", "remote", &request()).unwrap();
    assert_eq!(store.set_mcp_policy("remote", "write", Mode::Allow).unwrap(), vec![id.clone()]);
    assert_eq!(store.mcp_call(&id).unwrap().unwrap().status, "revoked");
    let second = store.create_mcp_call("bot", "remote", &request()).unwrap();
    assert_eq!(store.revoke_mcp_calls("remote", Some("write"), true).unwrap(), vec![second]);
    assert_eq!(store.mcp_policy("remote", "write").unwrap(), Mode::Ask);
    assert!(store.mcp_tool_settings("remote").unwrap().is_empty());
}

#[test]
fn bulk_revocation_preserves_policies_and_other_connections() {
    let store = temp_store();
    store.set_mcp_policy("remote", "write", Mode::Allow).unwrap();
    let first = store.create_mcp_call("bot", "remote", &request()).unwrap();
    let second = store.create_mcp_call("other-bot", "remote", &request()).unwrap();
    let foreign = store.create_mcp_call("bot", "other", &request()).unwrap();
    assert_eq!(store.revoke_mcp_calls("remote", None, false).unwrap().len(), 2);
    assert!(store.claim_mcp_call(&first).unwrap().is_none());
    assert!(store.claim_mcp_call(&second).unwrap().is_none());
    assert_eq!(store.mcp_policy("remote", "write").unwrap(), Mode::Allow);
    assert_eq!(store.list_mcp_calls("other", "").unwrap()[0].id, foreign);
    assert!(store.list_mcp_calls("remote", "").unwrap().is_empty());
}

#[test]
fn observed_read_only_default_never_overrides_an_explicit_owner_policy() {
    let store = temp_store();
    store.observe_mcp_tools("remote", &[json!({"name":"read","annotations":{"readOnlyHint":true}})]).unwrap();
    assert_eq!(store.mcp_policy("remote","read").unwrap(),Mode::Allow);
    store.set_mcp_policy("remote","read",Mode::Ask).unwrap();
    assert_eq!(store.mcp_policy("remote","read").unwrap(),Mode::Ask);
    store.revoke_mcp_calls("remote",Some("read"),true).unwrap();
    assert_eq!(store.mcp_policy("remote","read").unwrap(),Mode::Allow);
}

#[test]
fn result_recovery_preserves_the_original_reply_without_another_dispatch() {
    let store = temp_store();
    let id = store.create_mcp_call("bot","remote",&request()).unwrap();
    let reply = json!({"jsonrpc":"2.0","id":1,"result":{"content":[{"type":"text","text":"original"}]}});
    assert!(store.complete_mcp_call(&id,&reply).is_err());
    store.decide_mcp_call(&id,true).unwrap(); store.claim_mcp_call(&id).unwrap();
    store.complete_mcp_call(&id,&reply).unwrap(); store.complete_mcp_call(&id,&reply).unwrap();
    assert_eq!(store.mcp_result(&id).unwrap(),Some(reply));
    assert!(store.complete_mcp_call(&id,&json!({"jsonrpc":"2.0","id":1,"result":{}})).is_err());
    assert!(store.claim_mcp_call(&id).unwrap().is_none());
}
