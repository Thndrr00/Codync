//! The flow tools of the built-in `routines` MCP server: a bot builds its routines' flows by talking.

use super::{MAX_INSTRUCTION_BYTES, answers, api};
use crate::hub::Hub;
use crate::store::StepStatus;
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::sync::Arc;

pub const TOOLS: &[&str] = &[
    "link_routine_bots",
    "unlink_routine_bots",
    "list_routine_flow",
    "add_routine_media_step",
    "remove_routine_media_step",
    answers::ANSWER_TOOL,
];

pub fn tools() -> Vec<Value> {
    let id = json!({"type": "string"});
    vec![
        json!({"name": "link_routine_bots", "description": format!("Make one of your routines hand its reply on to another bot (a routine flow). When the routine's run ends, its reply goes to toBotId with your instruction (at most {MAX_INSTRUCTION_BYTES} bytes), and that bot's reply can go on to the next link. fromBotId is you unless you chain from a bot the flow already reaches. toBotId may also be a media step's nodeKey (see add_routine_media_step). A flow can't loop, can't reach more than a few steps deep, and can't end in a group or a terminal session. Use list_bots for ids. Linking only applies to the routine's later runs."),
            "inputSchema": {"type": "object", "properties": {"routineId": id, "toBotId": id, "fromBotId": id, "instruction": {"type": "string"}, "answer": {"type": "string"}, "check": {"type": "string"}}, "required": ["routineId", "toBotId"]}}),
        json!({"name": "unlink_routine_bots", "description": "Remove a link of one of your routines' flows: by linkId, or by routineId with toBotId (and fromBotId when it isn't you).",
            "inputSchema": {"type": "object", "properties": {"linkId": id, "routineId": id, "toBotId": id, "fromBotId": id}}}),
        json!({"name": "list_routine_flow", "description": "List the flows of your routines: each routine's links (who hands to whom, with the instruction), its media steps, and the steps of its latest run with their status. routineId narrows it to one routine.",
            "inputSchema": {"type": "object", "properties": {"routineId": id}}}),
        json!({"name": "add_routine_media_step", "description": "Add a step to one of your routines' flows that makes an image or a short video with the agent's own subscription (GPT Image on Codex, Grok Imagine on Grok) instead of a bot. Link to it with link_routine_bots and its nodeKey; whatever the steps before it hand on is added to its description. Model ids: gpt-image (image only), grok-imagine (image and video).",
            "inputSchema": {"type": "object", "properties": {"routineId": id, "kind": {"type": "string", "enum": ["image", "video"]}, "prompt": {"type": "string"}, "model": {"type": "string"}, "aspect": {"type": "string", "enum": ["1:1", "4:3", "16:9", "9:16"]}, "transparent": {"type": "boolean"}, "voice": {"type": "string", "enum": ["ara", "eve", "leo", "rex"]}}, "required": ["routineId", "kind"]}}),
        json!({"name": "remove_routine_media_step", "description": "Remove a media step of one of your routines, with the links that reach or leave it.",
            "inputSchema": {"type": "object", "properties": {"mediaId": id}, "required": ["mediaId"]}}),
        json!({"name": answers::ANSWER_TOOL, "description": "Give the checked answer of a routine-flow step. Pass the fields the step asks for under `answer`; the fields are checked before this tool returns, so you can correct them in the same turn. Ending your reply with the same JSON object also works.",
            "inputSchema": {"type": "object", "properties": {"answer": {"type": "object"}, "runId": id}, "required": ["answer"]}}),
    ]
}

fn text<'a>(args: &'a Value, key: &str) -> Result<&'a str> {
    args[key].as_str().filter(|s| !s.is_empty()).with_context(|| format!("{key} is required"))
}

fn spec_arg(args: &Value, key: &str) -> Result<Option<String>> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) if text.chars().count() <= 2000 => Ok(Some(text.clone())),
        _ => bail!("{key} must be text of at most 2000 characters"),
    }
}

/// The newest run with a step of this bot still waiting for a reply.
fn waiting_run(hub: &Hub, bot: &str) -> Result<String> {
    hub.store
        .flow_runs()?
        .into_iter()
        .find(|run| {
            hub.store
                .flow_run_steps(&run.run_id)
                .map(|steps| steps.iter().any(|step| step.bot_id == bot && step.status == StepStatus::Running))
                .unwrap_or(false)
        })
        .map(|run| run.run_id)
        .context("no workflow step is waiting for an answer")
}

/// An agent's `workflow_answer` tool call, checked before it returns so the agent can fix the answer
/// in the same turn. A valid answer is kept for the step that is waiting on it.
fn workflow_answer(hub: &Hub, bot: &str, args: &Value) -> Result<Value> {
    let run = args["runId"].as_str().map(str::to_owned).map(Ok).unwrap_or_else(|| waiting_run(hub, bot))?;
    let step = hub
        .store
        .flow_run_steps(&run)?
        .into_iter()
        .find(|step| step.bot_id == bot && step.status == StepStatus::Running)
        .context("no workflow step of yours is waiting for an answer in that run")?;
    let specs = answers::load_specs(&hub.store, &step.owner_bot_id);
    let fields = specs.get(&step.bot_id).map(answers::fields_of).unwrap_or_default();
    if fields.is_empty() {
        bail!("this step does not ask for answer fields; end your reply normally");
    }
    let (answer, problems) = answers::tool_answer(args, &fields);
    if !problems.is_empty() {
        return Ok(json!({
            "ok": false,
            "problem": answers::fix_text(&problems),
            "ask": answers::tool_fix_ask(&fields, &problems),
            "schema": answers::answer_tool_schema(&fields),
        }));
    }
    answers::save_tool_answer(&hub.store, &run, &step.id, &answer)?;
    Ok(json!({"ok": true, "answer": answer, "note": "Answer checked and saved for this step."}))
}

pub fn call(hub: &Arc<Hub>, bot: &str, name: &str, args: &Value) -> Result<Value> {
    match name {
        answers::ANSWER_TOOL => workflow_answer(hub, bot, args),
        "link_routine_bots" => {
            let routine_id = text(args, "routineId")?;
            let routine = owned(hub, bot, routine_id)?;
            let from = args["fromBotId"].as_str().unwrap_or(bot);
            let instruction = args["instruction"].as_str().unwrap_or_default().trim();
            if instruction.len() > MAX_INSTRUCTION_BYTES {
                bail!("instruction must be at most {MAX_INSTRUCTION_BYTES} bytes");
            }
            let answer = spec_arg(args, "answer")?;
            let check = spec_arg(args, "check")?;
            let link = api::connect(
                hub,
                &routine,
                from,
                text(args, "toBotId")?,
                instruction.to_owned(),
                answer.as_deref(),
                check.as_deref(),
            )?;
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
                json!({"id": l.id, "name": l.name, "fromBotId": l.from_bot_id, "from": label(hub, &l.from_bot_id),
                    "toBotId": l.to_bot_id, "to": label(hub, &l.to_bot_id), "instruction": l.instruction})
            })
            .collect();
        let media: Vec<Value> = hub
            .store
            .flow_media(&routine.id)?
            .into_iter()
            .map(|node| {
                json!({"id": node.id, "nodeKey": crate::store::flow_media::key(&node.id), "name": node.name,
                    "kind": node.kind, "prompt": node.prompt, "model": node.model})
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
        Some(media) => match hub.store.flow_media_node(media).ok().flatten() {
            Some(node) if !node.name.trim().is_empty() => node.name,
            Some(node) if node.kind == "video" => "video step".to_owned(),
            _ => "image step".to_owned(),
        },
        None => hub.bot_name(id),
    }
}
