use super::{Artifact, ArtifactKind, card_text, content_path, get, put, read, staged_key, staged_path, title};
use crate::hub::Hub;
use crate::store::now_ms;
use anyhow::{Context as _, Result, anyhow, bail, ensure};
use serde_json::{Value, json};
use std::sync::Arc;

/// The apps' artifact methods: `readArtifact {botId, artifactId, offset}`, `renameArtifact {botId, artifactId,
/// title}` and `saveArtifactScene {botId, artifactId, scene, sha256?}` (a diagram). The list is the chat's Library
/// (`chat::library`).
pub async fn api(hub: &Arc<Hub>, method: &str, b: &Value) -> Result<Value> {
    let bot = b["botId"].as_str().ok_or_else(|| anyhow!("`botId` is required"))?;
    ensure!(hub.store.bot(bot)?.is_some_and(|row| !row.deleted), "unknown bot");
    let id = b["artifactId"].as_str().ok_or_else(|| anyhow!("`artifactId` is required"))?;
    let mut artifact = get(&hub.store, bot, id).ok_or_else(|| anyhow!("unknown artifact"))?;
    ensure!(artifact.is_public(), "artifact is not yet published");
    match method {
        "readArtifact" => {
            let offset = b["offset"].as_u64().ok_or_else(|| anyhow!("offset must be a nonnegative integer"))?;
            let path = content_path(bot, id, artifact.kind)?;
            let size = artifact.size;
            let data = tokio::task::spawn_blocking(move || read(&path, size, offset)).await??;
            Ok(
                json!({"data": base64::engine::general_purpose::STANDARD.encode(data), "size": size, "artifact": artifact}),
            )
        }
        "saveArtifactScene" => {
            ensure!(artifact.kind == ArtifactKind::Diagram, "only a diagram has a scene");
            let scene = b["scene"].clone();
            crate::chat::diagrams::validate(&scene)?;
            if let Some(expected) = b["sha256"].as_str().filter(|sha| !sha.is_empty()) {
                ensure!(expected == artifact.sha256, "the diagram changed elsewhere; read it again");
            }
            let path = content_path(bot, id, ArtifactKind::Diagram)?;
            // The person's edit is the version they see; a scheduled replacement waiting aside goes.
            if let Ok(staged) = staged_path(bot, id, ArtifactKind::Diagram) {
                let _ = std::fs::remove_file(staged);
            }
            let _ = hub.store.kv_delete(&staged_key(bot, id));
            let (size, sha256) =
                tokio::task::spawn_blocking(move || crate::chat::diagrams::write(&path, &scene)).await??;
            artifact.size = size;
            artifact.sha256 = sha256;
            artifact.updated_at = now_ms();
            put(hub, &mut artifact)?;
            Ok(json!({"artifact": artifact}))
        }
        "renameArtifact" => {
            artifact.title = title(b["title"].as_str().unwrap_or_default())?;
            put(hub, &mut artifact)?;
            retitle_card(hub, &artifact);
            Ok(json!({"artifact": artifact}))
        }
        _ => bail!("unknown artifact method: {method}"),
    }
}

/// The artifact's newest card takes the new title too: older apps and a cache that never saw the rename show the card's.
fn retitle_card(hub: &Hub, artifact: &Artifact) {
    let Some(id) = artifact.entry_id.as_deref() else { return };
    let retitled = hub.edit_entry(id, |card| {
        if card.data["artifact"]["id"] != artifact.id.as_str() {
            return Ok(false);
        }
        card.data["text"] = card_text(&artifact.title).into();
        card.data["artifact"]["title"] = artifact.title.clone().into();
        Ok(true)
    });
    if let Err(e) = retitled {
        tracing::info!(artifact = artifact.id, error = format!("{e:#}"), "couldn't retitle the artifact's card");
    }
}
