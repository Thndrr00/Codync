//! Installing, updating, removing and moving a module: everything that touches the disk.

use super::manifest::{self, BUNDLE, Manifest};
use super::{Approved, Installed, Modules, Saved, module_dir, place, read_files};
use crate::store::{Store, now_ms};
use anyhow::{Context, Result, anyhow, bail, ensure};
use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Where a module comes from: a folder on this computer, or a bundle another one exported.
pub enum Source {
    Folder(PathBuf),
    Bundle(Value),
}

impl Source {
    /// Reads `folder` or `bundle` from an API body. A bundle wins when both are there.
    pub fn from_body(body: &Value) -> Result<Self> {
        if let Some(bundle) = body.get("bundle").filter(|b| b.is_object()) {
            return Ok(Self::Bundle(bundle.clone()));
        }
        let folder = body["folder"].as_str().map(str::trim).filter(|f| !f.is_empty());
        folder.map(|f| Self::Folder(PathBuf::from(f))).ok_or_else(|| anyhow!("a folder or a bundle is required"))
    }
}

/// A module's manifest, its signature and its files, checked before anything is written.
fn prepare(source: Source) -> Result<(Manifest, String, BTreeMap<String, Vec<u8>>)> {
    match source {
        Source::Folder(dir) => {
            ensure!(dir.is_dir(), "{} is not a folder", dir.display());
            let manifest = Manifest::read(&dir)?;
            let signature =
                std::fs::read_to_string(dir.join(manifest::SIGNATURE)).context("reading module.sig")?.trim().to_owned();
            let files = read_files(&dir, &manifest)?;
            Ok((manifest, signature, files))
        }
        Source::Bundle(bundle) => {
            ensure!(
                bundle["format"] == BUNDLE,
                "this is not a Codync module bundle (`format` is `{}`)",
                bundle["format"].as_str().unwrap_or_default()
            );
            ensure!(bundle["version"].as_u64() == Some(1), "this bundle is a newer format than this Codync reads");
            let manifest: Manifest =
                serde_json::from_value(bundle["manifest"].clone()).context("reading the bundle's manifest")?;
            manifest.check_contract()?;
            manifest.validate()?;
            let signature = bundle["signature"].as_str().context("a bundle carries its signature")?.trim().to_owned();
            let files = bundle["files"]
                .as_array()
                .context("a bundle carries its files")?
                .iter()
                .map(|file| {
                    let path = file["path"].as_str().context("a file needs a path")?.to_owned();
                    let bytes = B64.decode(file["content"].as_str().unwrap_or_default())?;
                    ensure!(bytes.len() as u64 <= manifest::MAX_FILE_BYTES, "{path} is larger than 4 MB");
                    Ok((path, bytes))
                })
                .collect::<Result<BTreeMap<_, _>>>()?;
            ensure!(files.len() <= manifest::MAX_FILES, "a module holds at most {} files", manifest::MAX_FILES);
            manifest.verify_contents(&signature, &files)?;
            Ok((manifest, signature, files))
        }
    }
}

/// Installs a module, off until the owner approves what it asks for.
pub fn install(modules: &Modules, store: &Store, source: Source) -> Result<Value> {
    let (manifest, signature, files) = prepare(source)?;
    if modules.get(&manifest.id).is_some() {
        bail!("{} is already installed; update it instead", manifest.name);
    }
    let dir = module_dir(&manifest.id)?;
    place(&dir, &files, &manifest, &signature)?;
    let saved = Saved {
        enabled: false,
        approved: Approved::default(),
        installed_at: now_ms(),
        key: manifest.publisher.key.clone(),
    };
    let id = manifest.id.clone();
    modules.put(Installed { id: id.clone(), dir, saved, manifest: Some(manifest), broken: None })?;
    modules.save(store)?;
    Ok(json!({"module": row(modules, &id), "modules": modules.rows()}))
}

/// Replaces a module's files. The owner's approval stays while the same publisher signs it; another key starts
/// over, off, with nothing approved.
pub fn update(modules: &Modules, store: &Store, id: &str, source: Source) -> Result<Value> {
    let previous = super::installed(modules, id)?;
    let (manifest, signature, files) = prepare(source)?;
    ensure!(manifest.id == id, "this bundle is {}, not {id}", manifest.id);
    let dir = module_dir(&manifest.id)?;
    // The old files go; a process still running would keep reading them.
    modules.stop(id);
    place(&dir, &files, &manifest, &signature)?;
    let saved = Saved {
        enabled: previous.saved.enabled,
        approved: previous.saved.approved.clone(),
        installed_at: previous.saved.installed_at,
        key: previous.saved.key.clone(),
    };
    modules.put(Installed { id: id.to_owned(), dir, saved, manifest: Some(manifest), broken: None })?;
    modules.save(store)?;
    Ok(json!({"module": row(modules, id), "modules": modules.rows()}))
}

/// Takes a module out: its process, its files and what the owner decided about it.
pub fn uninstall(modules: &Modules, store: &Store, id: &str) -> Result<Value> {
    let module = super::installed(modules, id)?;
    modules.stop(id);
    if module.dir.starts_with(super::root()) {
        std::fs::remove_dir_all(&module.dir).with_context(|| format!("removing {}", module.dir.display()))?;
    }
    modules.remove(id)?;
    modules.save(store)?;
    Ok(json!({"modules": modules.rows()}))
}

/// A module as one file another computer can install: its manifest, its signature and its exact files.
pub fn export(modules: &Modules, id: &str) -> Result<Value> {
    let module = super::installed(modules, id)?;
    let manifest = module
        .manifest
        .as_ref()
        .ok_or_else(|| anyhow!("this module can't run: {}", module.broken.clone().unwrap_or_default()))?;
    let signature = std::fs::read_to_string(module.dir.join(manifest::SIGNATURE))?.trim().to_owned();
    let files: Vec<Value> = read_files(&module.dir, manifest)?
        .into_iter()
        .map(|(path, bytes)| json!({"path": path, "content": B64.encode(bytes)}))
        .collect();
    Ok(json!({
        "name": format!("{}.module.json", manifest.name.replace(['/', '\\', ':'], "-")),
        "bundle": {
            "format": BUNDLE,
            "version": 1,
            "manifest": serde_json::to_value(manifest)?,
            "signature": signature,
            "files": files,
        },
    }))
}

/// One module's row, for the answer an install or update sends.
fn row(modules: &Modules, id: &str) -> Value {
    modules.rows().into_iter().find(|row| row["id"] == id).unwrap_or(Value::Null)
}
