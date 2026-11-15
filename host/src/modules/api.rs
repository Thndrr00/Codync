//! The module methods the apps and the bots' MCP server call.

use super::install::{self, Source};
use super::{Modules, id_arg, installed};
use crate::agent::bot::Cmd;
use crate::api::devices::Caller;
use crate::hub::Hub;
use anyhow::{Result, anyhow};
use serde_json::{Value, json};
use std::sync::Arc;

/// The methods a module answers. Installing and removing touch this computer's disk, so only its own apps ask.
pub const METHODS: [&str; 10] = [
    "modules",
    "installModule",
    "updateModule",
    "uninstallModule",
    "setModule",
    "approveModule",
    "exportModule",
    "moduleView",
    "moduleCall",
    "moduleTools",
];

/// The methods a paired phone may use: looking, turning one on or off, and approving what it asks for.
pub const DEVICE_METHODS: [&str; 4] = ["modules", "setModule", "approveModule", "moduleView"];

pub async fn call(hub: &Arc<Hub>, method: &str, b: Value) -> Result<Value> {
    Ok(match method {
        "modules" => json!({"modules": hub.modules.rows()}),
        "installModule" => {
            let answer = install::install(&hub.modules, &hub.store, Source::from_body(&b)?)?;
            refresh_bots(hub);
            answer
        }
        "updateModule" => {
            let id = id_arg(&b)?;
            let answer = install::update(&hub.modules, &hub.store, &id, Source::from_body(&b)?)?;
            refresh_bots(hub);
            answer
        }
        "uninstallModule" => {
            let id = id_arg(&b)?;
            let answer = install::uninstall(&hub.modules, &hub.store, &id)?;
            refresh_bots(hub);
            answer
        }
        "setModule" => {
            let id = id_arg(&b)?;
            let enabled = b["enabled"].as_bool().ok_or_else(|| anyhow!("`enabled` is required"))?;
            let answer = hub.modules.set_enabled(&hub.store, &id, enabled)?;
            refresh_bots(hub);
            answer
        }
        "approveModule" => {
            let id = id_arg(&b)?;
            let answer = hub.modules.approve(&hub.store, &id, &b["permissions"])?;
            refresh_bots(hub);
            answer
        }
        "exportModule" => install::export(&hub.modules, &id_arg(&b)?)?,
        "moduleView" => {
            let id = id_arg(&b)?;
            let view = b["view"].as_str().unwrap_or_default();
            let bot = b["botId"].as_str().unwrap_or_default();
            json!({"view": hub.modules.view(&id, view, bot).await?})
        }
        "moduleTools" => {
            let id = id_arg(&b)?;
            let module = installed(&hub.modules, &id)?;
            json!({"tools": tools(&module)})
        }
        "moduleCall" => {
            let id = id_arg(&b)?;
            let name = b["name"].as_str().filter(|n| !n.is_empty()).ok_or_else(|| anyhow!("`name` is required"))?;
            let arguments = b.get("arguments").filter(|a| a.is_object()).cloned().unwrap_or_else(|| json!({}));
            let bot = b["botId"].as_str().unwrap_or_default();
            match hub.modules.call(&id, name, &arguments, bot).await {
                Ok(result) => result,
                // A tool that failed is something the agent reads and works around, not a broken protocol.
                Err(error) => json!({"content": [{"type": "text", "text": format!("{error:#}")}], "isError": true}),
            }
        }
        other => return Err(anyhow!("unknown module method {other}")),
    })
}

/// The tools of one module a bot may call, in the shape MCP lists them.
pub fn tools(module: &super::Installed) -> Vec<Value> {
    let Some(manifest) = module.manifest.as_ref() else { return Vec::new() };
    manifest
        .tools
        .iter()
        .filter(|tool| module.approved().tool(&tool.name))
        .map(|tool| {
            json!({
                "name": tool.name,
                "description": tool.description,
                "inputSchema": tool.input_schema,
            })
        })
        .collect()
}

/// Every bot's tool list changes with a module, so they all ask again.
fn refresh_bots(hub: &Hub) {
    let Ok(bots) = hub.store.bots() else { return };
    for bot in bots.iter().filter(|b| !b.deleted && !b.config.is_group()) {
        if let Err(error) = hub.send_cmd(&bot.config.id, Cmd::RefreshTools) {
            tracing::warn!(bot = %bot.config.id, error = format!("{error:#}"), "asking a bot for its tools again");
        }
    }
}

/// What the MCP server of one module answers before anything else: how it works, for the agent reading it.
pub const INSTRUCTIONS: &str = "Tools from a Codync module the owner installed. Each one runs in the module's own \
process: it may take a few seconds, and it can stop working if the owner turned the module off. A tool that answers \
that it isn't approved is waiting for the owner; ask them to approve it in Extensions.";

/// Whether `method` is one this computer's own apps may call (the rest need the local token).
pub fn device_allowed(method: &str) -> bool {
    DEVICE_METHODS.contains(&method)
}

/// The module methods, or `None` when this isn't one.
pub async fn route(hub: &Arc<Hub>, caller: &Caller, method: &str, b: Value) -> Result<Option<Value>> {
    if !METHODS.contains(&method) {
        return Ok(None);
    }
    if !matches!(caller, Caller::Local) && !device_allowed(method) {
        return Err(crate::api::devices::Forbidden.into());
    }
    call(hub, method, b).await.map(Some)
}

/// The modules the bots get an MCP server for.
pub fn bot_servers(modules: &Modules) -> Vec<String> {
    modules.for_bots().into_iter().map(|module| module.id).collect()
}
