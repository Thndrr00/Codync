//! The template's wire forms: checked import, sanitized export, client values
//! and the publish/unpublish state machine.

use super::{
    MAX_INSTRUCTIONS, MAX_ITEMS, MAX_NAME, MAX_SUMMARY, avatar, checked_id, clean_instructions, clean_line,
    new_template_id, positive_or, redact, safe_catalog_id, safe_ids, share_url,
};
use crate::store::{AgentTemplate, TemplateVisibility};
use anyhow::{Result, bail};
use serde_json::{Value, json};

/// Checks a bounded text field before cleaning it: an oversized field is refused,
/// never silently cut short.
fn checked_text(value: Option<&str>, max: usize, what: &str) -> Result<String> {
    let raw = value.unwrap_or_default();
    if raw.chars().count() > max {
        bail!("{what} is too long");
    }
    Ok(clean_line(raw, max))
}

/// Checks an incoming (untrusted) template: one known version, bounded fields,
/// safe ids, and no visibility from the sender. Bad skills are skipped, not fatal.
pub fn import_template(value: &Value, now: i64) -> Result<AgentTemplate> {
    if value["version"].as_u64() != Some(1) {
        bail!("unknown agent template version");
    }
    let id = match value["id"].as_str() {
        Some(id) => checked_id(id)?,
        None => new_template_id(),
    };
    let name = checked_text(value["name"].as_str(), MAX_NAME, "agent template name")?;
    if name.is_empty() {
        bail!("an agent template needs a name");
    }
    let raw_instructions = value["instructions"].as_str().unwrap_or_default();
    if raw_instructions.chars().count() > MAX_INSTRUCTIONS {
        bail!("agent template instructions are too long");
    }
    let instructions = clean_instructions(raw_instructions);
    if instructions.trim().is_empty() {
        bail!("an agent template needs instructions");
    }
    let (color, shape) = avatar(value["avatarColor"].as_str(), value["avatarShape"].as_str());
    Ok(AgentTemplate {
        id,
        name,
        description: checked_text(value["description"].as_str(), MAX_SUMMARY, "agent template description")?,
        instructions,
        avatar_color: color,
        avatar_shape: shape,
        backend: safe_catalog_id(value["backend"].as_str()),
        model: safe_catalog_id(value["model"].as_str()),
        skills: safe_ids(&value["skills"], MAX_ITEMS),
        connectors: safe_ids(&value["connectors"], MAX_ITEMS),
        // An import is local until this computer's owner publishes it.
        visibility: TemplateVisibility::Local,
        created_at: positive_or(value["createdAt"].as_i64(), now),
        updated_at: positive_or(value["updatedAt"].as_i64(), now),
        published_at: None,
    })
}

/// The portable, sanitized form of a saved template.
pub fn export_template(template: &AgentTemplate) -> Value {
    json!({
        "version": 1,
        "id": template.id,
        "name": redact(&template.name),
        "description": redact(&template.description),
        "instructions": redact(&template.instructions),
        "avatarColor": template.avatar_color,
        "avatarShape": template.avatar_shape,
        "backend": template.backend,
        "model": template.model,
        "skills": template.skills,
        "connectors": template.connectors,
    })
}

/// The share link of a published template, or null while it is local.
pub fn publication(template: &AgentTemplate) -> Value {
    match (template.visibility, template.published_at) {
        (TemplateVisibility::Published, Some(at)) => json!({
            "templateId": template.id,
            "shareUrl": share_url(&template.id),
            "publishedAt": at,
        }),
        _ => Value::Null,
    }
}

/// What a client shows when it opens one template.
pub fn detail(template: &AgentTemplate) -> Value {
    json!({
        "id": template.id,
        "name": template.name,
        "description": template.description,
        "instructions": template.instructions,
        "avatarColor": template.avatar_color,
        "avatarShape": template.avatar_shape,
        "backend": template.backend,
        "model": template.model,
        "skills": template.skills,
        "connectors": template.connectors,
        "visibility": template.visibility,
        "createdAt": template.created_at,
        "updatedAt": template.updated_at,
        "publication": publication(template),
    })
}

/// The library list: no instructions, no gear ids and no content.
pub fn summary(template: &AgentTemplate) -> Value {
    json!({
        "id": template.id,
        "name": template.name,
        "description": template.description,
        "visibility": template.visibility,
        "updatedAt": template.updated_at,
        "publication": publication(template),
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VisibilityChange {
    Publish,
    Unpublish,
}

/// Both transitions are idempotent: repeating one leaves the same state, which
/// is what a retried call from a phone should do.
pub fn transition(current: TemplateVisibility, change: VisibilityChange) -> TemplateVisibility {
    match (current, change) {
        (TemplateVisibility::Local, VisibilityChange::Publish)
        | (TemplateVisibility::Published, VisibilityChange::Publish) => TemplateVisibility::Published,
        (TemplateVisibility::Published, VisibilityChange::Unpublish)
        | (TemplateVisibility::Local, VisibilityChange::Unpublish) => TemplateVisibility::Local,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::store::TemplateVisibility::Published;

    const ID: &str = "AbCdEf0123456789_xYz-0";

    fn sample() -> AgentTemplate {
        AgentTemplate {
            id: ID.into(),
            name: "Research".into(),
            description: "Finds things".into(),
            instructions: "Read carefully".into(),
            avatar_color: "blue".into(),
            avatar_shape: "blob".into(),
            backend: Some("claude".into()),
            model: Some("claude-sonnet".into()),
            skills: vec!["pdf".into(), "../etc/passwd".into(), "pdf".into()],
            connectors: vec!["linear".into()],
            visibility: TemplateVisibility::Local,
            created_at: 10,
            updated_at: 20,
            published_at: None,
        }
    }

    #[test]
    fn export_sanitizes_even_a_saved_template() {
        let mut template = sample();
        template.instructions = "API_KEY=sk-live-abcdefghijklmnop\nRead carefully".into();
        let exported = export_template(&template);
        assert_eq!(exported["version"], 1);
        assert_eq!(exported["name"], "Research");
        assert_eq!(exported["skills"][0], "pdf");
        assert!(exported.to_string().contains("Read carefully"));
        assert!(!exported.to_string().contains("sk-live-abcdefghijklmnop"));
    }

    #[test]
    fn import_rejects_unknown_versions_and_oversized_fields_and_skips_bad_skills() {
        assert!(import_template(&json!({"version": 2, "name": "x", "instructions": "y"}), 1).is_err());
        assert!(import_template(&json!({"name": "x", "instructions": "y"}), 1).is_err());
        assert!(import_template(&json!({"version": 1, "instructions": "y"}), 1).is_err());
        assert!(import_template(&json!({"version": 1, "name": "x", "instructions": ""}), 1).is_err());
        assert!(
            import_template(&json!({"version": 1, "name": "x".repeat(MAX_NAME + 1), "instructions": "y"}), 1).is_err()
        );
        assert!(
            import_template(&json!({"version": 1, "name": "x", "instructions": "y".repeat(MAX_INSTRUCTIONS + 1)}), 1)
                .is_err()
        );
        assert!(import_template(&json!({"version": 1, "id": "../bad", "name": "x", "instructions": "y"}), 1).is_err());

        let raw = json!({
            "version": 1,
            "id": ID,
            "name": "  Research  ",
            "description": "Finds things",
            "instructions": "Read carefully",
            "avatarColor": "neon",
            "avatarShape": "ring",
            "backend": "../bin/sh",
            "model": "claude-sonnet-4",
            "skills": ["pdf", "../etc/passwd", "pdf", "linear", 7],
            "connectors": ["linear", "A/B", "linear"],
            "visibility": "published",
            "publishedAt": 123
        });
        let imported = import_template(&raw, 99).unwrap();
        assert_eq!(imported.name, "Research");
        assert_eq!(imported.skills, ["pdf", "linear"], "invalid and duplicate skills are skipped");
        assert_eq!(imported.connectors, ["linear"]);
        assert_eq!(imported.avatar_color, "blue", "unknown looks fall back");
        assert_eq!(imported.avatar_shape, "blob");
        assert_eq!(imported.backend, None, "a path is never a catalog id");
        assert_eq!(imported.model.as_deref(), Some("claude-sonnet-4"));
        assert_eq!(imported.visibility, TemplateVisibility::Local, "imports never inherit visibility");
        assert_eq!(imported.published_at, None);
        assert_eq!(imported.created_at, 99);
    }

    #[test]
    fn publish_and_unpublish_are_idempotent() {
        let local = TemplateVisibility::Local;
        assert_eq!(transition(local, VisibilityChange::Publish), Published);
        assert_eq!(transition(Published, VisibilityChange::Publish), Published, "a retry is harmless");
        assert_eq!(transition(Published, VisibilityChange::Unpublish), local);
        assert_eq!(transition(local, VisibilityChange::Unpublish), local, "a retry is harmless");
        assert_eq!(publication(&sample()), Value::Null);
        let mut shared = sample();
        shared.visibility = Published;
        shared.published_at = Some(42);
        assert_eq!(publication(&shared)["shareUrl"], share_url(ID));
        assert_eq!(publication(&shared)["publishedAt"], 42);
    }
}
