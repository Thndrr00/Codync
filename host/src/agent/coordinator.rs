//! Chief of Staff: autonomous incident and escalation coordinator (`docs/team-incidents.md`
//! and `docs/team-escalations.md` from OpenMausBot).
//!
//! Supervises peer bots:
//! - Rate-limits incident reports to at most 5 per hour per bot to prevent failure storms.
//! - Caps automatic thread retries at 2 retries per thread (`MAX_THREAD_RETRIES = 2`).
//! - Provides MCP tool schemas for the Chief (`chief_retry_turn`, `chief_escalate_to_user`,
//!   `chief_coordinate_bots`).

use crate::store::{BotConfig, Store};
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;

pub const MAX_INCIDENTS_PER_HOUR: usize = 5;
pub const MAX_THREAD_RETRIES: u8 = 2;
const HOUR_MS: i64 = 3_600_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IncidentKind {
    TurnFailed,
    Stalled,
    EngineStartFailed,
    Escalation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IncidentReport {
    pub incident_id: String,
    pub bot_id: String,
    pub bot_name: String,
    pub thread_id: Option<String>,
    pub kind: IncidentKind,
    pub detail: String,
    pub created_at: i64,
}

/// Tracks per-bot incident rate limits and per-thread retry counts.
#[derive(Debug, Default)]
pub struct IncidentCoordinator {
    bot_incident_timestamps: HashMap<String, Vec<i64>>,
    thread_retries: HashMap<String, u8>,
}

impl IncidentCoordinator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Resolves the Chief of Staff bot for a given bot or group, if one is configured.
    pub fn resolve_chief(store: &Store, failing_bot_id: &str) -> Option<BotConfig> {
        let bots = store.bots().ok()?;
        // First check if any group containing `failing_bot_id` has a `chief_bot_id`.
        for row in &bots {
            if !row.deleted
                && row.config.is_group()
                && row.config.members.iter().any(|m| m == failing_bot_id)
                && let Some(chief_id) = &row.config.chief_bot_id
                && chief_id != failing_bot_id
                && let Some(chief) = bots
                    .iter()
                    .find(|b| !b.deleted && !b.config.is_group() && &b.config.id == chief_id)
            {
                return Some(chief.config.clone());
            }
        }
        // Otherwise look for a workspace-level Chief (`is_chief == true`).
        bots.into_iter()
            .find(|b| {
                !b.deleted
                    && !b.config.hidden
                    && !b.config.is_group()
                    && b.config.is_chief
                    && b.config.id != failing_bot_id
            })
            .map(|b| b.config)
    }

    /// Records a new incident if within the 5-incidents/hour rate limit for `bot_id`.
    /// Returns `Ok(true)` if admitted, or `Ok(false)` if throttled due to a failure storm.
    pub fn admit_incident(&mut self, bot_id: &str, now_ms: i64) -> bool {
        let history = self.bot_incident_timestamps.entry(bot_id.to_owned()).or_default();
        history.retain(|&ts| now_ms.saturating_sub(ts) < HOUR_MS);
        if history.len() >= MAX_INCIDENTS_PER_HOUR {
            return false;
        }
        history.push(now_ms);
        true
    }

    /// Checks and increments the retry counter for `(bot_id, thread_id)`.
    /// Fails if `MAX_THREAD_RETRIES` (2) has already been reached.
    pub fn record_thread_retry(&mut self, bot_id: &str, thread_id: Option<&str>) -> Result<u8> {
        let key = format!("{bot_id}@{}", thread_id.unwrap_or("main"));
        let count = self.thread_retries.entry(key).or_insert(0);
        if *count >= MAX_THREAD_RETRIES {
            bail!(
                "refused(retry_limit_exceeded): thread has already been retried {MAX_THREAD_RETRIES} times by Chief of Staff"
            );
        }
        *count += 1;
        Ok(*count)
    }

    /// Formats the structured incident prompt sent to the Chief of Staff's `Team incidents` thread.
    pub fn format_incident_prompt(report: &IncidentReport) -> String {
        format!(
            "[TEAM INCIDENT: {:?} | BOT: {} ({})]\n\
            Thread: {}\n\
            Detail: {}\n\n\
            You are the Chief of Staff. Diagnose the failure above:\n\
            - If transient, use `chief_retry_turn` (max {MAX_THREAD_RETRIES} retries per thread).\n\
            - If another specialist bot can handle it, use `chief_coordinate_bots`.\n\
            - If human authorization or input is required, use `chief_escalate_to_user` with a 1-2 sentence summary.",
            report.kind,
            report.bot_name,
            report.bot_id,
            report.thread_id.as_deref().unwrap_or("main"),
            report.detail
        )
    }
}

/// MCP tools exposed exclusively to a bot with `is_chief: true`.
pub fn chief_tools() -> Value {
    json!([
        {
            "name": "chief_retry_turn",
            "description": "Resume a failed or stalled bot thread where it stopped (max 2 retries per thread).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "targetBotId": { "type": "string" },
                    "threadId": { "type": "string" },
                    "guidance": { "type": "string", "description": "Optional recovery hint injected into the retry turn." }
                },
                "required": ["targetBotId"]
            }
        },
        {
            "name": "chief_coordinate_bots",
            "description": "Reassign a blocked or failed task to another specialist bot with corrected instructions.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "targetBotId": { "type": "string" },
                    "instructions": { "type": "string" }
                },
                "required": ["targetBotId", "instructions"]
            }
        },
        {
            "name": "chief_escalate_to_user",
            "description": "Synthesize an incident or permission block into 1-2 clear sentences and notify the user.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "summary": { "type": "string" },
                    "urgent": { "type": "boolean" }
                },
                "required": ["summary", "urgent"]
            }
        }
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limits_incidents_to_five_per_hour() {
        let mut coord = IncidentCoordinator::new();
        let t0 = 1_700_000_000_000;
        for i in 0..MAX_INCIDENTS_PER_HOUR {
            assert!(coord.admit_incident("bot-a", t0 + (i as i64) * 1000));
        }
        // 6th within the same hour is throttled
        assert!(!coord.admit_incident("bot-a", t0 + 10_000));
        // Another bot is unaffected
        assert!(coord.admit_incident("bot-b", t0 + 10_000));
        // After 1 hour, bot-a is admitted again
        assert!(coord.admit_incident("bot-a", t0 + HOUR_MS + 1));
    }

    #[test]
    fn enforces_max_two_retries_per_thread() {
        let mut coord = IncidentCoordinator::new();
        assert_eq!(coord.record_thread_retry("bot-a", Some("t1")).unwrap(), 1);
        assert_eq!(coord.record_thread_retry("bot-a", Some("t1")).unwrap(), 2);
        assert!(coord.record_thread_retry("bot-a", Some("t1")).is_err());
        // Different thread has its own retry budget
        assert_eq!(coord.record_thread_retry("bot-a", Some("t2")).unwrap(), 1);
        let report = IncidentReport {
            incident_id: "inc-1".into(),
            bot_id: "bot-a".into(),
            bot_name: "Alpha".into(),
            thread_id: Some("t1".into()),
            kind: IncidentKind::TurnFailed,
            detail: "timeout".into(),
            created_at: 0,
        };
        assert!(IncidentCoordinator::format_incident_prompt(&report).contains("Chief of Staff"));
        assert!(chief_tools().is_array());
        let db = std::env::temp_dir().join(format!("codync-coord-{}.db", uuid::Uuid::new_v4()));
        let s = Store::open(&db).unwrap();
        assert!(IncidentCoordinator::resolve_chief(&s, "bot-a").is_none());
        let _ = std::fs::remove_file(&db);
    }
}
