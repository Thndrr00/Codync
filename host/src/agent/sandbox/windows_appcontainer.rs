//! Windows process containment (`windows_appcontainer.rs`): environment masking,
//! Job Object process-tree limits, and restricted path policy for Windows agents.

use super::plan::SandboxPlan;

/// Environment overrides applied to a Windows agent process when sandboxing is enabled.
/// Masks `CODYNC_HOME` to a decoy folder inside the bot's workspace and strips host tokens.
pub fn restricted_env_overrides(plan: &SandboxPlan) -> Vec<(String, String)> {
    let decoy = plan.decoy_codync_home().to_string_lossy().into_owned();
    vec![
        ("CODYNC_HOME".into(), decoy.clone()),
        ("CODYNC_SANDBOX".into(), "1".into()),
        ("CODYNC_BOT_ID".into(), plan.bot_id.clone()),
    ]
}
