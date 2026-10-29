//! "Done" only when the check passes: a bot with a `verify_command` (build, tests, typecheck) has it
//! run in its workspace after every turn that ended normally. When it fails the turn ends as
//! `verification_failed`: the chat gets the check's last output, a routine run fails, and the alert
//! is a failure instead of "Finished".
//!
//! Idea from Mercury Code's "done means the build passed"; no code copied.

use super::{Actor, Done, NoticeStyle};
use std::path::Path;
use std::time::Duration;

/// The stop reason of a turn whose check failed.
pub(super) const FAILED: &str = "verification_failed";
/// Longest a check may run before it counts as failed.
const LIMIT: Duration = Duration::from_secs(300);
/// How much of the check's output the chat shows (its end, where the errors are).
const SHOWN: usize = 1200;

/// `None` when `command` passed in `cwd`; else why it didn't, with the end of its output.
async fn run(command: &str, cwd: &str) -> Option<String> {
    let mut shell = crate::shell::line(command).tokio();
    shell.kill_on_drop(true).stdin(std::process::Stdio::null());
    if !cwd.is_empty() && Path::new(cwd).is_dir() {
        shell.current_dir(cwd);
    }
    match tokio::time::timeout(LIMIT, shell.output()).await {
        Err(_) => Some(format!("It ran longer than {} minutes.", LIMIT.as_secs() / 60)),
        Ok(Err(error)) => Some(format!("It couldn't start: {error}")),
        Ok(Ok(out)) if out.status.success() => None,
        Ok(Ok(out)) => {
            let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
            let text = text.trim();
            let skip = text.chars().count().saturating_sub(SHOWN);
            Some(text.chars().skip(skip).collect())
        }
    }
}

impl Actor {
    /// Runs the bot's check on a turn that ended normally; a failure turns its stop reason into `FAILED`.
    pub(super) async fn verified(&mut self, done: Done) -> Done {
        let Some(command) = self.cfg.verify_command.clone().filter(|c| !c.trim().is_empty()) else { return done };
        if self.stop_requested || !matches!(&done, Ok(v) if v["stopReason"] == "end_turn") {
            return done;
        }
        let Some(output) = run(&command, &self.cfg.cwd).await else { return done };
        self.notice(&format!("The check `{command}` failed, so this isn't done.\n\n{output}"), NoticeStyle::Error);
        done.map(|mut v| {
            v["stopReason"] = FAILED.into();
            v
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_passing_check_is_none_and_a_failing_one_shows_its_output() {
        let dir = std::env::temp_dir();
        let cwd = dir.to_string_lossy();
        assert_eq!(run("exit 0", &cwd).await, None);
        let failure = run("echo boom && exit 1", &cwd).await.expect("the check fails");
        assert!(failure.contains("boom"), "{failure}");
    }

    #[tokio::test]
    async fn only_the_end_of_a_long_output_is_kept() {
        let dir = std::env::temp_dir();
        let long = "x".repeat(SHOWN * 2);
        let failure = run(&format!("echo {long} && exit 1"), &dir.to_string_lossy()).await.expect("fails");
        assert_eq!(failure.chars().count(), SHOWN);
    }
}
