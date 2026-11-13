//! Read-only pages shared by link: the host keeps a self-contained HTML page under a random token
//! and serves it at `/share/{token}`. The token is the whole secret; there is no account, no cookie
//! and no way to list pages from the web. Links expire after a week and can be revoked at any time.

use crate::hub::Hub;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

const PREFIX: &str = "share-link:";
const TTL_MS: i64 = 7 * 86_400_000;
const MAX_HTML_BYTES: usize = 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareLink {
    pub token: String,
    pub title: String,
    pub html: String,
    pub created_at: i64,
    pub expires_at: i64,
}

fn key(token: &str) -> String {
    format!("{PREFIX}{token}")
}

fn valid_token(token: &str) -> bool {
    token.len() >= 32 && token.len() <= 80 && token.chars().all(|c| c.is_ascii_alphanumeric())
}

/// Stores a page and returns its token (128 random bits, so it is not guessable).
pub fn publish(hub: &Hub, title: &str, html: &str) -> Result<String> {
    ensure!(!html.trim().is_empty(), "the page is empty");
    ensure!(html.len() <= MAX_HTML_BYTES, "the page is larger than 1 MiB");
    prune(hub);
    let token = format!("{}{}", uuid::Uuid::new_v4().simple(), uuid::Uuid::new_v4().simple());
    let now = crate::store::now_ms();
    let link = ShareLink {
        token: token.clone(),
        title: title.trim().chars().take(200).collect(),
        html: html.to_owned(),
        created_at: now,
        expires_at: now + TTL_MS,
    };
    hub.store.kv_set(&key(&token), &serde_json::to_string(&link)?)?;
    Ok(token)
}

/// The page behind a token, or `None` when it never existed, expired or was revoked.
pub fn get(hub: &Hub, token: &str) -> Result<Option<ShareLink>> {
    if !valid_token(token) {
        return Ok(None);
    }
    let Some(raw) = hub.store.kv_read(&key(token))? else { return Ok(None) };
    let link: ShareLink = serde_json::from_str(&raw)?;
    if link.expires_at <= crate::store::now_ms() {
        hub.store.kv_delete(&key(token))?;
        return Ok(None);
    }
    Ok(Some(link))
}

/// Removes a link now; `true` when it was still there.
pub fn revoke(hub: &Hub, token: &str) -> Result<bool> {
    if hub.store.kv_read(&key(token))?.is_none() {
        return Ok(false);
    }
    hub.store.kv_delete(&key(token))?;
    Ok(true)
}

/// Every link still valid, newest first. The API drops the page body before sending it.
pub fn list(hub: &Hub) -> Result<Vec<ShareLink>> {
    prune(hub);
    let mut links: Vec<ShareLink> = hub
        .store
        .kv_prefix(PREFIX)
        .into_iter()
        .filter_map(|key| hub.store.kv_read(&key).ok().flatten())
        .filter_map(|raw| serde_json::from_str(&raw).ok())
        .collect();
    links.sort_by_key(|link| std::cmp::Reverse(link.created_at));
    Ok(links)
}

/// Drops expired links; a link past its week never shows again, and its page is gone.
fn prune(hub: &Hub) {
    let now = crate::store::now_ms();
    for key in hub.store.kv_prefix(PREFIX) {
        let expired = hub
            .store
            .kv_read(&key)
            .ok()
            .flatten()
            .and_then(|raw| serde_json::from_str::<ShareLink>(&raw).ok())
            .is_none_or(|link| link.expires_at <= now);
        if expired && let Err(error) = hub.store.kv_delete(&key) {
            tracing::warn!(error = format!("{error:#}"), "couldn't drop an expired share link");
        }
    }
}
