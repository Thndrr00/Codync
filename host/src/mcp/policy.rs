//! Host authorization for remote MCP calls. The proxy receives only the stored request.
use crate::LockExt;
use crate::api::devices::Caller;
use crate::hub::Hub;
use crate::store::{EntryKind, Lane, mcp_policies::Mode};
use anyhow::{Context, Result, bail};
use serde::Deserialize;
use serde_json::{Value, json};
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

pub(super) fn field<'a>(body: &'a Value, name: &str) -> Result<&'a str> {
    body[name].as_str().with_context(|| format!("{name} is required"))
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
        if hub.store.mcp_policy(&call.connector, &call.tool)? == Mode::Deny {
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
            hub.store.complete_mcp_call(field(&body, "id")?, &body["reply"])?;
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
            if request.status != "dispatched" || hub.store.mcp_policy(&request.connector, &request.tool)? == Mode::Deny
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
            ensure_fingerprint(hub, connector, &body)?;
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
}
