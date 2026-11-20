//! Hourly, idle-gated updates of the provider CLIs Codync downloaded itself.
//!
//! `agent::registry` is the only downloader: a pending registry build is installed by
//! calling its normal launch command. This module only decides *when* to do that and
//! remembers a version the person cancelled, so it is not retried until the host restarts.

use crate::{
    LockExt as _,
    agent::{backends, registry},
    hub::Hub,
    store::Store,
};
use anyhow::{Result, anyhow};
use serde_json::{Value, json};
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
    time::Duration,
};

/// One hourly pass is plenty: the registry itself only refreshes daily.
const CHECK_EVERY: Duration = Duration::from_hours(1);
/// An update can wait this long for agents to go idle, then the pass gives up.
const IDLE_TIMEOUT: Duration = Duration::from_mins(10);
const IDLE_POLL: Duration = Duration::from_secs(30);

/// Versions a person cancelled, for this host process only: a restart clears them.
static CANCELLED: Mutex<HashSet<String>> = Mutex::new(HashSet::new());

fn key(id: &str, version: &str) -> String {
    format!("{id}\0{version}")
}

/// Remembers that `version` of `id` was cancelled. A newer version is still eligible.
pub fn cancel(id: &str, version: &str) -> Value {
    CANCELLED.locked().insert(key(id, version));
    json!({"cancelled": true, "id": id, "version": version})
}

fn cancelled(id: &str, version: &str) -> bool {
    CANCELLED.locked().contains(&key(id, version))
}

#[derive(Clone, Debug)]
struct Pending {
    id: String,
    version: String,
    agent: Value,
}

/// Provider CLIs that have a waiting build and may be updated on this computer.
fn pending(store: &Store) -> Vec<Pending> {
    backends::list()
        .into_iter()
        .filter(|provider| provider["updateAvailable"].as_bool().unwrap_or(false))
        .filter_map(|provider| {
            let id = provider["id"].as_str()?.to_owned();
            if !backends::auto_updatable(store, &id) {
                return None;
            }
            let agent = backends::pending_build(&id)?;
            let version = agent["version"].as_str().unwrap_or("latest").to_owned();
            Some(Pending { id, version, agent })
        })
        .collect()
}

/// Waits until no bot or setup terminal would be stopped. Reuses the host update's idle
/// gate, so "initialization" alone never holds a CLI update either.
async fn wait_idle(hub: &Hub) -> bool {
    let deadline = tokio::time::Instant::now() + IDLE_TIMEOUT;
    loop {
        let readiness = super::restart_readiness(hub);
        let reasons: Vec<&str> = readiness.reasons.iter().map(String::as_str).collect();
        if readiness.idle || !super::restart_would_stop_work(&reasons) {
            return true;
        }
        if tokio::time::Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(IDLE_POLL).await;
    }
}

/// Downloads a pending build through the registry's own install path.
async fn apply(update: &Pending) -> Result<()> {
    registry::command(&update.agent, |_| {}).await.map_err(|error| anyhow!("{id}: {error:#}", id = update.id))
}

/// Updates one provider now, if it has an eligible, un-cancelled build.
pub async fn update_one(hub: &Hub, id: &str) -> Result<Value> {
    let update = pending(&hub.store)
        .into_iter()
        .find(|update| update.id == id)
        .ok_or_else(|| anyhow!("{id} has no Codync-managed CLI update"))?;
    if cancelled(&update.id, &update.version) {
        return Ok(json!({"updated": [], "skipped": [update.id]}));
    }
    if !wait_idle(hub).await {
        anyhow::bail!("A bot is still working; the CLI update did not start.");
    }
    apply(&update).await?;
    Ok(json!({"updated": [update.id], "skipped": []}))
}

/// Updates every eligible provider at once, skipping versions the person cancelled.
pub async fn update_all(hub: &Hub) -> Result<Value> {
    let updates: Vec<Pending> = pending(&hub.store).into_iter().filter(|u| !cancelled(&u.id, &u.version)).collect();
    if updates.is_empty() {
        return Ok(json!({"updated": [], "skipped": []}));
    }
    if !wait_idle(hub).await {
        anyhow::bail!("A bot is still working; no CLI update started.");
    }
    let mut updated = Vec::new();
    let mut skipped = Vec::new();
    for update in updates {
        match apply(&update).await {
            Ok(()) => updated.push(update.id),
            Err(error) => {
                tracing::warn!(provider = %update.id, error = format!("{error:#}"), "provider CLI update failed");
                skipped.push(json!({"id": update.id, "error": format!("{error:#}")}));
            }
        }
    }
    Ok(json!({"updated": updated, "skipped": skipped}))
}

/// Checks hourly, waits for idle agents, and updates each eligible CLI once. A cancelled
/// version is left alone for the life of this process; the bounded wait never holds a bot.
pub async fn watch(hub: Arc<Hub>) {
    if crate::environment::Environment::current() != crate::environment::Environment::Main {
        return;
    }
    let mut last_pass = None;
    loop {
        tokio::time::sleep(Duration::from_mins(1)).await;
        if last_pass.is_some_and(|at: tokio::time::Instant| at.elapsed() < CHECK_EVERY) {
            continue;
        }
        let updates: Vec<Pending> = pending(&hub.store).into_iter().filter(|u| !cancelled(&u.id, &u.version)).collect();
        if updates.is_empty() {
            continue;
        }
        if !wait_idle(&hub).await {
            continue;
        }
        last_pass = Some(tokio::time::Instant::now());
        for update in updates {
            if cancelled(&update.id, &update.version) {
                continue;
            }
            match apply(&update).await {
                Ok(()) => tracing::info!(provider = %update.id, version = %update.version, "updated provider CLI"),
                Err(error) => {
                    tracing::warn!(provider = %update.id, error = format!("{error:#}"), "provider CLI update failed")
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cancelled_version_is_skipped_but_a_newer_one_is_not() {
        assert!(!cancelled("codex", "2.0.0"));
        cancel("codex", "2.0.0");
        assert!(cancelled("codex", "2.0.0"));
        assert!(!cancelled("codex", "2.1.0"), "a newer build may still update");
        assert!(!cancelled("claude", "2.0.0"), "one provider's cancel is another's");
    }
}
