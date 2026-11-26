//! A user's own Claude Code can start but never answer a sign-in check (OpenBot 0.70). Codync's
//! bundled ACP copy then answers within the same bounded wait, so Providers is not left guessing.

use super::search_path::is_executable;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// How long the user's copy gets before the bundled one takes over.
pub(crate) const FALLBACK_AFTER: Duration = Duration::from_secs(20);
/// The bundled copy's own bounded check after the switch.
pub(crate) const BUNDLED_BUDGET: Duration = Duration::from_secs(5);

/// Pure: once the user's copy has had ~20 s and still hasn't answered, use the bundled copy. A copy
/// that did answer wins however long it took, so a slow successful check is never replaced.
pub(crate) fn copy_fallback(elapsed: Duration, answered: bool) -> bool {
    !answered && elapsed >= FALLBACK_AFTER
}

/// The bundled `claude-acp` CLI, found the way `workbuddy::find` finds the WorkBuddy CLI: scan
/// known roots and take the first executable, never a binary from `PATH`.
pub(crate) fn bundled() -> Option<PathBuf> {
    find_in(&roots())
}

fn roots() -> Vec<PathBuf> {
    let base = crate::service::data_dir().join("agents").join("claude-acp");
    let mut roots: Vec<PathBuf> = match std::fs::read_dir(&base) {
        Ok(entries) => entries.filter_map(Result::ok).map(|entry| entry.path()).filter(|path| path.is_dir()).collect(),
        Err(_) => Vec::new(),
    };
    roots.sort_unstable_by(|a, b| b.cmp(a)); // newest pinned build first
    roots.push(base); // a flat, npx-style layout
    roots
}

fn find_in(roots: &[PathBuf]) -> Option<PathBuf> {
    let names: &[&str] = if cfg!(windows) { &["claude.cmd", "claude.exe"] } else { &["claude"] };
    roots.iter().flat_map(|root| names.iter().map(move |name| root.join(name))).find(|p| is_executable(p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_silent_copy_gives_way_after_twenty_seconds() {
        assert!(!copy_fallback(Duration::from_secs(19), false));
        assert!(copy_fallback(Duration::from_secs(20), false));
        assert!(copy_fallback(Duration::from_secs(60), false));
        assert!(!copy_fallback(Duration::from_secs(60), true), "an answer, however late, wins");
        assert!(!copy_fallback(Duration::from_secs(20), true));
    }

    #[test]
    fn only_an_executable_inside_a_bundled_root_counts() {
        let dir = std::env::temp_dir().join(format!("codync-claude-{}", uuid::Uuid::new_v4()));
        let version = dir.join("claude-acp").join("2026.11.26");
        let bin = version.join(if cfg!(windows) { "claude.exe" } else { "claude" });
        std::fs::create_dir_all(bin.parent().unwrap()).unwrap();
        std::fs::write(&bin, "#!/bin/sh\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        assert_eq!(find_in(&[version.clone()]).as_deref(), Some(bin.as_path()));
        let plain = dir.join("elsewhere");
        std::fs::create_dir_all(&plain).unwrap();
        let other = plain.join(if cfg!(windows) { "not-claude.exe" } else { "not-claude" });
        std::fs::write(&other, "#!/bin/sh\n").unwrap();
        assert_eq!(find_in(&[plain]), None, "an unrelated binary is never the bundled copy");
        std::fs::remove_dir_all(dir).unwrap();
    }
}
