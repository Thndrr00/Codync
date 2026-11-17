//! Modules: what a person adds to Codync beyond what it ships with.
//!
//! A module is a folder with a signed manifest (see [`manifest`]), a program the host runs in its own process, and
//! the tools and views it declares. The host is the only door: a bot calls a tool through Codync's MCP server, the
//! host checks the owner's approval and the module's quotas, and only then does the module's process hear anything.
//!
//! agentes.chat's ADR 0002 makes a module an object inside the server process, because their editions need Cloud
//! behaviour switched on without a fork. Codync has one edition and one person, so a module here is something the
//! person installs: it is signed, it runs beside the host rather than inside it, and it asks before it acts.

pub mod api;
mod install;
mod manifest;
mod process;
#[cfg(test)]
mod tests;

pub use manifest::{Files, Manifest, Permissions};

use crate::hub::Hub;
use crate::store::Store;
use anyhow::{Result, anyhow, ensure};
use process::Running;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

/// The kv key the installed modules' state lives under. What each module *is* is its folder, re-read and verified
/// on every start; this holds only the owner's decisions.
const KV: &str = "modules";

/// What the owner approved of one module. Never more than the manifest asks: approving is choosing, not granting.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct Approved {
    pub tools: Vec<String>,
    pub events: Vec<String>,
    pub network: Vec<String>,
    pub files: Files,
}

impl Approved {
    /// Everything the manifest asks for: what the install dialog proposes.
    pub fn all(permissions: &Permissions) -> Self {
        Self {
            tools: permissions.tools.clone(),
            events: permissions.events.clone(),
            network: permissions.network.clone(),
            files: permissions.files,
        }
    }

    /// The owner's choice, cut down to what the manifest declares.
    pub fn within(&self, permissions: &Permissions) -> Self {
        let keep = |asked: &[String], declared: &[String]| -> Vec<String> {
            asked.iter().filter(|v| declared.contains(v)).cloned().collect()
        };
        let files = match (self.files, permissions.files) {
            (Files::Write, Files::Write) => Files::Write,
            (Files::Write | Files::Read, Files::Write | Files::Read) => Files::Read,
            _ => Files::None,
        };
        Self {
            tools: keep(&self.tools, &permissions.tools),
            events: keep(&self.events, &permissions.events),
            network: keep(&self.network, &permissions.network),
            files,
        }
    }

    pub fn tool(&self, name: &str) -> bool {
        self.tools.iter().any(|t| t == name)
    }

    pub fn event(&self, name: &str) -> bool {
        self.events.iter().any(|e| e == name)
    }

    pub fn to_json(&self) -> Value {
        json!({
            "tools": self.tools,
            "events": self.events,
            "network": self.network,
            "files": self.files.name(),
        })
    }
}

/// The owner's decisions about one module, as the database keeps them.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct Saved {
    enabled: bool,
    approved: Approved,
    installed_at: i64,
    /// The key that signed the version on disk: an update from another key asks again.
    key: String,
}

/// One module on disk, with what the host knows about it.
pub struct Installed {
    pub id: String,
    pub dir: PathBuf,
    pub(crate) saved: Saved,
    /// `None` when the folder no longer holds a module this host can run, with the reason beside it.
    pub manifest: Option<Manifest>,
    pub broken: Option<String>,
}

impl Installed {
    pub fn name(&self) -> String {
        self.manifest.as_ref().map_or_else(|| self.id.clone(), |m| m.name.clone())
    }

    pub fn enabled(&self) -> bool {
        self.saved.enabled && self.manifest.is_some()
    }

    pub fn approved(&self) -> &Approved {
        &self.saved.approved
    }

    pub fn limits(&self) -> manifest::Limits {
        self.manifest.as_ref().map(|m| m.limits.clamped()).unwrap_or_default()
    }
}

/// Every installed module and the processes they run in.
pub struct Modules {
    installed: Mutex<BTreeMap<String, Installed>>,
    running: Mutex<BTreeMap<String, Arc<Running>>>,
    events: broadcast::Sender<Value>,
}

/// Where modules live on this computer.
pub fn root() -> PathBuf {
    crate::service::data_dir().join("modules")
}

impl Modules {
    /// Reads every installed module from disk and verifies it. One that doesn't verify stays listed with its
    /// reason, so the person can update or remove it instead of wondering where it went.
    pub fn load(store: &Store, events: broadcast::Sender<Value>) -> Self {
        let saved: BTreeMap<String, Saved> =
            store.kv_read(KV).ok().flatten().and_then(|text| serde_json::from_str(&text).ok()).unwrap_or_default();
        let mut installed = BTreeMap::new();
        for (id, state) in saved {
            let dir = root().join(&id);
            let (manifest, broken) = match Manifest::read(&dir) {
                Ok(manifest) => (Some(manifest), None),
                Err(error) => (None, Some(format!("{error:#}"))),
            };
            installed.insert(id.clone(), Installed { id, dir, saved: state, manifest, broken });
        }
        Self { installed: Mutex::new(installed), running: Mutex::default(), events }
    }

    /// What the apps show, in the order they list them.
    pub fn rows(&self) -> Vec<Value> {
        // A process that stopped stays in the map until the next call replaces it, so its reason can
        // still be shown. It is not "running" any more: only live processes count here.
        let running = self
            .running
            .lock()
            .map(|r| r.iter().filter(|(_, process)| process.alive()).map(|(id, _)| id.clone()).collect::<Vec<_>>())
            .unwrap_or_default();
        let mut rows: Vec<Value> =
            self.installed.lock().map(|all| all.values().map(|m| self.row(m, &running)).collect()).unwrap_or_default();
        rows.sort_by_key(|row| row["name"].as_str().unwrap_or_default().to_lowercase());
        rows
    }

    fn row(&self, module: &Installed, running: &[String]) -> Value {
        let manifest = module.manifest.as_ref();
        let state = if module.broken.is_some() {
            "broken"
        } else if !module.enabled() {
            "disabled"
        } else if running.iter().any(|id| id == &module.id) {
            "running"
        } else {
            "stopped"
        };
        let stopped = self.running.lock().ok().and_then(|r| r.get(&module.id).map(|p| p.stopped())).flatten();
        json!({
            "id": module.id,
            "name": module.name(),
            "version": manifest.map(|m| m.version.clone()),
            "description": manifest.map(|m| m.description.clone()).unwrap_or_default(),
            "symbol": manifest.and_then(|m| m.symbol.clone()),
            "contract": manifest.map(|m| m.contract),
            "enabled": module.enabled(),
            "broken": module.broken,
            "state": state,
            "stopped": stopped,
            "permissions": manifest.map(|m| json!({
                "tools": m.permissions.tools,
                "events": m.permissions.events,
                "network": m.permissions.network,
                "files": m.permissions.files.name(),
            })),
            "approved": module.approved().to_json(),
            "limits": module.limits(),
            "views": manifest.map(|m| json!(m.views)),
            "tools": manifest.map(|m| json!(m.tools.iter().map(|t| json!({
                "name": t.name, "description": t.description,
            })).collect::<Vec<_>>())),
            "installedAt": module.saved.installed_at,
            "folder": module.dir.to_string_lossy(),
        })
    }

    pub fn get(&self, id: &str) -> Option<Installed> {
        self.installed.lock().ok()?.get(id).map(|m| Installed {
            id: m.id.clone(),
            dir: m.dir.clone(),
            saved: m.saved.clone(),
            manifest: m.manifest.clone(),
            broken: m.broken.clone(),
        })
    }

    /// The modules a bot gets tools from: on, verified, with something approved to call.
    pub fn for_bots(&self) -> Vec<Installed> {
        self.installed
            .lock()
            .map(|all| {
                all.values()
                    .filter(|m| m.enabled() && !m.approved().tools.is_empty())
                    .map(|m| Installed {
                        id: m.id.clone(),
                        dir: m.dir.clone(),
                        saved: m.saved.clone(),
                        manifest: m.manifest.clone(),
                        broken: m.broken.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Inserts or replaces a module the installer verified, and keeps the owner's decisions when the publisher is
    /// the same one.
    pub(crate) fn put(&self, mut module: Installed) -> Result<()> {
        let mut all = self.installed.lock().map_err(|_| anyhow!("module lock poisoned"))?;
        if let Some(previous) = all.get(&module.id) {
            let key = module.manifest.as_ref().map(|m| m.publisher.key.clone()).unwrap_or_default();
            if key == previous.saved.key {
                module.saved = previous.saved.clone();
            } else {
                // Another key signs this one: what was approved for the old one says nothing about it.
                module.saved.approved = Approved::default();
                module.saved.enabled = false;
            }
            module.saved.installed_at = previous.saved.installed_at;
        }
        all.insert(module.id.clone(), module);
        Ok(())
    }

    pub(crate) fn save(&self, store: &Store) -> Result<()> {
        let all = self.installed.lock().map_err(|_| anyhow!("module lock poisoned"))?;
        let saved: BTreeMap<&str, &Saved> = all.iter().map(|(id, m)| (id.as_str(), &m.saved)).collect();
        store.kv_set(KV, &serde_json::to_string(&saved)?)?;
        Ok(())
    }

    /// Turns a module on or off. Off stops it at once; on starts it with the next call.
    pub fn set_enabled(&self, store: &Store, id: &str, enabled: bool) -> Result<Value> {
        {
            let mut all = self.installed.lock().map_err(|_| anyhow!("module lock poisoned"))?;
            let module = all.get_mut(id).ok_or_else(|| anyhow!("unknown module"))?;
            ensure!(module.manifest.is_some(), "this module can't run: {}", module.broken.clone().unwrap_or_default());
            module.saved.enabled = enabled;
        }
        if !enabled {
            self.stop(id);
        }
        self.save(store)?;
        Ok(json!({"modules": self.rows()}))
    }

    /// Records what the owner approved, cut down to what the manifest declares.
    pub fn approve(&self, store: &Store, id: &str, asked: &Value) -> Result<Value> {
        let mut all = self.installed.lock().map_err(|_| anyhow!("module lock poisoned"))?;
        let module = all.get_mut(id).ok_or_else(|| anyhow!("unknown module"))?;
        let permissions = module
            .manifest
            .as_ref()
            .ok_or_else(|| anyhow!("this module can't run: {}", module.broken.clone().unwrap_or_default()))?
            .permissions
            .clone();
        let asked: Approved = if asked.is_null() {
            Approved::all(&permissions)
        } else {
            serde_json::from_value(asked.clone()).map_err(|_| anyhow!("invalid permissions"))?
        };
        module.saved.approved = asked.within(&permissions);
        drop(all);
        self.save(store)?;
        Ok(json!({"modules": self.rows()}))
    }

    /// Stops one module's process, if it has one.
    pub fn stop(&self, id: &str) {
        let running = self.running.lock().ok().and_then(|mut r| r.remove(id));
        if let Some(running) = running {
            running.stop("stopped".into());
        }
    }

    /// Forgets a module the installer already took off the disk.
    pub(crate) fn remove(&self, id: &str) -> Result<()> {
        let mut all = self.installed.lock().map_err(|_| anyhow!("module lock poisoned"))?;
        all.remove(id);
        Ok(())
    }

    /// Stops everything: the host is going away, and no module outlives it.
    pub fn stop_all(&self) {
        let all: Vec<Arc<Running>> =
            self.running.lock().map(|mut r| std::mem::take(&mut *r).into_values().collect()).unwrap_or_default();
        for running in all {
            running.stop("the host stopped".into());
        }
    }

    /// The module's process, started on first use. A module that died is started again by the next call.
    fn running(&self, module: &Installed) -> Result<Arc<Running>> {
        let mut running = self.running.lock().map_err(|_| anyhow!("module lock poisoned"))?;
        if let Some(existing) = running.get(&module.id)
            && existing.alive()
        {
            return Ok(existing.clone());
        }
        running.remove(&module.id);
        let manifest = module.manifest.as_ref().ok_or_else(|| anyhow!("this module can't run"))?;
        let heard: Vec<String> =
            manifest.permissions.events.iter().filter(|e| module.approved().event(e)).cloned().collect();
        let started = Running::spawn(&module.dir, manifest, heard, self.events.clone())?;
        running.insert(module.id.clone(), started.clone());
        Ok(started)
    }

    /// Calls one of a module's tools for a bot: approval, quotas and a folder it may read, all here.
    pub async fn call(&self, id: &str, tool: &str, arguments: &Value, bot: &str) -> Result<Value> {
        let module = self.get(id).ok_or_else(|| anyhow!("unknown module"))?;
        ensure!(module.enabled(), "this module is off");
        ensure!(module.approved().tool(tool), "{tool} isn't approved for this module");
        ensure!(module.manifest.as_ref().is_some_and(|m| m.tool(tool).is_some()), "unknown tool {tool}");
        let running = self.running(&module)?;
        let folder = (module.approved().files != Files::None).then(|| bot_folder(bot)).flatten();
        let manifest = module.manifest.as_ref().ok_or_else(|| anyhow!("this module can't run"))?;
        running.ready(manifest, folder.as_deref()).await?;
        running
            .request(
                "tool",
                json!({
                    "name": tool,
                    "arguments": arguments,
                    "botFolder": folder.map(|p| p.to_string_lossy().into_owned()),
                }),
            )
            .await
    }

    /// One of a module's views, as rows the apps draw with their own components.
    pub async fn view(&self, id: &str, view: &str, bot: &str) -> Result<Value> {
        let module = self.get(id).ok_or_else(|| anyhow!("unknown module"))?;
        ensure!(module.enabled(), "this module is off");
        ensure!(module.manifest.as_ref().is_some_and(|m| m.views.iter().any(|v| v.id == view)), "unknown view {view}");
        let running = self.running(&module)?;
        let folder = (module.approved().files != Files::None).then(|| bot_folder(bot)).flatten();
        let manifest = module.manifest.as_ref().ok_or_else(|| anyhow!("this module can't run"))?;
        running.ready(manifest, folder.as_deref()).await?;
        let answer = running
            .request("view", json!({"id": view, "botFolder": folder.map(|p| p.to_string_lossy().into_owned())}))
            .await;
        match answer {
            Ok(result) => process::sections(&result),
            Err(error) => Err(error),
        }
    }

    /// The running modules that asked to hear `name`.
    fn listeners(&self, name: &str) -> Vec<Arc<Running>> {
        let Ok(running) = self.running.lock() else { return Vec::new() };
        let Ok(all) = self.installed.lock() else { return Vec::new() };
        running
            .iter()
            .filter(|(id, process)| {
                process.alive()
                    && all.get(*id).is_some_and(|m| {
                        m.enabled() && m.approved().event(name) && m.manifest.as_ref().is_some_and(|x| x.hears(name))
                    })
            })
            .map(|(_, process)| process.clone())
            .collect()
    }
}

/// The bot's own folder, when a module may read it.
fn bot_folder(bot: &str) -> Option<PathBuf> {
    let folder = crate::service::data_dir().join("bots").join(bot);
    folder.is_dir().then_some(folder)
}

/// Where an installed module's folder is, by id.
pub fn folder(id: &str) -> PathBuf {
    root().join(id)
}

/// The host event a module hears, if this one is worth telling: the small ones a module can act on.
fn forwarded(event: &Value) -> Option<(&'static str, Value)> {
    match event["type"].as_str()? {
        "entry" => {
            let entry = &event["entry"];
            if entry["author"] == "tool" || entry["kind"] == "notice" {
                return None;
            }
            let text: String = entry["text"].as_str().unwrap_or_default().chars().take(400).collect();
            Some(("messageAdded", json!({"bot": entry["botId"], "author": entry["author"], "text": text})))
        }
        "bot" => {
            let bot = &event["bot"];
            let id = bot["id"].as_str()?;
            // A deleted bot says so once; its status is the only thing a module acts on.
            let status = bot["status"].as_str()?;
            Some(("botStatus", json!({"bot": id, "status": status})))
        }
        _ => None,
    }
}

/// Tells the modules what happened, for as long as the host runs.
pub async fn watch(hub: Arc<Hub>) {
    let mut events = hub.events.subscribe();
    loop {
        let event = match events.recv().await {
            Ok(event) => event,
            Err(broadcast::error::RecvError::Lagged(_)) => continue,
            Err(broadcast::error::RecvError::Closed) => return,
        };
        let Some((name, body)) = forwarded(&event) else { continue };
        for process in hub.modules.listeners(name) {
            if let Err(error) = process.notify(name, body.clone()).await {
                tracing::warn!(module = %process.id, error = format!("{error:#}"), "telling a module what happened");
            }
        }
    }
}

/// Starts the host's side of modules: the event fan-out. The processes start with the first call.
pub fn boot(hub: &Arc<Hub>) {
    tokio::spawn(watch(hub.clone()));
}

/// What an install or update needs from the caller, in one place.
pub(crate) fn module_dir(id: &str) -> Result<PathBuf> {
    ensure!(!id.is_empty(), "a module needs an id");
    let dir = folder(id);
    ensure!(dir.starts_with(root()), "`{id}` is not a module id");
    Ok(dir)
}

/// Copies a verified folder into place, leaving nothing half-written if it fails.
pub(crate) fn place(dir: &Path, files: &BTreeMap<String, Vec<u8>>, manifest: &Manifest, signature: &str) -> Result<()> {
    let parent = dir.parent().ok_or_else(|| anyhow!("a module needs a folder"))?;
    std::fs::create_dir_all(parent)?;
    let staging = parent.join(format!(".{}.installing", dir.file_name().unwrap_or_default().to_string_lossy()));
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging)?;
    let written = (|| -> Result<()> {
        for (path, bytes) in files {
            let target = staging.join(path);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&target, bytes)?;
        }
        std::fs::write(staging.join(manifest::MANIFEST), serde_json::to_vec(manifest)?)?;
        std::fs::write(staging.join(manifest::SIGNATURE), signature)?;
        Ok(())
    })();
    if let Err(error) = written {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(error);
    }
    let _ = std::fs::remove_dir_all(dir);
    std::fs::rename(&staging, dir)?;
    Ok(())
}

/// The files a verified module folder holds, without its manifest and signature.
pub(crate) fn read_files(dir: &Path, manifest: &Manifest) -> Result<BTreeMap<String, Vec<u8>>> {
    manifest
        .files
        .iter()
        .map(|file| {
            let bytes = std::fs::read(dir.join(&file.path))?;
            Ok((file.path.clone(), bytes))
        })
        .collect()
}

/// A refusal that names the module, for the sentences the apps show.
pub(crate) fn missing(id: &str) -> anyhow::Error {
    anyhow!("no module called {id}")
}

/// An installed module, or a sentence saying why there isn't one.
pub(crate) fn installed(modules: &Modules, id: &str) -> Result<Installed> {
    modules.get(id).ok_or_else(|| missing(id))
}

/// The id from a body, trimmed.
pub(crate) fn id_arg(body: &Value) -> Result<String> {
    body["id"]
        .as_str()
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| anyhow!("`id` is required"))
}
