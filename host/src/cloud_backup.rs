//! Encrypted backups to any WebDAV folder (Nextcloud, ownCloud, Box, pCloud and similar): the host
//! takes a consistent copy of its database, seals it with the computer's vault key and uploads it.
//! The server only ever sees ciphertext; the key never leaves the keychain.

use crate::hub::Hub;
use crate::market::vault;
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

pub const METHODS: &[&str] = &["cloudBackupStatus", "saveCloudBackup", "runCloudBackup"];
const KEY: &str = "cloud-backup.v1";
const FILES: &str = "cloud-backup-files.v1";
const PASSWORD: &str = "cloud-backup.password";
const AAD: &str = "cloud-backup.v1";
const DAY: i64 = 86_400_000;
static RUNNING: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Settings {
    #[serde(default)]
    url: String,
    #[serde(default)]
    username: String,
    #[serde(default)]
    enabled: bool,
    #[serde(default = "default_keep")]
    keep: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    last_at: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    last_error: Option<String>,
}

fn default_keep() -> usize {
    14
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            url: String::new(),
            username: String::new(),
            enabled: false,
            keep: default_keep(),
            last_at: None,
            last_error: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Remote {
    name: String,
    at: i64,
}

fn settings(store: &crate::store::Store) -> Settings {
    store.kv_read(KEY).ok().flatten().and_then(|text| serde_json::from_str(&text).ok()).unwrap_or_default()
}

fn save_settings(store: &crate::store::Store, settings: &Settings) -> Result<()> {
    store.kv_set(KEY, &serde_json::to_string(settings)?)
}

fn files(store: &crate::store::Store) -> Vec<Remote> {
    store.kv_read(FILES).ok().flatten().and_then(|text| serde_json::from_str(&text).ok()).unwrap_or_default()
}

fn save_files(store: &crate::store::Store, files: &[Remote]) -> Result<()> {
    store.kv_set(FILES, &serde_json::to_string(files)?)
}

fn status(store: &crate::store::Store) -> Value {
    let settings = settings(store);
    let password = vault::read(store, PASSWORD).ok().flatten().is_some_and(|password| !password.is_empty());
    let ready = !settings.url.is_empty() && (settings.username.is_empty() || password);
    json!({
        "url": settings.url,
        "username": settings.username,
        "enabled": settings.enabled,
        "keep": settings.keep,
        "lastAt": settings.last_at,
        "lastError": settings.last_error,
        "configured": ready,
        "hasPassword": password,
        "files": files(store),
    })
}

pub async fn call(hub: &Arc<Hub>, method: &str, b: &Value) -> Result<Value> {
    match method {
        "cloudBackupStatus" => Ok(status(&hub.store)),
        "saveCloudBackup" => {
            let mut settings = settings(&hub.store);
            let url = b["url"].as_str().unwrap_or_default().trim().trim_end_matches('/');
            if url.is_empty() {
                settings.url = String::new();
                settings.enabled = false;
                save_settings(&hub.store, &settings)?;
                let _ = vault::write(&hub.store, PASSWORD, "");
                return Ok(status(&hub.store));
            }
            ensure!(url.len() <= 1_000, "that address is too long");
            let parsed = reqwest::Url::parse(url).context("that isn't a web address")?;
            ensure!(
                matches!(parsed.scheme(), "http" | "https"),
                "the backup address must start with https:// or http://"
            );
            settings.url = url.to_owned();
            settings.username = b["username"].as_str().unwrap_or_default().trim().chars().take(200).collect();
            settings.enabled = b["enabled"].as_bool().unwrap_or(settings.enabled);
            settings.keep = b["keep"].as_u64().map_or(settings.keep, |keep| keep.clamp(1, 100) as usize);
            save_settings(&hub.store, &settings)?;
            if let Some(password) = b["password"].as_str().filter(|password| !password.is_empty()) {
                vault::write(&hub.store, PASSWORD, password)?;
            }
            Ok(status(&hub.store))
        }
        "runCloudBackup" => {
            let mut settings = settings(&hub.store);
            match run(hub).await {
                Ok(result) => {
                    settings.last_at = Some(crate::store::now_ms());
                    settings.last_error = None;
                    save_settings(&hub.store, &settings)?;
                    Ok(result)
                }
                Err(error) => {
                    settings.last_error = Some(format!("{error:#}"));
                    let _ = save_settings(&hub.store, &settings);
                    Err(error)
                }
            }
        }
        other => bail!("unknown backup method {other}"),
    }
}

/// A manual backup uses the saved settings even when the daily schedule is off.
async fn run(hub: &Arc<Hub>) -> Result<Value> {
    ensure!(
        RUNNING.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire).is_ok(),
        "a backup is already running"
    );
    let result = run_inner(hub).await;
    RUNNING.store(false, Ordering::Release);
    result
}

async fn run_inner(hub: &Arc<Hub>) -> Result<Value> {
    let settings = settings(&hub.store);
    ensure!(!settings.url.is_empty(), "add the folder address for your cloud first");
    let password = vault::read(&hub.store, PASSWORD)?.unwrap_or_default();
    ensure!(settings.username.is_empty() || !password.is_empty(), "add the password for that cloud account");
    let folder = crate::service::data_dir().join("backups");
    tokio::fs::create_dir_all(&folder).await.context("creating the backups folder")?;
    let temp = folder.join(format!("cloud-{}.db", uuid::Uuid::new_v4()));
    let copy = temp.clone();
    let hub_for_copy = hub.clone();
    tokio::task::spawn_blocking(move || {
        let path = copy.to_string_lossy().to_string();
        hub_for_copy.store.backup_to(&path)
    })
    .await
    .context("copying the database")??;
    let bytes = tokio::fs::read(&temp).await.context("reading the backup")?;
    let _ = tokio::fs::remove_file(&temp).await;
    ensure!(!bytes.is_empty() && bytes.len() <= 512 * 1024 * 1024, "the database is too large for a cloud backup");
    let hub_for_seal = hub.clone();
    let sealed = tokio::task::spawn_blocking(move || vault::seal(&hub_for_seal.store, AAD, &bytes))
        .await
        .context("encrypting the backup")??;
    let name = backup_name();
    let url = format!("{}/{name}", settings.url.trim_end_matches('/'));
    let client = reqwest::Client::builder().timeout(Duration::from_secs(180)).build().context("starting the upload")?;
    let size = sealed.len();
    let mut request = client.put(&url).header("Content-Type", "application/octet-stream").body(sealed);
    if !settings.username.is_empty() {
        request = request.basic_auth(&settings.username, Some(&password));
    }
    let response = request.send().await.with_context(|| format!("uploading to {url}"))?;
    if !response.status().is_success() {
        let status = response.status();
        let detail = response.text().await.unwrap_or_default();
        bail!("the cloud answered {status}: {}", response_text(&detail));
    }
    let mut uploaded = files(&hub.store);
    uploaded.push(Remote { name: name.clone(), at: crate::store::now_ms() });
    let keep = settings.keep.max(1);
    while uploaded.len() > keep {
        let old = uploaded.remove(0);
        let mut request = client.delete(format!("{}/{}", settings.url.trim_end_matches('/'), old.name));
        if !settings.username.is_empty() {
            request = request.basic_auth(&settings.username, Some(&password));
        }
        let _ = request.send().await;
    }
    save_files(&hub.store, &uploaded)?;
    Ok(json!({"name": name, "bytes": size, "at": crate::store::now_ms()}))
}

fn response_text(body: &str) -> String {
    let body = body.trim();
    if body.is_empty() { "no detail".to_owned() } else { body.chars().take(300).collect() }
}

fn backup_name() -> String {
    let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    format!("codync-{stamp}-{}.codync", &suffix[..8])
}

/// Called every few hours: one backup a day while the schedule is on.
pub async fn maybe_run(hub: &Arc<Hub>) {
    let settings = settings(&hub.store);
    if !settings.enabled || settings.url.is_empty() {
        return;
    }
    if settings.last_at.is_some_and(|last| crate::store::now_ms() - last < DAY) {
        return;
    }
    match run(hub).await {
        Ok(_) => {
            let mut settings = settings(&hub.store);
            settings.last_at = Some(crate::store::now_ms());
            settings.last_error = None;
            if let Err(error) = save_settings(&hub.store, &settings) {
                tracing::warn!(error = format!("{error:#}"), "couldn't record the cloud backup");
            }
        }
        Err(error) => {
            tracing::warn!(error = format!("{error:#}"), "cloud backup failed");
            let mut settings = settings(&hub.store);
            settings.last_error = Some(format!("{error:#}"));
            let _ = save_settings(&hub.store, &settings);
        }
    }
}

pub async fn ticker(hub: Arc<Hub>) {
    let mut tick = tokio::time::interval(Duration::from_secs(6 * 60 * 60));
    loop {
        tick.tick().await;
        maybe_run(&hub).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_backup_name_is_a_codync_file() {
        let name = backup_name();
        assert!(name.starts_with("codync-"));
        assert!(name.ends_with(".codync"));
    }

    #[test]
    fn settings_round_trip() {
        let settings =
            Settings { url: "https://cloud.example.com/backups".into(), enabled: true, ..Default::default() };
        let text = serde_json::to_string(&settings).unwrap();
        let back: Settings = serde_json::from_str(&text).unwrap();
        assert_eq!(back.url, settings.url);
        assert_eq!(back.keep, 14);
    }
}
