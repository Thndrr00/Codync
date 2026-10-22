//! Sandbox plan and mount rules (`host/src/agent/sandbox/plan.rs`).
//!
//! Defines the immutable isolation plan for an agent process: hiding `host_data_dir`
//! (`~/.codync`, including `identity.json` and `codync.db`) behind an empty private mount
//! while binding back only the bot's own `workspace_dir` and masking `CODYNC_HOME`.

use std::path::{Component, Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NetworkIsolationMode {
    LoopbackOnlyWithGate { egress_socket: PathBuf },
    SharedWithHost,
    Disabled,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SandboxPlan {
    pub bot_id: String,
    pub workspace_dir: PathBuf,
    pub host_data_dir: PathBuf,
    pub read_only_system_paths: Vec<PathBuf>,
    pub network_mode: NetworkIsolationMode,
    pub allowed_egress_domains: Vec<String>,
}

impl SandboxPlan {
    pub fn new(bot_id: impl Into<String>, workspace_dir: PathBuf, host_data_dir: PathBuf) -> Self {
        Self {
            bot_id: bot_id.into(),
            workspace_dir,
            host_data_dir,
            read_only_system_paths: vec![
                PathBuf::from("/usr"),
                PathBuf::from("/bin"),
                PathBuf::from("/lib"),
                PathBuf::from("/lib64"),
                PathBuf::from("/sbin"),
                PathBuf::from("/etc"),
            ],
            network_mode: NetworkIsolationMode::SharedWithHost,
            allowed_egress_domains: Vec::new(),
        }
    }

    /// Decoy directory passed as `CODYNC_HOME` inside the sandbox so the agent never
    /// resolves the host's real `~/.codync` data directory.
    pub fn decoy_codync_home(&self) -> PathBuf {
        self.workspace_dir.join(".codync-sandbox-home")
    }

    /// Verifies that a relative or candidate path stays strictly within `workspace_dir`
    /// and never traverses `..` outside of it nor touches `host_data_dir`.
    pub fn is_within_workspace(&self, candidate: &Path) -> bool {
        let mut depth = 0i32;
        for comp in candidate.components() {
            match comp {
                Component::ParentDir => {
                    depth -= 1;
                    if depth < 0 {
                        return false;
                    }
                }
                Component::Normal(_) => depth += 1,
                Component::RootDir | Component::Prefix(_) => {
                    // Absolute path: must start with workspace_dir and not contain `..` escape.
                    return normalized_starts_with(candidate, &self.workspace_dir)
                        && (!normalized_starts_with(candidate, &self.host_data_dir)
                            || normalized_starts_with(&self.workspace_dir, &self.host_data_dir));
                }
                Component::CurDir => {}
            }
        }
        true
    }
}

fn normalized_starts_with(path: &Path, base: &Path) -> bool {
    let Some(path_norm) = normalize_lexically(path) else { return false };
    let Some(base_norm) = normalize_lexically(base) else { return false };
    path_norm.starts_with(&base_norm)
}

fn normalize_lexically(path: &Path) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::ParentDir => {
                if !out.pop() {
                    return None;
                }
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    Some(out)
}
