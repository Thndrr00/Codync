//! WebMCP (card K15): a small, declared set of Codync actions a host-served mini-app page may ask
//! for, and nothing else.
//!
//! The page calls `POST /apps/{bot}/{slug}/{token}/_webmcp` with one action of `ACTIONS`. The host
//! checks the app's explicit grant (empty until the person turns an action on in the app panel),
//! validates the arguments against stored bots and artifacts, and announces a `webMcpAction` so the
//! desktop opens the panel. No token, path or arbitrary host method ever reaches the page: its event
//! carries only the ids it already knows.

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

use crate::hub::Hub;
use crate::store::now_ms;
use anyhow::{Result, anyhow, bail};
use serde_json::{Value, json};

/// The whole allow list. Adding one is a code change, a schema and a documented permission.
pub const ACTIONS: [&str; 3] = ["open_chat", "show_artifact", "start_call"];
/// The host API methods the app panel uses to read and change the grant.
pub const METHODS: &[&str] = &["webMcpState", "setWebMcpGrant"];
/// Where each app's grant is stored.
const KEY_PREFIX: &str = "webmcp.v1.";

/// The declared actions, with the words a page and the permission panel show.
pub fn actions() -> Value {
    json!([
        {
            "name": "open_chat",
            "description": "Open a bot's chat in Codync.",
            "arguments": {"type": "object", "properties": {"botId": {"type": "string"}}, "required": ["botId"]}
        },
        {
            "name": "show_artifact",
            "description": "Show one of this bot's artifacts in Codync's library.",
            "arguments": {"type": "object", "properties": {"artifactId": {"type": "string"}}, "required": ["artifactId"]}
        },
        {
            "name": "start_call",
            "description": "Start a voice call with a bot on Codync.",
            "arguments": {"type": "object", "properties": {"botId": {"type": "string"}}, "required": ["botId"]}
        }
    ])
}

fn key(bot: &str, slug: &str) -> String {
    format!("{KEY_PREFIX}{bot}.{slug}")
}

/// The actions this app may ask for; empty until the person grants them.
pub fn granted(hub: &Hub, bot: &str, slug: &str) -> Result<Vec<String>> {
    let Some(raw) = hub.store.kv_read(&key(bot, slug))? else { return Ok(vec![]) };
    let stored: Value = serde_json::from_str(&raw).unwrap_or_else(|_| json!({}));
    let mut names: Vec<String> = stored["actions"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter(|name| ACTIONS.contains(name))
        .map(str::to_owned)
        .collect();
    names.sort();
    names.dedup();
    Ok(names)
}

/// Replaces an app's grant. Every name must be declared; an empty list turns every action off.
pub fn set_granted(hub: &Hub, bot: &str, slug: &str, names: &[String]) -> Result<Vec<String>> {
    let app = hub.store.get_bot_app(bot, slug)?.ok_or_else(|| anyhow!("unknown mini-app"))?;
    let mut kept: Vec<String> = vec![];
    for name in names {
        if !ACTIONS.contains(&name.as_str()) {
            bail!("\"{name}\" isn't a Codync action for pages: they are {}", ACTIONS.join(", "));
        }
        if !kept.contains(name) {
            kept.push(name.clone());
        }
    }
    kept.sort();
    let document = json!({"app": app.slug, "actions": kept, "updatedAt": now_ms()});
    hub.store.kv_set(&key(bot, slug), &document.to_string())?;
    Ok(kept)
}

/// Refuses a call the app wasn't granted, then announces the action to the apps. The page gets back
/// only what it asked for; no credentials, paths or host method names move.
pub fn invoke(hub: &Hub, bot: &str, slug: &str, action: &str, args: &Value) -> Result<Value> {
    if !ACTIONS.contains(&action) {
        bail!("unknown action \"{action}\": pages may call {}", ACTIONS.join(", "));
    }
    let granted = granted(hub, bot, slug)?;
    if !granted.iter().any(|name| name == action) {
        bail!("this app isn't allowed to \"{action}\" yet: turn it on in the mini-app panel");
    }
    let safe = match action {
        "open_chat" => {
            let id = bot_arg(hub, args)?;
            json!({"botId": id})
        }
        "start_call" => {
            let id = bot_arg(hub, args)?;
            let row = hub.store.bot(&id)?.filter(|row| !row.deleted).ok_or_else(|| anyhow!("unknown bot"))?;
            if row.config.is_group() {
                bail!("{} is a group chat, so it can't take a voice call", row.config.name);
            }
            if crate::external::is_external(&row.config) {
                bail!("{} is a terminal session, so it can't take a voice call", row.config.name);
            }
            json!({"botId": id})
        }
        "show_artifact" => {
            let id = args["artifactId"]
                .as_str()
                .map(str::trim)
                .filter(|id| !id.is_empty())
                .ok_or_else(|| anyhow!("`artifactId` is required"))?;
            if crate::chat::artifacts::get(&hub.store, bot, id).is_none() {
                bail!("that artifact isn't this bot's");
            }
            json!({"artifactId": id})
        }
        _ => unreachable!("checked against ACTIONS above"),
    };
    let _ = hub.events.send(json!({
        "type": "webMcpAction",
        "action": action,
        "arguments": safe,
        "botId": bot,
        "slug": slug,
        "at": now_ms(),
    }));
    Ok(safe)
}

fn bot_arg(hub: &Hub, args: &Value) -> Result<String> {
    let id = args["botId"]
        .as_str()
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .ok_or_else(|| anyhow!("`botId` is required"))?;
    hub.store.bot(id)?.filter(|row| !row.deleted).ok_or_else(|| anyhow!("unknown bot"))?;
    Ok(id.to_owned())
}

/// Whether an `Origin` is this computer's own host (the only pages served by the host, and so the
/// only ones a page call may come from). A missing header is a same-origin call without one, and
/// `null` is the opaque origin of a sandboxed host-served mini-app.
pub fn host_origin(port: u16, origin: Option<&str>) -> bool {
    let Some(origin) = origin else { return true };
    if origin == "null" {
        return true;
    }
    ["127.0.0.1", "localhost", "[::1]"].iter().any(|host| origin == format!("http://{host}:{port}"))
}

/// `webMcpState {botId, slug}` and `setWebMcpGrant {botId, slug, actions}`.
pub fn api(hub: &Hub, method: &str, b: &Value) -> Result<Value> {
    let bot = b["botId"].as_str().ok_or_else(|| anyhow!("`botId` is required"))?;
    let slug = b["slug"].as_str().ok_or_else(|| anyhow!("`slug` is required"))?;
    if hub.store.get_bot_app(bot, slug)?.is_none() {
        bail!("unknown mini-app");
    }
    match method {
        "webMcpState" => {
            let granted = granted(hub, bot, slug)?;
            let actions: Vec<Value> = actions()
                .as_array()
                .into_iter()
                .flatten()
                .map(|action| {
                    let name = action["name"].as_str().unwrap_or_default();
                    json!({
                        "name": name,
                        "description": action["description"],
                        "granted": granted.iter().any(|g| g == name),
                    })
                })
                .collect();
            Ok(json!({"actions": actions}))
        }
        "setWebMcpGrant" => {
            let names: Vec<String> = b["actions"]
                .as_array()
                .map(|list| list.iter().filter_map(Value::as_str).map(str::to_owned).collect())
                .unwrap_or_default();
            let actions = set_granted(hub, bot, slug, &names)?;
            Ok(json!({"actions": actions}))
        }
        other => bail!("unknown WebMCP method: {other}"),
    }
}
