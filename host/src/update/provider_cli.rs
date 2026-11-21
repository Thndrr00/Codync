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
    collections::{BTreeMap, HashSet},
    sync::{Arc, Mutex},
    time::Duration,
};

/// One hourly pass is plenty: the registry itself only refreshes daily.
const CHECK_EVERY: Duration = Duration::from_hours(1);
/// An update can wait this long for agents to go idle, then the pass gives up.
const IDLE_TIMEOUT: Duration = Duration::from_mins(10);
const IDLE_POLL: Duration = Duration::from_secs(30);
/// Where the version of each CLI Codync downloaded is kept (see [`installed_versions`]).
const INSTALLED_FILE: &str = "provider-cli-installed.json";

/// Versions a person cancelled, for this host process only: a restart clears them.
static CANCELLED: Mutex<HashSet<String>> = Mutex::new(HashSet::new());

fn key(id: &str, version: &str) -> String {
    format!("{id}\0{version}")
}

/// Whether this exact `(id, version)` was cancelled. Pure over the saved keys so the decision
/// is testable without the process-global latch.
fn cancelled_by(keys: &HashSet<String>, id: &str, version: &str) -> bool {
    keys.contains(&key(id, version))
}

/// Remembers that `version` of `id` was cancelled. A newer version is still eligible, and the
/// latch lives only until this host restarts — an update the person cancelled is not started
/// again for *that version* until then.
pub fn cancel(id: &str, version: &str) -> Value {
    CANCELLED.locked().insert(key(id, version));
    json!({"cancelled": true, "id": id, "version": version})
}

fn cancelled(id: &str, version: &str) -> bool {
    cancelled_by(&CANCELLED.locked(), id, version)
}

/// The versions of provider CLIs this host downloaded that are currently installed. Kept beside
/// the host's other update state and written through the same [`super::read`]/[`super::write`]
/// helpers, so the hourly check keeps saying "up to date" after an update until the registry
/// serves a build with a higher version.
fn installed_versions() -> BTreeMap<String, String> {
    super::read(INSTALLED_FILE).unwrap_or_default()
}

fn note_installed(id: &str, version: &str) {
    let mut versions = installed_versions();
    versions.insert(id.to_owned(), version.to_owned());
    if let Err(error) = super::write(INSTALLED_FILE, &versions) {
        tracing::warn!(provider = id, error = format!("{error:#}"), "couldn't remember the CLI version");
    }
}

/// Whether `available` is a newer build than the `installed` version of one CLI. A release
/// version comparison keeps a nightly tag from being reported again after it was installed;
/// tags the registry doesn't number as semver fall back to a plain difference so a changed
/// version string is still treated as a new build. `None` (never downloaded) is always newer.
pub(crate) fn newer_version(available: &str, installed: Option<&str>) -> bool {
    let Some(installed) = installed.map(str::trim).filter(|version| !version.is_empty()) else {
        return true;
    };
    let available = available.trim();
    match (semver::Version::parse(available), semver::Version::parse(installed)) {
        (Ok(available), Ok(installed)) => available > installed,
        _ => available != installed,
    }
}

#[derive(Clone, Debug)]
struct Pending {
    id: String,
    version: String,
    agent: Value,
}

/// Provider CLIs that have a waiting build and may be updated on this computer. A build whose
/// version is not newer than the one we last installed is left out, so an update Codync just
/// applied isn't reported again until the registry serves a higher version.
fn pending(store: &Store) -> Vec<Pending> {
    let installed = installed_versions();
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
            if !newer_version(&version, installed.get(&id).map(String::as_str)) {
                return None;
            }
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

/// Downloads a pending build through the registry's own install path, then remembers the version
/// so the next hourly check doesn't offer the same build again. Every update path (one, all and
/// the hourly watch) goes through here, so the version is recorded exactly once per success.
async fn apply(update: &Pending) -> Result<()> {
    registry::command(&update.agent, |_| {}).await.map_err(|error| anyhow!("{id}: {error:#}", id = update.id))?;
    note_installed(&update.id, &update.version);
    Ok(())
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

    #[test]
    fn a_cancelled_version_is_not_restarted_until_the_host_restarts() {
        // The latch is per (id, version) and lives only in this process. "Restart" clears it.
        let mut keys = HashSet::new();
        keys.insert(key("codex", "2.0.0"));
        assert!(cancelled_by(&keys, "codex", "2.0.0"), "the cancelled build is not started again");
        assert!(!cancelled_by(&keys, "codex", "2.1.0"), "a newer build is still eligible");
        assert!(!cancelled_by(&keys, "claude", "2.0.0"), "another provider is unaffected");
        keys.clear(); // host restart
        assert!(!cancelled_by(&keys, "codex", "2.0.0"), "the old build may be retried after a restart");
    }

    #[test]
    fn an_installed_version_stops_the_same_build_but_not_a_newer_one() {
        assert!(newer_version("2.0.0", None), "a CLI never downloaded is always eligible");
        assert!(newer_version("2.0.0", Some("")), "an empty record is no record");
        assert!(!newer_version("2.0.0", Some("2.0.0")), "the hourly check stops re-reporting it");
        assert!(!newer_version(" 2.0.0 ", Some("2.0.0")), "whitespace is ignored");
        assert!(newer_version("2.1.0", Some("2.0.0")), "a higher release is a new build");
        assert!(newer_version("2.0.1", Some("2.0.0")), "a patch counts");
        assert!(!newer_version("1.9.0", Some("2.0.0")), "an older build is never installed again");
        // Registry versions that aren't semver fall back to a plain difference, so a nightly
        // tag that changed still updates while an unchanged one doesn't.
        assert!(!newer_version("nightly-2026-11-21", Some("nightly-2026-11-21")));
        assert!(newer_version("nightly-2026-11-22", Some("nightly-2026-11-21")));
        assert!(!newer_version("v1", Some("v1")), "unparseable but equal tags are the same build");
    }
}
