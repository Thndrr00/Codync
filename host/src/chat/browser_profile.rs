//! Moving several bots to one browser profile (OpenBot 0.72.0: "Put Sales and Support on a new profile
//! named Client A"). A profile keeps its sign-ins, so the bots that share it see the same sites signed
//! in; each bot's open pages close when its profile changes. The bot asks in its own chat, and the chat
//! says which bots moved.

use crate::hub::Hub;
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::sync::Arc;

pub const TOOL: &str = "set_browser_profile";
/// Most bots one call moves.
const MAX_BOTS: usize = 20;
/// Longest profile name; the profile is a folder name on the computer.
const MAX_NAME: usize = 64;

pub fn tool() -> Value {
    json!({
        "name": TOOL,
        "description": "Only when the person asks: put bots on one browser profile, or move them back to their own. A profile keeps its sign-ins, so bots that share it see the same sites signed in; a bot's open pages close when its profile changes. Names one profile; use an empty name to give each bot its own profile again.",
        "inputSchema": {"type":"object","properties":{
            "bots": {"type":"array","items":{"type":"string"},"description":format!("Bot ids or names, at most {MAX_BOTS}.")},
            "profile": {"type":"string","description":format!("The profile's name, at most {MAX_NAME} characters; empty gives each bot its own.")}
        },"required":["bots"]},
        "annotations": {"readOnlyHint": false},
    })
}

/// The profile a call asks for: trimmed, checked, `None` to give each bot its own.
pub fn profile_of(args: &Value) -> Result<Option<String>> {
    let Some(raw) = args["profile"].as_str() else { return Ok(None) };
    let name = raw.trim();
    if name.is_empty() {
        return Ok(None);
    }
    if name.chars().count() > MAX_NAME {
        bail!("A profile's name is at most {MAX_NAME} characters.");
    }
    if name.contains(['/', '\\']) || name == "." || name == ".." {
        bail!("A profile's name holds no slashes.");
    }
    Ok(Some(name.to_owned()))
}

/// The bots a call names, by id or by name: every one has to be an existing agent.
pub fn bots_of(hub: &Hub, args: &Value) -> Result<Vec<crate::store::BotConfig>> {
    let asked: Vec<String> = args["bots"]
        .as_array()
        .context("`bots` lists the bots to move.")?
        .iter()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
        .collect();
    if asked.is_empty() {
        bail!("Name at least one bot.");
    }
    if asked.len() > MAX_BOTS {
        bail!("At most {MAX_BOTS} bots in one call.");
    }
    let rows = hub.store.bots()?;
    let mut out = Vec::with_capacity(asked.len());
    for wanted in asked {
        let row = rows
            .iter()
            .find(|row| {
                !row.deleted && !row.config.is_group() && (row.config.id == wanted || row.config.name == wanted)
            })
            .with_context(|| format!("No bot is called {wanted}."))?;
        out.push(row.config.clone());
    }
    Ok(out)
}

pub async fn call(hub: &Arc<Hub>, bot: &str, args: &Value) -> Result<Value> {
    let profile = profile_of(args)?;
    let bots = bots_of(hub, args)?;
    let mut moved = Vec::with_capacity(bots.len());
    for cfg in bots {
        hub.update_bot(&json!({"id": cfg.id, "browserProfile": profile}))?;
        moved.push(cfg.name);
    }
    let where_to = match &profile {
        Some(name) => format!("the browser profile {name}"),
        None => "their own browser profile".to_owned(),
    };
    let list = moved.join(", ");
    hub.add_entry(
        &crate::store::Lane::main(bot),
        crate::store::EntryKind::Notice,
        hub.store.max_turn(bot),
        &json!({"text": format!("Moved {list} to {where_to}.")}),
    );
    Ok(json!({"moved": moved, "profile": profile}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_profile_name_is_trimmed_and_never_a_path() {
        assert_eq!(profile_of(&json!({"profile": "  Client A "})).unwrap().as_deref(), Some("Client A"));
        assert_eq!(profile_of(&json!({"profile": "   "})).unwrap(), None);
        assert_eq!(profile_of(&json!({})).unwrap(), None);
        assert!(profile_of(&json!({"profile": "a/b"})).is_err());
        assert!(profile_of(&json!({"profile": ".."})).is_err());
        assert!(profile_of(&json!({"profile": "x".repeat(MAX_NAME + 1)})).is_err());
    }
}
