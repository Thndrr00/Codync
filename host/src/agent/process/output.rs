//! Bounded command output under the same descendant ownership as ACP agents.
use super::Process;
use anyhow::{Context, Result, bail};
use std::process::Stdio;
use tokio::io::{AsyncRead, AsyncReadExt};

async fn read(stream: impl AsyncRead + Unpin, limit: usize) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    stream.take(u64::try_from(limit)? + 1).read_to_end(&mut bytes).await?;
    if bytes.len() > limit {
        bail!("Command output exceeded its limit; the process was stopped.");
    }
    Ok(bytes)
}

pub(crate) async fn run(mut command: tokio::process::Command, limit: usize) -> Result<serde_json::Value> {
    command.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
    #[cfg(unix)]
    command.process_group(0);
    #[cfg(windows)]
    command.creation_flags(
        windows_sys::Win32::System::Threading::CREATE_SUSPENDED
            | windows_sys::Win32::System::Threading::CREATE_NO_WINDOW,
    );
    let mut child = command.spawn()?;
    let stdout = child.stdout.take().context("stdout is unavailable")?;
    let stderr = child.stderr.take().context("stderr is unavailable")?;
    let mut owned = Process::new(child)?;
    let (out, err, status) = tokio::try_join!(read(stdout, limit), read(stderr, limit), owned.wait_owned())?;
    Ok(serde_json::json!({"stdout":String::from_utf8_lossy(&out),"stderr":String::from_utf8_lossy(&err),
        "exitCode":status.code(),"success":status.success()}))
}
