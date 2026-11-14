//! The flow tools of the built-in `routines` MCP server: a bot builds its routines' flows by talking.

use super::{MAX_INSTRUCTION_BYTES, api};
use crate::hub::Hub;
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::sync::Arc;

pub const TOOLS: &[&str] = &[
    "link_routine_bots",
    "unlink_routine_bots",
    "list_routine_flow",
    "add_routine_media_step",
    "remove_routine_media_step",
];

pub fn tools() -> Vec<Value> {
    let id = json!({"type": "string"});
    vec![
        json!({"name": "link_routine_bots", "description": format!("Make one of your routines hand its reply on to another bot (a routine flow). When the routine's run ends, its reply goes to toBotId with your instruction (at most {MAX_INSTRUCTION_BYTES} bytes), and that bot's reply can go on to the next link. fromBotId is you unless you chain from a bot the flow already reaches. toBotId may also be a media step's nodeKey (see add_routine_media_step). A flow can't loop, can't reach more than a few steps deep, and can't end in a group or a terminal session. Use list_bots for ids. Linking only applies to the routine's later runs."),
            "inputSchema": {"type": "object", "properties": {"routineId": id, "toBotId": id, "fromBotId": id, "instruction": {"type": "string"}}, "required": ["routineId", "toBotId"]}}),
        json!({"name": "unlink_routine_bots", "description": "Remove a link of one of your routines' flows: by linkId, or by routineId with toBotId (and fromBotId when it isn't you).",
            "inputSchema": {"type": "object", "properties": {"linkId": id, "routineId": id, "toBotId": id, "fromBotId": id}}}),
        json!({"name": "list_routine_flow", "description": "List the flows of your routines: each routine's links (who hands to whom, with the instruction), its media steps, and the steps of its latest run with their status. routineId narrows it to one routine.",
            "inputSchema": {"type": "object", "properties": {"routineId": id}}}),
        json!({"name": "add_routine_media_step", "description": "Add a step to one of your routines' flows that makes an image or a short video with the agent's own subscription (GPT Image on Codex, Grok Imagine on Grok) instead of a bot. Link to it with link_routine_bots and its nodeKey; whatever the steps before it hand on is added to its description. Model ids: gpt-image (image only), grok-imagine (image and video).",
            "inputSchema": {"type": "object", "properties": {"routineId": id, "kind": {"type": "string", "enum": ["image", "video"]}, "prompt": {"type": "string"}, "model": {"type": "string"}, "aspect": {"type": "string", "enum": ["1:1", "4:3", "16:9", "9:16"]}, "transparent": {"type": "boolean"}, "voice": {"type": "string", "enum": ["ara", "eve", "leo", "rex"]}}, "required": ["routineId", "kind"]}}),
        json!({"name": "remove_routine_media_step", "description": "Remove a media step of one of your routines, with the links that reach or leave it.",
            "inputSchema": {"type": "object", "properties": {"mediaId": id}, "required": ["mediaId"]}}),
    ]
}

fn text<'a>(args: &'a Value, key: &str) -> Result<&'a str> {
    args[key].as_str().filter(|s| !s.is_empty()).with_context(|| format!("{key} is required"))
}

pub fn call(hub: &Arc<Hub>, bot: &str, name: &str, args: &Value) -> Result<Value> {
    match name {
        "link_routine_bots" => {
            let routine_id = text(args, "routineId")?;
            let routine = owned(hub, bot, routine_id)?;
            let from = args["fromBotId"].as_str().unwrap_or(bot);
            let instruction = args["instruction"].as_str().unwrap_or_default().trim();
            if instruction.len() > MAX_INSTRUCTION_BYTES {
                bail!("instruction must be at most {MAX_INSTRUCTION_BYTES} bytes");
            }
            let link = api::connect(hub, &routine, from, text(args, "toBotId")?, instruction.to_owned())?;
            Ok(json!({"link": link, "note": "Linked. The routine's next run hands its reply on."}))
        }
        "unlink_routine_bots" => {
            let link = if let Some(id) = args["linkId"].as_str() {
                hub.store.flow_link(id)?.context("link not found")?
            } else {
                let (routine_id, to) = (text(args, "routineId")?, text(args, "toBotId")?);
                let from = args["fromBotId"].as_str().unwrap_or(bot);
                hub.store
                    .flow_links(routine_id)?
                    .into_iter()
                    .find(|l| l.from_bot_id == from && l.to_bot_id == to)
                    .context("link not found")?
            };
            owned(hub, bot, &link.routine_id)?;
            api::disconnect(hub, &link.id)?;
            Ok(json!({"ok": true}))
        }
        "remove_routine_media_step" => {
            let id = text(args, "mediaId")?;
            let node = hub.store.flow_media_node(id)?.context("media step not found")?;
            owned(hub, bot, &node.routine_id)?;
            hub.emit_flow_change(|| Ok(((), hub.store.flow_remove_media(id)?)))?;
            Ok(json!({"ok": true}))
        }
        "add_routine_media_step" => {
            let routine = owned(hub, bot, text(args, "routineId")?)?;
            let mut body = json!({
                "routineId": routine.id,
                "kind": text(args, "kind")?,
            });
            for key in ["prompt", "model", "aspect", "transparent", "voice"] {
                if !args[key].is_null() {
                    body[key] = args[key].clone();
                }
            }
            let added = api::add_media(hub, &body)?;
            let key = crate::store::flow_media::key(added["media"]["id"].as_str().unwrap_or_default());
            Ok(json!({"media": added["media"], "nodeKey": key,
                "note": "Added. Link to it with link_routine_bots and this nodeKey."}))
        }
        _ => list(hub, bot, args["routineId"].as_str()),
    }
}

/// A routine of this bot; a flow is managed by the bot whose routine starts it.
fn owned(hub: &Hub, bot: &str, routine_id: &str) -> Result<crate::routines::Routine> {
    if !hub.routines.owns(bot, routine_id) {
        bail!("routine not found among yours: only the bot that owns a routine can change its flow");
    }
    api::routine(hub, routine_id)
}

fn list(hub: &Hub, bot: &str, only: Option<&str>) -> Result<Value> {
    let mut flows = vec![];
    for routine in hub.routines.canvas_routines(&hub.store, bot)?.into_iter().filter(|r| r.bot_id == bot) {
        if only.is_some_and(|id| id != routine.id) {
            continue;
        }
        let links: Vec<Value> = hub
            .store
            .flow_links(&routine.id)?
            .into_iter()
            .map(|l| {
                json!({"id": l.id, "fromBotId": l.from_bot_id, "from": label(hub, &l.from_bot_id),
                    "toBotId": l.to_bot_id, "to": label(hub, &l.to_bot_id), "instruction": l.instruction})
            })
            .collect();
        let media: Vec<Value> = hub
            .store
            .flow_media(&routine.id)?
            .into_iter()
            .map(|node| {
                json!({"id": node.id, "nodeKey": crate::store::flow_media::key(&node.id), "kind": node.kind,
                    "prompt": node.prompt, "model": node.model})
            })
            .collect();
        flows.push(json!({"routineId": routine.id, "name": routine.name, "links": links, "mediaSteps": media,
            "latestRun": hub.store.flow_last_steps(&routine.id)?}));
    }
    Ok(json!({"flows": flows}))
}

/// What a link's end is called: a bot's name, or the kind of a media step.
fn label(hub: &Hub, id: &str) -> String {
    match id.strip_prefix(crate::store::flow_media::PREFIX) {
        Some(media) => match hub.store.flow_media_node(media).ok().flatten().map(|node| node.kind) {
            Some(kind) if kind == "video" => "video step".to_owned(),
            _ => "image step".to_owned(),
        },
        None => hub.bot_name(id),
    }
}
