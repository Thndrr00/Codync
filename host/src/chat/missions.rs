//! The mission tools of the chat server: a bot reads its own missions and moves them along, so a long
//! objective survives the turn instead of being restated in the next one.

use crate::hub::Hub;
use crate::store::{MissionStatus, MissionStepStatus};
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

#[cfg(test)]
mod tests;

pub const TOOLS: &[&str] = &["missions", "update_mission"];

pub fn tools() -> Vec<Value> {
    let id = json!({"type": "string"});
    vec![
        json!({"name": "missions", "description": "List your missions, open ones first: each with its status, checkpoint and steps. missionId narrows it to one. A mission is a goal that outlives the turn, so read it before resuming one.",
            "inputSchema": {"type": "object", "properties": {"missionId": id, "all": {"type": "boolean"}}}}),
        json!({"name": "update_mission", "description": "Move one of your missions along: status (proposed, active, waiting, done, abandoned), the reason an abandonment or a block needs (stopReason), what is done and what is left (checkpoint), the routine run executing it (runId), or the approval blocking it (approvalId). Write the checkpoint whenever you stop: the next turn starts from it. Add a step with text, or record how one went with stepId and a step status (pending, active, done, skipped).",
            "inputSchema": {"type": "object", "properties": {"missionId": id, "status": id, "stopReason": id, "checkpoint": id, "runId": id, "approvalId": id, "text": id, "stepId": id, "stepStatus": id}, "required": ["missionId"]}}),
    ]
}

/// The mission, if it is this bot's.
fn own(hub: &Hub, bot: &str, id: &str) -> Result<crate::store::Mission> {
    let mission = hub.store.mission(id)?.with_context(|| format!("mission {id} not found"))?;
    if mission.bot_id != bot {
        bail!("mission {id} belongs to another bot");
    }
    Ok(mission)
}

pub fn call(hub: &Hub, bot: &str, name: &str, args: &Value) -> Result<Value> {
    if name == "missions" {
        let all = args["all"].as_bool().unwrap_or(false);
        let missions = match args["missionId"].as_str() {
            Some(id) => vec![own(hub, bot, id)?],
            None => hub.store.missions(Some(bot), None, !all)?,
        };
        return Ok(json!({"missions": missions}));
    }
    let id = args["missionId"].as_str().context("missionId is required")?;
    let before = own(hub, bot, id)?.status;
    let mut mission = hub.store.mission(id)?.context("mission not found")?;
    if let Some(raw) = args["status"].as_str() {
        let status = MissionStatus::parse(raw).with_context(|| format!("unknown mission status: {raw}"))?;
        mission = hub.store.set_mission_status(id, status, args["stopReason"].as_str())?;
    }
    if let Some(text) = args["checkpoint"].as_str() {
        mission = hub.store.set_mission_checkpoint(id, text)?;
    }
    if args.get("runId").is_some() {
        mission = hub.store.set_mission_run(id, args["runId"].as_str())?;
    }
    if args.get("approvalId").is_some() {
        mission = hub.store.set_mission_approval(id, args["approvalId"].as_str())?;
    }
    if let Some(text) = args["text"].as_str() {
        mission = hub.store.add_mission_step(id, text)?;
    }
    if let Some(step_id) = args["stepId"].as_str() {
        let raw = args["stepStatus"].as_str().context("stepStatus is required with stepId")?;
        let status = MissionStepStatus::parse(raw).with_context(|| format!("unknown step status: {raw}"))?;
        mission = hub.store.set_mission_step(id, step_id, status)?;
    }
    crate::api::missions::changed(hub, &mission, before, args);
    Ok(json!({"mission": mission, "note": "Recorded. Keep the checkpoint current as you go."}))
}
