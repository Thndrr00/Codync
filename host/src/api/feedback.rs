//! Sending a bug report or an idea straight from a client. The agent's tool
//! (`market::feedback`) asks for approval on a chat card; here the client's own dialog is the
//! approval, so the report goes to the same public repository as soon as this is called.

use crate::hub::Hub;
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::sync::Arc;

/// Codync's own repository, where these reports go.
const REPO: &str = "Thndrr00/Codync";

/// Posts a report already written and approved in the client. `details` is optional.
pub(super) async fn send(hub: &Arc<Hub>, b: &Value) -> Result<Value> {
    let kind = b["kind"].as_str().filter(|k| *k == "bug" || *k == "idea").context("kind must be bug or idea")?;
    let title = b["title"].as_str().map(str::trim).filter(|t| !t.is_empty()).context("a title is required")?;
    let text = b["text"].as_str().map(str::trim).filter(|t| !t.is_empty()).context("a report is required")?;
    let details = b["details"].as_str().map(str::trim).unwrap_or_default();
    let body = if details.is_empty() { text.to_owned() } else { format!("{text}\n\n{details}") };
    let token = crate::github::connection(&hub.store)?.context("GitHub is not configured")?.token;
    let url = format!("https://api.github.com/repos/{REPO}/issues");
    let client = crate::egress::client(crate::egress::Policy::Only(&["api.github.com"]), &url).await?;
    let response = client
        .post(&url)
        .bearer_auth(token)
        .header("accept", "application/vnd.github+json")
        .header("user-agent", "codync")
        .json(&json!({"title": format!("[{kind}] {title}"), "body": body}))
        .timeout(std::time::Duration::from_secs(20))
        .send()
        .await?;
    if !response.status().is_success() {
        bail!("GitHub answered {}", response.status());
    }
    let issue: Value = response.json().await.unwrap_or_else(|_| json!({}));
    Ok(json!({"url": issue["html_url"].clone()}))
}
