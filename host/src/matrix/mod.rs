//! Matrix as a way to talk to a bot: the host logs in to a homeserver, syncs, joins the rooms it
//! is invited to and answers in the rooms the owner bound. The access token lives in the host's
//! vault and the host makes every request, so nothing has to be reachable from the internet.

mod client;
mod ingress;
mod render;
mod sync;
#[cfg(test)]
mod tests;
mod worker;

pub(crate) use worker::claim_prompt;

use crate::{LockExt, hub::Hub, store::Store};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;

const SLOT: &str = "matrix.channel";
pub const METHODS: &[&str] =
    &["matrixStatus", "setMatrixChannel", "disconnectMatrix", "setChannelMemory", "setChannelIcon"];
const MAX_BINDINGS: usize = 50;
const MAX_ALLOWED: usize = 100;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Binding {
    pub room_id: String,
    pub bot_id: String,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub memory: String,
    /// An SF Symbol shown as the channel's picture, or empty.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub icon: String,
    pub allowed_users: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Config {
    pub homeserver: String,
    /// The account the access token belongs to, as the homeserver names it (`@bot:server`).
    pub user_id: String,
    pub access_token: String,
    #[serde(default)]
    pub device_id: String,
    pub bindings: Vec<Binding>,
    /// Changes whenever the connection is replaced, so an old sync's prompts are dropped.
    #[serde(default)]
    pub generation: String,
}

/// What a client sends to set the channel up: the password is used once for the login call and never
/// stored; an access token is kept instead.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Setup {
    pub homeserver: String,
    #[serde(default)]
    pub user: String,
    #[serde(default)]
    pub access_token: String,
    #[serde(default)]
    pub password: String,
    pub bindings: Vec<Binding>,
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

/// Runs the long poll and the sender while the host runs.
pub(crate) async fn run(hub: Arc<Hub>) {
    tokio::join!(sync::run(hub.clone()), worker::run(hub));
}

/// `https://matrix.example.org`, without the trailing slash. A self-hosted homeserver may sit on the
/// local network, so the address is the person's own choice and plain HTTP reaches it there.
pub(super) fn homeserver(value: &str) -> Result<String> {
    let trimmed = value.trim().trim_end_matches('/');
    anyhow::ensure!(!trimmed.is_empty() && trimmed.len() <= 300, "Write the homeserver's address.");
    let url = url::Url::parse(trimmed).context("The homeserver address is not a URL.")?;
    anyhow::ensure!(
        matches!(url.scheme(), "https" | "http"),
        "The homeserver address starts with https:// (or http:// on this computer's network)."
    );
    anyhow::ensure!(url.host_str().is_some(), "The homeserver address has no host.");
    anyhow::ensure!(url.username().is_empty() && url.password().is_none(), "Leave credentials out of the address.");
    Ok(trimmed.to_owned())
}

/// A full Matrix id: `@name:server` or `#room:server`.
pub(super) fn matrix_id(value: &str, sigil: char) -> bool {
    value.len() >= 4
        && value.len() <= 255
        && value.starts_with(sigil)
        && value.contains(':')
        && !value.chars().any(|c| c.is_whitespace() || c == '/')
}

fn binding_ok(hub: &Hub, binding: &Binding) -> Result<()> {
    if !matrix_id(&binding.room_id, '!') && !matrix_id(&binding.room_id, '#') {
        bail!("Each binding needs a room id (!room:server) or an alias (#room:server).");
    }
    if binding.allowed_users.is_empty() || binding.allowed_users.len() > MAX_ALLOWED {
        bail!("Each room needs 1–{MAX_ALLOWED} allowed Matrix users.");
    }
    if binding.allowed_users.iter().any(|user| !matrix_id(user, '@')) {
        bail!("Allowed Matrix users look like @name:server.");
    }
    hub.store
        .bot(&binding.bot_id)?
        .filter(|bot| !bot.deleted && !bot.config.is_group() && !crate::external::is_external(&bot.config))
        .context("A bound bot is unavailable.")?;
    Ok(())
}

fn validate(hub: &Hub, c: &Config) -> Result<()> {
    homeserver(&c.homeserver)?;
    if !matrix_id(&c.user_id, '@') || c.access_token.is_empty() || c.access_token.len() > 8192 {
        bail!("The Matrix user or access token is invalid.");
    }
    if c.bindings.is_empty() || c.bindings.len() > MAX_BINDINGS {
        bail!("Bind 1–{MAX_BINDINGS} rooms.");
    }
    let mut seen = std::collections::HashSet::new();
    for binding in &c.bindings {
        if !seen.insert(&binding.room_id) {
            bail!("A room is bound twice.");
        }
        binding_ok(hub, binding)?;
    }
    Ok(())
}

/// Logs in (or confirms the token) and resolves every room alias, before anything is saved.
async fn prepare(hub: &Hub, setup: &Setup) -> Result<Config> {
    let base = homeserver(&setup.homeserver)?;
    let existing = load(&hub.store)?;
    let (user_id, access_token, device_id) = if !setup.access_token.trim().is_empty() {
        let token = setup.access_token.trim().to_owned();
        let me = client::whoami(&base, &token).await?;
        if !setup.user.trim().is_empty() && setup.user.trim() != me.as_str() {
            bail!("That access token belongs to another Matrix account.");
        }
        (me, token, String::new())
    } else if !setup.password.is_empty() {
        let session = client::login(&base, setup.user.trim(), &setup.password).await?;
        (session.user_id, session.access_token, session.device_id)
    } else if let Some(c) =
        existing.filter(|c| c.homeserver == base && (setup.user.trim().is_empty() || c.user_id == setup.user.trim()))
    {
        // The token stays in the vault: changing rooms does not ask for it again.
        (c.user_id, c.access_token, c.device_id)
    } else {
        bail!("Enter the access token or the password once.");
    };
    let mut bindings = setup.bindings.clone();
    for binding in &mut bindings {
        if binding.room_id.starts_with('#') {
            binding.room_id = client::resolve_alias(&base, &access_token, &binding.room_id).await?;
        }
    }
    Ok(Config { homeserver: base, user_id, access_token, device_id, bindings, generation: String::new() })
}

pub(crate) async fn call(hub: &Arc<Hub>, method: &str, body: Value) -> Result<Value> {
    crate::market::vault::unlock(hub.clone()).await?;
    let prepared = if method == "setMatrixChannel" {
        let setup: Setup = serde_json::from_value(body.clone())?;
        let config = prepare(hub, &setup).await?;
        validate(hub, &config)?;
        Some(config)
    } else {
        None
    };
    let _guard = hub.store.connector_lock.locked();
    match method {
        "setMatrixChannel" => {
            let mut config = prepared.context("Missing Matrix configuration.")?;
            validate(hub, &config)?;
            let previous = load(&hub.store)?;
            config.generation = match previous.filter(|c| {
                c.homeserver == config.homeserver
                    && c.user_id == config.user_id
                    && c.access_token == config.access_token
            }) {
                Some(c) => c.generation,
                None => uuid::Uuid::new_v4().to_string(),
            };
            save(&hub.store, &config)?;
        }
        "disconnectMatrix" => {
            let generation = load(&hub.store)?.map(|c| c.generation);
            crate::market::vault::write(&hub.store, SLOT, "")?;
            if let Some(generation) = generation.filter(|g| !g.is_empty()) {
                hub.store.kv_delete(&format!("matrix.since.{generation}"))?;
            }
        }
        "setChannelMemory" => {
            let room = body["channel"].as_str().context("channel is required.")?;
            let memory = crate::channels::memory_arg(&body)?;
            let mut config = load(&hub.store)?.context("Matrix is not configured.")?;
            let binding = config.bindings.iter_mut().find(|b| b.room_id == room).context("Unknown Matrix room.")?;
            binding.memory = memory;
            save(&hub.store, &config)?;
        }
        "setChannelIcon" => {
            let room = body["channel"].as_str().context("channel is required.")?;
            let icon = crate::channels::icon_arg(&body)?;
            let mut config = load(&hub.store)?.context("Matrix is not configured.")?;
            let binding = config.bindings.iter_mut().find(|b| b.room_id == room).context("Unknown Matrix room.")?;
            binding.icon = icon;
            save(&hub.store, &config)?;
        }
        "matrixStatus" => {}
        _ => bail!("Unknown Matrix method."),
    }
    let c = load(&hub.store)?;
    if method != "matrixStatus" {
        for id in hub.store.reject_stale_matrix(c.as_ref().map(|c| c.generation.as_str()))? {
            if let Some(mut entry) = hub.store.entry(&id).filter(|e| e.data["status"] == "queued") {
                entry.data["status"] = "cancelled".into();
                hub.set_entry(&id, &entry.data).context("Could not update cancelled Matrix prompt.")?;
            }
        }
    }
    Ok(status(hub, c.as_ref()))
}

/// What a client shows: never the access token.
fn status(hub: &Hub, c: Option<&Config>) -> Value {
    let state = hub.store.kv_read("matrix.state").ok().flatten();
    json!({
        "configured": c.is_some_and(|c| !c.access_token.is_empty()),
        "homeserver": c.map(|c| &c.homeserver).filter(|h| !h.is_empty()),
        "userId": c.map(|c| &c.user_id).filter(|u| !u.is_empty()),
        "deviceId": c.map(|c| &c.device_id).filter(|d| !d.is_empty()),
        "bindings": c.map(|c| &c.bindings),
        "state": state.unwrap_or_else(|| "starting".into()),
        "deliveries": hub.store.matrix_deliveries().unwrap_or_default(),
    })
}

/// The display name of the room in the logs: its id without the `!` and server, or the id itself.
pub(super) fn room_label(room: &str) -> &str {
    room.split(':').next().unwrap_or(room).trim_start_matches(['!', '#'])
}
