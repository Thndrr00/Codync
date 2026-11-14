//! Microsoft Teams as a way to talk to a bot, through an Azure Bot resource and the Bot Framework
//! connector. The host exposes a messaging endpoint (`/hooks/teams/{id}`) that the registration
//! points at, checks every activity's Bot Framework JWT, and answers with the connector's REST API.
//! The app password stays in the host vault.

mod activity;
mod client;
mod inbound;
#[cfg(test)]
mod tests;
mod worker;

pub(crate) use inbound::receive;
pub(crate) use worker::claim_prompt;

use crate::{LockExt, hub::Hub, store::Store};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;

const SLOT: &str = "teams.channel";
pub const METHODS: &[&str] =
    &["teamsStatus", "setTeamsChannel", "disconnectTeams", "setChannelMemory", "setChannelIcon"];
const MAX_ALLOWED: usize = 100;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Config {
    /// The connection's public path segment; not a secret.
    pub id: String,
    /// The Azure Bot resource's Microsoft app id.
    pub app_id: String,
    /// The directory tenant used for the client-credentials token, or `botframework` for multi-tenant.
    #[serde(default)]
    pub tenant_id: String,
    /// The app registration's client secret. Empty with a managed identity.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub app_password: String,
    #[serde(default)]
    pub managed_identity: bool,
    /// The Codync bot that answers this Teams app.
    pub bot_id: String,
    /// AAD object ids, UPNs, Teams user ids or display names; empty with a tenant list means anyone there.
    #[serde(default)]
    pub allowed_users: Vec<String>,
    /// Tenant ids, or `*`; empty with a user list means any tenant.
    #[serde(default)]
    pub allowed_tenants: Vec<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub memory: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub icon: String,
    /// Changes when the app or the bot is replaced, so an old activity's prompts are dropped.
    #[serde(default)]
    pub generation: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Setup {
    #[serde(default)]
    pub app_id: String,
    #[serde(default)]
    pub tenant_id: String,
    /// Empty while editing keeps the saved password.
    #[serde(default)]
    pub app_password: String,
    #[serde(default)]
    pub managed_identity: bool,
    pub bot_id: String,
    #[serde(default)]
    pub allowed_users: Vec<String>,
    #[serde(default)]
    pub allowed_tenants: Vec<String>,
    #[serde(default)]
    pub memory: String,
    #[serde(default)]
    pub icon: String,
}

pub(super) fn load(store: &Store) -> Result<Option<Config>> {
    crate::market::vault::read(store, SLOT)?
        .filter(|s| !s.is_empty())
        .map(|s| serde_json::from_str(&s).map_err(Into::into))
        .transpose()
}

fn save(store: &Store, config: &Config) -> Result<()> {
    crate::market::vault::write(store, SLOT, &serde_json::to_string(config)?)
}

pub(crate) fn configuration_state(store: &Store) -> &'static str {
    match load(store) {
        Ok(Some(_)) => "configured",
        Ok(None) => "notConfigured",
        Err(_) => "unavailable",
    }
}

/// The conversation references and outbox are the worker's; inbound arrives on the HTTP route.
pub(crate) async fn run(hub: Arc<Hub>) {
    worker::run(hub).await;
}

fn app_id(value: &str) -> Result<String> {
    let trimmed = value.trim();
    anyhow::ensure!(
        trimmed.len() == 36
            && trimmed.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
            && trimmed.matches('-').count() == 4,
        "Write the Azure Bot resource's Microsoft app id (a GUID)."
    );
    Ok(trimmed.to_ascii_lowercase())
}

fn tenant(value: &str) -> Result<String> {
    let trimmed = value.trim();
    anyhow::ensure!(
        trimmed.is_empty()
            || ((2..=128).contains(&trimmed.len())
                && trimmed.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))),
        "A tenant id is letters, digits, dots, dashes or underscores."
    );
    Ok(trimmed.to_owned())
}

fn people(list: &[String], what: &str) -> Result<Vec<String>> {
    let cleaned: Vec<String> = list.iter().map(|item| item.trim().to_owned()).filter(|item| !item.is_empty()).collect();
    anyhow::ensure!(cleaned.len() <= MAX_ALLOWED, "At most {MAX_ALLOWED} {what}.");
    anyhow::ensure!(cleaned.iter().all(|item| item.len() <= 256), "A Teams {what} is too long.");
    Ok(cleaned)
}

fn validate(hub: &Hub, c: &Config) -> Result<()> {
    app_id(&c.app_id)?;
    tenant(&c.tenant_id)?;
    anyhow::ensure!(
        c.managed_identity || !c.app_password.is_empty(),
        "Write the Azure app registration's client secret."
    );
    anyhow::ensure!(
        !c.managed_identity || c.app_password.is_empty(),
        "A managed identity does not need a client secret."
    );
    anyhow::ensure!(
        !c.allowed_users.is_empty() || !c.allowed_tenants.is_empty(),
        "List the people or tenants allowed to write."
    );
    hub.store
        .bot(&c.bot_id)?
        .filter(|bot| !bot.deleted && !bot.config.is_group() && !crate::external::is_external(&bot.config))
        .context("The Teams bot is unavailable.")?;
    Ok(())
}

/// Azure's steps, as the settings screen shows them. This is the registration the person must do;
/// the host cannot create an Azure Bot resource for them.
pub(super) fn registration(endpoint: &str) -> String {
    format!(
        "In the Azure portal create an Azure Bot resource and copy its Microsoft app id here. \
         Under Configuration, set the messaging endpoint to https://<this computer's public host>{endpoint} \
         and enable the Microsoft Teams channel. Give the app registration a client secret (or enable a managed identity). \
         Teams only reaches a public HTTPS address: expose this host through a reverse proxy or tunnel first."
    )
}

pub(crate) async fn call(hub: &Arc<Hub>, method: &str, body: Value) -> Result<Value> {
    crate::market::vault::unlock(hub.clone()).await?;
    // The token request is a network call: it happens before the configuration lock so the lock is
    // never held across an await.
    let prepared = if method == "setTeamsChannel" {
        let setup: Setup = serde_json::from_value(body.clone())?;
        let previous = load(&hub.store)?;
        let app_id = app_id(&setup.app_id)?;
        let password = if setup.app_password.trim().is_empty() {
            previous
                .as_ref()
                .filter(|previous| previous.app_id == app_id)
                .map(|previous| previous.app_password.clone())
                .unwrap_or_default()
        } else {
            setup.app_password.trim().to_owned()
        };
        anyhow::ensure!(password.len() <= 2048, "The client secret is too long.");
        let config = Config {
            id: previous.as_ref().map_or_else(|| uuid::Uuid::new_v4().simple().to_string(), |p| p.id.clone()),
            app_id,
            tenant_id: tenant(&setup.tenant_id)?,
            app_password: password,
            managed_identity: setup.managed_identity,
            bot_id: setup.bot_id,
            allowed_users: people(&setup.allowed_users, "people")?,
            allowed_tenants: people(&setup.allowed_tenants, "tenants")?,
            memory: crate::channels::memory_arg(&json!({"memory": setup.memory}))?,
            icon: crate::channels::icon_arg(&json!({"icon": setup.icon}))?,
            generation: String::new(),
        };
        validate(hub, &config)?;
        // The credentials must work before anything is saved: a Teams channel that cannot get a
        // token is not connected.
        client::token(&config.app_id, &config.tenant_id, &config.app_password, config.managed_identity)
            .await
            .context("Microsoft did not accept these Teams credentials")?;
        Some((config, previous))
    } else {
        None
    };
    let _guard = hub.store.connector_lock.locked();
    match method {
        "setTeamsChannel" => {
            let (mut config, previous) = prepared.context("Missing Teams configuration.")?;
            validate(hub, &config)?;
            config.generation = match previous {
                Some(previous)
                    if previous.app_id == config.app_id
                        && previous.bot_id == config.bot_id
                        && previous.tenant_id == config.tenant_id =>
                {
                    previous.generation
                }
                _ => uuid::Uuid::new_v4().to_string(),
            };
            save(&hub.store, &config)?;
            hub.store.kv_set("teams.state", "ok")?;
        }
        "disconnectTeams" => {
            crate::market::vault::write(&hub.store, SLOT, "")?;
            hub.store.kv_delete("teams.state")?;
        }
        "setChannelMemory" => {
            let memory = crate::channels::memory_arg(&body)?;
            let mut config = load(&hub.store)?.context("Teams is not configured.")?;
            config.memory = memory;
            save(&hub.store, &config)?;
        }
        "setChannelIcon" => {
            let icon = crate::channels::icon_arg(&body)?;
            let mut config = load(&hub.store)?.context("Teams is not configured.")?;
            config.icon = icon;
            save(&hub.store, &config)?;
        }
        "teamsStatus" => {}
        other => bail!("Unknown Teams method `{other}`."),
    }
    let c = load(&hub.store)?;
    if method != "teamsStatus" {
        for id in hub.store.reject_stale_teams(c.as_ref().map(|c| c.generation.as_str()))? {
            if let Some(mut entry) = hub.store.entry(&id).filter(|e| e.data["status"] == "queued") {
                entry.data["status"] = "cancelled".into();
                hub.set_entry(&id, &entry.data).context("Could not update cancelled Teams prompt.")?;
            }
        }
    }
    Ok(status(hub, c.as_ref()))
}

fn status(hub: &Hub, c: Option<&Config>) -> Value {
    let state = if c.is_none() {
        "notConfigured".to_owned()
    } else {
        hub.store.kv_read("teams.state").ok().flatten().unwrap_or_else(|| "starting".into())
    };
    let endpoint = c.map(|c| format!("/hooks/teams/{}", c.id));
    json!({
        "configured": c.is_some(),
        "appId": c.map(|c| &c.app_id).filter(|id| !id.is_empty()),
        "tenantId": c.map(|c| &c.tenant_id).filter(|id| !id.is_empty()),
        "botId": c.map(|c| &c.bot_id).filter(|id| !id.is_empty()),
        "managedIdentity": c.is_some_and(|c| c.managed_identity),
        "allowedUsers": c.map_or_else(Vec::new, |c| c.allowed_users.clone()),
        "allowedTenants": c.map_or_else(Vec::new, |c| c.allowed_tenants.clone()),
        "memory": c.map(|c| c.memory.clone()).unwrap_or_default(),
        "icon": c.map(|c| c.icon.clone()).unwrap_or_default(),
        "endpoint": endpoint,
        "registration": registration(endpoint.as_deref().unwrap_or("/hooks/teams/<connection>")),
        "state": state,
        "deliveries": hub.store.teams_deliveries().unwrap_or_default(),
    })
}
