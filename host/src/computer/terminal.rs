use super::Lease;
use super::runtime::{self, Runtime};
use anyhow::{Result, bail};
use serde_json::Value;

pub(super) async fn run(computer: &Runtime, command: &str, mut lease: Lease) -> Result<Value> {
    if command.trim().is_empty() || command.len() > 8192 {
        bail!("Command must contain 1–8192 bytes.");
    }
    lease.check()?;
    // `shell_command` starts the bot's container (or falls back to its local workspace) before the deadline, so a
    // slow Docker start doesn't eat the command's 30 seconds.
    let process = runtime::shell_command(computer, command).await?;
    tokio::select! {
        biased;
        () = lease.cancelled() => bail!("Shell permission revoked; command and owned descendants were stopped."),
        result = tokio::time::timeout(std::time::Duration::from_secs(30), crate::agent::process::output::run(process, 64 * 1024)) => {
            result.map_err(|_| anyhow::anyhow!("Command exceeded 30 seconds and was stopped."))?
        }
    }
}
