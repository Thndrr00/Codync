//! Host API for the opt-in OpenAI-compatible mode of a custom bot.
//!
//! Only fixed mode names cross this boundary: no base URL, key, body or command is returned or
//! reported. The default path is the bot's own CLI.

use super::devices::Caller;
use crate::analytics::{self, Event};
use crate::hub::Hub;
use crate::store::CustomProviderMode;
use anyhow::{Result, anyhow, ensure};
use serde_json::{Map, Value, json};
use std::sync::Arc;

/// The methods this module answers.
pub(super) const METHODS: [&str; 3] = ["customProviderMode", "setCustomProviderMode", "customProviderRequest"];

pub(super) async fn call(hub: &Arc<Hub>, caller: &Caller, method: &str, b: &Value) -> Result<Value> {
    match method {
        "customProviderMode" => read(hub, b),
        "setCustomProviderMode" => set(hub, caller, b),
        "customProviderRequest" => request(hub, b).await,
        _ => anyhow::bail!("unknown custom provider method"),
    }
}

fn bot_row(hub: &Hub, b: &Value) -> Result<crate::store::BotRow> {
    let id = b["botId"].as_str().ok_or_else(|| anyhow!("`botId` is required"))?;
    hub.store.bot(id)?.filter(|row| !row.deleted).ok_or_else(|| anyhow!("unknown bot"))
}

/// The mode as a fixed name. Neither the saved endpoint nor the key is part of the answer.
fn read(hub: &Hub, b: &Value) -> Result<Value> {
    let row = bot_row(hub, b)?;
    Ok(json!({"mode": row.config.custom_provider_mode.as_str()}))
}

/// Saves the mode on the bot, leaving every other setting (including its CLI command) as it was.
fn set(hub: &Arc<Hub>, caller: &Caller, b: &Value) -> Result<Value> {
    let row = bot_row(hub, b)?;
    ensure!(row.config.backend == "custom", "the mode is only for a custom bot");
    let name = b["mode"].as_str().ok_or_else(|| anyhow!("`mode` is required"))?;
    let mode = CustomProviderMode::parse(name)?;

    // One bot's config is edited under the same lock the bot updates use, so a concurrent edit
    // can't be lost by this read-modify-write.
    let _edit = crate::chat::instruction_history::lock();
    let mut config = hub.store.bot(&row.config.id)?.ok_or_else(|| anyhow!("unknown bot"))?.config;
    if config.custom_provider_mode != mode {
        config.custom_provider_mode = mode;
        hub.store.save_bot(&config)?;
        hub.emit_bot(&config.id);
    }

    // The fixed mode name only: never a URL, a key, a command or a bot name.
    let client = if matches!(caller, Caller::Local) { "local" } else { "remote" };
    analytics::capture(
        hub,
        Event::SettingChanged,
        Map::from_iter([
            ("setting".into(), Value::from("custom_provider_mode")),
            ("value".into(), Value::from(mode.as_str())),
            ("client".into(), Value::from(client)),
        ]),
    );
    Ok(json!({"mode": mode.as_str()}))
}

/// Sends one turn through the opt-in path. With the mode off (or an endpoint that isn't OpenAI's
/// v1 with a key) the bot's CLI stays the path and the answer says `cli` — no request is sent.
async fn request(hub: &Arc<Hub>, b: &Value) -> Result<Value> {
    let row = bot_row(hub, b)?;
    let messages = b["messages"].as_array().cloned().unwrap_or_default();
    let tools = b["tools"].as_array().cloned().unwrap_or_default();
    ensure!(messages.len() <= 64 && tools.len() <= 64, "at most 64 messages and 64 tools for one request");
    // The saved key lives in the vault: unlock it only on this path, never for mode set/read.
    crate::market::vault::unlock(hub.clone()).await?;
    let model = b["model"].as_str().or(row.config.model.as_deref());
    let effort = b["reasoningEffort"].as_str().or(b["effort"].as_str());
    let answer =
        crate::agent::backends::openai_compatible::turn(&hub.store, &row.config, model, effort, &tools, &messages)
            .await?;
    Ok(answer.unwrap_or_else(|| json!({"path": "cli"})))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::api::dispatch;
    use crate::remote::identity::Identity;
    use crate::store::{BotConfig, Store};
    use serde_json::json;
    use std::path::Path;

    fn hub() -> (Arc<Hub>, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("codync-provider-mode-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::open(Path::new(":memory:")).unwrap();
        let hub = Arc::new(Hub::new(store, "test".into(), Identity::load_or_create(&dir).unwrap(), "token".into(), 0));
        let config: BotConfig = serde_json::from_value(json!({
            "id": "b", "name": "Bot", "backend": "custom", "command": "my-agent --acp"
        }))
        .unwrap();
        hub.store.save_bot(&config).unwrap();
        (hub, dir)
    }

    #[tokio::test]
    async fn a_bot_keeps_its_cli_path_before_the_mode_is_set() {
        let (hub, dir) = hub();
        let read = dispatch(&hub, &Caller::Local, "customProviderMode", json!({"botId": "b"})).await.unwrap();
        assert_eq!(read["mode"], "cli");
        assert!(!read.to_string().contains("api.openai.com"));

        let set =
            dispatch(&hub, &Caller::Local, "setCustomProviderMode", json!({"botId": "b", "mode": "openai-compatible"}))
                .await
                .unwrap();
        assert_eq!(set["mode"], "openai-compatible");
        assert!(!set.to_string().contains("sk-"));

        let stored = hub.store.bot("b").unwrap().unwrap().config;
        assert_eq!(stored.custom_provider_mode, CustomProviderMode::OpenaiCompatible);
        assert_eq!(stored.command.as_deref(), Some("my-agent --acp"), "the CLI command is untouched");
        assert_eq!(
            dispatch(&hub, &Caller::Local, "customProviderMode", json!({"botId": "b"})).await.unwrap()["mode"],
            "openai-compatible"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[tokio::test]
    async fn a_url_or_a_key_is_refused_as_a_mode() {
        let (hub, dir) = hub();
        for bad in ["https://api.openai.com/v1", "sk-secret", "OpenAI", ""] {
            let error = dispatch(&hub, &Caller::Local, "setCustomProviderMode", json!({"botId": "b", "mode": bad}))
                .await
                .unwrap_err();
            assert!(error.to_string().contains("mode must be"), "{error}");
        }
        assert_eq!(hub.store.bot("b").unwrap().unwrap().config.custom_provider_mode, CustomProviderMode::Cli);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[tokio::test]
    async fn a_request_with_the_mode_off_stays_on_the_cli_path() {
        let (hub, dir) = hub();
        crate::market::vault::write(
            &hub.store,
            "agent-env:custom",
            &json!({"OPENAI_BASE_URL": "https://api.openai.com/v1", "OPENAI_API_KEY": "sk-test"}).to_string(),
        )
        .unwrap();
        let answer = dispatch(
            &hub,
            &Caller::Local,
            "customProviderRequest",
            json!({"botId": "b", "model": "gpt-6-astra", "messages": [{"role": "user", "content": "hi"}]}),
        )
        .await
        .unwrap();
        assert_eq!(answer["path"], "cli");
        assert!(!answer.to_string().contains("sk-test"));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
