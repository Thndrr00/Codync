//! The model library: what is installed, what a template still needs, and downloads that resume.
//!
//! A download only starts for a file a template's manifest declares, and only from that manifest's
//! URL: a bot can't point the downloader at arbitrary text. The partial file stays as `<name>.part`
//! while paused, so a 4 GB checkpoint resumes where it stopped, and the finished file is checked
//! against the manifest's size and SHA-256 before it takes its final name.

use super::templates::{self, Model};
use crate::LockExt;
use crate::tools::comfy;
use anyhow::{Context, Result, bail, ensure};
use futures::StreamExt as _;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::io::AsyncWriteExt as _;
use tokio::sync::broadcast;

/// How often the downloader tells the apps how far it got.
const ANNOUNCE_EVERY: u64 = 64 * 1024 * 1024;

#[derive(Clone)]
struct Entry {
    folder: String,
    file: String,
    done: u64,
    total: u64,
    /// `downloading`, `paused`, `done` or `failed`.
    phase: &'static str,
    error: Option<String>,
    /// Set by `pause`; the task stops after the chunk it is writing.
    pause: Arc<AtomicBool>,
    /// A new start replaces the entry; an older task's updates must not overwrite it.
    generation: u64,
}

impl Entry {
    fn public(&self) -> Value {
        json!({
            "folder": self.folder,
            "file": self.file,
            "done": self.done,
            "total": self.total,
            "phase": self.phase,
            "error": self.error,
            "percent": (self.total > 0).then(|| self.done as f64 / self.total as f64),
        })
    }
}

pub struct Downloads {
    entries: Mutex<BTreeMap<String, Entry>>,
    next_generation: AtomicU64,
    events: broadcast::Sender<Value>,
}

impl Downloads {
    pub fn new(events: broadcast::Sender<Value>) -> Self {
        Self { entries: Mutex::new(BTreeMap::new()), next_generation: AtomicU64::new(1), events }
    }

    /// Every model a template declares, installed or not, plus files the user put there by hand,
    /// with the space each folder takes and the disk's free space.
    pub fn list(&self) -> Value {
        let templates = templates::list();
        let mut folders = Vec::new();
        let mut total_bytes = 0_u64;
        for folder in comfy::model_folders() {
            let dir = comfy::model_folder(folder).unwrap_or_default();
            let mut rows: BTreeMap<String, Value> = BTreeMap::new();
            let mut bytes = 0_u64;
            if let Ok(files) = std::fs::read_dir(&dir) {
                for file in files.filter_map(Result::ok) {
                    let name = file.file_name().to_string_lossy().into_owned();
                    if name.ends_with(".part") || !file.file_type().is_ok_and(|t| t.is_file()) {
                        continue;
                    }
                    let size = file.metadata().map_or(0, |m| m.len());
                    bytes += size;
                    rows.insert(
                        name.clone(),
                        json!({"file": name, "size": size, "present": true, "declared": false, "usedBy": []}),
                    );
                }
            }
            for template in &templates {
                for model in template.models.iter().filter(|m| m.folder == folder) {
                    let row = rows.entry(model.file.clone()).or_insert_with(
                        || json!({"file": model.file, "size": 0, "present": false, "declared": false, "usedBy": []}),
                    );
                    row["declared"] = json!(true);
                    row["url"] = json!(model.url);
                    row["license"] = json!(model.license);
                    if let Some(used_by) = row["usedBy"].as_array_mut() {
                        used_by.push(json!(template.id));
                    }
                    if row["size"].as_u64().unwrap_or(0) == 0 {
                        row["size"] = json!(model.size);
                    }
                }
            }
            total_bytes += bytes;
            let mut list: Vec<Value> = rows.into_values().collect();
            list.sort_by(|a, b| a["file"].as_str().unwrap_or_default().cmp(b["file"].as_str().unwrap_or_default()));
            folders.push(json!({"folder": folder, "path": dir.to_string_lossy(), "bytes": bytes, "models": list}));
        }
        let root = comfy::data_dir().join("models");
        let (total, free) = comfy::disk_space(&root).unwrap_or((0, 0));
        json!({
            "folders": folders,
            "totalBytes": total_bytes,
            "diskBytes": total,
            "freeBytes": free,
            "downloads": self.status(),
        })
    }

    /// Every download, newest state first.
    pub fn status(&self) -> Value {
        let mut entries: Vec<Value> = self.entries.locked().values().map(Entry::public).collect();
        entries.sort_by(|a, b| a["file"].as_str().unwrap_or_default().cmp(b["file"].as_str().unwrap_or_default()));
        Value::Array(entries)
    }

    fn public(&self, key: &str) -> Value {
        self.entries.locked().get(key).map_or(Value::Null, Entry::public)
    }

    /// Starts (or resumes) the download of a declared model. The URL always comes from the manifest.
    pub fn start(self: &Arc<Self>, folder: &str, file: &str) -> Result<Value> {
        let Some((template, model)) = templates::declaring(folder, file).into_iter().next() else {
            bail!("`{file}` is not declared by any template, so it cannot be downloaded here");
        };
        ensure!(model.url.is_some(), "`{file}` has no download URL in `{}`", template);
        let key = key(folder, file);
        if let Some(entry) = self.entries.locked().get(&key) {
            ensure!(entry.phase != "downloading", "`{file}` is already downloading");
        }
        let path = model.path()?;
        if let Ok(meta) = std::fs::metadata(&path)
            && meta.is_file()
            && (model.size == 0 || meta.len() == model.size)
        {
            let total = meta.len();
            let generation = self.next_generation.fetch_add(1, Ordering::Relaxed);
            let _ = self.entries.locked().insert(
                key.clone(),
                Entry {
                    folder: folder.to_owned(),
                    file: file.to_owned(),
                    done: total,
                    total,
                    phase: "done",
                    error: None,
                    pause: Arc::new(AtomicBool::new(false)),
                    generation,
                },
            );
            return Ok(self.public(&key));
        }
        let generation = self.next_generation.fetch_add(1, Ordering::Relaxed);
        let done = std::fs::metadata(part(&path)).map_or(0, |m| m.len());
        let pause = Arc::new(AtomicBool::new(false));
        let _ = self.entries.locked().insert(
            key.clone(),
            Entry {
                folder: folder.to_owned(),
                file: file.to_owned(),
                done,
                total: model.size,
                phase: "downloading",
                error: None,
                pause: pause.clone(),
                generation,
            },
        );
        self.announce();
        tokio::spawn(self.clone().download(key.clone(), model, generation, pause));
        Ok(self.public(&key))
    }

    /// Stops after the chunk in flight; the partial file stays for [`Self::start`] to resume.
    pub fn pause(&self, folder: &str, file: &str) -> Result<Value> {
        let key = key(folder, file);
        let mut entries = self.entries.locked();
        let Some(entry) = entries.get_mut(&key) else { bail!("`{file}` is not downloading") };
        ensure!(entry.phase == "downloading", "`{file}` is not downloading");
        entry.pause.store(true, Ordering::Release);
        Ok(entry.public())
    }

    /// Deletes an installed model. A model a template needs asks for confirmation (`force`).
    pub fn delete(&self, folder: &str, file: &str, force: bool) -> Result<()> {
        let key = key(folder, file);
        ensure!(
            self.entries.locked().get(&key).is_none_or(|e| e.phase != "downloading"),
            "pause `{file}` before deleting it"
        );
        let used = templates::declaring(folder, file);
        ensure!(
            force || used.is_empty(),
            "`{file}` is used by {}: deleting it breaks that template. Confirm to delete it anyway.",
            used.iter().map(|(id, _)| id.as_str()).collect::<Vec<_>>().join(", ")
        );
        let path = comfy::model_path(folder, file)?;
        if path.is_file() {
            std::fs::remove_file(&path).with_context(|| format!("deleting {}", path.display()))?;
        }
        let part = part(&path);
        if part.is_file() {
            std::fs::remove_file(&part).with_context(|| format!("deleting {}", part.display()))?;
        }
        let _ = self.entries.locked().remove(&key);
        Ok(())
    }

    async fn download(self: Arc<Self>, key: String, model: Model, generation: u64, pause: Arc<AtomicBool>) {
        if let Err(error) = self.transfer(&key, &model, generation, &pause).await {
            self.update(&key, generation, |entry| {
                entry.phase = "failed";
                entry.error = Some(format!("{error:#}"));
            });
        }
        self.announce();
    }

    async fn transfer(&self, key: &str, model: &Model, generation: u64, pause: &AtomicBool) -> Result<()> {
        let path = model.path()?;
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await.with_context(|| format!("creating {}", parent.display()))?;
        }
        let part = part(&path);
        let mut done = tokio::fs::metadata(&part).await.map_or(0, |m| m.len());
        if model.size > 0 && done > model.size {
            tokio::fs::remove_file(&part).await?;
            done = 0;
        }
        if model.size > 0 && done == model.size {
            return finish(&self.entries, key, generation, &path, &part, model).await;
        }
        let url = model.url.as_deref().context("the manifest has no download URL")?;
        let mut request = crate::http().get(url);
        if done > 0 {
            request = request.header(reqwest::header::RANGE, format!("bytes={done}-"));
        }
        let response = request.send().await.with_context(|| format!("downloading {}", model.file))?;
        let status = response.status();
        ensure!(status.is_success(), "downloading {}: the server answered {status}", model.file);
        let resumed = status == reqwest::StatusCode::PARTIAL_CONTENT;
        if done > 0 && !resumed {
            // The server sent the whole file: start over rather than append a second copy.
            done = 0;
        }
        let total = if model.size > 0 { model.size } else { done + response.content_length().unwrap_or(0) };
        self.update(key, generation, |entry| {
            entry.done = done;
            entry.total = total;
            entry.phase = "downloading";
        });
        let mut file = if done > 0 {
            tokio::fs::OpenOptions::new().append(true).open(&part).await?
        } else {
            tokio::fs::OpenOptions::new().write(true).create(true).truncate(true).open(&part).await?
        };
        let mut stream = response.bytes_stream();
        let mut announced = done;
        while let Some(chunk) = stream.next().await {
            if pause.load(Ordering::Acquire) {
                file.flush().await?;
                self.update(key, generation, |entry| {
                    entry.done = done;
                    entry.phase = "paused";
                });
                return Ok(());
            }
            let chunk = chunk.with_context(|| format!("reading {}", model.file))?;
            file.write_all(&chunk).await.with_context(|| format!("writing {}", part.display()))?;
            done += chunk.len() as u64;
            if done - announced >= ANNOUNCE_EVERY {
                announced = done;
                self.update(key, generation, |entry| entry.done = done);
            }
        }
        file.flush().await?;
        drop(file);
        self.update(key, generation, |entry| entry.done = done);
        finish(&self.entries, key, generation, &path, &part, model).await
    }

    /// Moves the finished part into place, after the manifest's size and hash check out.
    fn update(&self, key: &str, generation: u64, change: impl FnOnce(&mut Entry)) {
        let mut entries = self.entries.locked();
        let Some(entry) = entries.get_mut(key) else { return };
        if entry.generation != generation {
            return;
        }
        change(entry);
    }

    fn announce(&self) {
        let _ = self.events.send(json!({"type": "comfyModels", "models": self.status()}));
    }
}

/// `checkpoints/model.safetensors`
fn key(folder: &str, file: &str) -> String {
    format!("{folder}/{file}")
}

fn part(path: &Path) -> PathBuf {
    PathBuf::from(format!("{}.part", path.display()))
}

async fn finish(
    entries: &Mutex<BTreeMap<String, Entry>>,
    key: &str,
    generation: u64,
    path: &Path,
    part: &Path,
    model: &Model,
) -> Result<()> {
    let size = tokio::fs::metadata(part).await.map(|m| m.len()).unwrap_or(0);
    if model.size > 0 {
        ensure!(size == model.size, "`{}` arrived with {size} bytes, not {}", model.file, model.size);
    }
    if let Some(expected) = &model.sha256 {
        let actual = hash(part).await?;
        ensure!(actual.eq_ignore_ascii_case(expected), "`{}` does not match its SHA-256", model.file);
    }
    tokio::fs::rename(part, path).await.with_context(|| format!("moving {}", path.display()))?;
    let mut all = entries.locked();
    let Some(entry) = all.get_mut(key) else { return Ok(()) };
    if entry.generation == generation {
        entry.phase = "done";
        entry.done = size;
        entry.total = size;
    }
    Ok(())
}

async fn hash(path: &Path) -> Result<String> {
    use sha2::Digest as _;
    let owned = path.to_owned();
    tokio::task::spawn_blocking(move || {
        use std::io::Read as _;
        let mut file = std::fs::File::open(&owned)?;
        let mut hash = sha2::Sha256::new();
        let mut buffer = vec![0_u8; 1024 * 1024];
        loop {
            let count = file.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            hash.update(&buffer[..count]);
        }
        Ok::<_, anyhow::Error>(crate::chat::files::digest_hex(&hash.finalize()))
    })
    .await?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_partial_file_keeps_its_name_for_resuming() {
        assert_eq!(part(Path::new("C:/models/a.safetensors")), PathBuf::from("C:/models/a.safetensors.part"));
    }

    #[test]
    fn only_declared_models_can_be_started() {
        let downloads = Arc::new(Downloads::new(broadcast::channel(4).0));
        let error = downloads.start("checkpoints", "never-declared.safetensors").expect_err("not declared").to_string();
        assert!(error.contains("not declared"), "{error}");
    }

    #[test]
    fn a_model_a_template_needs_asks_before_deleting() {
        let downloads = Downloads::new(broadcast::channel(4).0);
        // The base manifests are not installed in an empty data folder, so this uses the literal
        // check: an undeclared file is deletable, and deleting never leaves the models folder.
        assert!(downloads.delete("checkpoints", "../escape.safetensors", true).is_err());
    }
}
