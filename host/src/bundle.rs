//! Portable bots: a bot's setup (name, instructions, agent, model, fallbacks, permission) as one JSON file to back
//! up or give to someone, and bots made from such a file. Nothing tied to this computer travels: no folder, account,
//! command, connector or skill ids, history or secrets. The person importing picks the folder.
//!
//! Idea from mercury-agent's `bundle.ts` (MIT); no code copied.

use crate::hub::Hub;
use crate::store::{BotConfig, BotKind, Fallback, Permission};
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::Arc;

const FORMAT: &str = "codync-bot-bundle";
const VERSION: u32 = 1;
/// Bots in one file; a bundle is a handful of bots, not an archive.
const MAX_BOTS: usize = 50;

/// What of a bot travels.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct Portable {
    name: String,
    /// The bot's instructions.
    #[serde(default)]
    description: String,
    #[serde(default)]
    avatar_color: Option<String>,
    #[serde(default)]
    avatar_shape: Option<String>,
    #[serde(default)]
    backend: String,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    model_options: BTreeMap<String, String>,
    #[serde(default)]
    fast_mode: bool,
    #[serde(default)]
    permission: Permission,
    /// Fallback agents by name only: their commands are paths on the exporting computer.
    #[serde(default)]
    fallbacks: Vec<Fallback>,
    #[serde(default)]
    self_fallbacks: bool,
}

impl Portable {
    fn of(bot: &BotConfig) -> Self {
        Self {
            name: bot.name.clone(),
            description: bot.description.clone(),
            avatar_color: Some(bot.avatar_color.clone()),
            avatar_shape: Some(bot.avatar_shape.clone()),
            backend: bot.backend.clone(),
            model: bot.model.clone(),
            model_options: bot.model_options.clone(),
            fast_mode: bot.fast_mode,
            permission: bot.permission,
            fallbacks: bot.fallbacks.iter().map(|f| Fallback { command: None, ..f.clone() }).collect(),
            self_fallbacks: bot.self_fallbacks,
        }
    }

    /// The body `createBot` takes, for a bot working in `cwd`.
    fn draft(&self, cwd: &str) -> Result<Value> {
        let mut body = serde_json::to_value(self)?;
        body["cwd"] = cwd.into();
        // Unset colours and shapes fall to the host's defaults.
        for key in ["avatarColor", "avatarShape"] {
            if body[key].is_null() {
                body.as_object_mut().map(|o| o.remove(key));
            }
        }
        Ok(body)
    }
}

/// A bundle of `bots` (agent bots only: a group's members are other bots, not part of one).
pub fn export(bots: &[BotConfig]) -> Result<Value> {
    let portable: Vec<Portable> = bots.iter().filter(|b| b.kind == BotKind::Agent).map(Portable::of).collect();
    if portable.is_empty() {
        bail!("There is no bot to export");
    }
    Ok(json!({"format": FORMAT, "version": VERSION, "bots": portable}))
}

/// The bots a bundle holds, as `createBot` bodies for `cwd`. Refuses a file that isn't a bundle or is too new.
fn drafts(bundle: &Value, cwd: &str) -> Result<Vec<Value>> {
    if bundle["format"] != FORMAT {
        bail!("This isn't a Codync bot bundle");
    }
    if bundle["version"].as_u64().is_none_or(|v| v > u64::from(VERSION)) {
        bail!("This bundle is from a newer Codync; update to import it");
    }
    let bots: Vec<Portable> = serde_json::from_value(bundle["bots"].clone())
        .map_err(|e| anyhow::anyhow!("The bundle's bots are unreadable: {e}"))?;
    if bots.is_empty() || bots.len() > MAX_BOTS {
        bail!("A bundle holds 1 to {MAX_BOTS} bots");
    }
    if cwd.trim().is_empty() {
        bail!("Choose the folder the imported bots work in");
    }
    bots.iter().map(|b| b.draft(cwd)).collect()
}

/// Makes the bots of `bundle` in `cwd`; answers the bots created.
pub async fn import(hub: &Arc<Hub>, bundle: &Value, cwd: &str) -> Result<Value> {
    let mut created = Vec::new();
    for body in drafts(bundle, cwd)? {
        let cfg = crate::agent::new_bot::with_defaults(hub, body).await?;
        let hub = hub.clone();
        created.push(tokio::task::spawn_blocking(move || hub.create_bot(cfg)).await??);
    }
    Ok(json!({"bots": created}))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bot(extra: Value) -> BotConfig {
        let mut base = json!({
            "id": "secret-id", "name": "Reviewer", "description": "Review diffs.", "backend": "claude",
            "cwd": "C:/Users/me/private", "command": "C:/bin/agent.exe", "account": "work",
            "connectors": ["mcp-1"], "skills": ["skill-1"], "permission": "ask",
            "fallbacks": [{"backend": "codex", "command": "C:/bin/codex.exe"}],
        });
        base.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        serde_json::from_value(base).unwrap()
    }

    #[test]
    fn nothing_tied_to_this_computer_is_exported() {
        let text = export(&[bot(json!({}))]).unwrap().to_string();
        for private in ["secret-id", "C:/Users/me/private", "agent.exe", "codex.exe", "work", "mcp-1", "skill-1"] {
            assert!(!text.contains(private), "{private} leaked");
        }
        assert!(text.contains("Review diffs.") && text.contains("codex"));
    }

    #[test]
    fn groups_are_not_exported() {
        let group = bot(json!({"kind": "group", "backend": "", "members": ["a", "b"]}));
        assert!(export(&[group]).is_err());
    }

    #[test]
    fn a_bundle_comes_back_as_bots_for_the_chosen_folder() {
        let bundle = export(&[bot(json!({}))]).unwrap();
        let made = drafts(&bundle, "D:/work").unwrap();
        assert_eq!(made.len(), 1);
        assert_eq!(made[0]["cwd"], "D:/work");
        assert_eq!(made[0]["name"], "Reviewer");
        assert_eq!(made[0]["permission"], "ask");
        assert!(made[0].get("id").is_none());
    }

    #[test]
    fn foreign_new_and_empty_bundles_are_refused() {
        let good = export(&[bot(json!({}))]).unwrap();
        assert!(drafts(&json!({"format": "other", "version": 1, "bots": []}), "D:/w").is_err());
        let mut newer = good.clone();
        newer["version"] = json!(VERSION + 1);
        assert!(drafts(&newer, "D:/w").is_err());
        assert!(drafts(&json!({"format": FORMAT, "version": 1, "bots": []}), "D:/w").is_err());
        assert!(drafts(&good, "  ").is_err());
    }
}
