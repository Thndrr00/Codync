//! Records outcomes at the shared entry point, independently of the agent harness.

use crate::hub::Hub;
use crate::store::ComputerActivityResult;
use anyhow::{Result, bail};
use serde_json::Value;
use std::sync::Arc;

fn offered(name: &str) -> bool {
    if super::cua::offered(name) || name == "type_login" || crate::computer::panel::is_tool(name) {
        return true;
    }
    #[cfg(target_os = "linux")]
    if super::helper_tools::tools().iter().any(|tool| tool["name"] == name) {
        return true;
    }
    false
}

/// Runs a computer tool and retains only its vetted name and outcome, never its data.
pub async fn computer(hub: &Arc<Hub>, bot: &str, name: &str, args: Value) -> Result<Value> {
    // Tool names are untrusted too: an unknown name could itself contain a secret.
    if !offered(name) {
        bail!("unknown computer tool");
    }
    let result = super::computer::computer(hub, bot, name, args).await;
    completed(hub, bot, name, result)
}

fn completed(hub: &Hub, bot: &str, name: &str, result: Result<Value>) -> Result<Value> {
    let outcome = match &result {
        Ok(value) if value["isError"] != true => ComputerActivityResult::Success,
        _ => ComputerActivityResult::Failed,
    };
    if hub.store.record_computer_activity(bot, name, outcome).is_err() {
        // Reporting a completed action as failed could make an agent repeat its effects.
        tracing::warn!("could not persist computer activity");
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Store;
    use serde_json::json;

    #[tokio::test]
    async fn refused_calls_are_audited_without_arguments_or_error_text() {
        let dir = std::env::temp_dir().join(format!("codync-audit-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let hub = Hub::new(
            Store::open(std::path::Path::new(":memory:")).unwrap(),
            "test".into(),
            crate::remote::identity::Identity::load_or_create(&dir).unwrap(),
            "test".into(),
            19222,
        );
        let config = serde_json::from_value(json!({"id": "a", "name": "A"})).unwrap();
        hub.store.save_bot(&config).unwrap();
        let result = computer(&hub, "a", "type_text", json!({"text": "secret-password"})).await;
        assert!(result.unwrap_err().to_string().contains("turned off"));
        let rows = hub.store.computer_activity("a").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].result, ComputerActivityResult::Failed);
        let serialized = serde_json::to_string(&rows).unwrap();
        assert!(!serialized.contains("secret-password"));
        assert!(!serialized.contains("turned off"));
        assert!(computer(&hub, "a", "secret-in-tool-name", json!({})).await.is_err());
        assert_eq!(hub.store.computer_activity("a").unwrap().len(), 1);

        let success = json!({"content": [{"type": "text", "text": "private-output"}]});
        assert_eq!(completed(&hub, "a", "type_text", Ok(success.clone())).unwrap(), success);
        assert_eq!(hub.store.computer_activity("a").unwrap()[0].result, ComputerActivityResult::Success);
        let mcp_error = json!({"isError": true, "content": [{"type": "text", "text": "private-MCP-error"}]});
        assert_eq!(completed(&hub, "a", "type_text", Ok(mcp_error.clone())).unwrap(), mcp_error);
        assert_eq!(hub.store.computer_activity("a").unwrap()[0].result, ComputerActivityResult::Failed);
        let error = anyhow::anyhow!("private-host-error");
        assert_eq!(completed(&hub, "a", "type_text", Err(error)).unwrap_err().to_string(), "private-host-error");
        let rows = hub.store.computer_activity("a").unwrap();
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].result, ComputerActivityResult::Failed);
        assert!(!serde_json::to_string(&rows).unwrap().contains("private-"));
        drop(hub);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
