//! Background jobs on the bot's own harness (dreaming, automatic naming): one prompt to a throwaway agent.
//! A subagent child reuses this runner with its parent's MCP servers declared (see `chat::subagents`).

use crate::agent::acp::{self, Acp, Incoming};
use crate::agent::bot::{Adapter, Instructions};
use crate::hub::Hub;
use crate::store::BotConfig;
use anyhow::{Result, anyhow, bail};
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_mins(3);

/// Runs one prompt on a throwaway agent of the bot's harness and returns its reply.
/// Claude gets a real system prompt, no tools, no settings and no saved session; an agent with ACP
/// `systemPrompt` gets the instructions there, other harnesses inline. None of them may remember the job.
pub async fn one_shot(hub: &Arc<Hub>, cfg: &BotConfig, system: &str, user: &str) -> Result<String> {
    run(hub, cfg, system, user, json!([]), false).await
}

/// The subagent's version of the same throwaway turn: it declares MCP servers, and lets Claude use
/// the harness's own tools, so a helper can browse and work with the parent's tools on part of a task.
pub(crate) async fn one_shot_with_servers(
    hub: &Arc<Hub>,
    cfg: &BotConfig,
    system: &str,
    user: &str,
    servers: Value,
) -> Result<String> {
    run(hub, cfg, system, user, servers, true).await
}

/// The MCP servers a normal session of `cfg` declares: its configured connectors and Codync's
/// built-ins. A subagent inherits this list (less the team server) without needing a live `Actor`.
pub(crate) fn bot_servers(hub: &Hub, cfg: &BotConfig) -> Result<Value> {
    let exe = std::env::current_exe()?;
    let port = hub.port;
    let mut servers: Vec<Value> =
        if cfg.connectors.is_empty() { Vec::new() } else { crate::market::connectors(&hub.store)? }
            .iter()
            .filter(|c| cfg.connectors.contains(&c.id) && c.id != crate::tools::comfy::CONNECTOR_ID)
            .filter_map(|c| c.acp(&exe, port))
            .map(|mut server| {
                server["env"] = json!([{"name":"CODYNC_BOT_ID","value":cfg.id}]);
                server
            })
            .collect();
    let composio = crate::market::composio::enabled_for(&hub.store, &cfg.connectors)?;
    let builtin = [
        Some("connectors"),
        Some("chat"),
        Some("team"),
        cfg.manage_team.then_some("manage"),
        Some("memory"),
        Some("routines"),
        cfg.computer.then_some("computer"),
        cfg.connectors.iter().any(|c| c == crate::tools::comfy::CONNECTOR_ID).then_some("comfy"),
        composio.then_some("composio"),
    ];
    for name in builtin.into_iter().flatten() {
        servers.push(crate::mcp::acp_server(name, &exe, &[name, "--bot", &cfg.id], port));
    }
    Ok(Value::Array(servers))
}

async fn run(
    hub: &Arc<Hub>,
    cfg: &BotConfig,
    system: &str,
    user: &str,
    servers: Value,
    with_tools: bool,
) -> Result<String> {
    let cwd = crate::service::data_dir().join("background");
    tokio::fs::create_dir_all(&cwd).await?;
    let cwd = cwd.to_string_lossy().into_owned();
    let need_vault = (with_tools && !cfg.connectors.is_empty())
        || crate::market::vault::saved(&hub.store, &format!("agent-env:{}", cfg.backend))?;
    if need_vault {
        crate::market::vault::unlock(hub.clone()).await?;
    }
    let (env, grants) = crate::agent::auth::launch_env(&hub.store, &cfg.backend, Some(&cfg.id))?;
    let candidates = crate::agent::bot::launch_commands(cfg)?;
    let workspace = cwd.as_str();
    let mut conn = crate::agent::bot::start_first(candidates, &env, |candidate, mut env, _| async move {
        let mut command = [candidate.resolve(|_| {}).await?.acp()];
        crate::agent::own_memory::turn_off(&cfg.backend, workspace, &mut env, &mut command).await;
        crate::agent::bot::start_agent(&command[0], workspace, &env, Duration::from_mins(1)).await
    })
    .await?;
    conn.acp.hold(grants);
    let acp = conn.acp.clone();
    let result = async {
        let mut params = json!({"cwd": cwd, "mcpServers": servers});
        let text = match conn.instructions() {
            Instructions::ClaudeMeta => {
                // A background job gets no tools at all; a subagent keeps its harness tools.
                let mut options = json!({"persistSession": false, "settingSources": [], "model": "haiku"});
                if !with_tools {
                    options["tools"] = json!([]);
                }
                params["_meta"] = json!({"systemPrompt": system, "claudeCode": {"options": options}});
                user.to_owned()
            }
            Instructions::SystemPrompt => {
                params["systemPrompt"] = crate::agent::bot::system_prompt(system);
                user.to_owned()
            }
            Instructions::FirstMessage => format!("{system}\n\n---\n\n{user}"),
        };
        let res = acp.request("session/new", params).await?;
        let sid = res["sessionId"].as_str().ok_or_else(|| anyhow!("agent returned no sessionId"))?.to_owned();
        // Same model as the bot: the harness default may be another (local) model that
        // would load next to the bot's, or a cloud one the user kept these chats away from.
        if conn.adapter != Adapter::Claude {
            crate::agent::bot::select_model(&acp, &res, &sid, cfg).await?;
        }
        let prompt =
            acp.request("session/prompt", json!({"sessionId": sid, "prompt": [{"type": "text", "text": text}]}));
        tokio::pin!(prompt);
        let deadline = tokio::time::sleep(TIMEOUT);
        tokio::pin!(deadline);
        let mut out = String::new();
        loop {
            tokio::select! {
                r = &mut prompt => { r.and_then(acp::codebuddy::prompt_result)?; break }
                inc = conn.rx.recv() => {
                    if !collect(&acp, inc, &mut out).await {
                        bail!("the background job's agent exited");
                    }
                }
                () = &mut deadline => bail!("the background job timed out"),
            }
        }
        while let Ok(inc) = conn.rx.try_recv() {
            collect(&acp, Some(inc), &mut out).await;
        }
        Ok(out)
    }
    .await;
    acp.kill().await;
    result
}

/// Keeps reply text; refuses anything the agent asks of us. False once the agent is gone.
async fn collect(acp: &Acp, inc: Option<Incoming>, out: &mut String) -> bool {
    match inc {
        None | Some(Incoming::Closed { .. }) => false,
        Some(Incoming::Notification { method, params }) => {
            let u = &params["update"];
            if method == "session/update" && u["sessionUpdate"] == "agent_message_chunk" {
                out.push_str(&acp::content_text(&u["content"]));
            }
            true
        }
        Some(Incoming::Request { id, method, .. }) => {
            let _ = if method == "session/request_permission" {
                acp.respond(id, json!({"outcome": {"outcome": "cancelled"}})).await
            } else {
                acp.respond_error(id, -32601, "not supported").await
            };
            true
        }
    }
}
