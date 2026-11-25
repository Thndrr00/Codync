//! Creating, changing and deleting bots.

use super::{Hub, copy_name, sections, skill_updates, validate};
use crate::LockExt;
use crate::agent::bot::Cmd;
use crate::store::BotConfig;
use anyhow::{Context as _, Result, anyhow, bail};
use serde_json::{Value, json};
use std::sync::Arc;

/// OpenBot 0.78.0: a model picker can offer the same name on another provider. A patch that names only
/// a model must keep the bot's own provider: apply the name when this provider lists it (or nobody
/// does), and ignore a name that belongs to another one. `available` is `(provider, model)` pairs.
fn model_change(provider: &str, model: Option<&str>, available: &[(String, String)]) -> Option<String> {
    let model = model.map(str::trim).filter(|m| !m.is_empty())?;
    let listed_by_another = available.iter().any(|(p, m)| m == model && p != provider);
    let listed_here = available.iter().any(|(p, m)| m == model && p == provider);
    if listed_by_another && !listed_here {
        return None;
    }
    Some(model.to_owned())
}

/// The model names a patch could have been picked from: this provider's cached catalog, and the models
/// the computer's other bots already use (which is where a same-named choice on another provider shows).
fn available_models(hub: &Hub, provider: &str) -> Vec<(String, String)> {
    let key = provider.to_owned();
    let now = std::time::Instant::now();
    let catalog = hub.model_lists.fresh(&key, now).or_else(|| hub.model_lists.stale(&key));
    let mut out: Vec<(String, String)> = catalog
        .as_ref()
        .and_then(|value| value["models"].as_array())
        .into_iter()
        .flatten()
        .filter_map(|m| m["id"].as_str().map(|id| (provider.to_owned(), id.to_owned())))
        .collect();
    if let Ok(rows) = hub.store.bots() {
        for row in rows {
            if let Some(model) = row.config.model.as_deref().filter(|m| !m.is_empty()) {
                out.push((row.config.backend.clone(), model.to_owned()));
            }
        }
    }
    out
}

impl Hub {
    /// Copies a bot's setup into a new one: same agent, folder, permissions, skills and persona, but no
    /// conversation, no pinned/lead place and no owner. Groups are not copied.
    pub fn duplicate_bot(self: &Arc<Self>, id: &str) -> Result<Value> {
        let row = self.store.bot(id)?.filter(|b| !b.deleted).context("unknown bot")?;
        if row.config.is_group() {
            bail!("a group is made from its bots, not copied");
        }
        let mut cfg = row.config;
        let taken: Vec<String> = self.store.bots()?.into_iter().map(|b| b.config.name).collect();
        cfg.name = copy_name(&taken, &cfg.name);
        cfg.pinned = false;
        cfg.lead = false;
        cfg.hidden = false;
        cfg.created_by = None;
        cfg.auto_name = false;
        // A bot with its own workspace gets a copy of it, links kept as links. The new id is picked
        // here so the copy is complete before the actor starts and the config points at that copy.
        let source = std::path::PathBuf::from(&cfg.cwd);
        if crate::agent::workspace::is_managed(&cfg) && source.is_dir() {
            let new_id = uuid::Uuid::new_v4().to_string();
            let target = crate::agent::workspace::path(&new_id);
            if let Err(error) = crate::store::workspace_copy::copy_workspace(&source, &target) {
                let _ = std::fs::remove_dir_all(&target);
                return Err(error).context("copying the agent's workspace");
            }
            cfg.cwd = target.to_string_lossy().into_owned();
            let made = self.create_bot_with_id(cfg, new_id.clone());
            if made.is_err() {
                let _ = std::fs::remove_dir_all(&target);
            }
            return made;
        }
        self.create_bot(cfg)
    }

    pub fn create_bot(self: &Arc<Self>, cfg: BotConfig) -> Result<Value> {
        self.create_bot_with_id(cfg, uuid::Uuid::new_v4().to_string())
    }

    /// Shared with duplication, which copies the workspace before the new id is persisted.
    fn create_bot_with_id(self: &Arc<Self>, mut cfg: BotConfig, id: String) -> Result<Value> {
        cfg.id = id;
        if crate::external::is_external(&cfg) {
            bail!("a terminal session is created by its terminal, not by an app");
        }
        cfg.created_at = crate::store::now_ms();
        cfg.position = self.new_bot_position()?;
        // No name yet (Grok Bot's flow): it gets one after its first few conversations.
        cfg.auto_name = !cfg.is_group() && cfg.name.trim().is_empty();
        if cfg.auto_name {
            crate::chat::naming::PLACEHOLDER.clone_into(&mut cfg.name);
        }
        if cfg.is_group() {
            cfg.members = crate::chat::group::valid_members(&self.store, &cfg.id, &cfg.members)?;
            // The same bots again: open the group they already share (Grok Bot's rule).
            if let Some(existing) = crate::chat::group::with_members(&self.store, &cfg.members)? {
                return Ok(self.bot_value(&existing).unwrap_or(Value::Null));
            }
        }
        crate::agent::fallback::tidy(&mut cfg);
        cfg.section = sections::normalize_section(cfg.section.as_deref())?;
        validate(&cfg)?;
        crate::agent::workspace::prepare(&mut cfg)?;
        {
            let _g = self.emit_lock.locked();
            self.store.save_bot(&cfg)?;
        }
        crate::network::publish(self);
        if !cfg.is_group() {
            self.spawn_bot(cfg.clone());
            crate::market::agent_sign_in::ask_before_first_message(self, &cfg);
        }
        self.emit_bot(&cfg.id);
        crate::usage::bots_changed(self);
        Ok(self.bot_value(&cfg.id).unwrap_or(Value::Null))
    }

    pub fn update_bot(&self, patch: &Value) -> Result<Value> {
        let id = patch["id"].as_str().ok_or_else(|| anyhow!("id required"))?.to_owned();
        // From reading the bot to saving it, so no other update saves a stale copy in between.
        let _one = crate::chat::instruction_history::lock();
        let row = self.store.bot(&id)?.filter(|b| !b.deleted).ok_or_else(|| anyhow!("unknown bot"))?;
        skill_updates::check(&row.config, patch)?;
        let before = row.config.clone();
        let mut merged = serde_json::to_value(&row.config)?;
        for (k, v) in patch.as_object().into_iter().flatten() {
            if !matches!(k.as_str(), "id" | "createdAt" | "createdBy" | "lead" | "ifSkills") {
                merged[k] = v.clone();
            }
        }
        let mut cfg: BotConfig = serde_json::from_value(merged)?;
        // A model-only patch keeps this bot's provider even when another lists the same name
        // (OpenBot 0.78.0). Null explicitly clears the model; a cross-provider pick keeps the old one.
        if patch.get("model").is_some() && patch.get("backend").is_none() && patch.get("command").is_none() {
            if patch["model"].is_null() {
                cfg.model = None;
            } else if let Some(model) =
                model_change(&row.config.backend, patch["model"].as_str(), &available_models(self, &row.config.backend))
            {
                cfg.model = Some(model);
            } else {
                cfg.model.clone_from(&row.config.model);
            }
        }
        if cfg.name.trim().is_empty() {
            cfg.name.clone_from(&row.config.name);
        }
        // A name the user picked ends automatic naming; the namer passes `autoName` itself.
        if cfg.name != row.config.name && patch.get("autoName").is_none() {
            cfg.auto_name = false;
        }
        // Options belong to one model; an app that doesn't know them can't clear them itself.
        if (cfg.model != row.config.model || cfg.backend != row.config.backend) && patch.get("modelOptions").is_none() {
            cfg.model_options.clear();
        }
        if cfg.kind != row.config.kind {
            bail!("a bot can't become a group, or a group a bot");
        }
        if crate::external::is_external(&row.config) && cfg.backend != row.config.backend {
            bail!("this bot runs in your terminal: its agent can't change");
        }
        if cfg.is_group() {
            cfg.members = crate::chat::group::valid_members(&self.store, &cfg.id, &cfg.members)?;
        }
        crate::agent::fallback::tidy(&mut cfg);
        validate(&cfg)?;
        let mut edit = crate::chat::instruction_history::begin(&row.config, &cfg, patch, &self.store)?;
        crate::agent::workspace::prepare(&mut cfg)?;
        crate::agent::own_memory::settle(&mut cfg, &row.config, patch);
        if let Some(edit) = &mut edit {
            edit.record(&self.store, &row.config, &cfg)?;
        }
        let saved = {
            let _connectors = self.store.connector_lock.locked();
            let _g = self.emit_lock.locked();
            self.save_bot_access(&id, &row.config, &cfg)
        };
        if let Err(error) = saved {
            if let Some(edit) = edit {
                edit.undo(&self.store)?;
            }
            return Err(error);
        }
        crate::network::publish(self);
        if !cfg.is_group() {
            if crate::chat::memory::shared(&cfg) != crate::chat::memory::shared(&before) {
                crate::chat::memory::changed(self, Some(&id));
            }
            // A terminal session has no actor to reconfigure: its settings are read on the next event.
            if !crate::external::is_external(&cfg) {
                self.send_cmd(&id, Cmd::Reconfigure(Box::new(cfg)))?;
            }
        }
        self.emit_bot(&id);
        crate::usage::bots_changed(self);
        Ok(self.bot_value(&id).unwrap_or(Value::Null))
    }

    /// Saves `cfg`, first revoking the connectors and computer access it dropped. Called under the
    /// connector and emit locks.
    fn save_bot_access(&self, id: &str, before: &BotConfig, cfg: &BotConfig) -> Result<()> {
        if before.computer && !cfg.computer {
            self.screen.forget_bot_snapshots(id);
        }
        for connector in &before.connectors {
            if !cfg.connectors.contains(connector) {
                self.store.revoke_mcp_connector(Some(id), connector)?;
            }
        }
        self.store.save_bot(cfg)?;
        self.computer.changed(id, before, cfg);
        if before.computer_permissions.browser && !cfg.computer_permissions.browser {
            self.screen.forget_bot_snapshots(id);
        }
        Ok(())
    }

    pub fn delete_bot(&self, id: &str) -> Result<()> {
        crate::chat::lead::may_delete(&self.store, id)?;
        // A terminal session leaves with its bot, and a hook waiting on one of its cards is freed at once.
        self.external.forget(self, id);
        self.computer.revoke_all(id);
        self.screen.forget_bot_snapshots(id);
        crate::routines::flow::forget_bot(self, id);
        self.routines.disable_bot(&self.store, id)?;
        // A locked vault keeps them; Credentials lists them under "A removed bot" to remove.
        if let Err(error) = crate::market::bot_secrets::remove_bot(&self.store, id) {
            tracing::warn!(bot = id, error = format!("{error:#}"), "couldn't forget a deleted bot's secrets");
        }
        self.team.cancel_bot(id);
        crate::chat::group::stop(self, id);
        let _ = self.send_cmd(id, Cmd::Shutdown);
        self.bots.locked().remove(id);
        self.runtime.locked().remove(id);
        self.dev_servers.forget_bot(&self.store, id);
        {
            let _g = self.emit_lock.locked();
            let rev = self.store.delete_bot(id)?;
            self.computer.revoke_all(id);
            let _ =
                self.events.send(json!({"type": "bot", "rev": rev, "bot": {"id": id, "deleted": true, "rev": rev}}));
        }
        crate::network::publish(self);
        // Its shared files and artifacts are unreachable once the bot is deleted.
        let folders = [crate::chat::files::root(id), crate::chat::artifacts::root(id)];
        let bot = id.to_owned();
        tokio::task::spawn_blocking(move || {
            crate::chat::memory::files::remove_bot(&bot);
            for folder in folders {
                if let Err(error) = std::fs::remove_dir_all(&folder)
                    && error.kind() != std::io::ErrorKind::NotFound
                {
                    tracing::warn!(error = %error, "couldn't remove a deleted bot's shared files or artifacts");
                }
            }
        });
        crate::usage::bots_changed(self);
        self.leave_groups(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_model_only_change_keeps_the_bots_provider() {
        let listed =
            |pairs: &[(&str, &str)]| pairs.iter().map(|(p, m)| ((*p).to_owned(), (*m).to_owned())).collect::<Vec<_>>();
        let both = listed(&[("codex", "gpt-5"), ("cursor", "gpt-5")]);
        assert_eq!(model_change("codex", Some("gpt-5"), &both).as_deref(), Some("gpt-5"));
        let other = listed(&[("cursor", "gpt-5")]);
        assert_eq!(model_change("codex", Some("gpt-5"), &other), None, "another provider's model is ignored");
        assert_eq!(model_change("codex", Some("gpt-5"), &[]).as_deref(), Some("gpt-5"), "no catalog to contradict");
        assert_eq!(model_change("codex", None, &both), None, "a blank name is not a change");
        assert_eq!(model_change("codex", Some(" gpt-5 "), &both).as_deref(), Some("gpt-5"));
    }
}
