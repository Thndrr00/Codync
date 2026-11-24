//! Copying an agent's workspace when it is duplicated (OpenBot nightly). Links stay links — an
//! installed package's `node_modules` is a junction on Windows — and a file a virus scanner or the
//! search index briefly holds is retried with a short bounded backoff instead of failing the copy.

use anyhow::{Context as _, Result, ensure};
use std::fs;
use std::io;
use std::path::Path;
use std::time::Duration;

/// How many times one filesystem step may run. A scanner holds a new file for a moment; a real
/// failure repeats on every try and must reach the caller.
pub(crate) const COPY_MAX_ATTEMPTS: u32 = 5;
/// The longest one backoff sleeps, so a transient lock costs well under a second.
const MAX_BACKOFF: Duration = Duration::from_millis(250);
/// Link targets are never followed, but a malformed tree still cannot recurse for ever.
const MAX_DEPTH: usize = 64;

/// The only error a short retry can fix: Windows' sharing violation (32) and a permission denial
/// while a security or search scan holds a newly created file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CopyError {
    Locked,
    Other,
}

impl CopyError {
    pub(crate) fn of(error: &io::Error) -> Self {
        if error.kind() == io::ErrorKind::PermissionDenied || error.raw_os_error() == Some(32) {
            Self::Locked
        } else {
            Self::Other
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CopyDecision {
    Retry,
    GiveUp,
}

/// Pure retry decision: a transient lock retries while attempts remain; nothing else does.
pub(crate) fn copy_retry(attempt: u32, max: u32, error: CopyError) -> CopyDecision {
    if error == CopyError::Locked && attempt < max { CopyDecision::Retry } else { CopyDecision::GiveUp }
}

/// 50, 100, 150… ms, capped: long enough for a scan to let go, short enough for one tool call.
fn backoff(attempt: u32) -> Duration {
    MAX_BACKOFF.min(Duration::from_millis(50 * u64::from(attempt)))
}

/// Runs one filesystem step, retrying only the errors a bounded wait can fix.
fn retry_step<T>(mut step: impl FnMut() -> io::Result<T>) -> Result<T> {
    let mut attempt = 1;
    loop {
        match step() {
            Ok(value) => return Ok(value),
            Err(error) => {
                if copy_retry(attempt, COPY_MAX_ATTEMPTS, CopyError::of(&error)) == CopyDecision::GiveUp {
                    return Err(error).context("the agent's workspace could not be copied");
                }
                std::thread::sleep(backoff(attempt));
                attempt += 1;
            }
        }
    }
}

/// Copies `from` into `to`, which must be a new folder. A link is recreated as a link: its target is
/// read, never walked. Anything that is not a folder, regular file or link has no bytes to copy.
pub(crate) fn copy_workspace(from: &Path, to: &Path) -> Result<()> {
    ensure!(from.is_dir(), "the source workspace is missing");
    retry_step(|| fs::create_dir_all(to)).context("creating the duplicate workspace")?;
    walk(from, to, 0)
}

fn walk(from: &Path, to: &Path, depth: usize) -> Result<()> {
    ensure!(depth < MAX_DEPTH, "the workspace tree is too deep to copy");
    let entries = fs::read_dir(from).with_context(|| format!("reading {}", from.display()))?;
    for entry in entries {
        let entry = entry.with_context(|| format!("reading {}", from.display()))?;
        let source = entry.path();
        let target = to.join(entry.file_name());
        // symlink_metadata, not metadata: a link must be recognized before its target is read.
        let kind = fs::symlink_metadata(&source)
            .with_context(|| format!("reading the kind of {}", source.display()))?
            .file_type();
        if kind.is_symlink() {
            retry_step(|| recreate_link(&source, &target))
                .with_context(|| format!("keeping the link {}", source.display()))?;
        } else if kind.is_dir() {
            retry_step(|| fs::create_dir(&target))?;
            walk(&source, &target, depth + 1)?;
        } else if kind.is_file() {
            retry_step(|| fs::copy(&source, &target).map(|_| ()))?;
        }
    }
    Ok(())
}

#[cfg(windows)]
fn recreate_link(from: &Path, to: &Path) -> io::Result<()> {
    let target = fs::read_link(from)?;
    // A junction's target is a directory. Recreate the same kind rather than copying its contents:
    // following an installed package's linked folder is what made the duplicate fail.
    if fs::metadata(from).is_ok_and(|meta| meta.is_dir()) {
        std::os::windows::fs::symlink_dir(target, to)
    } else {
        std::os::windows::fs::symlink_file(target, to)
    }
}

#[cfg(not(windows))]
fn recreate_link(from: &Path, to: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(fs::read_link(from)?, to)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_transient_lock_retries_but_any_other_error_does_not() {
        assert_eq!(copy_retry(1, COPY_MAX_ATTEMPTS, CopyError::Locked), CopyDecision::Retry);
        assert_eq!(copy_retry(1, COPY_MAX_ATTEMPTS, CopyError::Other), CopyDecision::GiveUp);
        assert_eq!(CopyError::of(&io::Error::from_raw_os_error(32)), CopyError::Locked, "sharing violation");
        assert_eq!(CopyError::of(&io::Error::new(io::ErrorKind::PermissionDenied, "denied")), CopyError::Locked);
        assert_eq!(CopyError::of(&io::Error::new(io::ErrorKind::NotFound, "gone")), CopyError::Other);
    }

    #[test]
    fn retries_stop_at_the_cap_and_stay_short() {
        assert_eq!(copy_retry(COPY_MAX_ATTEMPTS - 1, COPY_MAX_ATTEMPTS, CopyError::Locked), CopyDecision::Retry);
        assert_eq!(copy_retry(COPY_MAX_ATTEMPTS, COPY_MAX_ATTEMPTS, CopyError::Locked), CopyDecision::GiveUp);
        assert_eq!(copy_retry(COPY_MAX_ATTEMPTS + 1, COPY_MAX_ATTEMPTS, CopyError::Locked), CopyDecision::GiveUp);
        assert_eq!(backoff(1), Duration::from_millis(50));
        assert_eq!(backoff(COPY_MAX_ATTEMPTS), MAX_BACKOFF);
    }

    #[test]
    fn a_transient_lock_is_retried_until_the_step_succeeds() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let calls = AtomicUsize::new(0);
        let value = retry_step(|| {
            let call = calls.fetch_add(1, Ordering::SeqCst);
            if call < 2 { Err(io::Error::from_raw_os_error(32)) } else { Ok(call) }
        })
        .unwrap();
        assert_eq!(value, 2);
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn a_permanent_failure_stops_after_one_try() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let calls = AtomicUsize::new(0);
        let error = retry_step(|| {
            calls.fetch_add(1, Ordering::SeqCst);
            Err::<(), _>(io::Error::new(io::ErrorKind::NotFound, "gone"))
        })
        .unwrap_err();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(error.to_string().contains("workspace could not be copied"), "{error:#}");
    }

    #[cfg(unix)]
    #[test]
    fn a_copied_workspace_keeps_a_linked_folder_as_a_link() {
        let root = std::env::temp_dir().join(format!("codync-workspace-copy-{}", uuid::Uuid::new_v4()));
        let from = root.join("from");
        let to = root.join("to");
        fs::create_dir_all(from.join("real")).unwrap();
        fs::write(from.join("real").join("data.txt"), "value").unwrap();
        fs::write(from.join("note.txt"), "note").unwrap();
        std::os::unix::fs::symlink("real", from.join("linked")).unwrap();
        copy_workspace(&from, &to).unwrap();
        assert_eq!(fs::read_to_string(to.join("note.txt")).unwrap(), "note");
        let linked = to.join("linked");
        assert!(
            fs::symlink_metadata(&linked).unwrap().file_type().is_symlink(),
            "the linked folder was not followed into a full copy"
        );
        assert_eq!(fs::read_to_string(linked.join("data.txt")).unwrap(), "value");
        fs::remove_dir_all(&root).unwrap();
    }
}
