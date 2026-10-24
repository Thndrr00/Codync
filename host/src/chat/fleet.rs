//! Fleet management: a bot the user lets organize others (`fleetManagement`, off by default)
//! creates bots and groups and tunes the bots it created, through the `team` server's fleet
//! tools. The flag is read again on every call, so turning it off stops the next one.
//!
//! What it can't do: delete or move anything, edit a bot it didn't create (`createdBy`), change a
//! bot's permissions, approval rules, computer access or fleet flag, or give a bot more than it
//! has itself: a new bot gets its creator's agent, permission setting, approval rules, connectors
//! and skills, in a workspace of its own, without the computer or fleet tools. Each change leaves
//! a notice in the creator's chat (and an instruction change in the bot's own).
//!
//! Idea from rakazo's `packages/adapters/src/fleet-tools.ts` (Apache-2.0); no code copied.

use crate::chat::instruction_history as history;
use crate::hub::Hub;
use crate::store::{BotConfig, EntryKind, Lane};
use anyhow::{Context, Result, anyhow, bail};
use serde_json::{Value, json};
use std::sync::Arc;

/// Bots one bot may create.
const MAX_CHILDREN: usize = 20;
/// Bots on the computer past which no fleet tool creates more.
const MAX_BOTS: usize = 200;
const GROUP_MEMBERS: std::ops::RangeInclusive<usize> = 2..=10;
const MAX_INSTRUCTIONS_CHARS: usize = 4000;

const TOOLS: [&str; 6] =
    ["list_fleet", "create_bot", "create_group", "set_bot_instructions", "set_bot_model", "create_bot_routine"];

pub fn handles(name: &str) -> bool {
    TOOLS.contains(&name)
}

pub fn tools() -> Vec<Value> {
    let off = "Only for a bot the user turned fleet management on for.";
    vec![
        json!({"name":"list_fleet","description":format!("List the user's bots and groups: ids, names, agents, models, and which you created. {off}"),
            "inputSchema":{"type":"object","properties":{}},"annotations":{"readOnlyHint":true}}),
        json!({"name":"create_bot","description":format!("Create a bot with your agent, permissions, connectors and skills, in a workspace of its own. You can tune the bots you create; never create one the user didn't ask for. {off}"),
            "inputSchema":{"type":"object","properties":{
                "name":{"type":"string"},
                "instructions":{"type":"string","description":"What it does and how (its description)."},
                "model":{"type":"string","description":"A model id your agent offers; yours when omitted."}},
            "required":["name","instructions"]}}),
        json!({"name":"create_group","description":format!("Create a group chat of {}-{} bots (ids from list_fleet). {off}", GROUP_MEMBERS.start(), GROUP_MEMBERS.end()),
            "inputSchema":{"type":"object","properties":{"name":{"type":"string"},"memberIds":{"type":"array","items":{"type":"string"}}},"required":["name","memberIds"]}}),
        json!({"name":"set_bot_instructions","description":format!("Replace the instructions of a bot you created. Pass ifVersion (from list_fleet) so a change someone made since isn't overwritten. {off}"),
            "inputSchema":{"type":"object","properties":{"botId":{"type":"string"},"instructions":{"type":"string"},"reason":{"type":"string"},"ifVersion":{"type":"integer"}},"required":["botId","instructions"]}}),
        json!({"name":"set_bot_model","description":format!("Change the model of a bot you created. {off}"),
            "inputSchema":{"type":"object","properties":{"botId":{"type":"string"},"model":{"type":"string"}},"required":["botId","model"]}}),
        json!({"name":"create_bot_routine","description":format!("Create a routine for a bot you created; same fields as save_routine (name, instruction, triggers, timeoutSeconds). {off}"),
            "inputSchema":{"type":"object","properties":{"botId":{"type":"string"},"name":{"type":"string"},"instruction":{"type":"string"},"triggers":{"type":"array","items":{"type":"object"}},"timeoutSeconds":{"type":"integer"}},"required":["botId","name","instruction","triggers"]}}),
    ]
}

fn text<'a>(args: &'a Value, key: &str) -> Result<&'a str> {
    args[key].as_str().map(str::trim).filter(|s| !s.is_empty()).ok_or_else(|| anyhow!("{key} is required"))
}

/// A bot `source` created, by id.
fn child(hub: &Hub, source: &BotConfig, id: &str) -> Result<BotConfig> {
    let bot = hub.store.bot(id)?.filter(|r| !r.deleted).map(|r| r.config).ok_or_else(|| anyhow!("unknown bot {id}"))?;
    if bot.created_by.as_deref() != Some(source.id.as_str()) {
        bail!("{} wasn't created by you: you can only change the bots you created", bot.name);
    }
    Ok(bot)
}

fn notice(hub: &Hub, bot: &str, text: &str) {
    hub.add_entry(&Lane::main(bot), EntryKind::Notice, hub.store.max_turn(bot), &json!({"text": text}));
}

pub async fn call(hub: &Arc<Hub>, source: &BotConfig, name: &str, args: &Value) -> Result<Value> {
    if !source.fleet_management {
        bail!("Fleet management is off for you; the user can turn it on in your settings");
    }
    match name {
        "list_fleet" => list(hub, source),
        "create_bot" => create_bot(hub, source, args).await,
        "create_group" => create_group(hub, source, args).await,
        "set_bot_instructions" => {
            let bot = child(hub, source, text(args, "botId")?)?;
            let instructions = args["instructions"].as_str().unwrap_or_default().trim().to_owned();
            if instructions.chars().count() > MAX_INSTRUCTIONS_CHARS {
                bail!("instructions must be at most {MAX_INSTRUCTIONS_CHARS} characters");
            }
            let reason = args["reason"].as_str().unwrap_or("no reason given");
            let patch = json!({"id": bot.id, "description": instructions, history::AUTHOR: history::Author::Fleet,
                history::REASON: format!("{}: {reason}", source.name), history::IF_VERSION: args["ifVersion"]});
            update(hub, patch).await?;
            notice(hub, &bot.id, &format!("Instructions updated by {}", source.name));
            notice(hub, &source.id, &format!("Updated {}'s instructions", bot.name));
            Ok(json!({"saved": true}))
        }
        "set_bot_model" => {
            let bot = child(hub, source, text(args, "botId")?)?;
            let model = text(args, "model")?;
            update(hub, json!({"id": bot.id, "model": model})).await?;
            notice(hub, &source.id, &format!("Set {}'s model to {model}", bot.name));
            Ok(json!({"saved": true}))
        }
        "create_bot_routine" => {
            let bot = child(hub, source, text(args, "botId")?)?;
            let mut body = args.clone();
            if let Some(fields) = body.as_object_mut() {
                // A new routine, never one of the bot's existing ones.
                fields.remove("id");
                fields.remove("botId");
            }
            let saved = hub.routines.save(hub, &bot.id, &body)?;
            notice(hub, &source.id, &format!("Created a routine for {}: {}", bot.name, text(args, "name")?));
            Ok(saved)
        }
        _ => bail!("unknown fleet tool: {name}"),
    }
}

async fn update(hub: &Arc<Hub>, patch: Value) -> Result<Value> {
    let hub = hub.clone();
    tokio::task::spawn_blocking(move || hub.update_bot(&patch)).await?
}

fn list(hub: &Hub, source: &BotConfig) -> Result<Value> {
    let rows = hub.store.bots()?;
    let items: Vec<Value> = rows
        .iter()
        .filter(|r| !r.deleted && !r.config.hidden && r.config.id != source.id)
        .map(|r| {
            let b = &r.config;
            let version = history::list(&hub.store, b).map_or(Value::Null, |v| v["current"].clone());
            json!({"id": b.id, "name": b.name, "kind": b.kind, "members": b.members, "backend": b.backend,
                "model": b.model, "description": b.description, "instructionsVersion": version,
                "createdByYou": b.created_by.as_deref() == Some(source.id.as_str())})
        })
        .collect();
    Ok(json!({"bots": items}))
}

/// Refuses another bot past the limits.
fn room_for_one(hub: &Hub, source: &BotConfig) -> Result<()> {
    let live: Vec<BotConfig> = hub.store.bots()?.into_iter().filter(|r| !r.deleted).map(|r| r.config).collect();
    if live.len() >= MAX_BOTS {
        bail!("There are already {MAX_BOTS} bots on this computer");
    }
    if live.iter().filter(|b| b.created_by.as_deref() == Some(source.id.as_str())).count() >= MAX_CHILDREN {
        bail!("You already created {MAX_CHILDREN} bots and groups");
    }
    Ok(())
}

async fn create_bot(hub: &Arc<Hub>, source: &BotConfig, args: &Value) -> Result<Value> {
    if source.is_group() {
        bail!("A group can't create bots");
    }
    room_for_one(hub, source)?;
    let name = text(args, "name")?;
    let instructions = args["instructions"].as_str().unwrap_or_default().trim();
    if instructions.chars().count() > MAX_INSTRUCTIONS_CHARS {
        bail!("instructions must be at most {MAX_INSTRUCTIONS_CHARS} characters");
    }
    let model = args["model"]
        .as_str()
        .map(str::trim)
        .filter(|m| !m.is_empty())
        .map(str::to_owned)
        .or_else(|| source.model.clone());
    // Never more than its creator has: same agent and permissions, its connectors and skills.
    let cfg: BotConfig = serde_json::from_value(json!({
        "id": "", "name": name, "description": instructions, "backend": source.backend, "command": source.command,
        "cwd": "", "permission": source.permission, "approvalRules": source.approval_rules, "model": model,
        "account": source.account, "connectors": source.connectors, "skills": source.skills,
        "createdBy": source.id,
    }))
    .context("couldn't describe the new bot")?;
    let created = {
        let hub = hub.clone();
        tokio::task::spawn_blocking(move || hub.create_bot(cfg)).await??
    };
    notice(hub, &source.id, &format!("Created {name}"));
    Ok(json!({"bot": {"id": created["id"], "name": created["name"]}}))
}

async fn create_group(hub: &Arc<Hub>, source: &BotConfig, args: &Value) -> Result<Value> {
    room_for_one(hub, source)?;
    let name = text(args, "name")?;
    let members: Vec<String> =
        serde_json::from_value(args["memberIds"].clone()).context("memberIds is a list of bot ids")?;
    if !GROUP_MEMBERS.contains(&members.len()) {
        bail!("A group has {} to {} bots", GROUP_MEMBERS.start(), GROUP_MEMBERS.end());
    }
    let cfg: BotConfig = serde_json::from_value(
        json!({"id": "", "kind": "group", "name": name, "members": members, "createdBy": source.id}),
    )
    .context("couldn't describe the group")?;
    let created = {
        let hub = hub.clone();
        tokio::task::spawn_blocking(move || hub.create_bot(cfg)).await??
    };
    notice(hub, &source.id, &format!("Created the group {name}"));
    Ok(json!({"group": {"id": created["id"], "name": created["name"], "members": created["members"]}}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::identity::Identity;
    use crate::store::Store;

    fn hub(fleet: bool) -> (Arc<Hub>, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("codync-fleet-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        for (id, fleet) in [("lead", fleet), ("other", false)] {
            let cfg: BotConfig = serde_json::from_value(json!({"id": id, "name": id, "backend": "claude",
                "cwd": dir.to_string_lossy(), "fleetManagement": fleet, "computer": true}))
            .unwrap();
            store.save_bot(&cfg).unwrap();
        }
        let hub = Hub::new(store, "fleet-test".into(), Identity::load_or_create(&dir).unwrap(), "test".into(), 0);
        (hub, dir)
    }

    fn bot(hub: &Hub, id: &str) -> BotConfig {
        hub.store.bot(id).unwrap().unwrap().config
    }

    #[tokio::test]
    async fn a_lead_creates_and_tunes_only_its_own_bots() {
        let (hub, dir) = hub(true);
        let lead = bot(&hub, "lead");
        let made = call(&hub, &lead, "create_bot", &json!({"name": "Researcher", "instructions": "Finds sources."}))
            .await
            .unwrap();
        let id = made["bot"]["id"].as_str().unwrap().to_owned();
        let child_bot = bot(&hub, &id);
        assert_eq!(child_bot.created_by.as_deref(), Some("lead"));
        assert!(!child_bot.computer && !child_bot.fleet_management, "nothing its creator has to give");
        call(
            &hub,
            &lead,
            "set_bot_instructions",
            &json!({"botId": id, "instructions": "Finds primary sources.", "ifVersion": 1}),
        )
        .await
        .unwrap();
        assert_eq!(bot(&hub, &id).description, "Finds primary sources.");
        let stale = json!({"botId": id, "instructions": "Other.", "ifVersion": 1});
        assert!(call(&hub, &lead, "set_bot_instructions", &stale).await.is_err());
        let foreign = json!({"botId": "other", "instructions": "Mine now."});
        assert!(call(&hub, &lead, "set_bot_instructions", &foreign).await.is_err());
        let group = json!({"name": "Research", "memberIds": [id, "other"]});
        assert!(call(&hub, &lead, "create_group", &group).await.is_ok());
        assert!(call(&hub, &lead, "create_group", &json!({"name": "Solo", "memberIds": ["other"]})).await.is_err());
        let listed = call(&hub, &lead, "list_fleet", &json!({})).await.unwrap();
        assert!(listed["bots"].as_array().unwrap().iter().any(|b| b["createdByYou"] == true));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn the_flag_gates_every_tool() {
        let (hub, dir) = hub(false);
        let lead = bot(&hub, "lead");
        assert!(call(&hub, &lead, "list_fleet", &json!({})).await.is_err());
        let _ = std::fs::remove_dir_all(dir);
    }
}
