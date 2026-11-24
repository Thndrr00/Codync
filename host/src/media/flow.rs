//! Making the file of a flow's media step. Ported from `OpenBot` 0.70.0 `generateWorkflowMedia`
//! (`src/backend/agent-service.ts`) and `grok-media.ts`: the step runs on a throwaway agent of the
//! model's own provider, with the request as its only prompt, and the host keeps every file the
//! run left as a shared file of the routine's bot.
//!
//! The prompt asks for exactly one file and names the tool; the files are read from the provider's
//! own generated folder, because a tool writes there and its JSON items differ per provider
//! (Codex writes `generated_images/<thread>/*.png`, Grok answers `rawOutput.path`).
//!
//! Nothing waits for a person: the agent's permission questions are answered from the bot's own
//! approvals, and the run is bounded by the routine's timeout.

use super::command::Request;
use crate::agent::acp::{self, Incoming};
use crate::chat::files::SharedFile;
use crate::hub::Hub;
use crate::store::BotConfig;
use anyhow::{Context, Result, anyhow, bail, ensure};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

/// How many files one step keeps.
const MAX_FILES: usize = 4;
/// The longest reply that is read back for a path.
const MAX_REPLY: usize = 20_000;

/// The bot as the step's provider sees it: the owner's settings on the model's own backend.
fn runner(owner: &BotConfig, backend: &str) -> BotConfig {
    BotConfig {
        backend: backend.to_owned(),
        command: None,
        model: None,
        model_options: BTreeMap::default(),
        ..owner.clone()
    }
}

/// The folder a provider keeps its generated files in, when one is known.
fn generated_root(backend: &str, env: &[(String, String)]) -> Option<PathBuf> {
    let folder = match backend {
        "codex" => ".codex",
        "grok" => ".grok",
        _ => return None,
    };
    let home = env
        .iter()
        .find(|(key, _)| crate::agent::backends::profile_home(backend) == Some(key.as_str()))
        .map(|(_, value)| PathBuf::from(value))
        .or_else(|| Some(dirs::home_dir()?.join(folder)))?;
    Some(home.join("generated_images"))
}

/// Every file under `root`, as display paths; an absent folder is empty.
fn files_under(root: &Path) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let mut queue = vec![root.to_path_buf()];
    while let Some(dir) = queue.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            match entry.file_type() {
                Ok(kind) if kind.is_dir() => queue.push(path),
                Ok(kind) if kind.is_file() => {
                    found.insert(path.to_string_lossy().into_owned());
                }
                _ => {}
            }
        }
    }
    found
}

fn extension_allowed(kind: super::Kind, path: &str) -> bool {
    let extension = path.rsplit('.').next().unwrap_or_default().to_ascii_lowercase();
    match kind {
        super::Kind::Image => ["png", "jpg", "jpeg", "webp", "gif", "avif"].contains(&extension.as_str()),
        super::Kind::Video => ["mp4", "webm", "mov"].contains(&extension.as_str()),
    }
}

/// The files a run left, newest first: the ones under the provider's folder first, then any path
/// the reply names. Kept to `MAX_FILES` and to what the kind can show.
fn collected(before: &BTreeSet<String>, after: &BTreeSet<String>, kind: super::Kind, reply: &str) -> Vec<String> {
    let mut found: Vec<String> = after.difference(before).cloned().collect();
    for word in reply.split(|c: char| c.is_whitespace() || c == '`' || c == '"' || c == '(' || c == ')') {
        let word = word.trim_end_matches([',', '.', ':', ';', '\'']);
        if extension_allowed(kind, word) && Path::new(word).is_file() && !found.iter().any(|path| path == word) {
            found.push(word.to_owned());
        }
    }
    found.retain(|path| extension_allowed(kind, path));
    found.sort();
    found.dedup();
    found.truncate(MAX_FILES);
    found
}

/// Makes `request` on behalf of `owner` and keeps every file it left. `handed` is what the steps
/// before it passed on, empty when there were none.
pub async fn generate(
    hub: &Arc<Hub>,
    owner: &BotConfig,
    request: &Request,
    handed: &str,
    timeout: Duration,
) -> Result<Vec<SharedFile>> {
    super::command::check(request)?;
    let backend = request.model.backend;
    ensure!(
        crate::agent::backends::installed(backend),
        "{} is not installed on this computer",
        request.model.subscription
    );
    let cfg = runner(owner, backend);
    if crate::market::vault::saved(&hub.store, &format!("agent-env:{backend}"))? {
        crate::market::vault::unlock(hub.clone()).await?;
    }
    let folder = crate::service::data_dir().join("media/flow");
    tokio::fs::create_dir_all(&folder).await?;
    let workspace = folder.to_string_lossy().into_owned();
    let (env, grants) = crate::agent::auth::launch_env(&hub.store, backend, Some(&owner.id))?;
    let root = generated_root(backend, &env);
    let before = root.as_deref().map(files_under).unwrap_or_default();
    let mut conn =
        crate::agent::bot::start_first(crate::agent::bot::launch_commands(&cfg)?, &env, |candidate, mut env, _| {
            let workspace = workspace.clone();
            async move {
                let mut command = [candidate.resolve(|_| {}).await?.acp()];
                crate::agent::own_memory::turn_off(backend, &workspace, &mut env, &mut command).await;
                crate::agent::bot::start_agent(&command[0], &workspace, &env, Duration::from_mins(1)).await
            }
        })
        .await?;
    conn.acp.hold(grants);
    let acp = conn.acp.clone();
    let prompt = request.note_for_flow(handed);
    let result = async {
        let res = acp.request("session/new", json!({"cwd": workspace, "mcpServers": []})).await?;
        let sid = res["sessionId"].as_str().ok_or_else(|| anyhow!("agent returned no sessionId"))?.to_owned();
        if conn.adapter != crate::agent::bot::Adapter::Claude {
            crate::agent::bot::select_model(&acp, &res, &sid, &cfg).await?;
        }
        let asked =
            acp.request("session/prompt", json!({"sessionId": sid, "prompt": [{"type": "text", "text": prompt}]}));
        tokio::pin!(asked);
        let deadline = tokio::time::sleep(timeout);
        tokio::pin!(deadline);
        let mut reply = String::new();
        loop {
            tokio::select! {
                r = &mut asked => { r.and_then(acp::codebuddy::prompt_result)?; break }
                inc = conn.rx.recv() => {
                    if !collect(&acp, inc, &mut reply).await {
                        bail!("the {} agent exited", request.model.subscription);
                    }
                }
                () = &mut deadline => bail!("making the {} took too long", request.kind.as_str()),
            }
        }
        while let Ok(inc) = conn.rx.try_recv() {
            collect(&acp, Some(inc), &mut reply).await;
        }
        Ok(reply)
    }
    .await;
    acp.kill().await;
    let reply = result?;
    let after = root.as_deref().map(files_under).unwrap_or_default();
    let found = collected(&before, &after, request.kind, &reply);
    if found.is_empty() {
        let last = reply.lines().map(str::trim).rfind(|line| !line.is_empty()).unwrap_or_default();
        bail!(
            "the {} made no file{}",
            request.model.name,
            if last.is_empty() { String::new() } else { format!(": {last}") }
        );
    }
    keep(owner, request, found).await
}

/// Copies each made file into the owner's shared files, in the order they were found.
async fn keep(owner: &BotConfig, request: &Request, found: Vec<String>) -> Result<Vec<SharedFile>> {
    let root = crate::chat::files::root(&owner.id);
    let cwd = PathBuf::from(&owner.cwd);
    let mut kept = Vec::new();
    for (index, path) in found.iter().enumerate() {
        let display = super::display_name_from_prompt(request.kind, Some(path), index, &request.prompt);
        let source = path.clone();
        let (root, cwd) = (root.clone(), cwd.clone());
        let snapshot = tokio::task::spawn_blocking(move || {
            crate::chat::files::snapshot(&root, &cwd, &source, Some(&display), &AtomicBool::new(false))
        })
        .await
        .context("the file copy ended early")??;
        let mut snapshot = snapshot;
        snapshot.commit();
        kept.push(snapshot.meta.clone());
    }
    ensure!(!kept.is_empty(), "no file could be kept");
    Ok(kept)
}

/// Keeps reply text; refuses anything the agent asks of us. False once the agent is gone.
async fn collect(acp: &acp::Acp, inc: Option<Incoming>, out: &mut String) -> bool {
    match inc {
        None | Some(Incoming::Closed { .. }) => false,
        Some(Incoming::Notification { method, params }) => {
            let u = &params["update"];
            if method == "session/update" && u["sessionUpdate"] == "agent_message_chunk" && out.len() < MAX_REPLY {
                out.push_str(&acp::content_text(&u["content"]));
            }
            true
        }
        Some(Incoming::Request { id, method, params }) => {
            if method == "session/request_permission" {
                let options = params["options"].as_array();
                let allow = options
                    .and_then(|list| {
                        list.iter().find(|o| o["kind"].as_str().is_some_and(|kind| kind.starts_with("allow")))
                    })
                    .or_else(|| options.and_then(|list| list.first()));
                let _ = match allow.and_then(|o| o["optionId"].as_str()) {
                    Some(option) => {
                        acp.respond(id, json!({"outcome": {"outcome": "selected", "optionId": option}})).await
                    }
                    None => acp.respond(id, json!({"outcome": {"outcome": "cancelled"}})).await,
                };
            } else {
                let _ = acp.respond_error(id, -32601, "not supported").await;
            }
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_new_files_of_the_asked_kind_are_kept() {
        let dir = std::env::temp_dir().join(format!("codync-media-flow-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let old = dir.join("old.png");
        let made = dir.join("made.png");
        let clip = dir.join("made.mp4");
        for path in [&old, &made, &clip] {
            std::fs::write(path, b"x").unwrap();
        }
        let before: BTreeSet<String> = [old.to_string_lossy().into_owned()].into();
        let after = files_under(&dir);
        let images = collected(&before, &after, super::super::Kind::Image, "done");
        assert_eq!(images, [made.to_string_lossy().into_owned()]);
        let videos = collected(&before, &after, super::super::Kind::Video, "done");
        assert_eq!(videos, [clip.to_string_lossy().into_owned()]);
    }

    #[test]
    fn a_path_the_reply_names_is_read_when_the_folder_has_none() {
        let dir = std::env::temp_dir().join(format!("codync-media-flow-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("made.png");
        std::fs::write(&path, b"x").unwrap();
        let reply = format!("Made it: `{}`.", path.to_string_lossy());
        let found = collected(&BTreeSet::new(), &BTreeSet::new(), super::super::Kind::Image, &reply);
        assert_eq!(found, [path.to_string_lossy().into_owned()]);
        assert_eq!(
            collected(&BTreeSet::new(), &BTreeSet::new(), super::super::Kind::Video, &reply),
            Vec::<String>::new()
        );
    }

    #[test]
    fn a_path_that_is_gone_and_a_wrong_kind_are_left_out() {
        let dir = std::env::temp_dir().join(format!("codync-media-flow-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let missing = dir.join("gone.png");
        let found =
            collected(&BTreeSet::new(), &BTreeSet::new(), super::super::Kind::Image, &missing.to_string_lossy());
        assert_eq!(found, Vec::<String>::new());
    }

    #[test]
    fn a_runner_keeps_the_bot_but_takes_the_providers_own_command_and_model() {
        let mut owner: BotConfig =
            serde_json::from_value(json!({"id": "b", "name": "B", "backend": "claude"})).unwrap();
        owner.command = Some("mine".into());
        owner.model = Some("opus".into());
        let cfg = runner(&owner, "codex");
        assert_eq!((cfg.backend.as_str(), cfg.command, cfg.model), ("codex", None, None));
        assert_eq!(cfg.id, "b");
    }

    #[test]
    fn the_provider_folder_follows_its_profile_variable_or_the_home() {
        let env = vec![("CODEX_HOME".to_owned(), "/profiles/a".to_owned())];
        assert_eq!(generated_root("codex", &env), Some(PathBuf::from("/profiles/a/generated_images")));
        assert!(generated_root("codex", &[]).is_some_and(|path| path.ends_with(".codex/generated_images")));
        assert!(generated_root("claude", &[]).is_none());
    }
}
