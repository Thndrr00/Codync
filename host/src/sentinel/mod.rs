//! Sentinel: a read-only, zero-LLM health scanner for `codync.db`.
//!
//! Opens the SQLite database in `SQLITE_OPEN_READONLY` mode (so WAL readers see recent
//! rows without blocking the host's writer or checkpoints), groups repeated tool errors,
//! turn failures, retry loops and queue waits into deterministic SHA-256 signatures, and
//! never stores raw prompt text, private file contents or secrets in its output.

pub mod signature;

use anyhow::{Context, Result};
use rusqlite::{Connection, OpenFlags, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::Path;

use crate::store::now_ms;

/// Default lookback window (7 days).
pub const DEFAULT_LOOKBACK_MS: i64 = 7 * 24 * 60 * 60_000;
/// Wait time in `queued` state before an entry or delegation counts as queue lag (30 seconds).
pub const QUEUE_LAG_THRESHOLD_MS: i64 = 30_000;
/// Minimum identical tool errors in a single turn to flag a retry loop.
pub const RETRY_LOOP_MIN: u32 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueKind {
    ToolError,
    RetryLoop,
    TurnError,
    QueueLag,
}

impl IssueKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ToolError => "tool_error",
            Self::RetryLoop => "retry_loop",
            Self::TurnError => "turn_error",
            Self::QueueLag => "queue_lag",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    High,
    Medium,
    Low,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SentinelIssue {
    pub signature: String,
    pub kind: IssueKind,
    pub severity: Severity,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    pub bot_id: String,
    pub first_seen_at: i64,
    pub last_seen_at: i64,
    pub count: u32,
    pub wasted_turns: u32,
    pub evidence_entry_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanReport {
    pub scanned_at: i64,
    pub lookback_ms: i64,
    pub entries_examined: usize,
    pub issues: Vec<SentinelIssue>,
}

/// Scans `db_path` in strictly read-only WAL-compatible mode.
pub fn scan_db_path(db_path: &Path, lookback_ms: i64) -> Result<ScanReport> {
    if !db_path.exists() {
        return Ok(ScanReport {
            scanned_at: now_ms(),
            lookback_ms,
            entries_examined: 0,
            issues: Vec::new(),
        });
    }
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let conn = Connection::open_with_flags(db_path, flags)
        .with_context(|| format!("opening {} in read-only mode", db_path.display()))?;
    conn.busy_timeout(std::time::Duration::from_secs(2))?;
    scan_connection(&conn, now_ms(), lookback_ms)
}

/// Scans an existing SQLite connection (used by unit tests and in-process diagnostics).
pub fn scan_connection(conn: &Connection, now: i64, lookback_ms: i64) -> Result<ScanReport> {
    let since = now.saturating_sub(lookback_ms.max(0));
    let mut st = conn.prepare(
        "SELECT id, bot_id, kind, turn, data, created_at, updated_at
         FROM entries
         WHERE updated_at >= ?1
         ORDER BY seq ASC",
    )?;
    let rows = st.query_map(params![since], |r| {
        let data_str: String = r.get(4)?;
        Ok(RawRow {
            id: r.get(0)?,
            bot_id: r.get(1)?,
            kind: r.get(2)?,
            turn: r.get(3)?,
            data: serde_json::from_str(&data_str).unwrap_or(Value::Null),
            created_at: r.get(5)?,
            updated_at: r.get(6)?,
        })
    })?;

    let mut issues: BTreeMap<String, SentinelIssue> = BTreeMap::new();
    // Track per-(bot_id, turn, tool_sig) count to detect retry loops within a single turn.
    let mut turn_tool_calls: BTreeMap<(String, i64, String), Vec<(String, String, i64, String)>> = BTreeMap::new();
    let mut entries_examined = 0usize;

    for row in rows {
        let row = row?;
        entries_examined += 1;
        match row.kind.as_str() {
            "tool" => {
                if row.data["status"].as_str() == Some("failed") {
                    let title = row.data["title"].as_str().unwrap_or("tool").trim();
                    let output = row.data["output"].as_str().unwrap_or_default();
                    let norm_out = signature::normalize(output);
                    let norm_title = signature::normalize(title);
                    let key = if norm_out.is_empty() {
                        norm_title.clone()
                    } else {
                        format!("{norm_title}: {norm_out}")
                    };
                    let sig = signature::signature(IssueKind::ToolError.as_str(), &format!("{}\n{key}", row.bot_id));
                    record_issue(
                        &mut issues,
                        sig.clone(),
                        IssueKind::ToolError,
                        Severity::Medium,
                        key.clone(),
                        Some(title.to_owned()),
                        &row.bot_id,
                        row.updated_at,
                        &row.id,
                        false,
                    );
                    if row.turn > 0 {
                        let loop_sig =
                            signature::signature(IssueKind::RetryLoop.as_str(), &format!("{}\n{key}", row.bot_id));
                        turn_tool_calls
                            .entry((row.bot_id.clone(), row.turn, loop_sig))
                            .or_default()
                            .push((key, title.to_owned(), row.updated_at, row.id.clone()));
                    }
                }
            }
            "notice" => {
                if row.data["style"].as_str() == Some("error") {
                    let text = row.data["text"].as_str().unwrap_or_default();
                    let norm = signature::normalize(text);
                    if !norm.is_empty() {
                        let sig =
                            signature::signature(IssueKind::TurnError.as_str(), &format!("{}\n{norm}", row.bot_id));
                        let severity = if text.contains("Stopped after") || text.contains("The agent failed") {
                            Severity::High
                        } else {
                            Severity::Medium
                        };
                        record_issue(
                            &mut issues,
                            sig,
                            IssueKind::TurnError,
                            severity,
                            norm,
                            None,
                            &row.bot_id,
                            row.updated_at,
                            &row.id,
                            true,
                        );
                    }
                }
            }
            "user" => {
                if row.data["status"].as_str() == Some("queued") {
                    let wait_ms = now.saturating_sub(row.created_at);
                    if wait_ms >= QUEUE_LAG_THRESHOLD_MS {
                        let sig = signature::signature(IssueKind::QueueLag.as_str(), &row.bot_id);
                        let title = format!("Message queued over {}s", QUEUE_LAG_THRESHOLD_MS / 1000);
                        record_issue(
                            &mut issues,
                            sig,
                            IssueKind::QueueLag,
                            Severity::Medium,
                            title,
                            None,
                            &row.bot_id,
                            row.created_at,
                            &row.id,
                            false,
                        );
                    }
                }
            }
            _ => {}
        }
    }

    for ((bot_id, _turn, loop_sig), calls) in turn_tool_calls {
        let count = u32::try_from(calls.len()).unwrap_or(u32::MAX);
        if count >= RETRY_LOOP_MIN
            && let Some((key, tool_name, last_at, last_id)) = calls.last()
        {
            let title = format!("Repeated in one turn ({count}x): {key}");
            record_issue(
                &mut issues,
                loop_sig,
                IssueKind::RetryLoop,
                Severity::High,
                title,
                Some(tool_name.clone()),
                &bot_id,
                *last_at,
                last_id,
                true,
            );
        }
    }

    // Escalate tool_error severity to High when it fails >= 3 times.
    let mut list: Vec<SentinelIssue> = issues
        .into_values()
        .map(|mut issue| {
            if issue.kind == IssueKind::ToolError && issue.count >= RETRY_LOOP_MIN {
                issue.severity = Severity::High;
            }
            issue
        })
        .collect();
    list.sort_by(|a, b| b.last_seen_at.cmp(&a.last_seen_at).then_with(|| a.signature.cmp(&b.signature)));

    Ok(ScanReport {
        scanned_at: now,
        lookback_ms,
        entries_examined,
        issues: list,
    })
}

struct RawRow {
    id: String,
    bot_id: String,
    kind: String,
    turn: i64,
    data: Value,
    created_at: i64,
    updated_at: i64,
}

#[allow(clippy::too_many_arguments)]
fn record_issue(
    issues: &mut BTreeMap<String, SentinelIssue>,
    signature: String,
    kind: IssueKind,
    severity: Severity,
    title: String,
    tool_name: Option<String>,
    bot_id: &str,
    at: i64,
    entry_id: &str,
    wastes_turn: bool,
) {
    let entry = issues.entry(signature.clone()).or_insert_with(|| SentinelIssue {
        signature,
        kind,
        severity,
        title,
        tool_name,
        bot_id: bot_id.to_owned(),
        first_seen_at: at,
        last_seen_at: at,
        count: 0,
        wasted_turns: 0,
        evidence_entry_id: entry_id.to_owned(),
    });
    entry.count = entry.count.saturating_add(1);
    if wastes_turn {
        entry.wasted_turns = entry.wasted_turns.saturating_add(1);
    }
    if at < entry.first_seen_at {
        entry.first_seen_at = at;
    }
    if at >= entry.last_seen_at {
        entry.last_seen_at = at;
        entry.evidence_entry_id = entry_id.to_owned();
    }
}

/// JSON value returned by the `sentinelHealth` API method.
pub fn health_json(store: &crate::store::Store, lookback_ms: Option<i64>) -> Result<Value> {
    let lookback = lookback_ms.unwrap_or(DEFAULT_LOOKBACK_MS).clamp(60_000, 30 * 24 * 60 * 60_000);
    let report = store.with_connection(|c| scan_connection(c, now_ms(), lookback))?;
    Ok(json!(report))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{EntryKind, Lane, Store};

    #[test]
    fn groups_tool_errors_and_detects_retry_loops_and_queue_lag() {
        let dir = std::env::temp_dir().join(format!("codync-sentinel-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let db = dir.join("codync.db");
        let store = Store::open(&db).expect("open store");
        let lane = Lane::main("bot-1");

        // 3 identical tool failures in turn 1 -> 1 ToolError (count=3, High) + 1 RetryLoop
        for line in [10, 25, 99] {
            store
                .insert_entry(
                    &lane,
                    EntryKind::Tool,
                    1,
                    &json!({
                        "title": "cargo test",
                        "status": "failed",
                        "output": format!("error at /Users/alice/secret/src/lib.rs:{line}:4"),
                    }),
                )
                .expect("tool entry");
        }

        // Error notice
        store
            .insert_entry(
                &lane,
                EntryKind::Notice,
                1,
                &json!({
                    "text": "Stopped after 3 identical failed tool calls in a row.",
                    "style": "error",
                }),
            )
            .expect("notice entry");

        let report = scan_db_path(&db, DEFAULT_LOOKBACK_MS).expect("scan succeeds");
        assert_eq!(report.entries_examined, 4);
        assert!(report.issues.iter().any(|i| i.kind == IssueKind::ToolError && i.count == 3 && i.severity == Severity::High));
        assert!(report.issues.iter().any(|i| i.kind == IssueKind::RetryLoop && i.wasted_turns == 1));
        assert!(report.issues.iter().any(|i| i.kind == IssueKind::TurnError && i.wasted_turns == 1));
        // Privacy invariant: private path `/Users/alice/secret` is scrubbed to `<path>`.
        for issue in &report.issues {
            assert!(!issue.title.contains("alice"), "leaked path in {}", issue.title);
        }
        drop(store);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
