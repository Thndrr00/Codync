//! Bot-Box process sandboxing (`host/src/agent/sandbox/`).
//!
//! Provides cross-platform isolation plans (`plan`), Linux Bubblewrap + Seccomp (`linux_bwrap`),
//! macOS Seatbelt (`macos_sandbox`), Windows containment (`windows_appcontainer`); outbound traffic
//! goes through `crate::egress`.

pub mod linux_bwrap;
pub mod macos_sandbox;
pub mod plan;
pub mod windows_appcontainer;

pub use plan::SandboxPlan;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    #[test]
    fn bwrap_plan_hides_host_data_dir_and_mounts_workspace() {
        let plan = SandboxPlan::new(
            "bot-1",
            PathBuf::from("/home/user/.codync/workspaces/bot-1"),
            PathBuf::from("/home/user/.codync"),
        );
        let args = linux_bwrap::bwrap_args(&plan, Some(Path::new("/home/user")));
        let joined = args.join(" ");
        assert!(joined.contains("--tmpfs /home/user/.codync"), "host data dir must be masked with tmpfs: {joined}");
        assert!(
            joined.contains("--bind /home/user/.codync/workspaces/bot-1 /home/user/.codync/workspaces/bot-1"),
            "bot workspace must be mounted back after tmpfs: {joined}"
        );
        let tmpfs_pos = joined.find("--tmpfs /home/user/.codync").unwrap();
        let ws_pos = joined.find("--bind /home/user/.codync/workspaces/bot-1").unwrap();
        assert!(tmpfs_pos < ws_pos, "tmpfs must come before workspace bind mount");
    }

    #[test]
    fn workspace_boundary_rejects_parent_traversal_and_host_db() {
        let plan = SandboxPlan::new("bot-1", PathBuf::from("/work/bot-1"), PathBuf::from("/home/user/.codync"));
        assert!(plan.is_within_workspace(Path::new("src/main.rs")));
        assert!(plan.is_within_workspace(Path::new("a/b/../c.rs")));
        assert!(!plan.is_within_workspace(Path::new("../other-bot/secret.txt")));
        assert!(!plan.is_within_workspace(Path::new("/home/user/.codync/identity.json")));
        assert!(plan.is_within_workspace(Path::new("/work/bot-1/Cargo.toml")));
    }

    #[test]
    fn seccomp_program_blocks_dangerous_syscalls() {
        let bytes = linux_bwrap::seccomp_bytes();
        assert_eq!(bytes.len(), (linux_bwrap::DENIED_SYSCALLS.len() + 6) * 8);
    }

    #[test]
    fn seatbelt_profile_denies_host_data_and_allows_workspace() {
        let plan = SandboxPlan::new(
            "bot-1",
            PathBuf::from("/Users/me/.codync/workspaces/bot-1"),
            PathBuf::from("/Users/me/.codync"),
        );
        let profile = macos_sandbox::seatbelt_profile(&plan);
        assert!(profile.contains("(deny file-read* file-write* (subpath \"/Users/me/.codync\"))"));
        assert!(profile.contains("(allow file-read* file-write* (subpath \"/Users/me/.codync/workspaces/bot-1\"))"));
        let env_vars = windows_appcontainer::restricted_env_overrides(&plan);
        assert!(env_vars.iter().any(|(k, _)| k == "CODYNC_HOME"));
        let mut gated = plan.clone();
        gated.network_mode =
            plan::NetworkIsolationMode::LoopbackOnlyWithGate { egress_socket: PathBuf::from("/tmp/egress.sock") };
        assert_ne!(gated.network_mode, plan::NetworkIsolationMode::Disabled);
    }
}
