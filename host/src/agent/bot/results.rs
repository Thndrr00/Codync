//! Durable result attribution for every user message in an executed batch.
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
        entry.data["turnResult"] = json!({"version":1,"state":"interrupted","text":null,"stopReason":"host_restart"});
        hub.set_entry(&id, &entry.data).context("Could not mark interrupted user result")?;
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
    }
}
