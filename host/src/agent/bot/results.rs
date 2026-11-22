//! Durable result attribution for every user message in an executed batch. Also keeps the small
//! markers that tell a teammate result re-delivered after a restart that its provider turn had
//! already started, so an empty replay can end quietly (OpenBot nightly triage-14330).
use super::Actor;
use crate::{
    hub::Hub,
    store::{EntryKind, Lane},
};
use anyhow::{Context, Result, ensure};
use serde_json::json;
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedSources {
    chat: String,
    thread: Option<String>,
    entries: Vec<String>,
}
/// One kv key per bot holds the entries of its current turn that reached the provider. One key (not
/// one per message) keeps these markers bounded no matter how long the chat runs.
fn provider_key(bot: &str) -> String {
    format!("team.provider.{bot}")
}

/// The key holding the teammate results recovery gave again after their provider turn started.
fn replay_key(bot: &str) -> String {
    format!("team.replayed.{bot}")
}

/// The kv key of the last result this bot handled for a teammate request.
fn result_key(request: &str) -> String {
    format!("team.result.{request}")
}

fn saved_ids(hub: &Hub, key: &str) -> Vec<String> {
    hub.store.kv_get(key).and_then(|v| serde_json::from_str(&v).ok()).unwrap_or_default()
}

/// Notes that the entries of a turn actually reached `session/prompt`, so a later restart knows the
/// provider saw them (triage-14330). Overwritten each turn; a steer that was only queued isn't marked.
pub(super) fn note_provider_turn(hub: &Hub, bot: &str, ids: &[String]) {
    match serde_json::to_string(ids).map_err(anyhow::Error::from) {
        Ok(json) => {
            if let Err(error) = hub.store.kv_set(&provider_key(bot), &json) {
                tracing::warn!(bot, error = format!("{error:#}"), "couldn't mark a delivered teammate result");
            }
        }
        Err(error) => tracing::warn!(bot, error = format!("{error:#}"), "couldn't encode a delivered teammate result"),
    }
}

fn had_provider_turn(hub: &Hub, bot: &str, id: &str) -> bool {
    saved_ids(hub, &provider_key(bot)).iter().any(|e| e == id)
}

/// Notes that recovery gave a result again after its provider turn started.
fn note_replayed(hub: &Hub, bot: &str, id: &str) {
    let mut ids = saved_ids(hub, &replay_key(bot));
    if ids.iter().any(|e| e == id) {
        return;
    }
    ids.push(id.to_owned());
    if let Ok(json) = serde_json::to_string(&ids)
        && let Err(error) = hub.store.kv_set(&replay_key(bot), &json)
    {
        tracing::warn!(bot, error = format!("{error:#}"), "couldn't mark a replayed teammate result");
    }
}

/// Whether any of `ids` was given again by restart recovery.
pub(super) fn was_replayed(hub: &Hub, bot: &str, ids: &[String]) -> bool {
    let replayed = saved_ids(hub, &replay_key(bot));
    ids.iter().any(|id| replayed.contains(id))
}

/// Whether recovery left any result to give again for `bot`. Codync's restart resume starts its hidden
/// turn without the original message ids, so the one-shot marker is also read at bot level there.
pub(super) fn was_replayed_any(hub: &Hub, bot: &str) -> bool {
    !saved_ids(hub, &replay_key(bot)).is_empty()
}

/// Consumes the replay markers: a result is given back once, not on every later turn.
pub(super) fn consume_replays(hub: &Hub, bot: &str, ids: &[String]) {
    let name = replay_key(bot);
    let left: Vec<String> = saved_ids(hub, &name).into_iter().filter(|e| !ids.contains(e)).collect();
    let saved = if left.is_empty() {
        hub.store.kv_delete(&name)
    } else {
        serde_json::to_string(&left).map_err(anyhow::Error::from).and_then(|v| hub.store.kv_set(&name, &v))
    };
    if let Err(error) = saved {
        tracing::warn!(bot, error = format!("{error:#}"), "couldn't clear a replayed teammate result");
    }
}

/// Clears every replay marker of `bot`, used when a hidden resume turn carries no message ids.
pub(super) fn consume_all_replays(hub: &Hub, bot: &str) {
    if let Err(error) = hub.store.kv_delete(&replay_key(bot)) {
        tracing::warn!(bot, error = format!("{error:#}"), "couldn't clear replayed teammate results");
    }
}

/// Remembers the text of the last result handled for `request`, so a restart does not lose the
/// knowledge that the requester already had an answer (triage-14330).
pub(super) fn save_last_result(hub: &Hub, request: &str, text: &str) {
    if let Err(error) = hub.store.kv_set(&result_key(request), text) {
        tracing::warn!(request, error = format!("{error:#}"), "couldn't save the last teammate result");
    }
}

/// The last result text handled for `request`, before this restart too.
pub(super) fn last_result(hub: &Hub, request: &str) -> Option<String> {
    hub.store.kv_get(&result_key(request))
}

pub(crate) fn recover_results(hub: &Hub) -> Result<()> {
    for row in hub.store.bots()? {
        if !row.deleted {
            recover_bot_results(hub, &row.config.id)?;
        }
    }
    Ok(())
}
fn recover_bot_results(hub: &Hub, bot: &str) -> Result<()> {
    let key = format!("turn.sources.{bot}");
    let Some(saved) = hub.store.kv_read(&key)? else { return Ok(()) };
    let saved: SavedSources = serde_json::from_str(&saved).context("Unreadable executing message identities")?;
    ensure!(saved.chat == bot, "Executing message owner changed");
    for id in saved.entries {
        let Some(mut entry) = hub.store.entry(&id).filter(|entry| {
            entry.bot_id == saved.chat
                && entry.thread_id == saved.thread
                && entry.kind == EntryKind::User.as_str()
                && entry.data.get("turnResult").is_none_or(|result| result["state"] == "waitingForSignIn")
        }) else {
            continue;
        };
        let reached_provider = had_provider_turn(hub, bot, &id);
        entry.data["turnResult"] = json!({"version":1,"state":"interrupted","text":null,"stopReason":"host_restart"});
        hub.set_entry(&id, &entry.data).context("Could not mark interrupted user result")?;
        // triage-14330: only a result that had reached the provider before the restart may end its
        // empty replay quietly. One that was merely starting keeps the retry and the error.
        if reached_provider {
            note_replayed(hub, bot, &id);
        }
    }
    hub.store.kv_delete(&key)?;
    Ok(())
}
impl Actor {
    pub(super) fn interrupt_saved_results(&self, _lane: &Lane) -> Result<()> {
        recover_bot_results(&self.hub, &self.cfg.id)
    }
    pub(super) fn record_user_result(&self, state: &str, text: Option<&str>, stop_reason: &str) {
        for id in &self.turn_entries {
            let Some(mut entry) = self.hub.store.entry(id).filter(|entry| {
                entry.bot_id == self.lane.chat
                    && entry.thread_id == self.lane.thread
                    && entry.data.get("turnResult").is_none_or(|result| result["state"] == "waitingForSignIn")
            }) else {
                continue;
            };
            entry.data["turnResult"] = json!({"version":1,"state":state,"text":text,"stopReason":stop_reason});
            if self.hub.set_entry(id, &entry.data).is_none() {
                tracing::warn!(entry=%id,"User result attribution was not saved");
            }
        }
        // The turn is final now: a later restart must not treat it as still running.
        if let Err(error) = self.hub.store.kv_delete(&provider_key(&self.cfg.id)) {
            tracing::warn!(bot = %self.cfg.id, error = format!("{error:#}"), "couldn't clear a delivered teammate result");
        }
    }
}
