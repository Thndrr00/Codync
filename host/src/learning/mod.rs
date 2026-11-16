//! Automatic learning: durable facts and preferences from what the person tells a bot, proposed for review and saved
//! to its memory only when approved (or after the configured number of repeats).
//!
//! Only the person's own messages are read: the bot's replies, routines, group chats and outside text never propose
//! anything. A correction ("No, use pnpm") and a stated preference ("I always want the tests run before a commit") are
//! candidates at once; an ordinary instruction becomes one when the person says it again. Code, URLs, emails, numeric
//! identifiers and anything that looks like a secret are left alone.
//!
//! An approved fact is one line of the bot's `preferences.md` (see [`apply`]), kept as the person's, with the belief
//! header `feedback` / `high` / today. The same fact is never proposed or written twice. Storage is a small list per
//! bot in the key-value store, like the proposed skills.

pub mod apply;
pub mod detect;
#[cfg(test)]
mod tests;

use crate::hub::Hub;
use crate::store::{BotConfig, EntryKind, Lane};
use anyhow::{Result, anyhow, bail, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;

pub const METHODS: &[&str] = &["learning", "approveLearning", "dismissLearning", "saveLearningSettings"];

const MAX_PENDING: usize = 20;
const MAX_DISMISSED: usize = 200;
/// How many recent messages of the bot's own chat are read to spot a repeated instruction.
const HISTORY: i64 = 60;
const MAX_AUTO: u8 = 10;

/// A bot's automatic-learning settings.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct Settings {
    /// Propose facts from the person's corrections and repeated instructions.
    pub enabled: bool,
    /// Save a fact without review once the person has said it this many times. 0: always wait for review.
    pub auto_approve_after: u8,
}

impl Default for Settings {
    fn default() -> Self {
        Self { enabled: true, auto_approve_after: 0 }
    }
}

impl Settings {
    pub fn validate(&self) -> Result<()> {
        ensure!(self.auto_approve_after <= MAX_AUTO, "Learn without review after at most {MAX_AUTO} repeats.");
        Ok(())
    }
}

/// A fact waiting for the person to approve or dismiss it.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Proposal {
    pub id: String,
    pub bot_id: String,
    pub text: String,
    pub kind: detect::Kind,
    /// How many times the person said it.
    pub hits: u32,
    /// The message it came from, cut short, so the card has context.
    pub source: String,
    pub created_at: i64,
}

fn pending_key(bot: &str) -> String {
    format!("learning.pending.{bot}")
}

fn dismissed_key(bot: &str) -> String {
    format!("learning.dismissed.{bot}")
}

pub fn pending(store: &crate::store::Store, bot: &str) -> Vec<Proposal> {
    store.kv_read(&pending_key(bot)).ok().flatten().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

fn save_pending(store: &crate::store::Store, bot: &str, list: &[Proposal]) -> Result<()> {
    store.kv_set(&pending_key(bot), &serde_json::to_string(list)?)
}

fn dismissed(store: &crate::store::Store, bot: &str) -> Vec<String> {
    store.kv_read(&dismissed_key(bot)).ok().flatten().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

fn remember_dismissed(store: &crate::store::Store, bot: &str, fingerprint: &str) -> Result<()> {
    let mut list = dismissed(store, bot);
    if !list.iter().any(|f| f == fingerprint) {
        list.push(fingerprint.to_owned());
    }
    if list.len() > MAX_DISMISSED {
        list.drain(..list.len() - MAX_DISMISSED);
    }
    store.kv_set(&dismissed_key(bot), &serde_json::to_string(&list)?)
}

/// Reads the person's message after a turn (`message_id`: the entry it came from, so it isn't counted as an earlier
/// saying) and proposes what is worth remembering. Failures are logged and change nothing; this never runs in the way
/// of the turn.
pub fn consider(hub: Arc<Hub>, cfg: BotConfig, request: String, message_id: Option<String>) {
    if let Err(error) = try_consider(&hub, &cfg, &request, message_id.as_deref()) {
        tracing::warn!(bot = %cfg.id, error = format!("{error:#}"), "a memory could not be proposed");
    }
}

fn try_consider(hub: &Arc<Hub>, cfg: &BotConfig, request: &str, message_id: Option<&str>) -> Result<()> {
    if cfg.is_group() || !cfg.learning.enabled || crate::external::is_external(cfg) || request.trim().is_empty() {
        return Ok(());
    }
    let history = hub
        .store
        .history(&cfg.id, i64::MAX, HISTORY)?
        .into_iter()
        .filter(|entry| Some(entry.id.as_str()) != message_id)
        .filter(|entry| entry.kind == EntryKind::User.as_str())
        .filter_map(|entry| entry.data["text"].as_str().map(str::to_owned))
        .collect::<Vec<_>>();
    let candidates = detect::candidates(request, &history);
    if candidates.is_empty() {
        return Ok(());
    }
    let mut list = pending(&hub.store, &cfg.id);
    let dismissed = dismissed(&hub.store, &cfg.id);
    let threshold = cfg.learning.auto_approve_after;
    let mut changed = false;
    for candidate in candidates {
        let print = detect::fingerprint(&candidate.text);
        if dismissed.contains(&print) || apply::known(hub, cfg, &candidate.text)? {
            continue;
        }
        let existing = list.iter().position(|p| detect::fingerprint(&p.text) == print);
        let mut proposal = match existing {
            Some(at) => list.remove(at),
            None => Proposal {
                id: uuid::Uuid::new_v4().to_string(),
                bot_id: cfg.id.clone(),
                text: candidate.text.clone(),
                kind: candidate.kind,
                hits: candidate.hits,
                source: crate::agent::acp::truncate(request.trim(), 240),
                created_at: crate::store::now_ms(),
            },
        };
        proposal.text = candidate.text.clone();
        proposal.kind = candidate.kind;
        proposal.hits = proposal.hits.max(candidate.hits);
        proposal.source = crate::agent::acp::truncate(request.trim(), 240);
        changed = true;
        if threshold > 0 && proposal.hits >= u32::from(threshold) {
            match apply::save(hub, cfg, &proposal.text) {
                Ok(saved) => {
                    if saved {
                        announce(hub, cfg, &proposal.text);
                    }
                    // Saved: the fact is in memory now, which keeps it from being proposed again.
                    continue;
                }
                Err(error) => {
                    tracing::warn!(bot = %cfg.id, error = format!("{error:#}"), "couldn't save a learned fact");
                    if list.len() < MAX_PENDING {
                        list.push(proposal);
                    }
                    continue;
                }
            }
        }
        if list.len() < MAX_PENDING {
            list.push(proposal);
        }
    }
    if changed {
        save_pending(&hub.store, &cfg.id, &list)?;
        hub.emit_bot(&cfg.id);
    }
    Ok(())
}

/// Tells the chat that a fact was saved without review. One line, no undo data: the memory screen has the note and its
/// history.
fn announce(hub: &Hub, cfg: &BotConfig, text: &str) {
    let data = json!({
        "text": format!("I learned: {}", crate::agent::acp::truncate(text.trim(), 200)),
        "style": "info",
    });
    let _ = hub.add_entry(&Lane::main(&cfg.id), EntryKind::Notice, hub.store.max_turn(&cfg.id), &data);
}

/// The inbox as the apps see it.
fn view(hub: &Hub, bot: &str) -> Result<Value> {
    let cfg = hub
        .store
        .bot(bot)?
        .filter(|row| !row.deleted && !row.config.is_group())
        .ok_or_else(|| anyhow!("unknown bot"))?;
    Ok(json!({
        "enabled": cfg.config.learning.enabled,
        "autoApproveAfter": cfg.config.learning.auto_approve_after,
        "maxAutoApproveAfter": MAX_AUTO,
        "note": apply::NOTE,
        "proposals": pending(&hub.store, bot),
    }))
}

/// `learning {botId}`, `approveLearning`/`dismissLearning {botId, id}`, `saveLearningSettings {botId, enabled?,
/// autoApproveAfter?}`: each answers with the inbox as it is now.
pub fn call(hub: &Arc<Hub>, method: &str, b: &Value) -> Result<Value> {
    let bot = b["botId"].as_str().ok_or_else(|| anyhow!("botId required"))?.to_owned();
    hub.store.bot(&bot)?.filter(|row| !row.deleted && !row.config.is_group()).ok_or_else(|| anyhow!("unknown bot"))?;
    match method {
        "learning" => {}
        "saveLearningSettings" => {
            let cfg = hub.store.bot(&bot)?.filter(|row| !row.deleted).ok_or_else(|| anyhow!("unknown bot"))?.config;
            let enabled = b["enabled"].as_bool().unwrap_or(cfg.learning.enabled);
            let auto = b["autoApproveAfter"].as_u64().map_or(cfg.learning.auto_approve_after, |n| {
                u8::try_from(n.min(u64::from(MAX_AUTO))).unwrap_or(MAX_AUTO)
            });
            hub.update_bot(&json!({"id": &bot, "learning": {"enabled": enabled, "autoApproveAfter": auto}}))?;
        }
        "approveLearning" | "dismissLearning" => {
            let id = b["id"].as_str().ok_or_else(|| anyhow!("id required"))?;
            let mut list = pending(&hub.store, &bot);
            let at = list.iter().position(|p| p.id == id).ok_or_else(|| anyhow!("That learning is gone."))?;
            let proposal = list.remove(at);
            if method == "approveLearning" {
                let cfg = hub.store.bot(&bot)?.filter(|row| !row.deleted).ok_or_else(|| anyhow!("unknown bot"))?.config;
                if apply::save(hub, &cfg, &proposal.text)? {
                    announce(hub, &cfg, &proposal.text);
                }
            } else {
                remember_dismissed(&hub.store, &bot, &detect::fingerprint(&proposal.text))?;
            }
            save_pending(&hub.store, &bot, &list)?;
            hub.emit_bot(&bot);
        }
        _ => bail!("unknown method: {method}"),
    }
    view(hub, &bot)
}
