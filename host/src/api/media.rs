//! What a bot can ask its agent to make, so an app offers only the models, frames and lengths that
//! work instead of naming them itself.

use super::str_arg;
use crate::hub::Hub;
use anyhow::{Result, anyhow};
use serde_json::{Value, json};

pub fn models(hub: &Hub, body: &Value) -> Result<Value> {
    let bot = str_arg(body, "botId")?;
    let row = hub
        .store
        .bot(bot)?
        .filter(|row| !row.deleted && !row.config.is_group())
        .ok_or_else(|| anyhow!("unknown bot"))?;
    let backend = row.config.backend.as_str();
    Ok(json!({
        // Null when the bot has no agent that makes media at all.
        "agent": crate::media::catalog::models(backend, None).next().is_some().then_some(backend),
        "installed": crate::agent::backends::installed(backend),
        "models": crate::media::catalog::models(backend, None).map(crate::media::catalog::json).collect::<Vec<_>>(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::identity::Identity;
    use crate::store::{BotConfig, Store};
    use std::sync::Arc;

    fn hub_with(backend: &str) -> Arc<Hub> {
        let dir = std::env::temp_dir().join(format!("codync-media-api-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::open(&dir.join("t.db")).unwrap();
        let cfg: BotConfig =
            serde_json::from_value(json!({"id":"b1","name":"Bot","backend":backend,"cwd":dir})).unwrap();
        store.save_bot(&cfg).unwrap();
        Hub::new(store, "media-api-test".into(), Identity::load_or_create(&dir).unwrap(), "test".into(), 0)
    }

    #[test]
    fn a_codex_bot_is_offered_the_gpt_image_model_only() {
        let hub = hub_with("codex");
        let answer = models(&hub, &json!({"botId": "b1"})).unwrap();
        assert_eq!(answer["agent"], "codex");
        assert_eq!(answer["models"].as_array().unwrap().len(), 1);
        assert_eq!(answer["models"][0]["id"], "gpt-image");
    }

    #[test]
    fn a_bot_without_media_tools_is_offered_none_and_an_unknown_bot_is_refused() {
        let hub = hub_with("claude");
        let answer = models(&hub, &json!({"botId": "b1"})).unwrap();
        assert!(answer["agent"].is_null());
        assert_eq!(answer["models"].as_array().unwrap().len(), 0);
        assert!(models(&hub, &json!({"botId": "nope"})).is_err());
        assert!(models(&hub, &json!({})).is_err());
    }
}
