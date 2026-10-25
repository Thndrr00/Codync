//! Browser profile manager for isolated per-bot Chromium profiles (`local` mode)
//! and confined download directories (`workspace/downloads/`).

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BackendKind {
    /// Dedicated Chromium instance with an isolated profile per bot.
    Local,
    /// Attaches to the user's running Chrome instance (`ownsAllTabs: false`).
    MyChrome,
    /// Connects to a remote/cloud CDP endpoint.
    CdpUrl,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::struct_excessive_bools, reason = "independent capability flags of one browser backend")]
pub struct BackendCaps {
    pub persistent_profile: bool,
    pub window: bool,
    pub egress_gate: bool,
    pub owns_all_tabs: bool,
    pub bot_passkeys: bool,
}

impl BackendKind {
    pub fn capabilities(self) -> BackendCaps {
        match self {
            Self::Local => BackendCaps {
                persistent_profile: true,
                window: true,
                egress_gate: true,
                owns_all_tabs: true,
                bot_passkeys: false,
            },
            Self::MyChrome => BackendCaps {
                persistent_profile: true,
                window: true,
                egress_gate: false,
                owns_all_tabs: false,
                bot_passkeys: true,
            },
            Self::CdpUrl => BackendCaps {
                persistent_profile: false,
                window: false,
                egress_gate: false,
                owns_all_tabs: true,
                bot_passkeys: false,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BotBrowserProfile {
    pub bot_id: String,
    pub profile_dir: PathBuf,
    pub download_dir: PathBuf,
    pub last_browser_version: Option<String>,
}

/// Ensures the isolated Chromium profile directory and confined download folder
/// exist for `bot_id`, and refuses profile version downgrades.
pub fn ensure_bot_profile(
    workspaces_root: &Path,
    bot_id: &str,
    current_version: Option<&str>,
) -> Result<BotBrowserProfile> {
    if bot_id.is_empty() || bot_id.contains("..") || bot_id.contains('/') || bot_id.contains('\\') {
        bail!("invalid bot_id for browser profile");
    }
    let bot_root = workspaces_root.join(bot_id);
    let profile_dir = bot_root.join("browser-profile");
    let download_dir = bot_root.join("downloads");
    std::fs::create_dir_all(&profile_dir)
        .with_context(|| format!("failed to create browser profile dir {}", profile_dir.display()))?;
    std::fs::create_dir_all(&download_dir)
        .with_context(|| format!("failed to create download dir {}", download_dir.display()))?;

    let meta_path = profile_dir.join("codync-profile.json");
    let previous: Option<BotBrowserProfile> =
        std::fs::read_to_string(&meta_path).ok().and_then(|s| serde_json::from_str(&s).ok());

    if let (Some(prev), Some(curr)) =
        (previous.as_ref().and_then(|p| p.last_browser_version.as_deref()), current_version)
        && major_version(curr) < major_version(prev)
    {
        bail!("refused(profile_downgrade): profile was used with Chrome {prev}, cannot downgrade to {curr}");
    }

    let profile = BotBrowserProfile {
        bot_id: bot_id.to_owned(),
        profile_dir,
        download_dir,
        last_browser_version: current_version
            .map(str::to_owned)
            .or_else(|| previous.and_then(|p| p.last_browser_version)),
    };
    if let Ok(serialized) = serde_json::to_string_pretty(&profile) {
        let _ = std::fs::write(&meta_path, serialized);
    }
    Ok(profile)
}

fn major_version(v: &str) -> u32 {
    v.split('.').next().and_then(|s| s.parse().ok()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_profile_and_blocks_major_downgrade() {
        let dir = std::env::temp_dir().join(format!("codync-browser-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let p1 = ensure_bot_profile(&dir, "bot-alpha", Some("135.0.1.0")).unwrap();
        assert!(p1.profile_dir.ends_with("browser-profile"));
        assert!(p1.download_dir.ends_with("downloads"));

        // Upgrade succeeds
        assert!(ensure_bot_profile(&dir, "bot-alpha", Some("136.0.0.0")).is_ok());
        // Downgrade refused
        assert!(ensure_bot_profile(&dir, "bot-alpha", Some("134.0.0.0")).is_err());
        assert!(BackendKind::Local.capabilities().owns_all_tabs);
        assert!(!BackendKind::MyChrome.capabilities().owns_all_tabs);
        assert!(BackendKind::CdpUrl.capabilities().owns_all_tabs);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
