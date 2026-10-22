//! Chrome DevTools Protocol (CDP) JSON-RPC envelope builder and command dispatcher.

use super::passkey_detector::{BrowserPlace, PasskeyWall, passkey_notice, passkey_wall_at, passkey_wall_in};
use super::tab_guard::{TabGuard, validate_navigation_url};
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdpCommand {
    pub id: u64,
    pub method: String,
    #[serde(default)]
    pub params: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserObservation {
    pub target_id: String,
    pub url: String,
    pub title: String,
    pub snapshot_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passkey_wall: Option<PasskeyWall>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passkey_notice: Option<String>,
}

/// Stateful CDP session helper that enforces `TabGuard`, URL scheme blocking,
/// download directory confinement, and automatic passkey wall annotation.
pub struct CdpClient {
    next_id: AtomicU64,
    guard: TabGuard,
    place: BrowserPlace,
}

impl CdpClient {
    pub fn new(owns_all_tabs: bool, place: BrowserPlace) -> Self {
        Self {
            next_id: AtomicU64::new(1),
            guard: TabGuard::new(owns_all_tabs),
            place,
        }
    }

    pub fn guard_mut(&mut self) -> &mut TabGuard {
        &mut self.guard
    }

    fn alloc_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Builds a `Target.createTarget` command after validating the URL scheme.
    pub fn build_open_tab(&self, url: &str) -> Result<CdpCommand> {
        validate_navigation_url(url)?;
        Ok(CdpCommand {
            id: self.alloc_id(),
            method: "Target.createTarget".into(),
            params: json!({ "url": url, "background": !self.guard.owns_all_tabs() }),
            session_id: None,
        })
    }

    /// Builds a `Browser.setDownloadBehavior` command confining downloads to `download_dir`.
    pub fn build_confine_downloads(&self, download_dir: &Path) -> CdpCommand {
        CdpCommand {
            id: self.alloc_id(),
            method: "Browser.setDownloadBehavior".into(),
            params: json!({
                "behavior": "allow",
                "downloadPath": download_dir.to_string_lossy(),
                "eventsEnabled": true,
            }),
            session_id: None,
        }
    }

    /// Builds a `Page.navigate` command for an owned tab.
    pub fn build_navigate(&self, target_id: &str, session_id: &str, url: &str) -> Result<CdpCommand> {
        self.guard.check_access(target_id)?;
        validate_navigation_url(url)?;
        Ok(CdpCommand {
            id: self.alloc_id(),
            method: "Page.navigate".into(),
            params: json!({ "url": url }),
            session_id: Some(session_id.to_owned()),
        })
    }

    /// Builds a `Runtime.evaluate` command for clicking or typing into a selector on an owned tab.
    pub fn build_dom_action(
        &self,
        target_id: &str,
        session_id: &str,
        expression: &str,
    ) -> Result<CdpCommand> {
        self.guard.check_access(target_id)?;
        Ok(CdpCommand {
            id: self.alloc_id(),
            method: "Runtime.evaluate".into(),
            params: json!({
                "expression": expression,
                "returnByValue": true,
                "awaitPromise": true,
            }),
            session_id: Some(session_id.to_owned()),
        })
    }

    /// Evaluates a snapshot observation and attaches a PasskeyWall notice when applicable.
    pub fn inspect_observation(
        &self,
        target_id: &str,
        url: &str,
        title: &str,
        snapshot_text: &str,
    ) -> Result<BrowserObservation> {
        self.guard.check_access(target_id)?;
        if url.starts_with("chrome://") || url.starts_with("edge://") {
            bail!("refused(blocked_scheme): page switched to restricted internal browser URL");
        }
        // Only annotate passkey walls when the browser is not the user's personal Chrome (`owns_all_tabs == true`).
        let wall = if self.guard.owns_all_tabs() {
            passkey_wall_at(url).or_else(|| passkey_wall_in(snapshot_text))
        } else {
            None
        };
        let notice = wall.map(|w| passkey_notice(w, self.place));
        Ok(BrowserObservation {
            target_id: target_id.to_owned(),
            url: url.to_owned(),
            title: title.to_owned(),
            snapshot_text: snapshot_text.to_owned(),
            passkey_wall: wall,
            passkey_notice: notice,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cdp_client_enforces_tab_guard_and_passkey_detection() {
        let mut client = CdpClient::new(true, BrowserPlace::Desktop);
        client.guard_mut().register_tab("tab-1");
        let cmd = client.build_open_tab("https://accounts.google.com").unwrap();
        assert_eq!(cmd.method, "Target.createTarget");
        let nav = client.build_navigate("tab-1", "s1", "https://accounts.google.com").unwrap();
        assert_eq!(nav.method, "Page.navigate");
        let dom = client.build_dom_action("tab-1", "s1", "document.title").unwrap();
        assert_eq!(dom.method, "Runtime.evaluate");
        let dl = client.build_confine_downloads(Path::new("/tmp/downloads"));
        assert_eq!(dl.method, "Browser.setDownloadBehavior");
        client.guard_mut().unregister_tab("tab-1");
        client.guard_mut().register_tab("tab-1");

        let obs = client
            .inspect_observation(
                "tab-1",
                "https://accounts.google.com/v3/signin/challenge/pk",
                "Sign in - Google Accounts",
                "Use your passkey to continue",
            )
            .unwrap();
        assert_eq!(obs.passkey_wall, Some(PasskeyWall::Google));
        assert!(obs.passkey_notice.as_deref().unwrap_or_default().contains("Try another way"));
    }
}
