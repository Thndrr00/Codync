//! Owner management uses the same connector lock as authorization and dispatch claims.
use super::policy::{field, update_card};
use crate::{LockExt, hub::Hub};
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

pub const METHODS: &[&str] =
    &["getMcpToolSettings", "resetMcpToolPolicy", "listMcpToolCalls", "getMcpToolCall", "revokeMcpToolCalls"];

pub(super) fn fingerprint(hub: &Hub, connector: &str) -> Result<String> {
    if connector.starts_with(crate::market::composio::PREFIX) {
        return crate::market::composio::fingerprint(&hub.store, connector);
    }
    let item = crate::market::connectors(&hub.store)?
        .into_iter()
        .find(|item| item.id == connector)
        .context("Connector is no longer installed.")?;
    fingerprint_value(&item)
}
pub(crate) fn fingerprint_value(item: &crate::market::Connector) -> Result<String> {
    item.runtime_fingerprint()
}

pub(super) fn remote(hub: &Hub, connector: &str) -> Result<()> {
    if connector.starts_with(crate::market::composio::PREFIX)
        && crate::market::composio::installed(&hub.store, connector)?
    {
        return Ok(());
    }
    if !crate::market::connectors(&hub.store)?
        .iter()
        .any(|item| item.id == connector && (item.url.is_some() || item.command.is_some()))
    {
        bail!("connector is no longer installed");
    }
    Ok(())
}

pub fn call(hub: &Hub, method: &str, body: &Value) -> Result<Value> {
    let _configuration = hub.store.connector_lock.locked();
    let connector = field(body, "connectorId")?;
    remote(hub, connector)?;
    match method {
        "getMcpToolSettings" => Ok(json!({"items":hub.store.mcp_tool_settings(connector)?,"defaultMode":"ask"})),
        "listMcpToolCalls" => {
            let after =
                body.get("afterId").map(|v| v.as_str().context("afterId must be a string")).transpose()?.unwrap_or("");
            let mut items = hub.store.list_mcp_calls(connector, after)?;
            let more = items.len() > 50;
            items.truncate(50);
            let next = if more { items.last().map(|row| row.id.clone()) } else { None };
            Ok(json!({"items":items,"next":next}))
        }
        "getMcpToolCall" => {
            let request = hub.store.mcp_call(field(body, "id")?)?.context("unknown tool request")?;
            if request.connector != connector {
                bail!("tool request belongs to another connector");
            }
            Ok(json!({"id":body["id"],"botId":request.bot,"tool":request.tool,
                "status":request.status,"arguments":request.message["params"]["arguments"]}))
        }
        "resetMcpToolPolicy" | "revokeMcpToolCalls" => {
            let reset = method == "resetMcpToolPolicy";
            let tool = if reset { Some(field(body, "tool")?) } else { None };
            let ids = hub.store.revoke_mcp_calls(connector, tool, reset)?;
            for id in &ids {
                update_card(hub, id, "revoked")?;
            }
            Ok(json!({"revoked":ids.len()}))
        }
        _ => bail!("unknown MCP management method"),
    }
}
