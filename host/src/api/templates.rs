//! Agent template methods: save a bot's setup, share it by link and install it
//! as a new bot. All validation and storage live in `market::templates`; this
//! group only maps methods to calls and keeps template content out of analytics.

use super::devices::Caller;
use crate::hub::Hub;
use crate::market::templates;
use anyhow::Result;
use serde_json::{Map, Value, json};
use std::ops::ControlFlow;
use std::sync::Arc;

pub(super) async fn call(
    hub: &Arc<Hub>,
    _caller: &Caller,
    method: &str,
    b: Value,
) -> Result<ControlFlow<Value, Value>> {
    let answer = match method {
        "templateSave" => json!({"template": templates::save(hub, &b)?}),
        "templateList" => templates::list(&hub.store)?,
        "templateGet" => templates::get(&hub.store, &b)?,
        "templatePreview" => templates::preview(hub, &b)?,
        "templateRename" => templates::rename(&hub.store, &b)?,
        "templateDelete" => templates::delete(&hub.store, &b)?,
        "templatePublish" => templates::publish(&hub.store, &b)?,
        "templateUnpublish" => templates::unpublish(&hub.store, &b)?,
        "templateInstall" => {
            let bot = templates::install(hub, &b).await?;
            // No new event: this is a bot being created. Only fixed properties
            // travel — never a template name, description or instruction.
            let mut properties = Map::new();
            properties.insert("kind".into(), json!("bot"));
            properties.insert("via".into(), json!("template"));
            crate::analytics::capture(hub, crate::analytics::Event::BotCreated, properties);
            json!({"bot": bot})
        }
        _ => return Ok(ControlFlow::Continue(b)),
    };
    Ok(ControlFlow::Break(answer))
}
