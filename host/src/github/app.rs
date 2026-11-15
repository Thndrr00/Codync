//! A GitHub App installed on chosen repositories: the App's own signed webhooks and its installation
//! tokens replace the token of one person for routine triggers.
//!
//! Nothing here runs on its own: a delivery proves the App's webhook secret (with the same check the
//! routine endpoint uses), the `delivery` id is remembered against replays, and only the repositories a
//! person mapped reach a bot's routines.

use crate::LockExt;
use crate::routines::hooks::{self, Headers, MAX_BODY, Refused};
use crate::store::{Store, now_ms};
use anyhow::{Context, Result, bail};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::OnceLock;

const SLOT: &str = "github.app";
/// Repositories one App installation may map; the App's own limit is 500.
pub const MAX_REPOS: usize = 500;
pub const MAX_INSTALLATIONS: usize = 50;
/// A delivery id repeated inside this window is a replay; GitHub retries keep it.
pub const REPLAY_WINDOW_MS: i64 = 10 * 60 * 1000;
/// Deliveries older than this leave the replay table.
const KEEP_DELIVERIES_MS: i64 = 24 * 3600 * 1000;
/// An installation token is reused until shortly before it expires.
const TOKEN_SKEW_MS: i64 = 5 * 60 * 1000;
/// The events a routine trigger can match; GitHub sends many others.
pub const EVENTS: &[&str] = &["pull_request", "pull_request_review", "check_run", "issues"];

pub const METHODS: &[&str] = &[
    "githubAppStatus",
    "setGitHubApp",
    "disconnectGitHubApp",
    "githubInstallations",
    "mapGitHubRepo",
    "unmapGitHubRepo",
];

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Config {
    pub app_id: String,
    #[serde(default)]
    pub slug: String,
    pub private_key: String,
    pub webhook_secret: String,
    #[serde(default)]
    pub installations: Vec<Installation>,
    /// Repository full name to the bots and branches it reaches. Empty branches mean every branch.
    #[serde(default)]
    pub repos: BTreeMap<String, RepoMap>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Installation {
    pub id: String,
    pub account: String,
    #[serde(default)]
    pub repos: Vec<String>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RepoMap {
    #[serde(default)]
    pub bots: Vec<String>,
    #[serde(default)]
    pub branches: Vec<String>,
}

#[derive(Clone, Serialize)]
struct Token {
    token: String,
    expires_at: i64,
}

static TOKENS: OnceLock<Mutex<BTreeMap<String, Token>>> = OnceLock::new();

fn tokens() -> &'static Mutex<BTreeMap<String, Token>> {
    TOKENS.get_or_init(|| Mutex::new(BTreeMap::new()))
}

/// The replay table, next to the connections this vault-enabled host already keeps.
pub fn open(connection: &rusqlite::Connection) -> Result<()> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS github_deliveries(delivery TEXT PRIMARY KEY, at INTEGER NOT NULL);",
    )?;
    Ok(())
}

pub fn load(store: &Store) -> Result<Option<Config>> {
    crate::market::vault::read(store, SLOT)?
        .filter(|s| !s.is_empty())
        .map(|s| serde_json::from_str(&s).map_err(Into::into))
        .transpose()
}

fn save(store: &Store, config: &Config) -> Result<()> {
    crate::market::vault::write(store, SLOT, &serde_json::to_string(config)?)
}

pub(crate) fn configuration_state(store: &Store) -> &'static str {
    match load(store) {
        Ok(Some(_)) => "configured",
        Ok(None) => "notConfigured",
        Err(_) => "unavailable",
    }
}

fn app_id_ok(id: &str) -> bool {
    !id.is_empty() && id.len() <= 20 && id.bytes().all(|b| b.is_ascii_digit())
}

fn secret_ok(secret: &str) -> bool {
    (16..=200).contains(&secret.len()) && secret.bytes().all(|b| b.is_ascii_graphic())
}

fn key_ok(key: &str) -> bool {
    key.contains("BEGIN") && key.contains("PRIVATE KEY") && key.len() <= 16 * 1024
}

fn repo_ok(repo: &str) -> bool {
    let (owner, name) = repo.split_once('/').unwrap_or_default();
    !owner.is_empty()
        && !name.is_empty()
        && repo.len() <= 140
        && repo.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'/'))
}

/// What the settings page shows: whether an App is connected, where it is installed, which
/// repositories reach which bots, and when the next token runs out.
pub fn status(store: &Store) -> Value {
    let config = load(store).ok().flatten();
    let Some(config) = config else {
        return json!({"configured": false, "installations": [], "repos": [], "webhookUrl": Value::Null});
    };
    let mut expires = None;
    for installation in &config.installations {
        if let Some(token) = tokens().locked().get(&installation.id)
            && expires.is_none_or(|at: i64| token.expires_at > at)
        {
            expires = Some(token.expires_at);
        }
    }
    json!({
        "configured": true,
        "appId": config.app_id,
        "slug": config.slug,
        "installations": config.installations,
        "repos": config.repos.iter().map(|(repo, map)| json!({"repo": repo, "bots": map.bots, "branches": map.branches})).collect::<Vec<_>>(),
        "events": EVENTS,
        "tokenExpiresAt": expires,
    })
}

/// A JWT the App signs as itself; GitHub accepts it for minutes.
fn app_jwt(config: &Config) -> Result<String> {
    let now = now_ms() / 1000;
    let claims = json!({"iat": now - 60, "exp": now + 540, "iss": config.app_id});
    let key = jsonwebtoken::EncodingKey::from_rsa_pem(config.private_key.as_bytes())
        .context("The GitHub App private key is not an RSA PEM key.")?;
    jsonwebtoken::encode(&jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256), &claims, &key)
        .context("Could not sign the GitHub App request.")
}

fn client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(20))
        .build()?)
}

/// Every installation of the App and the repositories it may reach, refreshed from GitHub.
async fn installations(config: &Config) -> Result<Vec<Installation>> {
    let jwt = app_jwt(config)?;
    let client = client()?;
    let response = client
        .get("https://api.github.com/app/installations?per_page=100")
        .header("Accept", "application/vnd.github+json")
        .header("Authorization", format!("Bearer {jwt}"))
        .header("User-Agent", "codync-host")
        .send()
        .await?;
    if !response.status().is_success() {
        bail!("GitHub refused the App request ({}).", response.status().as_u16());
    }
    let body: Value = response.json().await?;
    let jwt = app_jwt(config)?;
    let mut out = Vec::new();
    for installation in body.as_array().map(Vec::as_slice).unwrap_or_default() {
        let id = installation["id"]
            .as_i64()
            .map(|id| id.to_string())
            .context("GitHub sent an installation without an id")?;
        let account = installation["account"]["login"].as_str().unwrap_or_default().to_owned();
        let response = client
            .get(format!("https://api.github.com/user/installations/{id}/repositories?per_page=100"))
            .header("Accept", "application/vnd.github+json")
            .header("Authorization", format!("Bearer {jwt}"))
            .header("User-Agent", "codync-host")
            .send()
            .await?;
        let repos = if response.status().is_success() {
            let body: Value = response.json().await?;
            body["repositories"]
                .as_array()
                .map(|repos| {
                    repos
                        .iter()
                        .filter_map(|repo| repo["full_name"].as_str().map(str::to_owned))
                        .take(MAX_REPOS)
                        .collect()
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        if out.len() >= MAX_INSTALLATIONS {
            break;
        }
        out.push(Installation { id, account, repos });
    }
    Ok(out)
}

/// An installation token, minted on demand and reused until shortly before it expires.
pub async fn installation_token(store: &Store, installation: &str) -> Result<String> {
    if let Some(token) = tokens().locked().get(installation)
        && now_ms() < token.expires_at - TOKEN_SKEW_MS
    {
        return Ok(token.token.clone());
    }
    let config = load(store)?.context("No GitHub App is connected.")?;
    let jwt = app_jwt(&config)?;
    let response = client()?
        .post(format!("https://api.github.com/app/installations/{installation}/access_tokens"))
        .header("Accept", "application/vnd.github+json")
        .header("Authorization", format!("Bearer {jwt}"))
        .header("User-Agent", "codync-host")
        .send()
        .await?;
    if !response.status().is_success() {
        bail!("GitHub refused an installation token ({}).", response.status().as_u16());
    }
    let body: Value = response.json().await?;
    let token = body["token"].as_str().context("GitHub sent no installation token")?.to_owned();
    let expires_at = body["expires_at"]
        .as_str()
        .and_then(|at| chrono::DateTime::parse_from_rfc3339(at).ok())
        .map_or_else(|| now_ms() + 50 * 60 * 1000, |at| at.timestamp_millis());
    tokens().locked().insert(installation.to_owned(), Token { token: token.clone(), expires_at });
    Ok(token)
}

/// A new App from what GitHub gave the manifest flow: validated, then its installations read.
async fn prepare(body: &Value) -> Result<Config> {
    let app_id = body["appId"].as_str().unwrap_or_default().trim().to_owned();
    let private_key = body["privateKey"].as_str().unwrap_or_default().trim().to_owned();
    let webhook_secret = body["webhookSecret"].as_str().unwrap_or_default().trim().to_owned();
    if !app_id_ok(&app_id) {
        bail!("Give the App's numeric id.");
    }
    if !key_ok(&private_key) {
        bail!("Paste the App's private key (a PEM file).");
    }
    if !secret_ok(&webhook_secret) {
        bail!("Give a webhook secret of at least 16 visible characters.");
    }
    let mut config = Config {
        app_id,
        slug: body["slug"].as_str().unwrap_or_default().trim().to_owned(),
        private_key,
        webhook_secret,
        installations: Vec::new(),
        repos: BTreeMap::new(),
    };
    config.installations = installations(&config).await.unwrap_or_default();
    Ok(config)
}

/// Maps one repository to bots, replacing any earlier mapping and leaving its branches alone.
fn map(store: &Store, body: &Value) -> Result<()> {
    let repo = body["repo"].as_str().unwrap_or_default().trim().to_owned();
    if !repo_ok(&repo) {
        bail!("Use a repository as owner/name.");
    }
    let Some(owner) = repo.split_once('/').map(|(owner, _)| owner) else {
        bail!("Use a repository as owner/name.");
    };
    let _ = owner;
    let mut config = load(store)?.context("Connect the GitHub App first.")?;
    let mut bots: Vec<String> = body["bots"]
        .as_array()
        .map(|bots| bots.iter().filter_map(|bot| bot.as_str().map(str::to_owned)).collect())
        .unwrap_or_default();
    if bots.is_empty() {
        bail!("Pick at least one bot for {repo}.");
    }
    for bot in &bots {
        if store.bot(bot)?.is_none() {
            bail!("Unknown bot: {bot}.");
        }
    }
    bots.sort();
    bots.dedup();
    let branches: Vec<String> = body["branches"]
        .as_array()
        .map(|branches| {
            branches
                .iter()
                .filter_map(|branch| branch.as_str().map(str::trim).filter(|b| !b.is_empty()).map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    let known = config.installations.iter().any(|i| i.repos.iter().any(|r| r == &repo));
    if !known {
        bail!("{repo} is not on any installation of this App.");
    }
    if !config.repos.contains_key(&repo) && config.repos.len() >= MAX_REPOS {
        bail!("{MAX_REPOS} repositories are already mapped.");
    }
    config.repos.insert(repo, RepoMap { bots, branches });
    save(store, &config)
}

fn unmap(store: &Store, body: &Value) -> Result<()> {
    let repo = body["repo"].as_str().unwrap_or_default();
    let mut config = load(store)?.context("Connect the GitHub App first.")?;
    if config.repos.remove(repo).is_none() {
        bail!("{repo} is not mapped.");
    }
    save(store, &config)
}

pub(crate) async fn call(hub: &std::sync::Arc<crate::hub::Hub>, method: &str, body: Value) -> Result<Value> {
    crate::market::vault::unlock(hub.clone()).await?;
    match method {
        "setGitHubApp" => {
            let config = prepare(&body).await?;
            let _guard = hub.store.connector_lock.locked();
            save(&hub.store, &config)?;
        }
        "disconnectGitHubApp" => {
            let _guard = hub.store.connector_lock.locked();
            save(
                &hub.store,
                &Config {
                    app_id: String::new(),
                    slug: String::new(),
                    private_key: String::new(),
                    webhook_secret: String::new(),
                    installations: Vec::new(),
                    repos: BTreeMap::new(),
                },
            )?;
            tokens().locked().clear();
        }
        "githubInstallations" => {
            let config = load(&hub.store)?.context("Connect the GitHub App first.")?;
            let mut config = config;
            config.installations = installations(&config).await?;
            let _guard = hub.store.connector_lock.locked();
            // The App may have changed twice meanwhile: keep what is stored now.
            let mut stored = load(&hub.store)?.context("Connect the GitHub App first.")?;
            stored.installations = config.installations;
            // A repository no installation carries can't run any more.
            let live: Vec<&String> = stored.installations.iter().flat_map(|i| i.repos.iter()).collect();
            stored.repos.retain(|repo, _| live.contains(&repo));
            save(&hub.store, &stored)?;
        }
        "mapGitHubRepo" => {
            let _guard = hub.store.connector_lock.locked();
            map(&hub.store, &body)?;
        }
        "unmapGitHubRepo" => {
            let _guard = hub.store.connector_lock.locked();
            unmap(&hub.store, &body)?;
        }
        _ => bail!("Unknown GitHub App method."),
    }
    Ok(status(&hub.store))
}

/// The secret the App's deliveries are signed with, for the relay's hook list.
pub fn hook(store: &Store) -> Option<Value> {
    let config = load(store).ok().flatten()?;
    Some(json!({"id": "github", "key": config.webhook_secret, "enabled": true}))
}

/// One delivery from GitHub: signed, inside the replay window, on a mapped repository.
///
/// The answer names how many routines it queued; a replay answers `duplicate` without queueing.
pub fn receive(hub: &crate::hub::Hub, headers: &Headers, body: &[u8]) -> Result<Value, Refused> {
    if body.len() > MAX_BODY {
        return Err(Refused::Invalid("event too large".into()));
    }
    let config = load(&hub.store)
        .ok()
        .flatten()
        .filter(|c| hooks::authorized(&c.webhook_secret, headers, body))
        .ok_or(Refused::Unauthorized)?;
    let name = headers.get("x-github-event").map(String::as_str).unwrap_or_default();
    if name == "ping" {
        return Ok(json!({"ping": true}));
    }
    if !EVENTS.contains(&name) {
        return Ok(json!({"ignored": "event does not reach routines"}));
    }
    let delivery = hooks::delivery_id(headers)?;
    if let Some(id) = delivery.as_deref() {
        let fresh = remember(&hub.store, id).map_err(|e| Refused::Invalid(format!("{e:#}")))?;
        if !fresh {
            return Ok(json!({"duplicate": true}));
        }
    }
    let Some(event) = hooks::event(headers, body)? else {
        return Ok(json!({"ping": true}));
    };
    let repo = event["repo"].as_str().unwrap_or_default();
    let Some(map) = config.repos.get(repo) else {
        return Ok(json!({"ignored": "repository is not mapped"}));
    };
    if !map.branches.is_empty() {
        let branch = branch_of(&event["payload"]);
        if !branch.is_some_and(|branch| map.branches.iter().any(|b| b == branch)) {
            return Ok(json!({"ignored": "branch is not mapped"}));
        }
    }
    let mut queued = 0;
    for bot in &map.bots {
        queued += hub.routines.receive_event_for(&hub.store, Some(bot), &event, delivery.clone()).len();
    }
    Ok(json!({"queued": queued}))
}

fn branch_of(payload: &Value) -> Option<&str> {
    [
        &payload["pull_request"]["base"]["ref"],
        &payload["pull_request"]["head"]["ref"],
        &payload["check_suite"]["head_branch"],
        &payload["ref"],
    ]
    .into_iter()
    .find_map(|branch| branch.as_str())
}

/// Marks a delivery id, answering whether it is new inside the replay window.
fn remember(store: &Store, delivery: &str) -> Result<bool> {
    let now = now_ms();
    let connection = store.db.locked();
    let seen: Option<i64> =
        connection.query_row("SELECT at FROM github_deliveries WHERE delivery = ?", [delivery], |row| row.get(0)).ok();
    if let Some(at) = seen
        && now - at <= REPLAY_WINDOW_MS
    {
        return Ok(false);
    }
    connection.execute(
        "INSERT INTO github_deliveries(delivery, at) VALUES(?, ?) ON CONFLICT(delivery) DO UPDATE SET at = ?",
        params![delivery, now, now],
    )?;
    connection.execute("DELETE FROM github_deliveries WHERE at < ?", [now - KEEP_DELIVERIES_MS])?;
    Ok(true)
}

#[cfg(test)]
mod tests;
