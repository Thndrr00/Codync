//! Signal as a way to talk to a bot, through a `signal-cli` daemon the person runs on this
//! computer. The host never links a number itself: the daemon owns the Signal account, and the
//! host only calls its JSON-RPC endpoint. The number and the account's keys stay in the daemon
//! (and the number in the host's vault); no Signal server sees Codync.

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

const SLOT: &str = "signal.channel";
pub const METHODS: &[&str] =
    &["signalStatus", "setSignalChannel", "disconnectSignal", "setChannelMemory", "setChannelIcon"];
const MAX_BINDINGS: usize = 50;
const MAX_ALLOWED: usize = 100;
const DEFAULT_DAEMON: &str = "http://127.0.0.1:7583";

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Binding {
    /// A phone number in E.164 (`+15551234567`) for a direct chat, or `group:<id>`.
    pub chat: String,
    pub bot_id: String,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub memory: String,
    /// An SF Symbol shown as the conversation's picture, or empty.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub icon: String,
    pub allowed_users: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Config {
    /// The linked number, in E.164.
    pub number: String,
    /// The daemon's base URL, without a trailing slash.
    pub daemon: String,
    pub bindings: Vec<Binding>,
    /// Changes whenever the number or the daemon is replaced, so an old stream's prompts are dropped.
    #[serde(default)]
    pub generation: String,
}

/// What a client sends to set the channel up. The daemon address and the number are all the host
/// needs; signal-cli keeps the account's keys.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Setup {
    pub number: String,
    #[serde(default)]
    pub daemon: String,
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

/// Runs the event stream and the sender while the host runs.
pub(crate) async fn run(hub: Arc<Hub>) {
    tokio::join!(sync::run(hub.clone()), worker::run(hub));
}

/// `+15551234567`: E.164, without spaces or dashes.
pub(super) fn number(value: &str) -> Result<String> {
    let trimmed = value.trim();
    anyhow::ensure!(
        trimmed.len() >= 8
            && trimmed.len() <= 16
            && trimmed.starts_with('+')
            && trimmed[1..].bytes().all(|b| b.is_ascii_digit())
            && !trimmed[1..].starts_with('0'),
        "Write the Signal number as E.164, starting with + and the country code."
    );
    Ok(trimmed.to_owned())
}

/// A phone number or a Signal group id.
pub(super) fn chat(value: &str) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.starts_with("group:") {
        let id = &trimmed[6..];
        anyhow::ensure!(
            (8..=512).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_alphanumeric() || b"+/=_-".contains(&b)),
            "A Signal group id looks like group:<id>."
        );
        return Ok(trimmed.to_owned());
    }
    number(trimmed).map_err(|_| anyhow::anyhow!("A chat is a phone number (+15551234567) or group:<id>."))
}

/// A number or a Signal UUID, as signal-cli reports the sender.
pub(super) fn person(value: &str) -> bool {
    let trimmed = value.trim();
    (trimmed.starts_with('+')
        && trimmed.len() >= 8
        && trimmed.len() <= 16
        && trimmed[1..].bytes().all(|b| b.is_ascii_digit()))
        || ((32..=64).contains(&trimmed.len())
            && trimmed.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-')
            && trimmed.contains('-'))
}

/// The daemon's base URL. A daemon on another computer of the person's own network is plain
/// HTTP; anything else must be HTTPS.
pub(super) fn daemon(value: &str) -> Result<String> {
    let trimmed = value.trim().trim_end_matches('/');
    anyhow::ensure!(!trimmed.is_empty() && trimmed.len() <= 300, "Write the signal-cli daemon's address.");
    let url = url::Url::parse(trimmed).context("The daemon address is not a URL.")?;
    anyhow::ensure!(
        matches!(url.scheme(), "https" | "http"),
        "The daemon address starts with http:// (this computer's network) or https://."
    );
    anyhow::ensure!(url.host_str().is_some(), "The daemon address has no host.");
    anyhow::ensure!(url.username().is_empty() && url.password().is_none(), "Leave credentials out of the address.");
    anyhow::ensure!(url.path().is_empty() || url.path() == "/", "The daemon address has no path.");
    Ok(trimmed.to_owned())
}

fn binding_ok(hub: &Hub, binding: &Binding) -> Result<()> {
    chat(&binding.chat)?;
    if binding.allowed_users.is_empty() || binding.allowed_users.len() > MAX_ALLOWED {
        bail!("Each chat needs 1-{MAX_ALLOWED} allowed people.");
    }
    if binding.allowed_users.iter().any(|user| !person(user)) {
        bail!("Allowed people are Signal numbers (+15551234567) or UUIDs.");
    }
    hub.store
        .bot(&binding.bot_id)?
        .filter(|bot| !bot.deleted && !bot.config.is_group() && !crate::external::is_external(&bot.config))
        .context("A bound bot is unavailable.")?;
    Ok(())
}

fn validate(hub: &Hub, c: &Config) -> Result<()> {
    number(&c.number)?;
    daemon(&c.daemon)?;
    if c.bindings.is_empty() || c.bindings.len() > MAX_BINDINGS {
        bail!("Bind 1-{MAX_BINDINGS} chats.");
    }
    let mut seen = std::collections::HashSet::new();
    for binding in &c.bindings {
        if !seen.insert(&binding.chat) {
            bail!("A chat is bound twice.");
        }
        binding_ok(hub, binding)?;
    }
    Ok(())
}

/// The `.setup` line a client shows when the person still has to start signal-cli or link a
/// number. It is a real instruction, never a pretend connection.
fn setup_text(number: &str) -> String {
    let number = if number.is_empty() { "+15551234567" } else { number };
    format!(
        "Signal needs a signal-cli daemon. On this computer, link the number once with \
         `signal-cli -a {number} link` (scan the QR code in Signal > Linked devices) or register it with \
         `signal-cli -a {number} register` and `signal-cli -a {number} verify CODE`. Then run \
         `signal-cli -a {number} daemon --http 127.0.0.1:7583 --receive-mode=on-start` and save the number and \
         daemon address here."
    )
}

pub(crate) async fn call(hub: &Arc<Hub>, method: &str, body: Value) -> Result<Value> {
    crate::market::vault::unlock(hub.clone()).await?;
    // The daemon checks are network calls; they happen before the configuration lock so the lock is
    // never held across an await.
    let prepared = if method == "setSignalChannel" {
        let setup: Setup = serde_json::from_value(body.clone())?;
        let number = number(&setup.number)?;
        let daemon = if setup.daemon.trim().is_empty() { DEFAULT_DAEMON.to_owned() } else { daemon(&setup.daemon)? };
        let mut config = Config {
            number: number.clone(),
            daemon: daemon.clone(),
            bindings: setup.bindings,
            generation: String::new(),
        };
        config.bindings.sort_by(|a, b| a.chat.cmp(&b.chat));
        validate(hub, &config)?;
        // If the daemon answers, the number must be one it is linked to. If it doesn't, the
        // configuration is still saved and the status says exactly how to start it.
        if let Ok(false) = client::has_account(&daemon, &number).await {
            bail!("signal-cli at {daemon} does not have {number}. Link it with `signal-cli -a {number} link` first.");
        }
        Some(config)
    } else {
        None
    };
    let _guard = hub.store.connector_lock.locked();
    match method {
        "setSignalChannel" => {
            let mut config = prepared.context("Missing Signal configuration.")?;
            validate(hub, &config)?;
            config.generation = match load(&hub.store)? {
                Some(previous) if previous.number == config.number && previous.daemon == config.daemon => {
                    previous.generation
                }
                _ => uuid::Uuid::new_v4().to_string(),
            };
            save(&hub.store, &config)?;
            hub.store.kv_set("signal.state", "starting")?;
        }
        "disconnectSignal" => {
            let generation = load(&hub.store)?.map(|c| c.generation);
            crate::market::vault::write(&hub.store, SLOT, "")?;
            hub.store.kv_delete("signal.state")?;
            if let Some(generation) = generation.filter(|g| !g.is_empty()) {
                hub.store.kv_delete(&format!("signal.cursor.{generation}"))?;
            }
        }
        "setChannelMemory" => {
            let chat = body["channel"].as_str().context("channel is required.")?;
            let memory = crate::channels::memory_arg(&body)?;
            let mut config = load(&hub.store)?.context("Signal is not configured.")?;
            let binding = config.bindings.iter_mut().find(|b| b.chat == chat).context("Unknown Signal chat.")?;
            binding.memory = memory;
            save(&hub.store, &config)?;
        }
        "setChannelIcon" => {
            let chat = body["channel"].as_str().context("channel is required.")?;
            let icon = crate::channels::icon_arg(&body)?;
            let mut config = load(&hub.store)?.context("Signal is not configured.")?;
            let binding = config.bindings.iter_mut().find(|b| b.chat == chat).context("Unknown Signal chat.")?;
            binding.icon = icon;
            save(&hub.store, &config)?;
        }
        "signalStatus" => {}
        other => bail!("Unknown Signal method `{other}`."),
    }
    let c = load(&hub.store)?;
    if method != "signalStatus" {
        for id in hub.store.reject_stale_signal(c.as_ref().map(|c| c.generation.as_str()))? {
            if let Some(mut entry) = hub.store.entry(&id).filter(|e| e.data["status"] == "queued") {
                entry.data["status"] = "cancelled".into();
                hub.set_entry(&id, &entry.data).context("Could not update cancelled Signal prompt.")?;
            }
        }
    }
    Ok(status(hub, c.as_ref()))
}

/// What a client shows: the number, never a signal-cli secret.
fn status(hub: &Hub, c: Option<&Config>) -> Value {
    let state = hub.store.kv_read("signal.state").ok().flatten().unwrap_or_else(|| "starting".into());
    let configured = c.is_some();
    let number = c.map(|c| c.number.clone()).unwrap_or_default();
    let setup = if !configured || state.starts_with("notRunning") || state.starts_with("error") {
        json!(setup_text(&number))
    } else {
        Value::Null
    };
    json!({
        "configured": configured,
        "number": c.map(|c| &c.number).filter(|n| !n.is_empty()),
        "daemon": c.map(|c| &c.daemon).filter(|d| !d.is_empty()),
        "bindings": c.map(|c| &c.bindings),
        "state": state,
        "setup": setup,
        "deliveries": hub.store.signal_deliveries().unwrap_or_default(),
        "signalCli": "signal-cli daemon --http 127.0.0.1:7583",
    })
}

/// What a chat is called in the logs and in the bot's prompt.
pub(super) fn chat_label(chat: &str) -> String {
    if let Some(group) = chat.strip_prefix("group:") {
        format!("group {}", group.chars().take(8).collect::<String>())
    } else {
        chat.to_owned()
    }
}
