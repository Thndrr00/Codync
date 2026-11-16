//! Wasm extensions: a folder per extension with an `extension.json` manifest and a `.wasm` entry
//! module, run by the wasmtime or wasmer command line when a tool is called. The host never adds a
//! wasm engine of its own: the runtime stays an external, user-installed binary, and a module gets
//! no directory access unless the manifest asks for it.

use crate::hub::Hub;
use anyhow::{Context, Result, bail, ensure};
use serde::Deserialize;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const METHODS: &[&str] =
    &["extensions", "extensionCall", "installExtension", "removeExtension", "approveExtension"];
const TIMEOUT: Duration = Duration::from_secs(60);
const MAX_OUTPUT: usize = 1 << 20;
const MAX_TOOLS: usize = 64;
const APPROVALS: &str = "extension-approvals.v1";

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    name: String,
    #[serde(default)]
    version: String,
    #[serde(default)]
    description: String,
    entry: String,
    /// `wasmtime` (default) or `wasmer`.
    #[serde(default)]
    runtime: Option<String>,
    /// What the extension asks for: `files` (its `dirs`), `network`, `env`. Nothing is granted
    /// before the person approves the list on this computer.
    #[serde(default)]
    permissions: Vec<String>,
    /// Directories the module may read and write, relative to its folder.
    #[serde(default)]
    dirs: Vec<String>,
    #[serde(default)]
    tools: Vec<Tool>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
struct Tool {
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    input_schema: Value,
}

enum Runtime {
    Wasmtime,
    Wasmer,
}

impl Runtime {
    fn detect(preferred: Option<&str>) -> Result<Self> {
        let wanted = preferred.unwrap_or_default();
        let found = if wanted == "wasmer" && which("wasmer") {
            Some(Self::Wasmer)
        } else if (wanted == "wasmtime" || wanted.is_empty()) && which("wasmtime") {
            Some(Self::Wasmtime)
        } else if wanted.is_empty() && which("wasmer") {
            Some(Self::Wasmer)
        } else {
            None
        };
        found.ok_or_else(|| {
            let name = if wanted.is_empty() { "wasmtime or wasmer" } else { wanted };
            anyhow::anyhow!("no wasm runtime: install {name} and add it to PATH")
        })
    }

    fn program(&self) -> &'static str {
        match self {
            Self::Wasmtime => "wasmtime",
            Self::Wasmer => "wasmer",
        }
    }

    fn id(&self) -> &'static str {
        self.program()
    }
}

fn which(program: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|paths| {
        std::env::split_paths(&paths).any(|dir| {
            let candidate = dir.join(program);
            candidate.is_file() || candidate.with_extension("exe").is_file()
        })
    })
}

/// The extension folders this computer keeps: `<data dir>/extensions/<id>/`.
fn root() -> PathBuf {
    crate::service::data_dir().join("extensions")
}

fn extension_dir_in(base: &Path, id: &str) -> Result<PathBuf> {
    ensure!(!id.is_empty() && id.chars().count() <= 64, "invalid extension id");
    ensure!(id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'), "invalid extension id");
    Ok(base.join(id))
}

fn extension_dir(id: &str) -> Result<PathBuf> {
    extension_dir_in(&root(), id)
}

fn manifest_in(base: &Path, id: &str) -> Result<Manifest> {
    let dir = extension_dir_in(base, id)?;
    let path = dir.join("extension.json");
    let text = std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let manifest: Manifest = serde_json::from_str(&text).context("invalid extension.json")?;
    ensure!(!manifest.name.trim().is_empty(), "an extension needs a name");
    ensure!(manifest.tools.len() <= MAX_TOOLS, "an extension offers at most {MAX_TOOLS} tools");
    let entry = dir.join(&manifest.entry);
    ensure!(
        !manifest.entry.contains("..") && !Path::new(&manifest.entry).is_absolute(),
        "an extension entry stays inside its folder"
    );
    ensure!(entry.is_file(), "the extension's entry module is missing");
    for tool in &manifest.tools {
        ensure!(!tool.name.trim().is_empty(), "a tool needs a name");
        ensure!(tool.name.chars().count() <= 80, "a tool name is at most 80 characters");
    }
    Ok(manifest)
}

fn manifest(id: &str) -> Result<Manifest> {
    manifest_in(&root(), id)
}

fn installed() -> Vec<(String, Result<Manifest>)> {
    let root = root();
    let Ok(entries) = std::fs::read_dir(&root) else { return Vec::new() };
    let mut found: Vec<(String, Result<Manifest>)> = entries
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .map(|entry| {
            let id = entry.file_name().to_string_lossy().into_owned();
            (id.clone(), manifest(&id))
        })
        .collect();
    found.sort_by(|a, b| a.0.cmp(&b.0));
    found
}

fn manifest_json(manifest: &Manifest) -> Value {
    json!({
        "name": manifest.name,
        "version": manifest.version,
        "description": manifest.description,
        "runtime": manifest.runtime,
        "permissions": manifest.permissions,
        "dirs": manifest.dirs,
        "tools": manifest.tools.iter().map(|tool| json!({
            "name": tool.name,
            "description": tool.description,
            "inputSchema": tool.input_schema,
        })).collect::<Vec<_>>(),
    })
}

pub async fn call(hub: &Hub, method: &str, b: &Value) -> Result<Value> {
    match method {
        "extensions" => {
            let runtime = Runtime::detect(None).map(|r| r.id().to_owned()).ok();
            let extensions: Vec<Value> = installed()
                .into_iter()
                .map(|(id, manifest)| match manifest {
                    Ok(manifest) => {
                        let granted = approved(&hub.store, &id);
                        json!({
                            "id": id,
                            "error": null,
                            "approved": manifest.permissions.iter().all(|p| granted.contains(p)),
                            "manifest": manifest_json(&manifest),
                        })
                    }
                    Err(error) => json!({"id": id, "error": format!("{error:#}"), "approved": false, "manifest": null}),
                })
                .collect();
            Ok(json!({
                "runtime": runtime,
                "folder": root().to_string_lossy(),
                "extensions": extensions,
            }))
        }
        "approveExtension" => {
            let id = str_arg(b, "id")?;
            let manifest = manifest(id)?;
            let mut granted = approved(&hub.store, id);
            for permission in &manifest.permissions {
                if !granted.iter().any(|g| g == permission) {
                    granted.push(permission.clone());
                }
            }
            save_approved(&hub.store, id, &granted)?;
            Ok(json!({"approved": granted}))
        }
        "installExtension" => {
            let path = str_arg(b, "path")?;
            let id = install(Path::new(path))?;
            Ok(json!({"id": id}))
        }
        "removeExtension" => {
            let id = str_arg(b, "id")?;
            let dir = extension_dir(id)?;
            ensure!(dir.is_dir(), "no such extension");
            std::fs::remove_dir_all(&dir)?;
            let _ = save_approved(&hub.store, id, &[]);
            Ok(json!({"removed": true}))
        }
        "extensionCall" => {
            let id = b["id"].as_str().ok_or_else(|| anyhow::anyhow!("`id` is required"))?;
            let tool = b["tool"].as_str().ok_or_else(|| anyhow::anyhow!("`tool` is required"))?;
            let arguments = b.get("arguments").cloned().unwrap_or(Value::Null);
            let answer = run_tool(hub, id, tool, arguments).await?;
            Ok(json!({"result": answer}))
        }
        other => bail!("unknown extension method {other}"),
    }
}

/// The permissions the person approved for an extension.
fn approved(store: &crate::store::Store, id: &str) -> Vec<String> {
    store
        .kv_read(APPROVALS)
        .ok()
        .flatten()
        .and_then(|text| serde_json::from_str::<serde_json::Map<String, Value>>(&text).ok())
        .and_then(|map| {
            map.get(id)
                .and_then(Value::as_array)
                .map(|list| list.iter().filter_map(Value::as_str).map(str::to_owned).collect())
        })
        .unwrap_or_default()
}

fn save_approved(store: &crate::store::Store, id: &str, permissions: &[String]) -> Result<()> {
    let mut map: serde_json::Map<String, Value> =
        store.kv_read(APPROVALS)?.and_then(|text| serde_json::from_str(&text).ok()).unwrap_or_default();
    map.insert(id.to_owned(), json!(permissions));
    store.kv_set(APPROVALS, &Value::Object(map).to_string())
}

fn str_arg<'a>(b: &'a Value, key: &str) -> Result<&'a str> {
    b[key].as_str().filter(|s| !s.trim().is_empty()).ok_or_else(|| anyhow::anyhow!("`{key}` is required"))
}

/// Installs a folder, or a `.codync-ext` (a tar.gz holding one), under `<data>/extensions/<id>`.
fn install(path: &Path) -> Result<String> {
    ensure!(path.exists(), "that path doesn't exist");
    let staging = std::env::temp_dir().join(format!("codync-ext-install-{}", uuid::Uuid::new_v4().simple()));
    let source = if path.is_dir() {
        path.to_path_buf()
    } else {
        std::fs::create_dir_all(&staging)?;
        let file = std::fs::File::open(path)?;
        let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(file));
        archive.unpack(&staging)?;
        // The manifest may sit at the archive's top or inside one folder.
        std::fs::read_dir(&staging)?
            .flatten()
            .map(|entry| entry.path())
            .find(|candidate| candidate.join("extension.json").is_file())
            .or_else(|| Some(staging.clone()).filter(|dir| dir.join("extension.json").is_file()))
            .ok_or_else(|| anyhow::anyhow!("the archive has no extension.json"))?
    };
    let text = std::fs::read_to_string(source.join("extension.json"))?;
    let manifest: Manifest = serde_json::from_str(&text).context("invalid extension.json")?;
    ensure!(!manifest.name.trim().is_empty(), "an extension needs a name");
    ensure!(manifest.tools.len() <= MAX_TOOLS, "an extension offers at most {MAX_TOOLS} tools");
    let entry = source.join(&manifest.entry);
    ensure!(
        !manifest.entry.contains("..") && !Path::new(&manifest.entry).is_absolute(),
        "an extension entry stays inside its folder"
    );
    ensure!(entry.is_file(), "the extension's entry module is missing");
    let folder = source
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty() && !name.starts_with("codync-ext-install-"))
        .map(str::to_owned);
    let id = folder
        .filter(|name| name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'))
        .unwrap_or_else(|| {
            manifest
                .name
                .chars()
                .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
                .collect::<String>()
                .trim_matches('-')
                .to_owned()
        });
    ensure!(!id.is_empty(), "the extension needs a folder-safe name");
    let target = root().join(&id);
    ensure!(!target.exists(), "an extension named `{id}` is already installed");
    std::fs::create_dir_all(root())?;
    copy_dir(&source, &target)?;
    let _ = std::fs::remove_dir_all(&staging);
    Ok(id)
}

fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)?.flatten() {
        let path = entry.path();
        let target = to.join(entry.file_name());
        if path.is_dir() {
            copy_dir(&path, &target)?;
        } else {
            std::fs::copy(&path, &target)?;
        }
    }
    Ok(())
}

/// Every installed tool, named `ext_<id>_<tool>` so it cannot shadow a built-in.
pub fn tools() -> Vec<Value> {
    let mut list = Vec::new();
    for (id, manifest) in installed() {
        let Ok(manifest) = manifest else { continue };
        for tool in &manifest.tools {
            list.push(json!({
                "name": tool_name(&id, &tool.name),
                "description": if tool.description.is_empty() {
                    format!("{}. {}", manifest.name, tool.name)
                } else {
                    format!("{}. {}", manifest.name, tool.description)
                },
                "inputSchema": if tool.input_schema.is_null() { json!({"type": "object"}) } else { tool.input_schema.clone() },
            }));
        }
    }
    list
}

fn tool_name(id: &str, tool: &str) -> String {
    format!("ext_{id}_{tool}")
}

/// Whether a tool belongs to an installed extension; the pair is its folder and its own name.
pub fn resolve(name: &str) -> Option<(String, String)> {
    for (id, manifest) in installed() {
        let Ok(manifest) = manifest else { continue };
        for tool in &manifest.tools {
            if tool_name(&id, &tool.name) == name {
                return Some((id, tool.name.clone()));
            }
        }
    }
    None
}

/// Runs an extension tool a bot asked for, after checking the permissions its manifest names.
pub async fn call_tool(hub: &Hub, name: &str, arguments: &Value) -> Result<Value> {
    let (id, tool) = resolve(name).ok_or_else(|| anyhow::anyhow!("unknown extension tool `{name}`"))?;
    run_tool(hub, &id, &tool, arguments.clone()).await
}

async fn run_tool(hub: &Hub, id: &str, tool: &str, arguments: Value) -> Result<Value> {
    let manifest = manifest(id)?;
    ensure!(manifest.tools.iter().any(|t| t.name == tool), "this extension has no tool `{tool}`");
    // A manifest's permissions are asked once, on this computer; until then nothing runs.
    let granted = approved(&hub.store, id);
    let missing: Vec<&str> =
        manifest.permissions.iter().map(String::as_str).filter(|p| !granted.iter().any(|g| g.as_str() == *p)).collect();
    ensure!(missing.is_empty(), "approve this extension in Settings › Extensions first: {}", missing.join(", "));
    let runtime = Runtime::detect(manifest.runtime.as_deref())?;
    let dir = extension_dir(id)?;
    let entry = dir.join(&manifest.entry);
    let mut command = tokio::process::Command::new(runtime.program());
    command.arg("run");
    // Only what the manifest names: a module starts with no filesystem access.
    for dir in &manifest.dirs {
        ensure!(!dir.contains("..") && !Path::new(dir).is_absolute(), "a shared directory stays inside the extension");
        let path = extension_dir(id)?.join(dir);
        command.arg("--dir").arg(path);
    }
    command.arg(&entry);
    if matches!(runtime, Runtime::Wasmer) {
        command.arg("--");
    }
    command.arg(tool).arg(arguments.to_string());
    command.current_dir(&dir);
    command.env_clear();
    command.env("PATH", std::env::var_os("PATH").unwrap_or_default());
    // Windows needs SystemRoot to start a child at all; wasm still sees only PATH and this.
    #[cfg(windows)]
    command.env("SystemRoot", std::env::var_os("SystemRoot").unwrap_or_default());
    let output = tokio::time::timeout(TIMEOUT, command.output())
        .await
        .map_err(|_| anyhow::anyhow!("the extension took too long"))??;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("the extension failed: {}", stderr.trim().chars().take(2000).collect::<String>());
    }
    ensure!(output.stdout.len() <= MAX_OUTPUT, "the extension wrote more than 1 MiB");
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(serde_json::from_str(text.trim()).unwrap_or_else(|_| Value::String(text.trim().to_owned())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Store;

    fn with_extension(files: &[(&str, &str)]) -> (PathBuf, String) {
        let base = std::env::temp_dir().join(format!("codync-ext-{}", uuid::Uuid::new_v4()));
        let id = "demo".to_owned();
        let dir = base.join(&id);
        std::fs::create_dir_all(&dir).unwrap();
        for (name, text) in files {
            std::fs::write(dir.join(name), text).unwrap();
        }
        (base, id)
    }

    #[test]
    fn a_manifest_names_its_tools_and_stays_in_its_folder() {
        let (base, id) = with_extension(&[
            (
                "extension.json",
                r#"{"name":"Demo","entry":"main.wasm","tools":[{"name":"hello","description":"Say hi"}]}"#,
            ),
            ("main.wasm", ""),
        ]);
        let manifest = manifest_in(&base, &id).unwrap();
        assert_eq!(manifest.name, "Demo");
        assert_eq!(manifest.tools[0].name, "hello");
        assert_eq!(manifest_json(&manifest)["tools"][0]["name"], "hello");
    }

    #[test]
    fn a_manifest_cannot_reach_outside_its_folder_or_use_a_bad_id() {
        let (base, id) = with_extension(&[
            ("extension.json", r#"{"name":"Demo","entry":"../main.wasm","tools":[]}"#),
            ("main.wasm", ""),
        ]);
        assert!(manifest_in(&base, &id).is_err());
        std::fs::write(base.join(&id).join("extension.json"), r#"{"name":"Demo","entry":"main.wasm","tools":[]}"#)
            .unwrap();
        assert!(manifest_in(&base, &id).is_ok());
        assert!(extension_dir_in(&base, "../evil").is_err());
        assert!(extension_dir_in(&base, "bad id").is_err());
    }

    #[test]
    fn permissions_are_approved_per_extension() {
        let store = Store::open(Path::new(":memory:")).unwrap();
        assert!(approved(&store, "demo").is_empty());
        save_approved(&store, "demo", &["network".into(), "files".into()]).unwrap();
        assert_eq!(approved(&store, "demo"), vec!["network".to_string(), "files".to_string()]);
        assert!(approved(&store, "other").is_empty());
        save_approved(&store, "demo", &[]).unwrap();
        assert!(approved(&store, "demo").is_empty());
    }
}
