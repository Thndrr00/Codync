//! A GitHub token the host keeps in its vault, so a workflow can comment, label or open an issue on
//! the user's behalf. A fine-grained token with the repository permissions the outputs need is enough;
//! no GitHub App install is involved.
//!
//! Creating a repository needs the Administration permission (write) on a fine-grained token, or the
//! `repo` scope on a classic one. The repository is created on the account the token belongs to, and
//! the workspace that asked for it only gets an `origin` remote when it has none yet.

pub(crate) mod island;
#[cfg(test)]
mod tests;

use crate::{LockExt, hub::Hub, store::Store};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;

const SLOT: &str = "github.connection";
pub const METHODS: &[&str] = &["githubStatus", "setGitHubToken", "disconnectGitHub", "githubCreateRepo"];
/// GitHub's own limit on a repository name.
const MAX_REPO_NAME: usize = 100;
const MAX_DESCRIPTION: usize = 350;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Config {
    pub token: String,
    pub login: String,
    /// What the token may do, as GitHub reports it.
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(default)]
    pub generation: String,
}

fn load(store: &Store) -> Result<Option<Config>> {
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

fn token_ok(token: &str) -> bool {
    let token = token.trim();
    (20..=255).contains(&token.len()) && token.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// A repository name GitHub accepts: letters, digits, `-`, `_` and `.`, not starting with a dot.
fn repo_name_ok(name: &str) -> bool {
    let name = name.trim();
    !name.is_empty()
        && name.len() <= MAX_REPO_NAME
        && !name.starts_with('.')
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

/// Whether the token may create repositories. A classic token lists `repo` among its scopes; a
/// fine-grained one lists none, so its Administration permission is only proven by trying.
fn can_create_repos(c: Option<&Config>) -> bool {
    c.is_some_and(|c| c.scopes.is_empty() || c.scopes.iter().any(|s| s == "repo"))
}

fn client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(20))
        .build()?)
}

/// Asks GitHub who the token belongs to and what it may do; nothing is saved on a refusal.
async fn verify(token: &str) -> Result<(String, Vec<String>)> {
    let client = client()?;
    let response = client
        .get("https://api.github.com/user")
        .header("authorization", format!("Bearer {token}"))
        .header("accept", "application/vnd.github+json")
        .header("x-github-api-version", "2022-11-28")
        .header("user-agent", "codync")
        .send()
        .await?;
    if !response.status().is_success() {
        bail!("GitHub refused the token ({}).", response.status());
    }
    let scopes: Vec<String> = response
        .headers()
        .get("x-oauth-scopes")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.split(',').map(|s| s.trim().to_owned()).filter(|s| !s.is_empty()).collect())
        .unwrap_or_default();
    let body: Value = response.json().await?;
    let login = body["login"].as_str().filter(|l| !l.is_empty()).context("GitHub sent no login for that token")?;
    Ok((login.to_owned(), scopes))
}

/// Creates a repository on the token's account. Nothing is created here; GitHub owns the name.
async fn create_repo(token: &str, name: &str, private: bool, description: &str) -> Result<Value> {
    let body = json!({"name": name, "private": private, "description": description, "auto_init": false});
    let response = client()?
        .post("https://api.github.com/user/repos")
        .bearer_auth(token)
        .header("accept", "application/vnd.github+json")
        .header("x-github-api-version", "2022-11-28")
        .header("user-agent", "codync")
        .json(&body)
        .send()
        .await?;
    let status = response.status();
    let value: Value = response.json().await.unwrap_or(Value::Null);
    if !status.is_success() {
        let reason = value["message"].as_str().unwrap_or("no reason given");
        let errors: Vec<&str> =
            value["errors"].as_array().into_iter().flatten().filter_map(|e| e["message"].as_str()).collect();
        let detail = if errors.is_empty() { reason.to_owned() } else { format!("{reason}: {}", errors.join(", ")) };
        bail!("GitHub would not create the repository ({status}): {detail}");
    }
    Ok(json!({
        "fullName": value["full_name"],
        "htmlUrl": value["html_url"],
        "cloneUrl": value["clone_url"],
    }))
}

/// Points the workspace at the new repository when it has no `origin` yet; an existing one stays.
async fn set_origin(cwd: &str, url: &str) -> bool {
    let dir = std::path::Path::new(cwd);
    if !dir.is_dir() {
        return false;
    }
    if crate::chat::calls::project::git_output(dir, &["remote", "get-url", "origin"]).await.is_ok() {
        return false;
    }
    crate::chat::calls::project::git_output(dir, &["remote", "add", "origin", url]).await.is_ok()
}

pub(crate) async fn call(hub: &Arc<Hub>, method: &str, body: Value) -> Result<Value> {
    crate::market::vault::unlock(hub.clone()).await?;
    let token = body["token"].as_str().unwrap_or_default().trim().to_owned();
    let prepared = if method == "setGitHubToken" {
        if !token_ok(&token) {
            bail!("Paste a GitHub token (20–255 letters, digits, - or _).");
        }
        Some(verify(&token).await?)
    } else {
        None
    };
    let name = body["name"].as_str().unwrap_or_default().trim().to_owned();
    let private = body["private"].as_bool().unwrap_or(true);
    let description: String =
        body["description"].as_str().unwrap_or_default().trim().chars().take(MAX_DESCRIPTION).collect();
    let cwd = body["cwd"].as_str().unwrap_or_default().trim().to_owned();
    // The network round trip and Git run outside the connector lock, so other connector work goes on.
    if method == "githubCreateRepo" {
        if !repo_name_ok(&name) {
            bail!("Name the repository with letters, digits, - and . (up to {MAX_REPO_NAME}).");
        }
        let config = load(&hub.store)?.filter(|c| !c.token.is_empty()).context("Connect GitHub first.")?;
        let mut repo = create_repo(&config.token, &name, private, &description).await?;
        let url = repo["cloneUrl"].as_str().unwrap_or_default().to_owned();
        let _guard = hub.store.connector_lock.locked();
        repo["originSet"] = json!(!cwd.is_empty() && set_origin(&cwd, &url).await);
        let mut out = status(load(&hub.store)?.as_ref());
        out["created"] = repo;
        return Ok(out);
    }
    let _guard = hub.store.connector_lock.locked();
    match method {
        "setGitHubToken" => {
            let (login, scopes) = prepared.context("Missing GitHub token.")?;
            save(&hub.store, &Config { token, login, scopes, generation: uuid::Uuid::new_v4().to_string() })?;
        }
        "disconnectGitHub" => {
            save(
                &hub.store,
                &Config { token: String::new(), login: String::new(), scopes: Vec::new(), generation: String::new() },
            )?;
        }
        "githubStatus" => {}
        _ => bail!("Unknown GitHub method."),
    }
    Ok(status(load(&hub.store)?.as_ref()))
}

fn status(c: Option<&Config>) -> Value {
    let live = c.filter(|c| !c.token.is_empty());
    json!({
        "configured": live.is_some(),
        "login": live.map(|c| &c.login),
        "scopes": live.map_or(&[][..], |c| c.scopes.as_slice()),
        "canCreateRepos": can_create_repos(live),
    })
}

/// The token and the account it belongs to, for a caller that needs to make a request.
pub(crate) fn connection(store: &Store) -> Result<Option<Config>> {
    Ok(load(store)?.filter(|c| !c.token.is_empty()))
}
