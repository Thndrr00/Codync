//! Team templates: a group with its members saved as a reusable team. The bot configs travel with
//! the template, so installing it on this computer makes new bots with the same agents, skills,
//! connectors and instructions, then a new group with the same roles and hierarchy.

use crate::hub::Hub;
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;

const KEY: &str = "team-templates.v1";
const LIMIT: usize = 30;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub created_at: i64,
    /// Each member's full bot config, with its old id kept so roles can be remapped.
    pub members: Vec<Value>,
    pub mode: String,
    pub voice: String,
    /// `group.team.members`, each with the old `botId`.
    pub roles: Vec<Value>,
    pub chief: Option<String>,
}

fn read(hub: &Hub) -> Result<Vec<TeamTemplate>> {
    match hub.store.kv_read(KEY)? {
        Some(value) => Ok(serde_json::from_str(&value)?),
        None => Ok(Vec::new()),
    }
}

/// The templates this computer keeps, newest first.
pub fn list(hub: &Hub) -> Result<Vec<TeamTemplate>> {
    Ok(read(hub)?)
}

/// Saves `group_id` as a template called `name` (or the group's name).
pub fn save(hub: &Hub, group_id: &str, name: &str, now: i64) -> Result<TeamTemplate> {
    let group = hub.store.bot(group_id)?.filter(|row| !row.deleted).context("unknown group")?.config;
    ensure!(group.is_group(), "only a group can be saved as a team template");
    let mut members = Vec::new();
    for id in &group.members {
        if let Some(row) = hub.store.bot(id)? {
            members.push(serde_json::to_value(row.config)?);
        }
    }
    ensure!(members.len() >= 2, "a team needs at least two members");
    let template = TeamTemplate {
        id: uuid::Uuid::new_v4().to_string(),
        name: {
            let name = name.trim();
            if name.is_empty() { group.name.clone() } else { name.chars().take(120).collect() }
        },
        description: group.description.clone(),
        created_at: now,
        members,
        mode: group.team.mode.clone(),
        voice: group.team.voice.clone(),
        roles: serde_json::to_value(&group.team.members)?.as_array().cloned().unwrap_or_default(),
        chief: group.chief_bot_id.clone(),
    };
    let mut templates = read(hub)?;
    ensure!(templates.len() < LIMIT, "there are already {LIMIT} templates; delete one first");
    templates.insert(0, template.clone());
    hub.store.kv_set(KEY, &serde_json::to_string(&templates)?)?;
    Ok(template)
}

pub fn remove(hub: &Hub, id: &str) -> Result<bool> {
    let mut templates = read(hub)?;
    let before = templates.len();
    templates.retain(|template| template.id != id);
    let removed = templates.len() != before;
    if removed {
        hub.store.kv_set(KEY, &serde_json::to_string(&templates)?)?;
    }
    Ok(removed)
}

/// Creates the team's bots and its group, with roles and the hierarchy pointing at the new bots.
pub async fn install(hub: &Arc<Hub>, id: &str) -> Result<Value> {
    let template = read(hub)?.into_iter().find(|template| template.id == id).context("unknown team template")?;
    let mut created: Vec<(String, String)> = Vec::new();
    let mut new_ids: Vec<String> = Vec::new();
    let result = install_members(hub, &template, &mut created, &mut new_ids).await;
    if result.is_err() {
        for (_, new_id) in &created {
            let hub = hub.clone();
            let new_id = new_id.clone();
            if let Err(error) = tokio::task::spawn_blocking(move || hub.delete_bot(&new_id))
                .await
                .map_err(anyhow::Error::from)
                .and_then(|r| r)
            {
                tracing::warn!(error = format!("{error:#}"), "couldn't remove a bot from a failed template install");
            }
        }
    }
    result
}

async fn install_members(
    hub: &Arc<Hub>,
    template: &TeamTemplate,
    created: &mut Vec<(String, String)>,
    new_ids: &mut Vec<String>,
) -> Result<Value> {
    for config in &template.members {
        let mut body = config.clone();
        let old = body["id"].as_str().unwrap_or_default().to_owned();
        if old.is_empty() {
            bail!("a template member has no id");
        }
        body["id"] = json!("");
        body["lead"] = json!(false);
        let cfg = crate::agent::new_bot::with_defaults(hub, body).await?;
        let bot = hub.create_bot(cfg)?;
        let new_id = bot["id"].as_str().context("created bot has an id")?.to_owned();
        created.push((old, new_id.clone()));
        new_ids.push(new_id);
    }
    let remap: std::collections::HashMap<&str, &str> =
        created.iter().map(|(old, new)| (old.as_str(), new.as_str())).collect();
    let roles: Vec<Value> = template
        .roles
        .iter()
        .map(|role| {
            let mut role = role.clone();
            let old = role["botId"].as_str().unwrap_or_default();
            role["botId"] = json!(remap.get(old).copied().unwrap_or_default());
            if let Some(reports_to) = role["reportsTo"].as_str() {
                role["reportsTo"] = json!(remap.get(reports_to).copied());
            }
            role
        })
        .collect();
    let chief = template.chief.as_deref().and_then(|old| remap.get(old).copied());
    let body = json!({
        "kind": "group",
        "name": template.name,
        "description": template.description,
        "members": new_ids,
        "chiefBotId": chief,
        "team": {"mode": template.mode, "voice": template.voice, "members": roles},
    });
    let cfg = crate::agent::new_bot::with_defaults(hub, body).await?;
    hub.create_bot(cfg)
}
