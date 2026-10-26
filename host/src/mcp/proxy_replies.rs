//! Observe actual upstream replies before returning them to the agent.
use super::policy;
use crate::LockExt;
use anyhow::{Context, Result};
use serde_json::{Value, json};
use std::{collections::HashMap, sync::Mutex};

enum Pending {
    Catalog,
    Call(String),
}
pub(super) struct ProxyReplies {
    port: u16,
    token: String,
    connector: String,
    pub fingerprint: String,
    pending: Mutex<HashMap<String, Pending>>,
}
impl ProxyReplies {
    pub async fn new(port: u16, token: &str, connector: &str) -> Result<Self> {
        let config = policy::proxy_call(port, token, "getMcpProxyConfig", &json!({"connectorId":connector})).await?;
        Ok(Self::bound(
            port,
            token,
            connector,
            config["fingerprint"].as_str().context("Missing proxy fingerprint.")?.into(),
        ))
    }
    pub fn bound(port: u16, token: &str, connector: &str, fingerprint: String) -> Self {
        Self { port, token: token.into(), connector: connector.into(), fingerprint, pending: Mutex::default() }
    }
    pub fn pending(&self, id: &Value) -> bool {
        self.pending.locked().contains_key(&id.to_string())
    }
    pub fn track(&self, message: &Value, call: Option<&str>) {
        if message["id"].is_null() {
            return;
        }
        let kind = if let Some(id) = call {
            Pending::Call(id.into())
        } else if message["method"] == "tools/list" {
            Pending::Catalog
        } else {
            return;
        };
        self.pending.locked().insert(message["id"].to_string(), kind);
    }
    pub fn forget(&self, id: &Value) {
        self.pending.locked().remove(&id.to_string());
    }
    pub async fn observe(&self, reply: &Value) {
        if reply.get("result").is_none() && reply.get("error").is_none() {
            return;
        }
        let pending = self.pending.locked().remove(&reply["id"].to_string());
        let result = match pending {
            Some(Pending::Catalog) => {
                let Some(tools) = reply["result"]["tools"].as_array() else {
                    return;
                };
                policy::proxy_call(
                    self.port,
                    &self.token,
                    "observeMcpTools",
                    &json!({"connectorId":self.connector,"fingerprint":self.fingerprint,"tools":tools}),
                )
                .await
            }
            Some(Pending::Call(id)) => {
                policy::proxy_call(self.port, &self.token, "completeMcpCall", &json!({"id":id,"reply":reply})).await
            }
            None => return,
        };
        if let Err(error) = result {
            // Never turn a successful upstream effect into an error that could encourage a retry.
            tracing::warn!(%error,"couldn't retain MCP metadata/receipt; upstream reply is still returned");
        }
    }
}
