//! Host authorization for remote MCP calls. The proxy receives only the stored request.
use crate::LockExt;
use crate::api::devices::Caller;
use crate::hub::Hub;
use crate::store::{EntryKind, Lane, mcp_policies::Mode};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::sync::Arc;

pub const METHODS: &[&str] = &[
    "getMcpToolPolicy",
    "setMcpToolPolicy",
    "authorizeMcpCall",
    "claimMcpCall",
    "resolveToolCall",
    "checkMcpDispatch",
    "cancelMcpCall",
    "getMcpProxyConfig",
    "observeMcpTools",
    "completeMcpCall",
    "getMcpToolResult",
];

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Decision {
    id: String,
    approve: bool,
}

/// The chat's startup-failure latch for a connector's tools, after a Test connection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ToolsAfterVerify {
    /// A passing check cleared the latch: declare the tools again from the next message.
    Restore,
    /// The check failed: keep the latch, so the tools stay out of this chat.
    KeepFailed { needs_sign_in: bool },
    /// There was no latch and no new failure: nothing to refresh.
    Unchanged,
}

/// Pure decision for a Test connection (OpenBot triage-15335). A passing check clears a stale
/// startup-failure latch whether or not the server needs sign-in; a failed check keeps it. Only the
/// latch is decided here — per-tool authorization (allow/ask/deny, including the deny blocklist)
/// stays a later gate.
pub(crate) fn tools_after_verify(previous_failure: bool, verify_ok: bool, requires_sign_in: bool) -> ToolsAfterVerify {
    match (verify_ok, previous_failure) {
        (true, true) => ToolsAfterVerify::Restore,
        (true, false) => ToolsAfterVerify::Unchanged,
        (false, _) => ToolsAfterVerify::KeepFailed { needs_sign_in: requires_sign_in },
    }
}

/// The latch never widens authorization: a `deny` (blocklist) policy still refuses dispatch.
pub(crate) fn policy_allows_dispatch(mode: Mode) -> bool {
    mode != Mode::Deny
}

/// Pure: this tool call was refused because the server's saved sign-in lacks a scope, not because
/// the tool is disabled or denied by the owner (OpenBot 0.70). Only the server is marked; a policy
/// denial or a generic error is never mistaken for missing scope.
pub(crate) fn insufficient_scope(error: &str) -> bool {
    let lower = error.to_ascii_lowercase();
    let missing = lower.contains("insufficient_scope")
        || lower.contains("insufficient scope")
        || lower.contains("missing required scope")
        || lower.contains("missing scope")
        || lower.contains("required scope")
        || lower.contains("scope is required")
        || (lower.contains("scope") && (lower.contains("not authorized") || lower.contains("not permitted")));
    missing && !lower.contains("policy")
}

/// The card for a tool call refused because the server needs a key (OpenBot 0.70). The notice is
/// fixed: no error text, tool output or key value is shown. Pure.
pub(crate) fn key_request_notice(error: &str) -> Option<&'static str> {
    const NOTICE: &str = "This server needs a key. Sign in or add its key in Marketplace, Apps.";
    let lower = error.to_ascii_lowercase();
    let no_key = lower.contains("api key")
        || lower.contains("api_key")
        || lower.contains("missing key")
        || lower.contains("no key")
        || lower.contains("needs a key")
        || lower.contains("key is required");
    no_key.then_some(NOTICE)
}

/// Mark one server as needing sign-in without touching any tool's allow/ask/deny policy, so the
/// server's other tools stay usable. The fixed label comes from the shared MCP wording.
fn mark_sign_in_needed(hub: &Hub, connector: &str) -> Result<String> {
    let notice = super::notices::mcp_notice(super::notices::McpNotice::SignInNeeded).to_owned();
    hub.store.kv_set(&format!("mcp-sign-in-needed:{connector}"), &notice)?;
    Ok(notice)
}

pub(super) fn field<'a>(body: &'a Value, name: &str) -> Result<&'a str> {
    body[name].as_str().with_context(|| format!("{name} is required"))
}

/// One cause of a dropped MCP server: its name and the host's classified failure kind. The pair is
/// the toast's dedup key, so the same failure seen twice stays quiet while a different cause is named.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub(crate) struct FailureCause {
    pub server: String,
    pub kind: String,
}

impl FailureCause {
    /// The stable chat entry id for this one cause.
    pub(crate) fn notice_id(&self, bot: &str) -> String {
        format!("mcp-start-notice:{bot}:{}:{}", self.server, self.kind)
    }
}

/// The plain notice a new cause posts: only the server's name, never error text or tool output.
/// A cause already in `seen` gets no toast (OpenBot plan 003). Generalizes the old pi/custom-only
/// notice to every provider; the stable id keeps a repeated tool call from posting it twice. Pure.
pub(crate) fn failure_notice(cause: &FailureCause, seen: &BTreeSet<FailureCause>) -> Option<String> {
    if seen.contains(cause) {
        return None;
    }
    crate::market::verify::mcp_start_notice(&cause.server, true)
}

/// At most this many causes are remembered per bot, so the dedup set cannot grow without bound.
const MAX_SEEN_CAUSES: usize = 64;

fn seen_key(bot: &str) -> String {
    format!("mcp-notice-causes:{bot}")
}

fn seen_causes(hub: &Hub, bot: &str) -> BTreeSet<FailureCause> {
    hub.store
        .kv_get(&seen_key(bot))
        .and_then(|s| serde_json::from_str::<Vec<FailureCause>>(&s).ok())
        .map(|causes| causes.into_iter().collect())
        .unwrap_or_default()
}

/// Post one plain notice in the bot's chat when one of its MCP servers failed to start. Every
/// provider gets it, not just Pi/custom endpoints, and a cause already announced stays quiet.
fn notice_failed_server(hub: &Hub, bot: &str, connector: &str) {
    let Some(failure) = crate::market::verify::last_failure(&hub.store, connector) else {
        return;
    };
    let cause =
        FailureCause { server: connector.to_owned(), kind: failure["kind"].as_str().unwrap_or("tool").to_owned() };
    let seen = seen_causes(hub, bot);
    let Some(text) = failure_notice(&cause, &seen) else {
        return;
    };
    let lane = hub.runtime(bot).lane.unwrap_or_else(|| Lane::main(bot));
    if let Err(e) = hub.add_entry_once(
        &cause.notice_id(bot),
        &lane,
        EntryKind::Notice,
        hub.store.max_turn(bot),
        &json!({"text": text}),
    ) {
        tracing::warn!(error = format!("{e:#}"), "couldn't post the MCP start notice");
        return;
    }
    let mut seen = seen;
    seen.insert(cause);
    let mut remembered: Vec<FailureCause> = seen.into_iter().collect();
    if remembered.len() > MAX_SEEN_CAUSES {
        remembered.drain(..remembered.len() - MAX_SEEN_CAUSES);
    }
    match serde_json::to_string(&remembered) {
        Ok(encoded) => {
            if let Err(e) = hub.store.kv_set(&seen_key(bot), &encoded) {
                tracing::warn!(error = format!("{e:#}"), "couldn't remember the MCP start notice");
            }
        }
        Err(e) => tracing::warn!(error = format!("{e:#}"), "couldn't serialize MCP notice causes"),
    }
}

fn enabled(hub: &Hub, bot: &str, connector: &str) -> Result<()> {
    let bot = hub.store.bot(bot)?.filter(|row| !row.deleted && !row.config.is_group()).context("unknown bot")?;
    if !bot.config.connectors.iter().any(|id| id == connector) {
        bail!("connector is disabled for this bot");
    }
    if !connector.starts_with(crate::market::composio::PREFIX)
        && !crate::market::connectors(&hub.store)?
            .iter()
            .any(|item| item.id == connector && (item.url.is_some() || item.command.is_some()))
    {
        bail!("connector is no longer installed");
    }
    if connector.starts_with(crate::market::composio::PREFIX)
        && !crate::market::composio::installed(&hub.store, connector)?
    {
        bail!("Composio account is disconnected.");
    }
    Ok(())
}

pub(crate) fn update_card(hub: &Hub, id: &str, state: &str) -> Result<()> {
    if let Some(mut entry) = hub.store.entry(id) {
        entry.data["status"] = if state == "pending" { "pending" } else { "answered" }.into();
        entry.data["selected"] = match state {
            "approved" | "dispatched" => json!("approve"),
            _ => json!("decline"),
        };
        entry.data["mcpStatus"] = state.into();
        hub.set_entry(id, &entry.data).context("couldn't update tool approval card")?;
    }
    Ok(())
}

pub fn respond(hub: &Hub, id: &str, option: Option<&str>) -> Result<bool> {
    let _configuration = hub.store.connector_lock.locked();
    let Some(call) = hub.store.mcp_call(id)? else {
        return Ok(false);
    };
    let approve = match option {
        Some("approve") => true,
        Some("decline") | None => false,
        _ => bail!("unknown tool decision"),
    };
    if approve {
        enabled(hub, &call.bot, &call.connector)?;
        if !policy_allows_dispatch(hub.store.mcp_policy(&call.connector, &call.tool)?) {
            bail!("tool is disabled");
        }
    }
    hub.store.decide_mcp_call(id, approve)?;
    let current = hub.store.mcp_call(id)?.context("tool request disappeared")?;
    update_card(hub, id, &current.status)?;
    Ok(true)
}

pub async fn call(hub: &Arc<Hub>, caller: &Caller, method: &str, body: Value) -> Result<Value> {
    // Proxy endpoints cannot be driven by paired devices or through a browser origin.
    if matches!(
        method,
        "authorizeMcpCall"
            | "claimMcpCall"
            | "checkMcpDispatch"
            | "cancelMcpCall"
            | "getMcpProxyConfig"
            | "observeMcpTools"
            | "completeMcpCall"
    ) && !matches!(caller, Caller::Local)
    {
        bail!("this method is restricted to the local MCP proxy");
    }
    crate::market::vault::unlock(hub.clone()).await?;
    match method {
        "getMcpProxyConfig" => {
            let _configuration = hub.store.connector_lock.locked();
            Ok(json!({"fingerprint":super::policy_management::fingerprint(hub, field(&body,"connectorId")?)?}))
        }
        "observeMcpTools" => {
            let _configuration = hub.store.connector_lock.locked();
            let connector = field(&body, "connectorId")?;
            ensure_fingerprint(hub, connector, &body)?;
            let revoked =
                hub.store.observe_mcp_tools(connector, body["tools"].as_array().context("tools must be an array")?)?;
            for id in revoked {
                update_card(hub, &id, "revoked")?;
            }
            Ok(json!({}))
        }
        "completeMcpCall" => {
            let id = field(&body, "id")?;
            // A tool result that failed for want of a sign-in scope marks only its server; the
            // server's other tools and every tool policy stay as they were.
            let denied = body["reply"]["result"]["isError"] == true || body["reply"]["error"].is_object();
            if denied
                && insufficient_scope(&body["reply"].to_string())
                && let Some(call) = hub.store.mcp_call(id)?
            {
                mark_sign_in_needed(hub, &call.connector)?;
            }
            hub.store.complete_mcp_call(id, &body["reply"])?;
            Ok(json!({}))
        }
        "getMcpToolResult" => {
            let id = field(&body, "id")?;
            let request = hub.store.mcp_call(id)?.context("Unknown tool request.")?;
            if request.connector != field(&body, "connectorId")? {
                bail!("Request belongs to another connector.");
            }
            Ok(json!({"status":request.status,"reply":hub.store.mcp_result(id)?}))
        }
        "cancelMcpCall" => {
            let id = field(&body, "id")?;
            hub.store.cancel_mcp_call(id)?;
            update_card(hub, id, "revoked")?;
            Ok(json!({}))
        }
        "checkMcpDispatch" => {
            let _configuration = hub.store.connector_lock.locked();
            let request = hub.store.mcp_call(field(&body, "id")?)?.context("unknown tool request")?;
            enabled(hub, &request.bot, &request.connector)?;
            if request.status != "dispatched"
                || !policy_allows_dispatch(hub.store.mcp_policy(&request.connector, &request.tool)?)
            {
                bail!("tool request is no longer authorized");
            }
            Ok(json!({}))
        }
        "getMcpToolPolicy" => {
            Ok(json!({"mode":hub.store.mcp_policy(field(&body,"connectorId")?, field(&body,"tool")?)?}))
        }
        "setMcpToolPolicy" => {
            let _configuration = hub.store.connector_lock.locked();
            super::policy_management::remote(hub, field(&body, "connectorId")?)?;
            let mode: Mode = serde_json::from_value(body["mode"].clone()).context("mode must be allow, ask or deny")?;
            let revoked = hub.store.set_mcp_policy(field(&body, "connectorId")?, field(&body, "tool")?, mode)?;
            for id in revoked {
                update_card(hub, &id, "revoked")?;
            }
            Ok(json!({}))
        }
        "resolveToolCall" => {
            let decision: Decision = serde_json::from_value(body)?;
            if !respond(hub, &decision.id, Some(if decision.approve { "approve" } else { "decline" }))? {
                bail!("unknown tool request");
            }
            Ok(json!({}))
        }
        "authorizeMcpCall" => {
            let _configuration = hub.store.connector_lock.locked();
            let bot = field(&body, "botId")?;
            let connector = field(&body, "connectorId")?;
            enabled(hub, bot, connector)?;
            // A tool the server refused for want of a scope asks for sign-in on that server only;
            // no tool policy changes, so its other tools stay usable.
            if let Some(error) = body["denial"].as_str().filter(|e| insufficient_scope(e)) {
                let notice = mark_sign_in_needed(hub, connector)?;
                let mut result = json!({"signInNeeded":true,"notice":notice});
                // A refusal that says the key is missing gets the Marketplace, Apps card too.
                if let Some(text) = key_request_notice(error) {
                    result["keyNotice"] = text.into();
                }
                return Ok(result);
            }
            ensure_fingerprint(hub, connector, &body)?;
            notice_failed_server(hub, bot, connector);
            let id = hub.store.create_mcp_call(bot, connector, &body["message"])?;
            let request = hub.store.mcp_call(&id)?.context("tool request was not saved")?;
            if request.status == "pending" {
                let lane = hub.runtime(bot).lane.unwrap_or_else(|| Lane::main(bot));
                if lane.chat != bot
                    && !hub
                        .store
                        .bot(&lane.chat)?
                        .is_some_and(|row| !row.deleted && row.config.members.iter().any(|id| id == bot))
                {
                    bail!("bot no longer belongs to this conversation");
                }
                hub.add_entry_once(
                    &id,
                    &lane,
                    EntryKind::Permission,
                    hub.store.max_turn(&lane.chat),
                    &json!({
                        "title":format!("Run {} · {}", request.connector, request.tool), "toolKind":"other",
                        "detail":serde_json::to_string_pretty(&request.message["params"]["arguments"])?,
                        "command":null,"diffs":[],"cwd":"","author":bot,"status":"pending","selected":null,
                        "mcpToolCallId":id,"options":[
                            {"optionId":"approve","name":"Approve & run","kind":"allow_once"},
                            {"optionId":"decline","name":"Decline","kind":"reject_once"}
                        ]
                    }),
                )?;
            }
            Ok(json!({"id":id}))
        }
        "claimMcpCall" => {
            let _configuration = hub.store.connector_lock.locked();
            let id = field(&body, "id")?;
            let request = hub.store.mcp_call(id)?.context("unknown tool request")?;
            enabled(hub, &request.bot, &request.connector)?;
            let message = hub.store.claim_mcp_call(id)?;
            let current = hub.store.mcp_call(id)?.context("tool request disappeared")?;
            if current.status != "pending" {
                update_card(hub, id, &current.status)?;
            }
            Ok(json!({"status":current.status,"message":message}))
        }
        _ => bail!("unknown MCP policy method"),
    }
}

/// No caller-supplied replacement arguments cross the approval boundary.
pub(super) async fn authorize(
    port: u16,
    token: &str,
    connector: &str,
    mut message: Value,
    cancelled: &super::cancellation::Cancellation,
    fingerprint: &str,
) -> Result<(Value, Option<String>)> {
    if message.is_array() {
        bail!("batched remote MCP requests are not supported");
    }
    if message["method"] != "tools/call" {
        return Ok((message, None));
    }
    let params = message["params"].as_object_mut().context("tool call params must be an object")?;
    params.entry("arguments").or_insert_with(|| json!({}));
    let bot = std::env::var("CODYNC_BOT_ID").context("remote tool calls require a bot identity")?;
    let request = proxy_call(
        port,
        token,
        "authorizeMcpCall",
        &json!({"botId":bot,"connectorId":connector,"message":message,"fingerprint":fingerprint}),
    )
    .await?;
    let id = request["id"].as_str().context("host returned no request id")?;
    loop {
        if cancelled.cancelled(&message["id"]) {
            proxy_call(port, token, "cancelMcpCall", &json!({"id":id})).await?;
            bail!("the agent cancelled this tool request");
        }
        let result = proxy_call(port, token, "claimMcpCall", &json!({"id":id})).await?;
        if result["message"].is_object() {
            if cancelled.cancelled(&message["id"]) {
                proxy_call(port, token, "cancelMcpCall", &json!({"id":id})).await?;
                bail!("the agent cancelled this tool request");
            }
            return Ok((result["message"].clone(), Some(id.to_owned())));
        }
        if result["status"] != "pending" {
            bail!("tool request was {} and was not dispatched", result["status"]);
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}

pub(super) async fn check(port: u16, token: &str, id: Option<&str>) -> Result<()> {
    if let Some(id) = id {
        proxy_call(port, token, "checkMcpDispatch", &json!({"id":id})).await?;
    }
    Ok(())
}

pub(super) async fn proxy_call(port: u16, token: &str, method: &str, body: &Value) -> Result<Value> {
    let reply = crate::http()
        .post(format!("http://127.0.0.1:{port}/api/{method}"))
        .bearer_auth(token)
        .json(body)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await?;
    let success = reply.status().is_success();
    let value: Value = reply.json().await?;
    if !success {
        bail!("{}", value["error"].as_str().unwrap_or("MCP authorization failed"));
    }
    Ok(value)
}

fn ensure_fingerprint(hub: &Hub, connector: &str, body: &Value) -> Result<()> {
    if field(body, "fingerprint")? != super::policy_management::fingerprint(hub, connector)? {
        bail!("Connector configuration changed. Restart this MCP proxy before using it.");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn approval_never_accepts_replacement_arguments() {
        assert!(serde_json::from_value::<Decision>(json!({"id":"call","approve":true})).is_ok());
        assert!(
            serde_json::from_value::<Decision>(json!({"id":"call","approve":true,"arguments":{"changed":true}}))
                .is_err()
        );
    }

    #[test]
    fn a_failed_start_then_a_passing_verify_restores_the_tools() {
        assert_eq!(tools_after_verify(true, true, true), ToolsAfterVerify::Restore);
    }

    #[test]
    fn a_passing_verify_on_a_server_with_no_sign_in_restores_the_tools() {
        assert_eq!(tools_after_verify(true, true, false), ToolsAfterVerify::Restore);
    }

    #[test]
    fn a_still_failing_verify_keeps_the_latch() {
        assert_eq!(tools_after_verify(true, false, false), ToolsAfterVerify::KeepFailed { needs_sign_in: false });
        assert_eq!(tools_after_verify(false, false, true), ToolsAfterVerify::KeepFailed { needs_sign_in: true });
    }

    #[test]
    fn a_denied_tool_stays_denied_even_after_the_latch_is_cleared() {
        // The decision restores declarations only; authorization still runs its own gate.
        assert_eq!(tools_after_verify(true, true, false), ToolsAfterVerify::Restore);
        assert!(!policy_allows_dispatch(Mode::Deny));
        assert!(policy_allows_dispatch(Mode::Allow));
        assert!(policy_allows_dispatch(Mode::Ask));
    }

    #[test]
    fn a_passing_verify_without_a_prior_failure_changes_nothing() {
        assert_eq!(tools_after_verify(false, true, false), ToolsAfterVerify::Unchanged);
    }

    #[test]
    fn a_dropped_server_gets_a_plain_notice_only_once_per_cause() {
        let cause = FailureCause { server: "filesystem".into(), kind: "unreachable".into() };
        let none = BTreeSet::new();
        let text = failure_notice(&cause, &none).expect("a failed server gets a notice");
        assert!(text.contains("filesystem") && text.contains("could not start"));
        assert!(text.contains("Marketplace"), "the notice says where to check");
        assert!(!text.contains("error"), "no server error text reaches the chat");

        let seen = BTreeSet::from([cause.clone()]);
        assert_eq!(failure_notice(&cause, &seen), None, "the same cause stays quiet");
    }

    #[test]
    fn a_different_server_or_cause_still_gets_its_own_notice() {
        let cause = FailureCause { server: "filesystem".into(), kind: "unreachable".into() };
        let seen = BTreeSet::from([cause.clone()]);
        let other_server = FailureCause { server: "linear".into(), kind: "unreachable".into() };
        let other_kind = FailureCause { server: "filesystem".into(), kind: "auth".into() };
        assert!(failure_notice(&other_server, &seen).is_some(), "every dropped server is named");
        assert!(failure_notice(&other_kind, &seen).is_some(), "a changed cause is named again");
    }

    #[test]
    fn a_missing_scope_is_not_a_policy_or_generic_denial() {
        for error in [
            "insufficient_scope: token is missing required scopes",
            "Insufficient Scope for repositories",
            "Error: missing scope 'repo'",
            "the required scope was not granted",
            "scope is required for this tool",
            "scope not authorized",
        ] {
            assert!(insufficient_scope(error), "{error}");
        }
        for error in [
            "",
            "tool request was denied by policy",
            "the tool is disabled by policy",
            "permission denied",
            "network timeout",
            "HTTP 403 Forbidden",
        ] {
            assert!(!insufficient_scope(error), "{error}");
        }
    }

    #[test]
    fn a_missing_key_gets_the_marketplace_apps_card() {
        const CARD: &str = "This server needs a key. Sign in or add its key in Marketplace, Apps.";
        for error in [
            "missing API key",
            "No api_key was provided",
            "insufficient_scope: api key is required",
            "the server needs a key",
        ] {
            assert_eq!(key_request_notice(error), Some(CARD), "{error}");
        }
        for error in ["", "insufficient_scope: token lacks scope", "tool disabled by policy", "network timeout"] {
            assert_eq!(key_request_notice(error), None, "{error}");
        }
    }

    #[test]
    fn marking_a_server_for_sign_in_leaves_every_tool_usable() {
        use crate::remote::identity::Identity;
        use crate::store::Store;

        let dir = std::env::temp_dir().join(format!("codync-mcp-scope-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::open(&dir.join("test.db")).unwrap();
        let create = store.mcp_policy("linear", "create_issue").unwrap();
        let read = store.mcp_policy("linear", "read_issue").unwrap();
        let hub = Hub::new(store, "test".into(), Identity::load_or_create(&dir).unwrap(), "token".into(), 19225);
        assert_eq!(mark_sign_in_needed(&hub, "linear").unwrap(), "Sign-in needed");
        assert_eq!(hub.store.kv_get("mcp-sign-in-needed:linear").as_deref(), Some("Sign-in needed"));
        assert_eq!(hub.store.mcp_policy("linear", "create_issue").unwrap(), create);
        assert_eq!(hub.store.mcp_policy("linear", "read_issue").unwrap(), read);
        assert!(policy_allows_dispatch(hub.store.mcp_policy("linear", "create_issue").unwrap()));
        drop(hub);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_notice_id_is_stable_and_scoped_to_one_bot_and_cause() {
        let cause = FailureCause { server: "filesystem".into(), kind: "unreachable".into() };
        assert_eq!(cause.notice_id("alice"), "mcp-start-notice:alice:filesystem:unreachable");
        assert_eq!(cause.notice_id("alice"), cause.notice_id("alice"));
        assert_ne!(cause.notice_id("alice"), cause.notice_id("bob"));
        assert_ne!(cause.notice_id("alice"), FailureCause { kind: "auth".into(), ..cause }.notice_id("alice"));
    }
}
