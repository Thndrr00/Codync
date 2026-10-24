use super::Lease;
use anyhow::{Result, bail};
use serde_json::Value;
use std::path::Path;

pub(super) async fn run(root: &Path, command: &str, mut lease: Lease) -> Result<Value> {
    if command.trim().is_empty() || command.len() > 8192 {
        bail!("Command must contain 1–8192 bytes.");
    }
    lease.check()?;
    let mut process = crate::shell::line(command).tokio();
    process.current_dir(root);
    tokio::select! {
        biased;
        () = lease.cancelled() => bail!("Shell permission revoked; command and owned descendants were stopped."),
        result = tokio::time::timeout(std::time::Duration::from_secs(30), crate::agent::process::output::run(process, 64 * 1024)) => {
            result.map_err(|_| anyhow::anyhow!("Command exceeded 30 seconds and was stopped."))?
        }
    }
}
