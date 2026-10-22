//! Tab ownership guard (`ownsAllTabs: false` for `my-chrome` mode).
//!
//! Ensures that when a bot attaches to the user's personal Chrome browser, it
//! can only interact with tabs that the bot itself opened, and blocks access to
//! internal browser configuration schemes (`chrome://*`, `edge://*`, `devtools://*`).

use anyhow::{Result, bail};
use std::collections::HashSet;

const BLOCKED_SCHEMES: &[&str] = &[
    "chrome://",
    "edge://",
    "brave://",
    "devtools://",
    "chrome-extension://",
    "view-source:",
    "javascript:",
    "file://",
];

/// Validates that a URL does not target internal browser settings or local file schemes.
pub fn validate_navigation_url(url: &str) -> Result<()> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        bail!("navigation URL cannot be empty");
    }
    let lower = trimmed.to_ascii_lowercase();
    for scheme in BLOCKED_SCHEMES {
        if lower.starts_with(scheme) {
            bail!(
                "refused(blocked_scheme): navigation to '{scheme}*' is forbidden by Codync browser security"
            );
        }
    }
    Ok(())
}

/// Tracks which CDP target/tab IDs a bot is permitted to control.
#[derive(Debug, Clone)]
pub struct TabGuard {
    owns_all_tabs: bool,
    owned_targets: HashSet<String>,
}

impl TabGuard {
    pub fn new(owns_all_tabs: bool) -> Self {
        Self {
            owns_all_tabs,
            owned_targets: HashSet::new(),
        }
    }

    pub fn owns_all_tabs(&self) -> bool {
        self.owns_all_tabs
    }

    /// Registers a newly created tab/target ID as owned by this bot.
    pub fn register_tab(&mut self, target_id: impl Into<String>) {
        self.owned_targets.insert(target_id.into());
    }

    /// Removes a closed tab/target ID from the owned set.
    pub fn unregister_tab(&mut self, target_id: &str) {
        self.owned_targets.remove(target_id);
    }

    /// Checks whether the bot is allowed to act on `target_id`.
    pub fn check_access(&self, target_id: &str) -> Result<()> {
        if self.owns_all_tabs || self.owned_targets.contains(target_id) {
            Ok(())
        } else {
            bail!(
                "refused(unowned_tab): target '{target_id}' belongs to the user's personal session and was not opened by this bot"
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_internal_browser_schemes() {
        assert!(validate_navigation_url("https://example.com").is_ok());
        assert!(validate_navigation_url("chrome://settings").is_err());
        assert!(validate_navigation_url("EDGE://flags").is_err());
        assert!(validate_navigation_url("chrome-extension://abcdef/popup.html").is_err());
        assert!(validate_navigation_url("file:///etc/passwd").is_err());
    }

    #[test]
    fn my_chrome_guard_rejects_unowned_tabs() {
        let mut guard = TabGuard::new(false);
        guard.register_tab("tab-bot-1");
        assert!(guard.check_access("tab-bot-1").is_ok());
        assert!(guard.check_access("tab-user-personal").is_err());
    }

    #[test]
    fn local_isolated_browser_allows_all_tabs() {
        let guard = TabGuard::new(true);
        assert!(guard.check_access("any-tab").is_ok());
    }
}
