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

pub const METHODS: &[&str] = &["extensions", "extensionCall"];
const TIMEOUT: Duration = Duration::from_secs(60);
const MAX_OUTPUT: usize = 1 << 20;
const MAX_TOOLS: usize = 64;

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
        let found = match wanted {
            "wasmer" if which("wasmer") => Some(Self::Wasmer),
            "wasmtime" if which("wasmtime") => Some(Self::Wasmtime),
            "" if which("wasmtime") => Some(Self::Wasmtime),
            "" if which("wasmer") => Some(Self::Wasmer),
            _ => None,
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
        .filter_map(|entry| {
            let id = entry.file_name().to_string_lossy().into_owned();
            Some((id.clone(), manifest(&id)))
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
                    Ok(manifest) => json!({"id": id, "error": null, "manifest": manifest_json(&manifest)}),
                    Err(error) => json!({"id": id, "error": format!("{error:#}"), "manifest": null}),
                })
                .collect();
            Ok(json!({
                "runtime": runtime,
                "folder": root().to_string_lossy(),
                "extensions": extensions,
            }))
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

async fn run_tool(_hub: &Hub, id: &str, tool: &str, arguments: Value) -> Result<Value> {
    let manifest = manifest(id)?;
    ensure!(manifest.tools.iter().any(|t| t.name == tool), "this extension has no tool `{tool}`");
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
        assert!(manifest_json(&manifest)["tools"][0]["name"] == "hello");
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
}
