//! macOS `sandbox-exec` (Seatbelt) profile generator (`macos_sandbox.rs`).

use super::plan::SandboxPlan;

/// Generates a Seatbelt profile (`sandbox-exec -p <profile>`) that denies access to
/// `host_data_dir` (`~/.codync`) while allowing read-write access to `workspace_dir`.
pub fn seatbelt_profile(plan: &SandboxPlan) -> String {
    let data = escape_scheme_string(&plan.host_data_dir.to_string_lossy());
    let ws = escape_scheme_string(&plan.workspace_dir.to_string_lossy());
    format!(
        "(version 1)\n\
         (allow default)\n\
         (deny file-read* file-write* (subpath \"{data}\"))\n\
         (allow file-read* file-write* (subpath \"{ws}\"))\n"
    )
}

fn escape_scheme_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
