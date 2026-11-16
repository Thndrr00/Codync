use super::{Capability, runtime, terminal, workspace};
use crate::hub::Hub;
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::sync::Arc;

pub const METHODS: &[&str] = &["computerPanel", "listComputerFiles", "readComputerFile", "runComputerCommand"];
pub fn is_tool(name: &str) -> bool {
    matches!(name, "workspace_list" | "workspace_read" | "workspace_shell")
}
pub fn tools() -> Vec<Value> {
    vec![
        json!({"name":"workspace_list","description":"List up to 500 entries in a bot workspace directory. Paths are relative; no parent traversal.","inputSchema":{"type":"object","properties":{"path":{"type":"string"}}},"annotations":{"readOnlyHint":true}}),
        json!({"name":"workspace_read","description":"Preview a UTF-8 workspace file (128 KiB maximum).","inputSchema":{"type":"object","properties":{"path":{"type":"string"}},"required":["path"]},"annotations":{"readOnlyHint":true}}),
        json!({"name":"workspace_shell","description":"Run one command in the bot computer (30 seconds; output limited). It runs here, or in the bot's Docker container when its computer runtime is Container; without Docker it falls back to the local workspace and says so in the panel. Requires shell permission.","inputSchema":{"type":"object","properties":{"command":{"type":"string"}},"required":["command"]}}),
    ]
}
pub async fn call(hub: &Arc<Hub>, method: &str, body: &Value, actor_kind: &str, actor_id: &str) -> Result<Value> {
    let bot = body["botId"].as_str().context("botId is required")?;
    if method == "computerPanel" {
        let row = hub.store.bot(bot)?.filter(|row| !row.deleted && !row.config.is_group()).context("unknown bot")?;
        let computer = runtime::Runtime::resolve(&row.config, &row.config.computer_runtime);
        return Ok(json!({"enabled":row.config.computer,"permissions":row.config.computer_permissions,
            "workspace":computer.root().display().to_string(),"screen":hub.screen.state(),"activity":hub.store.computer_activity(bot)?,"protectedBrowserAvailable":cfg!(windows),
            "runtime":runtime::status(hub, &row.config).await,"runtimeChoices":["local","container"],
            "browser":{"profile":computer.browser_profile().display().to_string(),"inContainer":computer.is_container()},
            "scriptLimit":{"perHour":crate::routines::limits::saved(&hub.store),"choices":crate::routines::limits::CHOICES},
            "localApps":crate::local_apps::status(&hub.store)?,
            "planStart":{"afterMs":crate::external::plan::saved(&hub.store),"presets":crate::external::plan::PRESETS},
            "services":{"research":crate::research::configuration_state(&hub.store),"slack":crate::slack::configuration_state(&hub.store)}}));
    }
    let name = match method {
        "listComputerFiles" => "workspace_list",
        "readComputerFile" => "workspace_read",
        "runComputerCommand" => "workspace_shell",
        _ => bail!("unknown computer panel method"),
    };
    let result = tool(hub, bot, name, body).await;
    let outcome = if result.as_ref().is_ok_and(|value| value["success"] != false) {
        crate::store::ComputerActivityResult::Success
    } else {
        crate::store::ComputerActivityResult::Failed
    };
    if let Err(error) = hub.store.record_computer_activity_as(bot, name, outcome, actor_kind, actor_id) {
        tracing::warn!(%error, "couldn't retain owner computer activity");
    }
    result
}
pub async fn tool(hub: &Arc<Hub>, bot: &str, name: &str, args: &Value) -> Result<Value> {
    let capability = if name == "workspace_shell" { Capability::Shell } else { Capability::Files };
    let mut lease = hub.computer.begin(hub, bot, capability)?;
    let cfg = hub.store.bot(bot)?.filter(|row| !row.deleted).context("unknown bot")?.config;
    let runtime = runtime::Runtime::resolve(&cfg, &cfg.computer_runtime);
    if name == "workspace_shell" {
        return terminal::run(&runtime, args["command"].as_str().context("command is required")?, lease).await;
    }
    let root = runtime.root().to_path_buf();
    let guard = lease.clone();
    let work = async {
        match name {
            "workspace_list" => workspace::list(&root, args["path"].as_str().unwrap_or(""), &guard).await,
            "workspace_read" => {
                workspace::read(&root, args["path"].as_str().context("path is required")?, &guard).await
            }
            _ => bail!("unknown workspace tool"),
        }
    };
    tokio::select! { biased; () = lease.cancelled() => bail!("File permission revoked; request cancelled."), result = work => result }
}
