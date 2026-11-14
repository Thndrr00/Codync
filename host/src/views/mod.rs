//! Shared view state (card K4): a small, versioned JSON document per view (`chat:<botId>`,
//! `routine:<botId>`) that the apps patch as the person moves around, and named actions the agent
//! invokes so it acts on what the person sees without touching storage itself.

mod actions;
mod state;

pub use actions::invoke;
pub use state::{Kind, ViewId, ViewState};

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

use crate::api::devices::Caller;
use crate::hub::Hub;
use anyhow::{Result, anyhow, bail};
use serde_json::{Value, json};
use std::sync::Arc;

/// The host API methods of view state, with strict validation of every input.
pub const METHODS: &[&str] = &["readViewState", "patchViewState"];

/// The `chat` MCP server's view tool.
pub const TOOLS: [&str; 1] = ["view"];

pub fn tool() -> Value {
    json!({
        "name": "view",
        "description": "Work with what the person is looking at. `list` shows this bot's views (its chat and its routine \
            canvas) with their current state and the named actions each declares. `read` shows one view's state. `act` \
            invokes one of its actions; the host checks the action against the view's declared list and performs it \
            through the same host API the apps use, so the person's screen updates and the database is never changed \
            from the model. Views: chat:<botId> and routine:<botId> (your own bot only). Actions: chat open_thread \
            {rootId}; routine create_routine {name, instruction, schedule} and select_routine {routineId}. A schedule is \
            the routine editor's draft: {kind: cron, calendarStyle: daily|weekdays|weekly|monthly|yearly|hours, hour, \
            minute, weekday, monthDay, zone}, {kind: once, at: epoch ms} or {kind: interval, amount, unit}. Use it when \
            the person says \"put a routine for Monday at 8\" while the routine canvas is open: create it there instead \
            of only describing it.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "op": {"type": "string", "enum": ["list", "read", "act"]},
                "view": {"type": "string", "description": "chat:<botId> or routine:<botId>"},
                "action": {"type": "string", "description": "A declared action of the view, for op act."},
                "arguments": {"type": "object", "description": "The action's own arguments."}
            },
            "required": ["op"]
        },
        "annotations": {"readOnlyHint": false, "idempotentHint": false}
    })
}

/// The `chat` server's view tool: a bot works only with its own views.
pub fn call(hub: &Arc<Hub>, bot: &str, name: &str, args: &Value) -> Result<Value> {
    if !TOOLS.contains(&name) {
        bail!("unknown view tool: {name}");
    }
    match args["op"].as_str().unwrap_or("list") {
        "list" => list(hub, bot),
        "read" => {
            let view = own_view(args, bot)?;
            let state = state::read(&hub.store, &view)?;
            Ok(json!({
                "view": view.as_str(),
                "rev": state.rev,
                "state": state.state,
                "actions": actions::names(view.kind),
            }))
        }
        "act" => {
            let view = own_view(args, bot)?;
            let action = args["action"].as_str().ok_or_else(|| anyhow!("`action` is required for op act"))?;
            let arguments =
                args.get("arguments").filter(|value| value.is_object()).cloned().unwrap_or_else(|| json!({}));
            actions::invoke(hub, &view, action, &arguments)
        }
        other => bail!("`op` must be list, read or act (not \"{other}\")"),
    }
}

/// The views a bot has: its chat, and its routine canvas unless it is a group.
fn list(hub: &Hub, bot: &str) -> Result<Value> {
    let row = hub.store.bot(bot)?.filter(|row| !row.deleted).ok_or_else(|| anyhow!("unknown bot"))?;
    let mut views = vec![summary(hub, &ViewId { kind: Kind::Chat, bot: bot.to_owned() })?];
    if !row.config.is_group() {
        views.push(summary(hub, &ViewId { kind: Kind::Routine, bot: bot.to_owned() })?);
    }
    Ok(json!({"views": views}))
}

fn summary(hub: &Hub, view: &ViewId) -> Result<Value> {
    let state = state::read(&hub.store, view)?;
    Ok(json!({
        "view": view.as_str(),
        "rev": state.rev,
        "state": state.state,
        "actions": actions::names(view.kind),
    }))
}

fn own_view(args: &Value, bot: &str) -> Result<ViewId> {
    let view = args["view"].as_str().ok_or_else(|| anyhow!("`view` is required (chat:<botId> or routine:<botId>)"))?;
    let view = ViewId::parse(view)?;
    if view.bot != bot {
        bail!("\"{}\" isn't your view: use message_bot to ask that bot about its own views", view.as_str());
    }
    Ok(view)
}

/// `readViewState {view}` and `patchViewState {view, baseRev?, ops}`.
pub async fn api_call(hub: &Arc<Hub>, caller: &Caller, method: &str, b: &Value) -> Result<Value> {
    let view = ViewId::parse(b["view"].as_str().ok_or_else(|| anyhow!("`view` is required"))?)?;
    visible(hub, caller, &view)?;
    match method {
        "readViewState" => {
            let state = state::read(&hub.store, &view)?;
            Ok(json!({
                "view": view.as_str(),
                "rev": state.rev,
                "state": state.state,
                "actions": actions::names(view.kind),
            }))
        }
        "patchViewState" => {
            let mut state = state::read(&hub.store, &view)?.state;
            state::apply(&mut state, &b["ops"])?;
            let base = b["baseRev"].as_i64();
            match state::write(hub, &view, state, base, "client")? {
                state::Write::Saved(saved) => Ok(json!({
                    "view": view.as_str(),
                    "rev": saved.rev,
                    "state": saved.state,
                })),
                // Someone else wrote first: the client reads this answer and decides what to re-send.
                state::Write::Conflict(current) => Ok(json!({
                    "conflict": true,
                    "view": view.as_str(),
                    "rev": current.rev,
                    "state": current.state,
                })),
            }
        }
        other => bail!("unknown view method: {other}"),
    }
}

/// The bot's name in the view exists and takes that kind of view; a member's device only reaches its
/// own person's bots, as everywhere else.
fn visible(hub: &Hub, caller: &Caller, view: &ViewId) -> Result<()> {
    let row = hub.store.bot(&view.bot)?.filter(|row| !row.deleted).ok_or_else(|| anyhow!("unknown bot"))?;
    if view.kind == Kind::Routine && row.config.is_group() {
        bail!("a group has no routine canvas");
    }
    if let Some(person) = crate::people::caller_person(hub, caller) {
        crate::people::check_bot(hub, &json!({"botId": view.bot}), &person)?;
    }
    Ok(())
}
