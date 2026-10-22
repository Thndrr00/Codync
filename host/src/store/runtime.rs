//! The Job Runtime Contract (`assignments` and `assignment_events` in SQLite).
//!
//! Tracks every inter-bot delegation as a formal tree node with immutable origin,
//! depth cap (`MAX_ASSIGNMENT_DEPTH`), step budget, permission ceiling (`effective_permission`),
//! and atomic SQLite state transitions.

use super::{Permission, Store, now_ms};
use crate::LockExt;
use anyhow::{Result, bail, ensure};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

/// Maximum delegation depth in an assignment tree (OpenMausBot §4).
pub const MAX_ASSIGNMENT_DEPTH: u8 = 4;
/// Default step budget for a newly created assignment.
pub const DEFAULT_STEP_BUDGET: i64 = 48;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AssignmentOrigin {
    Person,
    Routine,
    External,
}

impl AssignmentOrigin {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Person => "person",
            Self::Routine => "routine",
            Self::External => "external",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "person" => Self::Person,
            "routine" => Self::Routine,
            _ => Self::External,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AssignmentMode {
    Work,
    Consult,
    Notify,
}

impl AssignmentMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Work => "work",
            Self::Consult => "consult",
            Self::Notify => "notify",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "consult" => Self::Consult,
            "notify" => Self::Notify,
            _ => Self::Work,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReplyMode {
    Resume,
    Poll,
    None,
}

impl ReplyMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Resume => "resume",
            Self::Poll => "poll",
            Self::None => "none",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "poll" => Self::Poll,
            "none" => Self::None,
            _ => Self::Resume,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssignmentStatus {
    Queued,
    Running,
    Waiting,
    NeedsInput,
    Continue,
    Settled,
}

impl AssignmentStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Waiting => "waiting",
            Self::NeedsInput => "needs_input",
            Self::Continue => "continue",
            Self::Settled => "settled",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "running" => Self::Running,
            "waiting" => Self::Waiting,
            "needs_input" => Self::NeedsInput,
            "continue" => Self::Continue,
            "settled" => Self::Settled,
            _ => Self::Queued,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AssignmentOutcome {
    Completed,
    Failed,
    Cancelled,
}

impl AssignmentOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "completed" => Self::Completed,
            "cancelled" => Self::Cancelled,
            _ => Self::Failed,
        }
    }
}

/// Governance rule (INIC-11): a delegated turn never exceeds either the assignee's own
/// configured permission or the permission ceiling inherited from the root/requester.
/// If either side requires `Ask`, the effective permission is `Ask`.
pub fn effective_permission(own: Permission, inherited: Permission) -> Permission {
    match (own, inherited) {
        (Permission::Auto, Permission::Auto) => Permission::Auto,
        _ => Permission::Ask,
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Assignment {
    pub assignment_id: String,
    pub root_id: String,
    pub parent_id: Option<String>,
    pub depth: u8,
    pub origin: AssignmentOrigin,
    pub requester_bot_id: Option<String>,
    pub assignee_bot_id: String,
    pub thread_id: Option<String>,
    pub mode: AssignmentMode,
    pub reply_mode: ReplyMode,
    pub status: AssignmentStatus,
    pub outcome: Option<AssignmentOutcome>,
    pub outcome_detail: Option<String>,
    pub auth_level: Permission,
    pub step_budget: i64,
    pub steps_used: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentEvent {
    pub event_id: i64,
    pub assignment_id: String,
    pub event_type: String,
    pub payload: Option<String>,
    pub created_at: i64,
}

pub struct NewAssignment<'a> {
    pub assignment_id: &'a str,
    pub parent_id: Option<&'a str>,
    pub origin: AssignmentOrigin,
    pub requester_bot_id: Option<&'a str>,
    pub assignee_bot_id: &'a str,
    pub thread_id: Option<&'a str>,
    pub mode: AssignmentMode,
    pub reply_mode: ReplyMode,
    pub own_permission: Permission,
    pub inherited_permission: Permission,
    pub step_budget: i64,
}

pub(super) fn create_tables(c: &Connection) -> Result<()> {
    c.execute_batch(
        "CREATE TABLE IF NOT EXISTS assignments (
            assignment_id TEXT PRIMARY KEY,
            root_id TEXT NOT NULL,
            parent_id TEXT,
            depth INTEGER NOT NULL,
            origin TEXT NOT NULL,
            requester_bot_id TEXT,
            assignee_bot_id TEXT NOT NULL,
            thread_id TEXT,
            mode TEXT NOT NULL,
            reply_mode TEXT NOT NULL,
            status TEXT NOT NULL,
            outcome TEXT,
            outcome_detail TEXT,
            auth_level TEXT NOT NULL,
            step_budget INTEGER NOT NULL,
            steps_used INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS assignments_root ON assignments(root_id);
        CREATE INDEX IF NOT EXISTS assignments_assignee ON assignments(assignee_bot_id, status);
        CREATE TABLE IF NOT EXISTS assignment_events (
            event_id INTEGER PRIMARY KEY AUTOINCREMENT,
            assignment_id TEXT NOT NULL,
            event_type TEXT NOT NULL,
            payload TEXT,
            created_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS assignment_events_aid ON assignment_events(assignment_id, event_id);",
    )?;
    Ok(())
}

fn row_assignment(r: &rusqlite::Row<'_>) -> rusqlite::Result<Assignment> {
    let auth: String = r.get(13)?;
    let auth_level = if auth == "ask" { Permission::Ask } else { Permission::Auto };
    let outcome: Option<String> = r.get(11)?;
    Ok(Assignment {
        assignment_id: r.get(0)?,
        root_id: r.get(1)?,
        parent_id: r.get(2)?,
        depth: r.get::<_, u8>(3)?,
        origin: AssignmentOrigin::parse(&r.get::<_, String>(4)?),
        requester_bot_id: r.get(5)?,
        assignee_bot_id: r.get(6)?,
        thread_id: r.get(7)?,
        mode: AssignmentMode::parse(&r.get::<_, String>(8)?),
        reply_mode: ReplyMode::parse(&r.get::<_, String>(9)?),
        status: AssignmentStatus::parse(&r.get::<_, String>(10)?),
        outcome: outcome.as_deref().map(AssignmentOutcome::parse),
        outcome_detail: r.get(12)?,
        auth_level,
        step_budget: r.get(14)?,
        steps_used: r.get(15)?,
        created_at: r.get(16)?,
        updated_at: r.get(17)?,
    })
}

const COLS: &str = "assignment_id, root_id, parent_id, depth, origin, requester_bot_id, assignee_bot_id, \
                    thread_id, mode, reply_mode, status, outcome, outcome_detail, auth_level, step_budget, \
                    steps_used, created_at, updated_at";

impl Store {
    /// Enqueues a new assignment atomically in SQLite, checking for cycles and depth limits.
    pub fn enqueue_assignment(&self, req: &NewAssignment<'_>) -> Result<Assignment> {
        ensure!(req.step_budget > 0, "step_budget must be positive");
        if let Some(requester) = req.requester_bot_id
            && requester == req.assignee_bot_id
        {
            bail!("refused(cycle_detected): a bot cannot assign work to itself");
        }
        let mut c = self.db.locked();
        let tx = c.transaction()?;
        let (root_id, depth, origin, inherited_perm) = if let Some(pid) = req.parent_id {
            let parent = tx
                .query_row(&format!("SELECT {COLS} FROM assignments WHERE assignment_id = ?1"), [pid], row_assignment)
                .optional()?
                .ok_or_else(|| anyhow::anyhow!("parent assignment {pid} not found"))?;
            if parent.depth >= MAX_ASSIGNMENT_DEPTH {
                bail!("refused(depth_exceeded): max assignment depth ({MAX_ASSIGNMENT_DEPTH}) reached");
            }
            // Walk up the ancestor chain to reject any cycle (A -> B -> A).
            let mut cursor = Some(parent.clone());
            while let Some(node) = cursor {
                if node.assignee_bot_id == req.assignee_bot_id
                    || node.requester_bot_id.as_deref() == Some(req.assignee_bot_id)
                {
                    bail!("refused(cycle_detected): assignee {} is already in the ancestor chain", req.assignee_bot_id);
                }
                cursor = match node.parent_id.as_deref() {
                    Some(next_pid) => tx
                        .query_row(
                            &format!("SELECT {COLS} FROM assignments WHERE assignment_id = ?1"),
                            [next_pid],
                            row_assignment,
                        )
                        .optional()?,
                    None => None,
                };
            }
            (
                parent.root_id,
                parent.depth + 1,
                parent.origin,
                effective_permission(req.inherited_permission, parent.auth_level),
            )
        } else {
            (req.assignment_id.to_owned(), 0, req.origin, req.inherited_permission)
        };

        let auth_level = effective_permission(req.own_permission, inherited_perm);
        let auth_str = match auth_level {
            Permission::Ask => "ask",
            Permission::Auto => "auto",
        };
        let now = now_ms();
        tx.execute(
            "INSERT INTO assignments (
                assignment_id, root_id, parent_id, depth, origin, requester_bot_id, assignee_bot_id,
                thread_id, mode, reply_mode, status, outcome, outcome_detail, auth_level,
                step_budget, steps_used, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'queued', NULL, NULL, ?11, ?12, 0, ?13, ?13)",
            params![
                req.assignment_id,
                root_id,
                req.parent_id,
                depth,
                origin.as_str(),
                req.requester_bot_id,
                req.assignee_bot_id,
                req.thread_id,
                req.mode.as_str(),
                req.reply_mode.as_str(),
                auth_str,
                req.step_budget,
                now,
            ],
        )?;
        tx.execute(
            "INSERT INTO assignment_events (assignment_id, event_type, payload, created_at)
             VALUES (?1, 'enqueued', NULL, ?2)",
            params![req.assignment_id, now],
        )?;
        let row = tx.query_row(
            &format!("SELECT {COLS} FROM assignments WHERE assignment_id = ?1"),
            [req.assignment_id],
            row_assignment,
        )?;
        tx.commit()?;
        Ok(row)
    }

    pub fn assignment(&self, id: &str) -> Result<Option<Assignment>> {
        let c = self.db.locked();
        Ok(c.query_row(&format!("SELECT {COLS} FROM assignments WHERE assignment_id = ?1"), [id], row_assignment)
            .optional()?)
    }

    pub fn assignment_events(&self, id: &str) -> Result<Vec<AssignmentEvent>> {
        let c = self.db.locked();
        let mut st = c.prepare(
            "SELECT event_id, assignment_id, event_type, payload, created_at
             FROM assignment_events WHERE assignment_id = ?1 ORDER BY event_id ASC",
        )?;
        let rows = st.query_map([id], |r| {
            Ok(AssignmentEvent {
                event_id: r.get(0)?,
                assignment_id: r.get(1)?,
                event_type: r.get(2)?,
                payload: r.get(3)?,
                created_at: r.get(4)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Transitions an assignment to `running` and logs a `started` event.
    pub fn mark_assignment_started(&self, id: &str) -> Result<Option<Assignment>> {
        self.transition_assignment(id, AssignmentStatus::Running, None, None, "started")
    }

    /// Parks an assignment on `needs_input`; if it has a parent, parks the parent on `waiting`.
    pub fn mark_assignment_needs_input(&self, id: &str, question: Option<&str>) -> Result<Option<Assignment>> {
        let mut c = self.db.locked();
        let tx = c.transaction()?;
        let now = now_ms();
        let Some(current) = tx
            .query_row(&format!("SELECT {COLS} FROM assignments WHERE assignment_id = ?1"), [id], row_assignment)
            .optional()?
        else {
            return Ok(None);
        };
        tx.execute(
            "UPDATE assignments SET status = 'needs_input', outcome_detail = ?2, updated_at = ?3 WHERE assignment_id = ?1",
            params![id, question, now],
        )?;
        tx.execute(
            "INSERT INTO assignment_events (assignment_id, event_type, payload, created_at) VALUES (?1, 'needs_input', ?2, ?3)",
            params![id, question, now],
        )?;
        if let Some(pid) = current.parent_id.as_deref() {
            tx.execute(
                "UPDATE assignments SET status = 'waiting', updated_at = ?2 WHERE assignment_id = ?1 AND status = 'running'",
                params![pid, now],
            )?;
            tx.execute(
                "INSERT INTO assignment_events (assignment_id, event_type, payload, created_at) VALUES (?1, 'waiting', ?2, ?3)",
                params![pid, id, now],
            )?;
        }
        let updated =
            tx.query_row(&format!("SELECT {COLS} FROM assignments WHERE assignment_id = ?1"), [id], row_assignment)?;
        tx.commit()?;
        Ok(Some(updated))
    }

    /// Records a tool/step execution on an assignment; if `steps_used` exceeds `step_budget`,
    /// settles the assignment as `failed(budget_exceeded)`.
    pub fn record_assignment_step(&self, id: &str) -> Result<Option<Assignment>> {
        let mut c = self.db.locked();
        let tx = c.transaction()?;
        let now = now_ms();
        let Some(current) = tx
            .query_row(&format!("SELECT {COLS} FROM assignments WHERE assignment_id = ?1"), [id], row_assignment)
            .optional()?
        else {
            return Ok(None);
        };
        if current.status == AssignmentStatus::Settled {
            return Ok(Some(current));
        }
        let next_steps = current.steps_used + 1;
        if next_steps > current.step_budget {
            let detail = format!("budget_exceeded: used {next_steps} steps (budget {})", current.step_budget);
            tx.execute(
                "UPDATE assignments SET steps_used = ?2, status = 'settled', outcome = 'failed', outcome_detail = ?3, updated_at = ?4
                 WHERE assignment_id = ?1",
                params![id, next_steps, detail, now],
            )?;
            tx.execute(
                "INSERT INTO assignment_events (assignment_id, event_type, payload, created_at)
                 VALUES (?1, 'budget_exceeded', ?2, ?3)",
                params![id, detail, now],
            )?;
        } else {
            tx.execute(
                "UPDATE assignments SET steps_used = ?2, updated_at = ?3 WHERE assignment_id = ?1",
                params![id, next_steps, now],
            )?;
        }
        let updated =
            tx.query_row(&format!("SELECT {COLS} FROM assignments WHERE assignment_id = ?1"), [id], row_assignment)?;
        tx.commit()?;
        Ok(Some(updated))
    }

    /// Settles an assignment (`completed`, `failed`, or `cancelled`) atomically.
    pub fn settle_assignment(
        &self,
        id: &str,
        outcome: AssignmentOutcome,
        detail: Option<&str>,
    ) -> Result<Option<Assignment>> {
        self.transition_assignment(id, AssignmentStatus::Settled, Some(outcome), detail, "settled")
    }

    fn transition_assignment(
        &self,
        id: &str,
        status: AssignmentStatus,
        outcome: Option<AssignmentOutcome>,
        detail: Option<&str>,
        event_type: &str,
    ) -> Result<Option<Assignment>> {
        let mut c = self.db.locked();
        let tx = c.transaction()?;
        let now = now_ms();
        let changed = tx.execute(
            "UPDATE assignments SET status = ?2, outcome = ?3, outcome_detail = ?4, updated_at = ?5
             WHERE assignment_id = ?1",
            params![id, status.as_str(), outcome.map(AssignmentOutcome::as_str), detail, now],
        )?;
        if changed == 0 {
            return Ok(None);
        }
        tx.execute(
            "INSERT INTO assignment_events (assignment_id, event_type, payload, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![id, event_type, detail, now],
        )?;
        let updated =
            tx.query_row(&format!("SELECT {COLS} FROM assignments WHERE assignment_id = ?1"), [id], row_assignment)?;
        tx.commit()?;
        Ok(Some(updated))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::tests::temp_store;

    #[test]
    fn rejects_cycles_and_enforces_step_budget_and_permission_ceiling() {
        let s = temp_store();
        let root = s
            .enqueue_assignment(&NewAssignment {
                assignment_id: "a1",
                parent_id: None,
                origin: AssignmentOrigin::Person,
                requester_bot_id: Some("bot_a"),
                assignee_bot_id: "bot_b",
                thread_id: None,
                mode: AssignmentMode::Work,
                reply_mode: ReplyMode::Resume,
                own_permission: Permission::Auto,
                inherited_permission: Permission::Ask,
                step_budget: 2,
            })
            .expect("root enqueued");
        assert_eq!(root.depth, 0);
        // Governance: Auto bot under Ask root degrades to Ask.
        assert_eq!(root.auth_level, Permission::Ask);

        // Cycle A -> B -> A is rejected immediately.
        let cycle = s.enqueue_assignment(&NewAssignment {
            assignment_id: "a2",
            parent_id: Some("a1"),
            origin: AssignmentOrigin::Person,
            requester_bot_id: Some("bot_b"),
            assignee_bot_id: "bot_a",
            thread_id: None,
            mode: AssignmentMode::Work,
            reply_mode: ReplyMode::Resume,
            own_permission: Permission::Auto,
            inherited_permission: Permission::Auto,
            step_budget: 5,
        });
        assert!(cycle.unwrap_err().to_string().contains("refused(cycle_detected)"));

        // Valid child B -> C works and inherits Person origin and Ask ceiling.
        let child = s
            .enqueue_assignment(&NewAssignment {
                assignment_id: "a3",
                parent_id: Some("a1"),
                origin: AssignmentOrigin::External,
                requester_bot_id: Some("bot_b"),
                assignee_bot_id: "bot_c",
                thread_id: None,
                mode: AssignmentMode::Consult,
                reply_mode: ReplyMode::Resume,
                own_permission: Permission::Auto,
                inherited_permission: Permission::Auto,
                step_budget: 2,
            })
            .expect("child enqueued");
        assert_eq!(child.depth, 1);
        assert_eq!(child.origin, AssignmentOrigin::Person, "origin is immutable down the tree");
        assert_eq!(child.auth_level, Permission::Ask);

        // Child needs_input parks parent on waiting.
        s.mark_assignment_started("a1").unwrap();
        s.mark_assignment_started("a3").unwrap();
        s.mark_assignment_needs_input("a3", Some("Which region?")).unwrap();
        assert_eq!(s.assignment("a1").unwrap().unwrap().status, AssignmentStatus::Waiting);

        // Step budget enforcement on a3 (budget = 2).
        s.record_assignment_step("a3").unwrap();
        let step2 = s.record_assignment_step("a3").unwrap().unwrap();
        assert_ne!(step2.status, AssignmentStatus::Settled);
        let step3 = s.record_assignment_step("a3").unwrap().unwrap();
        assert_eq!(step3.status, AssignmentStatus::Settled);
        assert_eq!(step3.outcome, Some(AssignmentOutcome::Failed));
        assert!(step3.outcome_detail.unwrap().starts_with("budget_exceeded"));
        let events = s.assignment_events("a3").unwrap();
        assert!(!events.is_empty());
    }
}
