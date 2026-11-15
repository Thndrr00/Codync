//! Missions over the API: the list, one with its steps, and every move a person or a bot makes.
//! A change is broadcast as a `mission` event and, when it finishes or blocks, it reaches the phone.

use super::str_arg;
use crate::hub::Hub;
use crate::remote::push::{self, AlertKind};
use crate::store::{Mission, MissionStatus, MissionStepStatus};
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

#[cfg(test)]
mod tests;

pub const METHODS: &[&str] = &["createMission", "missions", "mission", "updateMission", "missionStep"];

pub fn call(hub: &Hub, method: &str, body: &Value) -> Result<Value> {
    let answer = match method {
        "createMission" => {
            let steps: Vec<String> = body["steps"]
                .as_array()
                .map(|s| s.iter().filter_map(|v| v.as_str().map(str::to_owned)).collect())
                .unwrap_or_default();
            let mission = hub.store.create_mission(
                str_arg(body, "botId")?,
                str_arg(body, "title")?,
                body["goal"].as_str().unwrap_or_default(),
                &steps,
            )?;
            changed(hub, &mission, mission.status, body);
            json!({"mission": mission})
        }
        "missions" => {
            let open = body["open"].as_bool().unwrap_or(false);
            json!({"missions": hub.store.missions(body["botId"].as_str(), body["query"].as_str(), open)?})
        }
        "mission" => json!({"mission": own(hub, str_arg(body, "id")?)?}),
        "updateMission" => json!({"mission": update(hub, body)?}),
        _ => json!({"mission": step(hub, body)?}),
    };
    Ok(answer)
}

/// The mission, which must exist.
fn own(hub: &Hub, id: &str) -> Result<Mission> {
    hub.store.mission(id)?.with_context(|| format!("mission {id} not found"))
}

fn update(hub: &Hub, body: &Value) -> Result<Mission> {
    // Nothing to change is a mistake worth naming, not a silent success.
    if body["status"].is_null()
        && body["checkpoint"].is_null()
        && body.get("runId").is_none()
        && body.get("approvalId").is_none()
    {
        bail!("give a status, a checkpoint, a runId or an approvalId");
    }
    let id = str_arg(body, "id")?;
    let before = own(hub, id)?.status;
    let mut mission = hub.store.mission(id)?.context("mission not found")?;
    if let Some(raw) = body["status"].as_str() {
        let status = MissionStatus::parse(raw).with_context(|| format!("unknown mission status: {raw}"))?;
        mission = hub.store.set_mission_status(id, status, body["stopReason"].as_str())?;
    }
    if let Some(checkpoint) = body["checkpoint"].as_str() {
        mission = hub.store.set_mission_checkpoint(id, checkpoint)?;
    }
    if body.get("runId").is_some() {
        mission = hub.store.set_mission_run(id, body["runId"].as_str())?;
    }
    if body.get("approvalId").is_some() {
        mission = hub.store.set_mission_approval(id, body["approvalId"].as_str())?;
    }
    changed(hub, &mission, before, body);
    Ok(mission)
}

fn step(hub: &Hub, body: &Value) -> Result<Mission> {
    let id = str_arg(body, "id")?;
    let mission = match body["stepId"].as_str() {
        Some(step_id) => {
            let raw = str_arg(body, "status")?;
            let status = MissionStepStatus::parse(raw).with_context(|| format!("unknown step status: {raw}"))?;
            hub.store.set_mission_step(id, step_id, status)?
        }
        None => hub.store.add_mission_step(id, str_arg(body, "text")?)?,
    };
    Ok(mission)
}

/// Broadcasts the new state; a finish or a block also reaches the phone.
pub fn changed(hub: &Hub, mission: &Mission, before: MissionStatus, body: &Value) {
    let _ = hub.events.send(json!({"type": "mission", "mission": mission}));
    let bot = hub.store.bot(&mission.bot_id).ok().flatten().map(|row| row.config);
    let Some(bot) = bot else { return };
    match mission.status {
        MissionStatus::Done if before != MissionStatus::Done => {
            if !mission.title.is_empty() {
                push::notify(hub, &bot, None, "Mission complete", &mission.title, AlertKind::Done);
            }
        }
        MissionStatus::Waiting if before != MissionStatus::Waiting => {
            let blocked = body["stopReason"].as_str().unwrap_or("A mission is waiting on you.");
            push::notify(hub, &bot, None, "Mission blocked", blocked, AlertKind::NeedsInput);
        }
        _ => {}
    }
}
