//! The ready-made bot and team catalog (`market::catalog`): listing it and creating a whole team.

use super::str_arg;
use crate::hub::Hub;
use crate::market::catalog::{self, TeamProfile};
use anyhow::{Context, Result};
use serde_json::{Value, json};
use std::ops::ControlFlow;
use std::sync::Arc;

pub(super) async fn call(hub: &Arc<Hub>, method: &str, b: Value) -> Result<ControlFlow<Value, Value>> {
    Ok(ControlFlow::Break(match method {
        "botCatalog" => json!(catalog::catalog()),
        "installTeam" => {
            let team = catalog::team(str_arg(&b, "id")?)?;
            install_team(hub, team, b["backend"].as_str(), b["model"].as_str()).await?
        }
        _ => return Ok(ControlFlow::Continue(b)),
    }))
}

/// Creates the coordinator and its teammates, then their group led by the coordinator.
/// All or nothing: a failure removes the bots this call already created.
async fn install_team(
    hub: &Arc<Hub>,
    team: &'static TeamProfile,
    backend: Option<&str>,
    model: Option<&str>,
) -> Result<Value> {
    let mut created = Vec::new();
    let result = create_team(hub, team, backend, model, &mut created).await;
    if result.is_err() {
        for id in created {
            let hub = hub.clone();
            let removed = tokio::task::spawn_blocking(move || hub.delete_bot(&id)).await;
            if let Err(e) = removed.map_err(anyhow::Error::from).and_then(|r| r) {
                tracing::warn!(
                    team = team.id,
                    error = format!("{e:#}"),
                    "couldn't remove a bot from a failed team install"
                );
            }
        }
    }
    result
}

async fn create_team(
    hub: &Arc<Hub>,
    team: &'static TeamProfile,
    backend: Option<&str>,
    model: Option<&str>,
    created: &mut Vec<String>,
) -> Result<Value> {
    for profile in team.profiles()? {
        let bot = create(hub, profile.draft(backend, model)).await?;
        created.push(bot["id"].as_str().context("created bot has an id")?.to_owned());
    }
    let group = json!({
        "kind": "group",
        "name": team.name,
        "description": team.purpose,
        "members": created,
        "chiefBotId": created.first(),
    });
    let group = create(hub, group).await?;
    Ok(json!({"bot": group}))
}

async fn create(hub: &Arc<Hub>, body: Value) -> Result<Value> {
    let cfg = crate::agent::new_bot::with_defaults(hub, body).await?;
    let hub = hub.clone();
    tokio::task::spawn_blocking(move || hub.create_bot(cfg)).await?
}
