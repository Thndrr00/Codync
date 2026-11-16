//! Artifacts, Grok Bot's way: a self-contained HTML document a bot publishes for the user to come back to (a report,
//! a tracker, a dashboard). The host keeps each artifact's latest HTML and title; publishing again with its id replaces
//! it, and the apps show the newest. A publish in the bot's own chat also posts an artifact card there, and the user's
//! replies to that card reach the bot as messages about the artifact (`chat::replies`).

use crate::agent::bot::Cmd;
use crate::chat::files;
use crate::hub::Hub;
use crate::store::{Store, now_ms};
use anyhow::{Context as _, Result, anyhow, bail, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::oneshot;

/// Largest artifact, in bytes (images ride along as data URLs).
pub const MAX_ARTIFACT: u64 = 5 * 1024 * 1024;
const MAX_TITLE: usize = 200;

/// The `chat` server's artifact tools (when to use them: `outbox::INSTRUCTIONS`).
pub const TOOLS: [&str; 3] = ["publish_artifact", "publish_diagram", "list_artifacts"];

/// What an artifact holds: a self-contained HTML page, or an editable diagram scene.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ArtifactKind {
    #[default]
    Html,
    Diagram,
}

impl ArtifactKind {
    fn extension(self) -> &'static str {
        match self {
            Self::Html => "html",
            Self::Diagram => "scene.json",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Artifact {
    pub id: String,
    pub bot_id: String,
    pub title: String,
    #[serde(default)]
    pub kind: ArtifactKind,
    pub size: u64,
    pub sha256: String,
    pub created_at: i64,
    pub updated_at: i64,
    /// Its newest card in the bot's chat: what a message about the artifact replies to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry_id: Option<String>,
    /// The sync rev of its last change, so a client's catch-up brings changes it missed.
    #[serde(default)]
    pub rev: i64,
    /// Not announced until the actor decides whether this turn is a private check-in.
    #[serde(default, skip_serializing_if = "announced")]
    publication: Publication,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum Publication {
    #[default]
    Public,
    Pending,
    Internal,
}

impl Publication {
    fn is_public(self) -> bool {
        self == Self::Public
    }
}

/// Serde's predicate: public metadata omits the flag on the wire, so clients keep their legacy shape.
#[allow(clippy::trivially_copy_pass_by_ref)] // serde calls it with a reference.
fn announced(publication: &Publication) -> bool {
    publication.is_public()
}

impl Artifact {
    pub(crate) fn is_public(&self) -> bool {
        self.publication.is_public()
    }

    /// An artifact card entry's data, as published: what the tests post as its card.
    #[cfg(test)]
    fn card_entry(&self) -> Value {
        card(&self.id, &self.title, self.updated_at, self.kind)
    }
}

/// An artifact card's data. A replacement's card is built from its own title and time, before that version is public.
fn card(id: &str, title: &str, updated_at: i64, kind: ArtifactKind) -> Value {
    json!({
        "text": card_text(title),
        "final": true,
        "artifact": {"id": id, "title": title, "updatedAt": updated_at, "kind": kind},
    })
}

/// An artifact card's text: what older apps show, and what a reply to the card quotes.
pub fn card_text(title: &str) -> String {
    format!("Artifact: {title}")
}

pub fn tools() -> [Value; 3] {
    [
        json!({
            "name": "publish_artifact",
            "description": "Publish an Artifact: one self-contained HTML file (up to 5 MiB) the user opens from a card in your chat and from its Library. Without artifact, it creates a new artifact (title required). With artifact (an id from an earlier publish or list_artifacts), it replaces that artifact's HTML, and its title when given. Paths are absolute or relative to your working directory.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "The HTML file."},
                    "title": {"type": "string", "description": "The artifact's title, as the user should see it."},
                    "artifact": {"type": "string", "description": "The id of the artifact to update."},
                },
                "required": ["path"],
            },
            "annotations": {"readOnlyHint": false, "idempotentHint": false},
        }),
        json!({
            "name": "list_artifacts",
            "description": "Your artifacts, newest first: id, kind, title, when each was last published, and the path of its current content (to make changes, edit a copy and publish that; never edit this file in place).",
            "inputSchema": {"type": "object", "properties": {}},
            "annotations": {"readOnlyHint": true},
        }),
        crate::chat::diagrams::tool(),
    ]
}

const KEY_PREFIX: &str = "artifact.";

fn key(bot: &str, id: &str) -> String {
    format!("{KEY_PREFIX}{bot}.{id}")
}

/// The artifacts whose details are saved under keys starting with `prefix`. Only the artifacts themselves: a staged
/// replacement lives under `<id>.staged`, which isn't one.
fn saved<'a>(store: &'a Store, prefix: &str) -> impl Iterator<Item = Artifact> + 'a {
    store
        .kv_prefix(prefix)
        .into_iter()
        .filter(|key| key.rsplit('.').next().is_some_and(|id| uuid::Uuid::parse_str(id).is_ok()))
        .filter_map(|k| serde_json::from_str(&store.kv_get(&k)?).ok())
}

/// Where a bot's artifacts live: `<id>.html` each.
pub fn root(bot: &str) -> PathBuf {
    crate::service::data_dir().join("bots").join(bot).join("artifacts")
}

/// The file holding an artifact's current content. Ids are UUIDs, so they never leave the folder:
/// `<id>.html` for an HTML artifact, `<id>.scene.json` for a diagram.
pub fn content_path(bot: &str, id: &str, kind: ArtifactKind) -> Result<PathBuf> {
    let id = uuid::Uuid::parse_str(id).context("invalid artifact id")?;
    Ok(root(bot).join(format!("{id}.{}", kind.extension())))
}

/// The file holding an HTML artifact's current page.
pub fn html_path(bot: &str, id: &str) -> Result<PathBuf> {
    content_path(bot, id, ArtifactKind::Html)
}

pub fn get(store: &Store, bot: &str, id: &str) -> Option<Artifact> {
    serde_json::from_str(&store.kv_get(&key(bot, id))?).ok()
}

/// A scheduled turn's replacement of a public artifact: its HTML waits beside the current one until the turn proves
/// useful, so a silent or failed check-in never replaces or hides what the user already has.
#[derive(Clone, Debug, Deserialize, Serialize)]
struct StagedUpdate {
    title: String,
    size: u64,
    sha256: String,
    updated_at: i64,
}

fn staged_key(bot: &str, id: &str) -> String {
    format!("{}.staged", key(bot, id))
}

/// Where a staged replacement waits, next to the published content.
fn staged_path(bot: &str, id: &str, kind: ArtifactKind) -> Result<PathBuf> {
    Ok(content_path(bot, id, kind)?.with_extension(match kind {
        ArtifactKind::Html => "staged.html",
        ArtifactKind::Diagram => "staged.scene.json",
    }))
}

fn staged(store: &Store, bot: &str, id: &str) -> Option<StagedUpdate> {
    serde_json::from_str(&store.kv_get(&staged_key(bot, id))?).ok()
}

fn save_staged(store: &Store, bot: &str, id: &str, update: &StagedUpdate) -> Result<()> {
    store.kv_set(&staged_key(bot, id), &serde_json::to_string(update)?)
}

/// Applies a staged replacement: the public metadata and HTML become the check-in's version, whole or not at all.
fn commit_staged(store: &Store, bot: &str, artifact: &mut Artifact) -> Result<bool> {
    let Some(update) = staged(store, bot, &artifact.id) else { return Ok(false) };
    let (from, to) = (staged_path(bot, &artifact.id, artifact.kind)?, content_path(bot, &artifact.id, artifact.kind)?);
    ensure!(from.exists(), "the staged artifact update is missing");
    std::fs::rename(&from, &to).context("applying the staged artifact update")?;
    artifact.title = update.title;
    artifact.size = update.size;
    artifact.sha256 = update.sha256;
    artifact.updated_at = update.updated_at;
    store.kv_delete(&staged_key(bot, &artifact.id))?;
    Ok(true)
}

/// A bot's artifacts, the most recently published first.
pub fn list(store: &Store, bot: &str) -> Vec<Artifact> {
    let mut artifacts: Vec<Artifact> = saved(store, &format!("{KEY_PREFIX}{bot}.")).collect();
    artifacts.sort_by_key(|artifact| std::cmp::Reverse(artifact.updated_at));
    artifacts
}

/// Saves an artifact's details under a new rev and tells the apps (`artifact` event; they upsert it by id).
fn put(hub: &Hub, artifact: &mut Artifact) -> Result<()> {
    if !artifact.is_public() {
        // Private metadata is durable control state, not a sync mutation. Promotion stamps its public rev.
        artifact.rev = 0;
        return save(&hub.store, artifact);
    }
    hub.emit_change(|| {
        hub.store.kv_set_stamped(&key(&artifact.bot_id, &artifact.id), |rev| {
            artifact.rev = rev;
            Ok(serde_json::to_string(artifact)?)
        })?;
        Ok(event(artifact))
    })
}

/// Saves details without a new rev or an event: nothing the apps show has changed yet.
fn save(store: &Store, artifact: &Artifact) -> Result<()> {
    store.kv_set(&key(&artifact.bot_id, &artifact.id), &serde_json::to_string(artifact)?)
}

fn event(artifact: &Artifact) -> Value {
    json!({"type": "artifact", "rev": artifact.rev, "artifact": artifact})
}

/// The `artifact` events a client's catch-up holds: every artifact changed after `since`.
// ponytail: scans every artifact's details; an index by rev if a computer ever keeps thousands.
pub fn since(store: &Store, since: i64) -> Vec<Value> {
    saved(store, KEY_PREFIX)
        .filter(|artifact| artifact.is_public() && artifact.rev > since)
        .map(|artifact| event(&artifact))
        .collect()
}

/// The actor classifies the saved artifact before the card's first event, including ordinary public turns.
pub(crate) fn prepare_card(hub: &Hub, bot: &str, data: &Value, internal: bool) -> Result<()> {
    let Some(id) = data["artifact"]["id"].as_str() else { return Ok(()) };
    let mut artifact = get(&hub.store, bot, id).context("the artifact metadata is missing")?;
    if internal {
        // A check-in's replacement waits aside until the turn has something useful: the user's version stays public.
        if artifact.is_public() {
            return Ok(());
        }
        artifact.publication = Publication::Internal;
    } else {
        commit_staged(&hub.store, bot, &mut artifact)?;
        artifact.publication = Publication::Public;
    }
    put(hub, &mut artifact)
}

/// A successful scheduled turn announces metadata before its useful artifact card becomes visible.
pub(crate) fn promote_card(hub: &Hub, bot: &str, id: &str, entry: &str) -> Result<()> {
    let mut artifact = get(&hub.store, bot, id).context("the artifact metadata is missing")?;
    if artifact.entry_id.as_deref() != Some(entry) {
        return Ok(());
    }
    if artifact.is_public() && staged(&hub.store, bot, id).is_none() {
        return Ok(());
    }
    commit_staged(&hub.store, bot, &mut artifact)?;
    artifact.publication = Publication::Public;
    put(hub, &mut artifact)
}

pub fn title(value: &str) -> Result<String> {
    let value = value.trim();
    ensure!(!value.is_empty() && value.len() <= MAX_TITLE, "title must be 1–{MAX_TITLE} bytes");
    ensure!(!value.chars().any(char::is_control), "title can't hold control characters");
    Ok(value.to_owned())
}

/// Blocking: copies the bot's HTML file over the artifact's own, whole or not at all. Returns its size and SHA-256.
fn write(dest: &Path, source: &Path) -> Result<(u64, String)> {
    let input = files::open_regular(source)?;
    ensure!(input.metadata()?.len() <= MAX_ARTIFACT, "an artifact can be at most 5 MiB");
    let mut html = Vec::new();
    input.take(MAX_ARTIFACT + 1).read_to_end(&mut html)?;
    ensure!(u64::try_from(html.len())? <= MAX_ARTIFACT, "an artifact can be at most 5 MiB");
    ensure!(!html.is_empty(), "the artifact is empty");
    std::fs::create_dir_all(dest.parent().context("artifact folder")?).context("creating the artifacts folder")?;
    let partial = dest.with_extension("partial");
    std::fs::write(&partial, &html)?;
    std::fs::rename(&partial, dest)?;
    Ok((u64::try_from(html.len())?, files::digest_hex(&Sha256::digest(&html))))
}

/// The `chat` server's artifact tools.
pub async fn call(hub: &Arc<Hub>, bot: &str, name: &str, args: &Value) -> Result<Value> {
    match name {
        "publish_artifact" => publish(hub, bot, args, ArtifactKind::Html).await,
        "publish_diagram" => publish(hub, bot, args, ArtifactKind::Diagram).await,
        "list_artifacts" => {
            let artifacts: Vec<Value> = list(&hub.store, bot)
                .iter()
                .map(|artifact| {
                    let path = content_path(bot, &artifact.id, artifact.kind)
                        .map(|path| path.display().to_string())
                        .unwrap_or_default();
                    json!({"id": artifact.id, "kind": artifact.kind, "title": artifact.title,
                        "updatedAt": artifact.updated_at, "path": path})
                })
                .collect();
            Ok(json!({"artifacts": artifacts}))
        }
        _ => bail!("unknown artifact tool: {name}"),
    }
}

async fn publish(hub: &Arc<Hub>, bot: &str, args: &Value, kind: ArtifactKind) -> Result<Value> {
    let row = hub.store.bot(bot)?.filter(|r| !r.deleted).ok_or_else(|| anyhow!("unknown bot"))?;
    let existing = match args["artifact"].as_str().filter(|s| !s.is_empty()) {
        Some(id) => {
            Some(get(&hub.store, bot, id).ok_or_else(|| anyhow!("no artifact {id}: list_artifacts shows yours"))?)
        }
        None => None,
    };
    if let Some(artifact) = &existing {
        ensure!(
            artifact.kind == kind,
            "that artifact is a {}; use {} to change it",
            if artifact.kind == ArtifactKind::Diagram { "diagram" } else { "page" },
            if artifact.kind == ArtifactKind::Diagram { "publish_diagram" } else { "publish_artifact" }
        );
    }
    let title = match (args["title"].as_str(), &existing) {
        (Some(value), _) => title(value)?,
        (None, Some(artifact)) => artifact.title.clone(),
        (None, None) => bail!("title is required for a new artifact"),
    };
    let source = args["path"].as_str().filter(|s| !s.is_empty()).map(str::to_owned);
    let scene = args["scene"].as_object().map(|_| args["scene"].clone());
    let inline_scene = kind == ArtifactKind::Diagram && scene.is_some();
    if !inline_scene && source.is_none() {
        bail!("path is required");
    }
    let id = existing.as_ref().map_or_else(|| uuid::Uuid::new_v4().to_string(), |artifact| artifact.id.clone());
    // Replacing what the user already has waits aside: a silent or failed check-in must leave that version intact.
    let public = existing.as_ref().is_some_and(Artifact::is_public);
    let dest = if public { staged_path(bot, &id, kind)? } else { content_path(bot, &id, kind)? };
    let cwd = PathBuf::from(&row.config.cwd);
    let (size, sha256) = if kind == ArtifactKind::Diagram {
        let scene = match (scene, source) {
            (Some(scene), _) => scene,
            (None, Some(source)) => {
                let source = cwd.join(source);
                tokio::task::spawn_blocking(move || crate::chat::diagrams::from_path(&source)).await??
            }
            (None, None) => bail!("a diagram needs path or scene"),
        };
        let dest = dest.clone();
        tokio::task::spawn_blocking(move || crate::chat::diagrams::write(&dest, &scene)).await??
    } else {
        let source = cwd.join(source.context("path is required")?);
        tokio::task::spawn_blocking(move || write(&dest, &source)).await??
    };
    let now = now_ms();
    let mut artifact = match existing {
        Some(artifact) if public => {
            save_staged(&hub.store, bot, &id, &StagedUpdate { title: title.clone(), size, sha256, updated_at: now })?;
            artifact
        }
        Some(mut artifact) => {
            // Not public yet (another card this turn): the same artifact keeps waiting for its turn's decision.
            artifact.title = title.clone();
            artifact.size = size;
            artifact.sha256 = sha256;
            artifact.updated_at = now;
            put(hub, &mut artifact)?;
            artifact
        }
        None => {
            let mut artifact = Artifact {
                id: id.clone(),
                bot_id: bot.to_owned(),
                title: title.clone(),
                kind,
                size,
                sha256,
                created_at: now,
                updated_at: now,
                entry_id: None,
                rev: 0,
                publication: Publication::Pending,
            };
            // Saved before its card goes out, so an app opening the card finds it.
            put(hub, &mut artifact)?;
            artifact
        }
    };
    let mut answer = json!({"published": true, "artifact": {"id": artifact.id, "title": title, "kind": kind}});
    let (reply, posted) = oneshot::channel();
    hub.send_cmd(bot, Cmd::PostArtifact { data: card(&artifact.id, &title, now, kind), reply })?;
    let posted = tokio::time::timeout(Duration::from_secs(10), posted)
        .await
        .map_err(|_| anyhow!("the artifact was saved, but its card wasn't posted in time"))?
        .map_err(|_| anyhow!("the artifact was saved, but the bot shut down before posting its card"))?;
    match posted {
        Ok(entry) => {
            // The actor decided visibility before emitting its card. Do not overwrite that decision with Pending.
            artifact = get(&hub.store, bot, &artifact.id).context("the artifact metadata is missing")?;
            artifact.entry_id = Some(entry);
            // A staged replacement is announced only when the turn promotes it.
            if staged(&hub.store, bot, &artifact.id).is_some() {
                save(&hub.store, &artifact)?;
            } else {
                put(hub, &mut artifact)?;
            }
        }
        // A routine, a room or a bot request: the artifact is in the chat's Library, and its reply says so.
        Err(error) => {
            artifact = get(&hub.store, bot, &artifact.id).context("the artifact metadata is missing")?;
            if artifact.is_public() {
                // No check-in to wait for: the replacement lands in the Library, as before.
                if commit_staged(&hub.store, bot, &mut artifact)? {
                    put(hub, &mut artifact)?;
                }
            } else if artifact.publication == Publication::Pending {
                // Rooms, routines and bot requests publish in Library without a card, as before.
                artifact.publication = Publication::Public;
                put(hub, &mut artifact)?;
            }
            answer["card"] = format!("not posted ({error:#}); mention the artifact in your reply").into();
        }
    }
    Ok(answer)
}

/// One chunk of an artifact's HTML, from `offset`.
fn read(path: &Path, size: u64, offset: u64) -> Result<Vec<u8>> {
    ensure!(offset <= size, "invalid artifact offset");
    files::read_chunk(path, size, offset, "the artifact is missing or damaged")
}

/// The note a bot reads before a message about one of its artifacts (a reply to the artifact's card).
pub fn note(store: &Store, bot: &str, id: &str) -> Option<String> {
    let artifact = get(store, bot, id)?;
    let path = content_path(bot, id, artifact.kind).ok()?;
    let (what, tool) = match artifact.kind {
        ArtifactKind::Html => ("HTML", "publish_artifact"),
        ArtifactKind::Diagram => ("scene", "publish_diagram"),
    };
    Some(format!(
        "[Codync: this is about your artifact \"{}\" (id {id}). Its current {what} is {}; to change the artifact, edit a \
         copy and {tool} with artifact: \"{id}\".]",
        artifact.title,
        path.display()
    ))
}

#[cfg(test)]
#[path = "artifact_visibility_tests.rs"]
mod visibility_tests;

#[cfg(test)]
#[path = "artifact_tests.rs"]
mod tests;

#[path = "artifact_api.rs"]
mod artifact_api;
pub use artifact_api::api;
