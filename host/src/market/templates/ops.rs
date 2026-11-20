//! Host operations: save from a bot, list, read, preview, edit, share and install.

use super::{
    DEFAULT_NAME, MAX_ITEMS, MAX_NAME, MAX_SUMMARY, VisibilityChange, avatar, checked_id, clean_instructions,
    clean_line, detail, new_template_id, publication, safe_catalog_id, safe_ids, summary, transition,
};
use crate::hub::Hub;
use crate::market;
use crate::store::{AgentTemplate, BotConfig, Store, TemplateVisibility, now_ms};
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::sync::Arc;

fn arg<'a>(b: &'a Value, key: &str) -> Result<&'a str> {
    b[key].as_str().with_context(|| format!("`{key}` is required"))
}

/// Builds a template from an existing bot. Only setup fields are read; `cwd`,
/// `command`, credentials, accounts, avatars and permissions stay with the bot.
fn from_bot(cfg: &BotConfig, summary: Option<&str>, existing: Option<&AgentTemplate>, now: i64) -> AgentTemplate {
    let (id, created_at, visibility, published_at) = match existing {
        Some(template) => (template.id.clone(), template.created_at, template.visibility, template.published_at),
        None => (new_template_id(), now, TemplateVisibility::Local, None),
    };
    let name = clean_line(&cfg.name, MAX_NAME);
    let (color, shape) = avatar(Some(&cfg.avatar_color), Some(&cfg.avatar_shape));
    AgentTemplate {
        id,
        name: if name.is_empty() { DEFAULT_NAME.into() } else { name },
        description: match summary {
            Some(text) => clean_line(text, MAX_SUMMARY),
            None => clean_line(&cfg.description, MAX_SUMMARY),
        },
        instructions: clean_instructions(&cfg.description),
        avatar_color: color,
        avatar_shape: shape,
        backend: safe_catalog_id(Some(&cfg.backend)),
        model: safe_catalog_id(cfg.model.as_deref()),
        skills: safe_ids(&json!(cfg.skills), MAX_ITEMS),
        connectors: safe_ids(&json!(cfg.connectors), MAX_ITEMS),
        visibility,
        created_at,
        updated_at: now,
        published_at,
    }
}

/// `templateSave`: saves (or refreshes) a template from `botId`. `templateId`
/// updates an existing one; `description` overrides the short summary.
pub fn save(hub: &Arc<Hub>, b: &Value) -> Result<Value> {
    let bot = arg(b, "botId")?;
    let row = hub.store.bot(bot)?.filter(|r| !r.deleted).context("unknown bot")?;
    let existing = match b["templateId"].as_str() {
        Some(id) => {
            let id = checked_id(id)?;
            Some(
                hub.store
                    .templates()?
                    .into_iter()
                    .find(|template| template.id == id)
                    .context("unknown agent template")?,
            )
        }
        None => None,
    };
    let template = from_bot(&row.config, b["description"].as_str(), existing.as_ref(), now_ms());
    hub.store.save_template(&template)?;
    Ok(detail(&template))
}

/// `templateList`: newest save last, without the instructions or gear ids.
pub fn list(store: &Store) -> Result<Value> {
    let items = store.templates()?.iter().map(summary).collect::<Vec<_>>();
    Ok(json!({"items": items}))
}

/// Finds a template by id, or by a share ref (which must be published).
fn resolve(store: &Store, b: &Value) -> Result<AgentTemplate> {
    let (id, by_ref) = if let Some(reference) = b["ref"].as_str() {
        (super::parse_template_ref(reference)?, true)
    } else if let Some(id) = b["templateId"].as_str() {
        (checked_id(id)?, false)
    } else {
        bail!("`templateId` or `ref` is required")
    };
    let template =
        store.templates()?.into_iter().find(|template| template.id == id).context("unknown agent template")?;
    if by_ref && template.visibility != TemplateVisibility::Published {
        bail!("that agent template isn't published");
    }
    Ok(template)
}

/// `templateGet`: the full detail of a saved template.
pub fn get(store: &Store, b: &Value) -> Result<Value> {
    Ok(detail(&resolve(store, b)?))
}

/// `templatePreview`: sanitized setup of a bot before saving, or of a saved
/// template opened from a share link.
pub fn preview(hub: &Arc<Hub>, b: &Value) -> Result<Value> {
    if let Some(bot) = b["botId"].as_str() {
        let row = hub.store.bot(bot)?.filter(|r| !r.deleted).context("unknown bot")?;
        let template = from_bot(&row.config, b["description"].as_str(), None, now_ms());
        let mut value = detail(&template);
        if let Some(map) = value.as_object_mut() {
            map.remove("id"); // a preview isn't saved yet, so it has no share id
        }
        value["botId"] = json!(bot);
        return Ok(value);
    }
    Ok(detail(&resolve(&hub.store, b)?))
}

/// `templateRename`.
pub fn rename(store: &Store, b: &Value) -> Result<Value> {
    let id = checked_id(arg(b, "templateId")?)?;
    let name = clean_line(arg(b, "name")?, MAX_NAME);
    if name.is_empty() {
        bail!("a template needs a name");
    }
    let mut all = store.templates()?;
    let template = all.iter_mut().find(|template| template.id == id).context("unknown agent template")?;
    template.name = name;
    template.updated_at = now_ms();
    let answer = detail(template);
    store.save_template(template)?;
    Ok(answer)
}

/// `templateDelete`.
pub fn delete(store: &Store, b: &Value) -> Result<Value> {
    let id = checked_id(arg(b, "templateId")?)?;
    Ok(json!({"deleted": store.remove_template(&id)?}))
}

/// `templatePublish`: makes a template reachable by its share link.
pub fn publish(store: &Store, b: &Value) -> Result<Value> {
    let id = checked_id(arg(b, "templateId")?)?;
    let mut all = store.templates()?;
    let template = all.iter_mut().find(|template| template.id == id).context("unknown agent template")?;
    let now = now_ms();
    template.visibility = transition(template.visibility, VisibilityChange::Publish);
    if template.published_at.is_none() {
        template.published_at = Some(now);
    }
    template.updated_at = now;
    let answer = publication(template);
    store.save_template(template)?;
    Ok(answer)
}

/// `templateUnpublish`: takes the share link away. The template stays local.
pub fn unpublish(store: &Store, b: &Value) -> Result<Value> {
    let id = checked_id(arg(b, "templateId")?)?;
    let mut all = store.templates()?;
    let template = all.iter_mut().find(|template| template.id == id).context("unknown agent template")?;
    template.visibility = transition(template.visibility, VisibilityChange::Unpublish);
    template.published_at = None;
    template.updated_at = now_ms();
    let answer = json!({"templateId": template.id.clone(), "visibility": template.visibility});
    store.save_template(template)?;
    Ok(answer)
}

/// The `createBot` body made from a template: a different name, and nothing
/// that belongs to the source computer. Gear ids that aren't installed here
/// are left out (the caller supplies the installed ids).
fn draft(template: &AgentTemplate, skills: &[String], connectors: &[String], taken: &[String]) -> Value {
    let keep = |ids: &[String], available: &[String]| -> Vec<String> {
        ids.iter().filter(|id| available.contains(id)).cloned().collect()
    };
    json!({
        "name": unique_name(taken, &template.name),
        "description": template.instructions,
        "avatarColor": template.avatar_color,
        "avatarShape": template.avatar_shape,
        "backend": template.backend,
        "model": template.model,
        "skills": keep(&template.skills, skills),
        "connectors": keep(&template.connectors, connectors),
    })
}

fn unique_name(taken: &[String], name: &str) -> String {
    let base = format!("{} (from template)", name.trim());
    let base = if base.trim() == "(from template)" { format!("{DEFAULT_NAME} (from template)") } else { base };
    if !taken.iter().any(|name| name == &base) {
        return base;
    }
    for n in 2..1000 {
        let candidate = format!("{base} ({n})");
        if !taken.iter().any(|name| name == &candidate) {
            return candidate;
        }
    }
    format!("{base} (copy)")
}

/// `templateInstall`: creates a real new bot through the normal bot-create path.
pub async fn install(hub: &Arc<Hub>, b: &Value) -> Result<Value> {
    let template = resolve(&hub.store, b)?;
    let skills = market::skills(&hub.store).into_iter().map(|skill| skill.id).collect::<Vec<_>>();
    // Connector ids live behind the credential store. Mirror `new_bot`'s rule:
    // nothing saved keeps the vault shut, and an unavailable one costs the new
    // bot its connectors rather than failing the install.
    let connectors = if market::connectors_saved(&hub.store)? {
        match market::vault::unlock(hub.clone()).await {
            Ok(()) => market::connector_ids(&hub.store)?,
            Err(e) => {
                tracing::warn!(
                    error = format!("{e:#}"),
                    "credential store unavailable; installed template starts without connectors"
                );
                Vec::new()
            }
        }
    } else {
        Vec::new()
    };
    let taken = hub.store.bots()?.into_iter().map(|bot| bot.config.name).collect::<Vec<_>>();
    let body = draft(&template, &skills, &connectors, &taken);
    // `with_defaults` fills in the workspace, permissions and the computer's
    // own defaults; no path, account or credential comes from the template.
    let config = crate::agent::new_bot::with_defaults(hub, body).await?;
    let hub = hub.clone();
    tokio::task::spawn_blocking(move || hub.create_bot(config)).await?
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn saving_from_a_bot_keeps_no_credentials_paths_or_avatars() {
        let raw = json!({
            "id": "bot", "name": "Research", "description": "API_KEY=sk-secret\nRead carefully",
            "backend": "claude", "model": "claude-sonnet", "avatarColor": "green", "avatarShape": "hex",
            "cwd": "C:\\Users\\Ada\\work", "command": "/bin/sh -c x", "account": "ada@example.com",
            "avatarPhoto": "data:image/png;base64,AAAA", "connectors": ["linear", "../etc/passwd"],
            "skills": ["pdf"], "env": {"TOKEN": "abc"}, "permission": "auto"
        });
        let cfg: BotConfig = serde_json::from_value(raw).unwrap();
        let template = from_bot(&cfg, None, None, 100);
        let value = serde_json::to_value(&template).unwrap().to_string();
        for leak in [
            "sk-secret",
            r"C:\Users",
            "/bin/sh",
            "ada@example.com",
            "avatarPhoto",
            "avatar_photo",
            "TOKEN",
            "cwd",
            "command",
            "account",
            "env",
        ] {
            assert!(!value.contains(leak), " leaked {leak}: {value}");
        }
        assert_eq!(template.backend.as_deref(), Some("claude"));
        assert_eq!(template.skills, ["pdf"]);
        assert_eq!(template.connectors, ["linear"]);
        assert_eq!(template.visibility, TemplateVisibility::Local);
        assert!(super::is_template_id(&template.id));
    }

    #[test]
    fn an_install_draft_has_a_different_name_and_no_computer_state() {
        let template = AgentTemplate {
            id: "AbCdEf0123456789_xYz-0".into(),
            name: "Research".into(),
            description: "Finds things".into(),
            instructions: "Read carefully".into(),
            avatar_color: "blue".into(),
            avatar_shape: "blob".into(),
            backend: Some("claude".into()),
            model: None,
            skills: vec!["pdf".into(), "../etc/passwd".into()],
            connectors: vec!["linear".into()],
            visibility: TemplateVisibility::Local,
            created_at: 1,
            updated_at: 2,
            published_at: None,
        };
        let taken = vec![format!("{} (from template)", template.name)];
        let body = draft(&template, &["pdf".into()], &["linear".into()], &taken);
        assert_eq!(body["name"], format!("{} (from template) (2)", template.name));
        assert_eq!(body["description"], template.instructions);
        assert_eq!(body["backend"], "claude");
        assert_eq!(body["skills"], json!(["pdf"]), "uninstalled gear is left out");
        assert_eq!(body["connectors"], json!(["linear"]));
        for forbidden in ["cwd", "command", "account", "env", "headers", "avatarPhoto", "personality", "browserProfile"]
        {
            assert!(body.get(forbidden).is_none(), "{forbidden} must not travel with a template");
        }
    }
}
