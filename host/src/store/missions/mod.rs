//! Missions: an objective that outlives the turn. A mission remembers what was done, what is left and
//! why it stopped, so a later turn — or a person — can pick it up instead of guessing.
//!
//! A mission moves through `proposed` → `active` ⇄ `waiting` → `done`, and from any open state to
//! `abandoned` with a reason; the steps and the checkpoints stay as the history. The run of a routine
//! that executes it and the approval that blocks it are links, not copies, so both can go without
//! losing the mission.

use super::{Store, now_ms};
use crate::LockExt;
use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;

/// A title is a line; a goal, a checkpoint and a stop reason are paragraphs.
pub const MAX_TITLE: usize = 200;
pub const MAX_TEXT: usize = 4000;
/// Steps one mission may carry; past it the checkpoint is the summary.
pub const MAX_STEPS: i64 = 100;
/// Open missions a bot and the whole fleet may run at once: a fleet of parallel missions decides nothing.
pub const MAX_ACTIVE_PER_BOT: i64 = 3;
pub const MAX_ACTIVE_FLEET: i64 = 12;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MissionStatus {
    Proposed,
    Active,
    Waiting,
    Done,
    Abandoned,
}

impl MissionStatus {
    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "proposed" => Some(Self::Proposed),
            "active" => Some(Self::Active),
            "waiting" => Some(Self::Waiting),
            "done" => Some(Self::Done),
            "abandoned" => Some(Self::Abandoned),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Proposed => "proposed",
            Self::Active => "active",
            Self::Waiting => "waiting",
            Self::Done => "done",
            Self::Abandoned => "abandoned",
        }
    }

    pub fn open(self) -> bool {
        matches!(self, Self::Proposed | Self::Active | Self::Waiting)
    }

    /// Where this state may go next: one step along the mission, or straight out of it with a reason.
    /// A finished or proposed one is reopened or dropped, never finished straight away: only a
    /// mission somebody had running can claim to be done.
    fn next(self) -> &'static [MissionStatus] {
        match self {
            Self::Active | Self::Waiting => {
                &[MissionStatus::Active, MissionStatus::Waiting, MissionStatus::Done, MissionStatus::Abandoned]
            }
            Self::Proposed | Self::Done => &[MissionStatus::Active, MissionStatus::Abandoned],
            Self::Abandoned => &[MissionStatus::Active],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MissionStepStatus {
    Pending,
    Active,
    Done,
    Skipped,
}

impl MissionStepStatus {
    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "pending" => Some(Self::Pending),
            "active" => Some(Self::Active),
            "done" => Some(Self::Done),
            "skipped" => Some(Self::Skipped),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Active => "active",
            Self::Done => "done",
            Self::Skipped => "skipped",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mission {
    pub id: String,
    pub bot_id: String,
    pub title: String,
    pub goal: String,
    pub status: MissionStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
    /// What is done and what is left, in the bot's words.
    pub checkpoint: String,
    /// The routine run executing it, while one does.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    /// The approval entry it waits on, while one blocks it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub steps: Vec<MissionStep>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MissionStep {
    pub id: String,
    pub mission_id: String,
    pub position: i64,
    pub text: String,
    pub status: MissionStepStatus,
    pub at: i64,
}

/// The columns of a mission row, in [`row`]'s order.
const COLUMNS: &str = "id, bot_id, title, goal, status, stop_reason, checkpoint, run_id, approval_id, created_at, \
                       updated_at";

fn row(r: &rusqlite::Row) -> rusqlite::Result<Mission> {
    Ok(Mission {
        id: r.get(0)?,
        bot_id: r.get(1)?,
        title: r.get(2)?,
        goal: r.get(3)?,
        status: MissionStatus::parse(&r.get::<_, String>(4)?).unwrap_or(MissionStatus::Proposed),
        stop_reason: r.get(5)?,
        checkpoint: r.get(6)?,
        run_id: r.get(7)?,
        approval_id: r.get(8)?,
        created_at: r.get(9)?,
        updated_at: r.get(10)?,
        steps: Vec::new(),
    })
}

fn row_step(r: &rusqlite::Row) -> rusqlite::Result<MissionStep> {
    Ok(MissionStep {
        id: r.get(0)?,
        mission_id: r.get(1)?,
        position: r.get(2)?,
        text: r.get(3)?,
        status: MissionStepStatus::parse(&r.get::<_, String>(4)?).unwrap_or(MissionStepStatus::Pending),
        at: r.get(5)?,
    })
}

pub(super) fn open(c: &Connection) -> Result<()> {
    c.execute_batch(
        "CREATE TABLE IF NOT EXISTS missions (
            id TEXT PRIMARY KEY,
            bot_id TEXT NOT NULL,
            title TEXT NOT NULL,
            goal TEXT NOT NULL DEFAULT '',
            status TEXT NOT NULL CHECK(status IN ('proposed','active','waiting','done','abandoned')),
            stop_reason TEXT,
            checkpoint TEXT NOT NULL DEFAULT '',
            run_id TEXT,
            approval_id TEXT,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL);
         CREATE INDEX IF NOT EXISTS missions_bot ON missions(bot_id, status, updated_at DESC);
         CREATE INDEX IF NOT EXISTS missions_status ON missions(status, updated_at DESC);
         CREATE INDEX IF NOT EXISTS missions_run ON missions(run_id);
         CREATE TABLE IF NOT EXISTS mission_steps (
            id TEXT PRIMARY KEY,
            mission_id TEXT NOT NULL,
            position INTEGER NOT NULL,
            text TEXT NOT NULL,
            status TEXT NOT NULL CHECK(status IN ('pending','active','done','skipped')),
            at INTEGER NOT NULL);
         CREATE INDEX IF NOT EXISTS mission_steps_mission ON mission_steps(mission_id, position);",
    )?;
    Ok(())
}

fn trimmed(raw: &str, key: &str, max: usize) -> Result<String> {
    let text = raw.trim();
    if text.is_empty() {
        bail!("{key} is required");
    }
    if text.len() > max {
        bail!("{key} must be at most {max} bytes");
    }
    Ok(text.to_owned())
}

impl Store {
    /// Records a new mission for `bot`, with its steps still pending. It starts `proposed`: a person or
    /// the bot activates it, which is when it counts against the limits.
    pub fn create_mission(&self, bot_id: &str, title: &str, goal: &str, steps: &[String]) -> Result<Mission> {
        let title = trimmed(title, "title", MAX_TITLE)?;
        let goal = if goal.trim().is_empty() { title.clone() } else { trimmed(goal, "goal", MAX_TEXT)? };
        if i64::try_from(steps.len())? > MAX_STEPS {
            bail!("a mission takes at most {MAX_STEPS} steps");
        }
        let now = now_ms();
        let id = uuid::Uuid::new_v4().to_string();
        let c = self.db.locked();
        c.execute(
            "INSERT INTO missions(id, bot_id, title, goal, status, checkpoint, created_at, updated_at)
             VALUES(?1, ?2, ?3, ?4, 'proposed', '', ?5, ?5)",
            params![id, bot_id, title, goal, now],
        )?;
        for text in steps.iter().map(|text| text.trim()).filter(|text| !text.is_empty()) {
            c.execute(
                "INSERT INTO mission_steps(id, mission_id, position, text, status, at)
                 VALUES(?1, ?2, (SELECT COALESCE(MAX(position) + 1, 0) FROM mission_steps WHERE mission_id = ?2),
                        ?3, 'pending', ?4)",
                params![uuid::Uuid::new_v4().to_string(), id, text, now],
            )?;
        }
        drop(c);
        self.mission(&id)?.context("the mission just written is gone")
    }

    /// Missions, newest first. A bot narrows it to one, `query` matches the title, and `open` keeps only
    /// the ones that are still running.
    pub fn missions(&self, bot: Option<&str>, query: Option<&str>, open: bool) -> Result<Vec<Mission>> {
        let c = self.db.locked();
        let mut sql = format!("SELECT {COLUMNS} FROM missions WHERE 1 = 1");
        let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        if let Some(bot) = bot {
            args.push(Box::new(bot.to_owned()));
            write!(sql, " AND bot_id = ?{}", args.len())?;
        }
        if let Some(query) = query.map(str::trim).filter(|q| !q.is_empty()) {
            args.push(Box::new(format!("%{query}%")));
            write!(sql, " AND title LIKE ?{} ESCAPE '\\'", args.len())?;
        }
        if open {
            sql.push_str(" AND status IN ('proposed','active','waiting')");
        }
        sql.push_str(" ORDER BY updated_at DESC, id LIMIT 200");
        let mut st = c.prepare(&sql)?;
        let mut missions = st
            .query_map(rusqlite::params_from_iter(args.iter().map(AsRef::as_ref)), row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        drop(st);
        for mission in &mut missions {
            mission.steps = steps(&c, &mission.id)?;
        }
        Ok(missions)
    }

    /// One mission with its steps.
    pub fn mission(&self, id: &str) -> Result<Option<Mission>> {
        let c = self.db.locked();
        let Some(mut mission) =
            c.query_row(&format!("SELECT {COLUMNS} FROM missions WHERE id = ?1"), [id], row).optional()?
        else {
            return Ok(None);
        };
        mission.steps = steps(&c, id)?;
        Ok(Some(mission))
    }

    fn existing(&self, id: &str) -> Result<MissionStatus> {
        self.mission(id)?.map(|m| m.status).context("mission not found")
    }

    /// Moves a mission to a status, with the reason an abandonment needs and the run or approval it now
    /// waits on. The limits are checked when it goes active.
    pub fn set_mission_status(&self, id: &str, status: MissionStatus, reason: Option<&str>) -> Result<Mission> {
        let from = self.existing(id)?;
        if from != status && !from.next().contains(&status) {
            bail!("a {} mission cannot become {}", from.as_str(), status.as_str());
        }
        let reason =
            reason.map(str::trim).filter(|r| !r.is_empty()).map(|r| trimmed(r, "reason", MAX_TEXT)).transpose()?;
        if status == MissionStatus::Abandoned && reason.is_none() && from != MissionStatus::Abandoned {
            bail!("abandoning a mission needs a reason");
        }
        if status == MissionStatus::Active && from != MissionStatus::Active {
            self.check_limits(id)?;
        }
        let now = now_ms();
        let c = self.db.locked();
        c.execute(
            "UPDATE missions SET status = ?2, stop_reason = ?3, updated_at = ?4 WHERE id = ?1",
            params![id, status.as_str(), reason, now],
        )?;
        drop(c);
        self.mission(id)?.context("mission not found")
    }

    /// How many missions may still go active: one bot's own open ones, and the fleet's.
    fn check_limits(&self, id: &str) -> Result<()> {
        let c = self.db.locked();
        let (mine, fleet): (i64, i64) = c.query_row(
            "SELECT COALESCE(SUM(bot_id = (SELECT bot_id FROM missions WHERE id = ?1)), 0), COUNT(*)
             FROM missions WHERE status = 'active' AND id != ?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        if mine >= MAX_ACTIVE_PER_BOT {
            bail!("this bot already runs {MAX_ACTIVE_PER_BOT} active missions; finish or abandon one first");
        }
        if fleet >= MAX_ACTIVE_FLEET {
            bail!("the fleet already runs {MAX_ACTIVE_FLEET} active missions");
        }
        Ok(())
    }

    /// Records what is done and what is left, in the bot's words.
    pub fn set_mission_checkpoint(&self, id: &str, checkpoint: &str) -> Result<Mission> {
        self.existing(id)?;
        let checkpoint =
            if checkpoint.trim().is_empty() { String::new() } else { trimmed(checkpoint, "checkpoint", MAX_TEXT)? };
        self.db.locked().execute(
            "UPDATE missions SET checkpoint = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, checkpoint, now_ms()],
        )?;
        self.mission(id)?.context("mission not found")
    }

    /// Links the routine run executing the mission, or drops the link with `None`.
    pub fn set_mission_run(&self, id: &str, run_id: Option<&str>) -> Result<Mission> {
        self.existing(id)?;
        self.db
            .locked()
            .execute("UPDATE missions SET run_id = ?2, updated_at = ?3 WHERE id = ?1", params![id, run_id, now_ms()])?;
        self.mission(id)?.context("mission not found")
    }

    /// Links the approval blocking the mission, or drops the link with `None`.
    pub fn set_mission_approval(&self, id: &str, approval_id: Option<&str>) -> Result<Mission> {
        self.existing(id)?;
        self.db.locked().execute(
            "UPDATE missions SET approval_id = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, approval_id, now_ms()],
        )?;
        self.mission(id)?.context("mission not found")
    }

    /// Adds a step at the end, still pending.
    pub fn add_mission_step(&self, id: &str, text: &str) -> Result<Mission> {
        self.existing(id)?;
        let text = trimmed(text, "step", MAX_TEXT)?;
        let c = self.db.locked();
        let count: i64 = c.query_row("SELECT COUNT(*) FROM mission_steps WHERE mission_id = ?1", [id], |r| r.get(0))?;
        if count >= MAX_STEPS {
            bail!("a mission takes at most {MAX_STEPS} steps");
        }
        c.execute(
            "INSERT INTO mission_steps(id, mission_id, position, text, status, at)
             VALUES(?1, ?2, (SELECT COALESCE(MAX(position) + 1, 0) FROM mission_steps WHERE mission_id = ?2), ?3,
                    'pending', ?4)",
            params![uuid::Uuid::new_v4().to_string(), id, text, now_ms()],
        )?;
        drop(c);
        self.mission(id)?.context("mission not found")
    }

    /// Records how one step went.
    pub fn set_mission_step(&self, mission_id: &str, step_id: &str, status: MissionStepStatus) -> Result<Mission> {
        self.existing(mission_id)?;
        let changed = self.db.locked().execute(
            "UPDATE mission_steps SET status = ?3, at = ?4 WHERE id = ?1 AND mission_id = ?2",
            params![step_id, mission_id, status.as_str(), now_ms()],
        )?;
        if changed == 0 {
            bail!("step not found on this mission");
        }
        self.mission(mission_id)?.context("mission not found")
    }

    /// The active missions a restart interrupted, back to `waiting` with the reason. Their steps and
    /// checkpoint stay, so resuming is a matter of activating them again.
    pub fn park_interrupted_missions(&self) -> Result<usize> {
        Ok(self.db.locked().execute(
            "UPDATE missions SET status = 'waiting', stop_reason = 'the host restarted mid-run', run_id = NULL,
                    updated_at = ?1
             WHERE status = 'active' AND run_id IS NOT NULL",
            [now_ms()],
        )?)
    }
}

fn steps(c: &Connection, mission_id: &str) -> Result<Vec<MissionStep>> {
    let mut st = c.prepare(
        "SELECT id, mission_id, position, text, status, at FROM mission_steps WHERE mission_id = ?1 ORDER BY position",
    )?;
    Ok(st.query_map([mission_id], row_step)?.collect::<rusqlite::Result<_>>()?)
}

#[cfg(test)]
mod tests;
