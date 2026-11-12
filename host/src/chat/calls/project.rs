//! The project a call's bot works in, from git: what the voice knows of it before it speaks.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, ensure};
use tokio::io::AsyncReadExt;

const GIT_TIMEOUT: Duration = Duration::from_secs(2);
const MAX_GIT_OUTPUT: u64 = 1024 * 1024;

/// The project in `cwd` now: branch, changed files and the last commits; nothing outside a repository.
pub async fn now(cwd: &str) -> String {
    let dir = Path::new(cwd);
    if cwd.is_empty() || !dir.ancestors().any(|d| d.join(".git").exists()) {
        return String::new();
    }
    let run = |args: &'static [&'static str]| {
        let dir = dir.to_owned();
        async move {
            let out = git_output(&dir, args).await.ok()?;
            // Only the end: a porcelain line starts with its status column, a space when the change is unstaged.
            Some(String::from_utf8_lossy(&out).trim_end().to_owned())
        }
    };
    let (branch, changes, commits) = tokio::join!(
        run(&["rev-parse", "--abbrev-ref", "HEAD"]),
        run(&["status", "--porcelain"]),
        run(&["log", "-3", "--format=%s"]),
    );
    let mut lines = Vec::new();
    if let Some(branch) = branch.filter(|b| !b.is_empty()) {
        lines.push(format!("Branch: {branch}"));
    }
    if let Some(changes) = changes {
        let files: Vec<&str> = changes.lines().map(|l| l.get(3..).unwrap_or(l)).collect();
        if files.is_empty() {
            lines.push("No uncommitted changes".into());
        } else {
            let shown = files.iter().take(10).copied().collect::<Vec<_>>().join(", ");
            let more = if files.len() > 10 { format!(" and {} more", files.len() - 10) } else { String::new() };
            lines.push(format!("Changed files ({}): {shown}{more}", files.len()));
        }
    }
    if let Some(commits) = commits.filter(|c| !c.is_empty()) {
        lines.push(format!("Last commits: {}", commits.lines().collect::<Vec<_>>().join(" · ")));
    }
    lines.join("\n")
}

/// A read-only Git probe shared with the island; bounded in time and output, with no inherited repository redirect.
pub(crate) async fn git_output(cwd: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let git = git_program().await.context("Git is not available")?;
    let mut cmd = tokio::process::Command::new(git);
    for (key, _) in std::env::vars_os().filter(|(key, _)| key.to_string_lossy().starts_with("GIT_")) {
        cmd.env_remove(key);
    }
    cmd.args(["-c", "core.fsmonitor=false"])
        .args(args)
        .current_dir(cwd)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true);
    let mut child = cmd.spawn().context("Could not start Git")?;
    let stdout = child.stdout.take().context("Git output is unavailable")?;
    let read = async {
        let mut output = Vec::new();
        stdout.take(MAX_GIT_OUTPUT + 1).read_to_end(&mut output).await.context("Could not read Git output")?;
        ensure!(
            u64::try_from(output.len()).unwrap_or(u64::MAX) <= MAX_GIT_OUTPUT,
            "Git output exceeded the read limit"
        );
        ensure!(
            child.wait().await.context("Could not finish Git read")?.success(),
            "Git could not read this workspace"
        );
        Ok(output)
    };
    tokio::time::timeout(GIT_TIMEOUT, read).await.context("Git read timed out")?
}

/// git, where running it can't open macOS's "install the command line tools" prompt.
async fn git_program() -> Option<PathBuf> {
    let git = crate::agent::backends::which("git")?;
    if cfg!(target_os = "macos") && git == Path::new("/usr/bin/git") {
        let mut command = tokio::process::Command::new("/usr/bin/xcode-select");
        command.arg("-p").stdin(std::process::Stdio::null()).kill_on_drop(true);
        let tools = tokio::time::timeout(GIT_TIMEOUT, command.output()).await.ok()?.ok()?;
        if !tools.status.success() {
            return None;
        }
    }
    Some(git)
}
