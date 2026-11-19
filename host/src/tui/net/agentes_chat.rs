//! Typed calls for the behaviors brought from agentes.chat. The TUI renders host state; it never
//! decides any of it. Mirrors the methods in `chat/{presence,setup,stretch,agent_request,vault}`.

use super::Net;
use serde_json::{Value, json};

impl Net {
    pub async fn presence(&self, bot: &str) -> Result<Option<Value>, String> {
        let res = self.call("botPresence", &json!({"botId": bot})).await?;
        Ok(res["status"].clone().filter(|v| !v.is_null()))
    }

    pub async fn set_presence(&self, bot: &str, status: &str, until: Option<i64>) -> Result<Option<Value>, String> {
        let res = self.call("botPresence", &json!({"botId": bot, "status": status, "until": until})).await?;
        Ok(res["status"].clone().filter(|v| !v.is_null()))
    }

    pub async fn setup(&self, bot: &str) -> Result<Value, String> {
        self.call("botSetup", &json!({"botId": bot})).await
    }

    pub async fn stretch_card(&self, agent: &str, reason: &str) -> Result<Value, String> {
        self.call("stretchCard", &json!({"agent": agent, "reason": reason})).await
    }

    pub async fn approval_card(&self, agent: &str, reason: &str, request: &str) -> Result<Value, String> {
        self.call("agentApproval", &json!({"agent": agent, "reason": reason, "requestId": request})).await
    }

    pub async fn decide_approval(&self, card: &Value, option: &str) -> Result<Value, String> {
        self.call("agentApproval", &json!({"op": "decide", "card": card, "option": option})).await
    }

    pub async fn vault_mask(&self, name: &str, secret: &str, source: &str) -> Result<Value, String> {
        self.call("vaultMask", &json!({"name": name, "secret": secret, "source": source})).await
    }
}
