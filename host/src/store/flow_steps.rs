//! The steps of routine flow runs: one per bot that took part in a run (`UNIQUE(run_id, bot_id)`,
//! which is also what makes retrying a run idempotent).

use super::flows::{FlowChange, audience, touch};
use super::{Store, now_ms};
use anyhow::Result;
use rusqlite::{Connection, params};
use serde::Serialize;

/// How much of a step's input or output is kept.
const MAX_TEXT_BYTES: usize = 32_000;
/// Runs of one routine whose steps are kept.
const KEPT_RUNS: i64 = 50;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum StepStatus {
    /// OpenBot 0.77.0: the message is posted but the agent has not taken it yet.
    Queued,
    Running,
    Succeeded,
    Failed,
    Skipped,
    Cancelled,
}

impl StepStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
            Self::Cancelled => "cancelled",
        }
    }

    fn parse(s: &str) -> Self {
        match s {
            "queued" => Self::Queued,
            "succeeded" => Self::Succeeded,
            "failed" => Self::Failed,
            "skipped" => Self::Skipped,
            "cancelled" => Self::Cancelled,
            _ => Self::Running,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowStep {
    pub id: String,
    pub run_id: String,
    pub bot_id: String,
    /// The bot request that carried the input; null for the routine's own bot.
    pub delivery_id: Option<String>,
    pub input: String,
    pub output: Option<String>,
    pub status: StepStatus,
    pub error: Option<String>,
    /// The pictures or clips a media step made, as shared files of the owner bot.
    #[serde(default)]
    pub files: Vec<crate::chat::files::SharedFile>,
    /// One status per result of a batched media step, read from its saved settings. Older steps
    /// have none, which a client reads as one result made before batches existed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub results: Option<Vec<crate::media::BatchResult>>,
    pub created_at: i64,
    pub updated_at: i64,
    #[serde(skip)]
    pub routine_id: String,
    #[serde(skip)]
    pub owner_bot_id: String,
    /// When the run began; only the links made before it apply.
    #[serde(skip)]
    pub run_at: i64,
}

/// A run whose flow left steps.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FlowRun {
    pub run_id: String,
    pub routine_id: String,
    pub owner_bot_id: String,
    pub run_at: i64,
}

pub struct NewStep<'a> {
    pub run_id: &'a str,
    pub routine_id: &'a str,
    pub owner_bot_id: &'a str,
    pub bot_id: &'a str,
    pub run_at: i64,
    pub input: &'a str,
    pub status: StepStatus,
    pub output: Option<&'a str>,
    pub error: Option<&'a str>,
}

const COLUMNS: &str = "id, run_id, bot_id, delivery_id, input, output, status, error, created_at, updated_at, \
                       routine_id, owner_bot_id, run_at, files";
/// The insert columns plus the settings a media step uses for its per-result progress.
const READ_COLUMNS: &str = "id, run_id, bot_id, delivery_id, input, output, status, error, created_at, updated_at, \
                            routine_id, owner_bot_id, run_at, files, settings";

fn row_step(r: &rusqlite::Row) -> rusqlite::Result<FlowStep> {
    Ok(FlowStep {
        id: r.get(0)?,
        run_id: r.get(1)?,
        bot_id: r.get(2)?,
        delivery_id: r.get(3)?,
        input: r.get(4)?,
        output: r.get(5)?,
        status: StepStatus::parse(&r.get::<_, String>(6)?),
        error: r.get(7)?,
        created_at: r.get(8)?,
        updated_at: r.get(9)?,
        routine_id: r.get(10)?,
        owner_bot_id: r.get(11)?,
        run_at: r.get(12)?,
        files: r.get::<_, Option<String>>(13)?.and_then(|files| serde_json::from_str(&files).ok()).unwrap_or_default(),
        results: r
            .get::<_, Option<String>>(14)?
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
            .and_then(|settings| settings.get("results").cloned())
            .and_then(|results| serde_json::from_value(results).ok()),
    })
}

fn query(c: &Connection, filter: &str, args: impl rusqlite::Params) -> Result<Vec<FlowStep>> {
    let mut st =
        c.prepare(&format!("SELECT {READ_COLUMNS} FROM routine_flow_steps WHERE {filter} ORDER BY created_at, rowid"))?;
    Ok(st.query_map(args, row_step)?.collect::<rusqlite::Result<_>>()?)
}

fn kept(text: &str) -> String {
    crate::agent::acp::truncate(text, MAX_TEXT_BYTES)
}

impl Store {
    /// Records a step. A bot already holding a step in the run (a retry) changes nothing: `None`.
    pub fn flow_add_step(&self, new: &NewStep) -> Result<Option<(FlowStep, FlowChange)>> {
        self.with_connection(|c| {
            let tx = c.unchecked_transaction()?;
            let now = now_ms();
            let id = uuid::Uuid::new_v4().to_string();
            let added = tx.execute(
                &format!(
                    "INSERT OR IGNORE INTO routine_flow_steps({COLUMNS})
                     VALUES(?1, ?2, ?3, NULL, ?4, ?5, ?6, ?7, ?8, ?8, ?9, ?10, ?11, NULL)"
                ),
                params![
                    id,
                    new.run_id,
                    new.bot_id,
                    kept(new.input),
                    new.output.map(kept),
                    new.status.as_str(),
                    new.error,
                    now,
                    new.routine_id,
                    new.owner_bot_id,
                    new.run_at
                ],
            )?;
            if added == 0 {
                return Ok(None);
            }
            tx.execute(
                "DELETE FROM routine_flow_steps WHERE routine_id = ?1 AND run_id NOT IN (
                   SELECT run_id FROM routine_flow_steps WHERE routine_id = ?1
                   GROUP BY run_id ORDER BY MAX(run_at) DESC LIMIT ?2)",
                params![new.routine_id, KEPT_RUNS],
            )?;
            let step = query(&tx, "id = ?1", [&id])?.pop().expect("the step was just inserted");
            let bots = audience(&tx, new.routine_id, new.owner_bot_id)?;
            let change = touch(&tx, bots)?.expect("the owner is always in the audience");
            tx.commit()?;
            Ok(Some((step, change)))
        })
    }

    /// Every run that left steps, newest first.
    pub fn flow_runs(&self) -> Result<Vec<FlowRun>> {
        self.with_connection(|c| {
            let mut st = c.prepare(
                "SELECT run_id, routine_id, owner_bot_id, run_at FROM routine_flow_steps
                 GROUP BY run_id ORDER BY run_at DESC",
            )?;
            let rows = st.query_map([], |r| {
                Ok(FlowRun { run_id: r.get(0)?, routine_id: r.get(1)?, owner_bot_id: r.get(2)?, run_at: r.get(3)? })
            })?;
            Ok(rows.collect::<rusqlite::Result<_>>()?)
        })
    }

    /// The steps of one run, oldest first.
    pub fn flow_run_steps(&self, run_id: &str) -> Result<Vec<FlowStep>> {
        self.with_connection(|c| query(c, "run_id = ?1", [run_id]))
    }

    /// The steps of a routine's newest run.
    pub fn flow_last_steps(&self, routine_id: &str) -> Result<Vec<FlowStep>> {
        self.with_connection(|c| {
            query(
                c,
                "routine_id = ?1 AND run_id = (SELECT run_id FROM routine_flow_steps WHERE routine_id = ?1
                   ORDER BY run_at DESC, created_at DESC LIMIT 1)",
                [routine_id],
            )
        })
    }

    pub fn flow_attach_delivery(&self, step_id: &str, delivery_id: &str) -> Result<()> {
        self.with_connection(|c| {
            c.execute(
                "UPDATE routine_flow_steps SET delivery_id = ?2 WHERE id = ?1 AND status = 'running'",
                params![step_id, delivery_id],
            )?;
            Ok(())
        })
    }

    /// OpenBot 0.77.0: the hand-off has reached the agent's queue, so a queued step starts running
    /// and keeps the delivery that carried its message. A step that already started keeps its status.
    pub fn flow_start_queued_step(&self, step_id: &str, delivery_id: &str) -> Result<Option<FlowChange>> {
        self.with_connection(|c| {
            let tx = c.unchecked_transaction()?;
            let changed = tx.execute(
                "UPDATE routine_flow_steps SET status = 'running', delivery_id = ?2, updated_at = ?3
                 WHERE id = ?1 AND status = 'queued'",
                params![step_id, delivery_id, now_ms()],
            )?;
            if changed == 0 {
                return Ok(None);
            }
            let step = query(&tx, "id = ?1", [step_id])?.pop().expect("the step was just updated");
            let bots = audience(&tx, &step.routine_id, &step.owner_bot_id)?;
            let change = touch(&tx, bots)?.expect("the owner is always in the audience");
            tx.commit()?;
            Ok(Some(change))
        })
    }

    /// Ends a running or queued step; a step that already ended stays as it is (`None`).
    pub fn flow_settle_step(
        &self,
        id: &str,
        status: StepStatus,
        output: Option<&str>,
        error: Option<&str>,
        files: &[crate::chat::files::SharedFile],
    ) -> Result<Option<FlowChange>> {
        self.with_connection(|c| {
            let tx = c.unchecked_transaction()?;
            let files = serde_json::to_string(files).ok();
            let changed = tx.execute(
                "UPDATE routine_flow_steps SET status = ?2, output = ?3, error = ?4, updated_at = ?5, files = ?6
                 WHERE id = ?1 AND status IN ('running', 'queued')",
                params![id, status.as_str(), output.map(kept), error, now_ms(), files],
            )?;
            if changed == 0 {
                return Ok(None);
            }
            let step = query(&tx, "id = ?1", [id])?.pop().expect("the step was just updated");
            let bots = audience(&tx, &step.routine_id, &step.owner_bot_id)?;
            let change = touch(&tx, bots)?;
            tx.commit()?;
            Ok(change)
        })
    }

    /// Steps still running or queued when the host started belong to a process that is gone: they
    /// fail with `reason`. Returns them as they were, and the change.
    pub fn flow_fail_orphans(&self, reason: &str) -> Result<(Vec<FlowStep>, Option<FlowChange>)> {
        self.with_connection(|c| {
            let tx = c.unchecked_transaction()?;
            let orphans = query(&tx, "status IN ('running', 'queued')", [])?;
            if orphans.is_empty() {
                return Ok((orphans, None));
            }
            tx.execute(
                "UPDATE routine_flow_steps SET status = 'failed', error = ?1, updated_at = ?2 WHERE status IN ('running', 'queued')",
                params![reason, now_ms()],
            )?;
            let mut bots = std::collections::BTreeSet::new();
            for step in &orphans {
                bots.extend(audience(&tx, &step.routine_id, &step.owner_bot_id)?);
            }
            let change = touch(&tx, bots)?;
            tx.commit()?;
            Ok((orphans, change))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_queued_step_runs_when_its_delivery_reaches_the_agent_and_settles_either_way() {
        let store = Store::open(std::path::Path::new(":memory:")).expect("a store");
        let new = NewStep {
            run_id: "run",
            routine_id: "r",
            owner_bot_id: "a",
            bot_id: "b",
            run_at: 1,
            input: "the task",
            status: StepStatus::Queued,
            output: None,
            error: None,
        };
        let (step, _) = store.flow_add_step(&new).unwrap().unwrap();
        assert_eq!(step.status, StepStatus::Queued);
        assert_eq!(serde_json::to_value(&step).unwrap()["status"], json!("queued"));
        assert!(store.flow_start_queued_step(&step.id, "delivery-1").unwrap().is_some());
        let running = store.flow_run_steps("run").unwrap().pop().unwrap();
        assert_eq!((running.status, running.delivery_id.as_deref()), (StepStatus::Running, Some("delivery-1")));
        assert!(store.flow_start_queued_step(&step.id, "delivery-2").unwrap().is_none(), "it only starts once");
        assert!(store.flow_settle_step(&step.id, StepStatus::Succeeded, Some("answer"), None, &[]).unwrap().is_some());
        assert_eq!(store.flow_run_steps("run").unwrap().pop().unwrap().status, StepStatus::Succeeded);

        // A message that never reached the agent is settled from queued with its error too.
        let queued = NewStep {
            run_id: "run2",
            routine_id: "r",
            owner_bot_id: "a",
            bot_id: "c",
            run_at: 1,
            input: "",
            status: StepStatus::Queued,
            output: None,
            error: None,
        };
        let (step, _) = store.flow_add_step(&queued).unwrap().unwrap();
        assert!(
            store
                .flow_settle_step(&step.id, StepStatus::Failed, None, Some("the agent stopped"), &[])
                .unwrap()
                .is_some()
        );

        // A message still queued when the host restarts fails like a running one.
        let stuck = NewStep { run_id: "run3", bot_id: "d", ..queued };
        let (stuck, _) = store.flow_add_step(&stuck).unwrap().unwrap();
        let (orphans, _) = store.flow_fail_orphans("host restarted").unwrap();
        assert_eq!(orphans.len(), 1);
        assert_eq!(orphans[0].id, stuck.id);
        assert_eq!(store.flow_run_steps("run3").unwrap().pop().unwrap().status, StepStatus::Failed);
    }

    #[test]
    fn a_batch_progress_is_read_with_the_step_for_clients() {
        let store = Store::open(std::path::Path::new(":memory:")).expect("a store");
        let new = NewStep {
            run_id: "run",
            routine_id: "r",
            owner_bot_id: "a",
            bot_id: "media:m",
            run_at: 1,
            input: "",
            status: StepStatus::Running,
            output: None,
            error: None,
        };
        let (step, _) = store.flow_add_step(&new).unwrap().unwrap();
        assert!(step.results.is_none(), "a step made before a batch has no per-result status");
        let results = crate::media::batch_results(
            2,
            &std::collections::BTreeSet::from([0u32]),
            &std::collections::BTreeMap::new(),
        );
        store.flow_set_step_settings(&step.id, Some(json!({"kind": "image", "batch": 2, "results": results}))).unwrap();
        let read = store.flow_run_steps("run").unwrap().pop().unwrap();
        assert_eq!(read.results.unwrap().len(), 2);
        assert_eq!(serde_json::to_value(&read).unwrap()["results"][0]["status"], json!("succeeded"));
    }
}
