//! Read-only workspace context for the island. Git and measured timing estimates stay on the host.

use crate::chat::calls::project::git_output;
use crate::hub::Hub;
use crate::routines::{Run, Status};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

pub(super) async fn read(hub: &Arc<Hub>, bot_id: &str) -> Result<Value> {
    let bot = hub.store.bot(bot_id)?.filter(|bot| !bot.deleted).context("Unknown bot")?;
    let runs = hub.routines.runs_for(bot_id);
    let git = git_snapshot(&bot.config.cwd).await;
    Ok(json!({ "git": git, "timings": timing_estimates(&runs) }))
}

async fn git_snapshot(cwd: &str) -> Value {
    if cwd.is_empty() {
        return json!({ "branch": null, "changes": null, "error": "This bot has no workspace folder." });
    }
    let dir = Path::new(cwd);
    let (branch, status) = tokio::join!(
        git_output(dir, &["rev-parse", "--abbrev-ref", "HEAD"]),
        git_output(dir, &["status", "--porcelain=v1", "-z", "--untracked-files=normal"]),
    );
    let mut errors = Vec::new();
    let branch = match branch {
        Ok(bytes) => {
            let branch = String::from_utf8_lossy(&bytes).trim().to_owned();
            (!branch.is_empty() && branch != "HEAD").then_some(branch)
        }
        Err(error) => {
            errors.push(format!("Branch: {error}"));
            None
        }
    };
    let changes = match status.and_then(|bytes| status_count(&bytes)) {
        Ok(count) => Some(count),
        Err(error) => {
            errors.push(format!("Changes: {error}"));
            None
        }
    };
    let error = (!errors.is_empty()).then(|| errors.join(" · "));
    json!({ "branch": branch, "changes": changes, "error": error })
}

/// Porcelain -z gives a second path for a rename/copy, which is not another changed file.
fn status_count(output: &[u8]) -> Result<usize> {
    if output.is_empty() {
        return Ok(0);
    }
    ensure!(output.last() == Some(&0), "Incomplete Git status");
    let mut records = output.split(|byte| *byte == 0).peekable();
    let mut count = 0;
    while let Some(record) = records.next() {
        if record.is_empty() {
            ensure!(records.peek().is_none(), "Invalid Git status record");
            break;
        }
        ensure!(record.len() >= 4 && record[2] == b' ', "Invalid Git status record");
        count += 1;
        if record[..2].contains(&b'R') || record[..2].contains(&b'C') {
            ensure!(records.next().is_some_and(|path| !path.is_empty()), "Incomplete Git rename");
        }
    }
    Ok(count)
}

/// At least five successful, timed runs of the same routine; queued time is never execution time.
fn timing_estimates(runs: &[Run]) -> Vec<Value> {
    let mut samples: BTreeMap<&str, Vec<i64>> = BTreeMap::new();
    for run in runs {
        if run.status != Status::Succeeded {
            continue;
        }
        let Some((start, finish)) = run.started_at.zip(run.finished_at) else { continue };
        let Some(duration) = finish.checked_sub(start).filter(|duration| start >= 0 && *duration > 0) else { continue };
        samples.entry(&run.routine_id).or_default().push(duration);
    }
    samples
        .into_iter()
        .filter_map(|(routine, mut durations)| {
            if durations.len() < 5 {
                return None;
            }
            durations.sort_unstable();
            let middle = durations.len() / 2;
            let estimate = if durations.len() % 2 == 0 {
                let low = durations[middle - 1];
                low + (durations[middle] - low) / 2
            } else {
                durations[middle]
            };
            Some(json!({ "routineId": routine, "estimateMs": estimate, "samples": durations.len() }))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{status_count, timing_estimates};
    use crate::routines::Run;
    use serde_json::json;

    fn run(routine: &str, duration: i64, status: &str) -> Run {
        serde_json::from_value(json!({
            "id": format!("{routine}-{duration}"), "routineId": routine, "botId": "bot", "status": status,
            "createdAt": 0, "startedAt": 1000, "finishedAt": 1000 + duration,
            "detail": null, "event": null, "deliveryId": null
        }))
        .expect("valid routine fixture")
    }

    #[test]
    fn git_counts_renames_and_copies_once_and_accepts_unusual_names() {
        assert_eq!(status_count(b"").unwrap(), 0);
        assert_eq!(
            status_count(b" M note with spaces\0?? new\nline\0R  new name\0old name\0 C copy\0original\0").unwrap(),
            4
        );
        assert_eq!(status_count(" M 中文.txt\0".as_bytes()).unwrap(), 1);
    }

    #[test]
    fn git_refuses_truncated_or_malformed_status() {
        for bytes in [b" M file".as_slice(), b"R  new\0".as_slice(), b" M \0".as_slice(), b"\0?? file\0".as_slice()] {
            assert!(status_count(bytes).is_err());
        }
    }

    #[test]
    fn measured_estimates_require_five_successes_of_the_same_routine() {
        let mut runs: Vec<_> = (1..=4).map(|duration| run("a", duration, "succeeded")).collect();
        runs.extend((1..=4).map(|duration| run("b", duration, "succeeded")));
        runs.push(run("a", 5, "failed"));
        runs.push(run("a", 0, "succeeded"));
        let mut untimed = run("a", 30, "succeeded");
        untimed.started_at = None;
        runs.push(untimed);
        assert_eq!(timing_estimates(&runs).len(), 0);
    }

    #[test]
    fn measured_estimates_use_the_median_not_an_outlier_or_queue_time() {
        let runs: Vec<_> =
            [20, 10, 10000, 30, 40].into_iter().map(|duration| run("a", duration, "succeeded")).collect();
        assert_eq!(timing_estimates(&runs), vec![json!({ "routineId": "a", "estimateMs": 30, "samples": 5 })]);
        let runs: Vec<_> =
            [10, 20, 30, 40, 50, 60].into_iter().map(|duration| run("a", duration, "succeeded")).collect();
        assert_eq!(timing_estimates(&runs), vec![json!({ "routineId": "a", "estimateMs": 35, "samples": 6 })]);
    }
}
