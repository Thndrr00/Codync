//! A channel request's task tree: the request, its assignments to members, and their children
//! (`chat::group::tasks`). Rows live per lane, so a request in a thread never mixes with the chat's.

use super::{Store, now_ms};
use crate::LockExt;
use anyhow::Result;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Where one row of the tree stands. A paused or failed row offers Continue and Reassign; Cancel
/// ends a branch for good.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskState {
    /// Its member hasn't taken the turn yet.
    Queued,
    /// Its member is taking it now.
    Running,
    /// Its member took it (it answered, or passed).
    Completed,
    /// Its member's turn couldn't finish.
    Failed,
    /// Stopped: nothing below this row runs until Continue or Reassign resumes it.
    Paused,
    /// Cancelled or dropped: neither this row nor anything below it runs again.
    Cancelled,
}

impl TaskState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Paused => "paused",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        [Self::Queued, Self::Running, Self::Completed, Self::Failed, Self::Paused, Self::Cancelled]
            .into_iter()
            .find(|state| state.as_str() == raw)
    }

    /// Still part of the request's work: it may take a turn, or wait for one.
    pub fn is_open(self) -> bool {
        matches!(self, Self::Queued | Self::Running | Self::Paused | Self::Failed)
    }
}

/// The request itself, or one member's assignment under it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskKind {
    Request,
    Assignment,
}

impl TaskKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Request => "request",
            Self::Assignment => "assignment",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "request" => Some(Self::Request),
            "assignment" => Some(Self::Assignment),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GroupTask {
    pub id: String,
    pub group_id: String,
    /// The lane's key (`Lane::key`): the chat, or its thread.
    pub lane: String,
    /// The user's message this tree answers.
    pub request_id: String,
    /// The root row (itself, for the request).
    pub root_id: String,
    pub parent_id: Option<String>,
    /// The member the row belongs to; none for the request when the channel has no lead.
    pub owner_id: Option<String>,
    pub kind: TaskKind,
    pub state: TaskState,
    /// The room turn's round it belongs to; the request is 0.
    pub round: i32,
    /// The request's text, or the line that brought its member in.
    pub instruction: String,
    /// Why it stopped or failed, in the members' words.
    pub note: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

pub(super) fn open(c: &Connection) -> Result<()> {
    c.execute_batch(
        "CREATE TABLE IF NOT EXISTS group_tasks (
            id TEXT PRIMARY KEY,
            group_id TEXT NOT NULL,
            lane TEXT NOT NULL,
            request_id TEXT NOT NULL,
            root_id TEXT NOT NULL,
            parent_id TEXT,
            owner_id TEXT,
            kind TEXT NOT NULL,
            state TEXT NOT NULL,
            round INTEGER NOT NULL DEFAULT 0,
            instruction TEXT NOT NULL DEFAULT '',
            note TEXT,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_group_tasks_lane ON group_tasks(group_id, lane, created_at);
        CREATE INDEX IF NOT EXISTS idx_group_tasks_root ON group_tasks(root_id);",
    )?;
    Ok(())
}

fn row_task(r: &rusqlite::Row) -> rusqlite::Result<GroupTask> {
    let kind: String = r.get(7)?;
    let state: String = r.get(8)?;
    Ok(GroupTask {
        id: r.get(0)?,
        group_id: r.get(1)?,
        lane: r.get(2)?,
        request_id: r.get(3)?,
        root_id: r.get(4)?,
        parent_id: r.get(5)?,
        owner_id: r.get(6)?,
        kind: TaskKind::parse(&kind).unwrap_or(TaskKind::Assignment),
        state: TaskState::parse(&state).unwrap_or(TaskState::Queued),
        round: r.get(9)?,
        instruction: r.get(10)?,
        note: r.get(11)?,
        created_at: r.get(12)?,
        updated_at: r.get(13)?,
    })
}

const COLS: &str = "id, group_id, lane, request_id, root_id, parent_id, owner_id, kind, state, round, \
                    instruction, note, created_at, updated_at";

impl Store {
    pub fn save_group_task(&self, task: &GroupTask) -> Result<()> {
        let c = self.db.locked();
        c.execute(
            "INSERT INTO group_tasks (id, group_id, lane, request_id, root_id, parent_id, owner_id, kind, state,
                                      round, instruction, note, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
             ON CONFLICT(id) DO UPDATE SET parent_id = ?6, owner_id = ?7, state = ?9, round = ?10,
                                           instruction = ?11, note = ?12, updated_at = ?14",
            params![
                task.id,
                task.group_id,
                task.lane,
                task.request_id,
                task.root_id,
                task.parent_id,
                task.owner_id,
                task.kind.as_str(),
                task.state.as_str(),
                task.round,
                task.instruction,
                task.note,
                task.created_at,
                task.updated_at
            ],
        )?;
        Ok(())
    }

    pub fn group_task(&self, id: &str) -> Result<Option<GroupTask>> {
        let c = self.db.locked();
        Ok(c.query_row(&format!("SELECT {COLS} FROM group_tasks WHERE id = ?1"), [id], row_task).optional()?)
    }

    /// The newest request's tree in `lane`: the request itself first, then each assignment as it was made.
    pub fn group_tasks(&self, group: &str, lane: &str) -> Result<Vec<GroupTask>> {
        let Some(root) = self.latest_root(group, lane)? else { return Ok(vec![]) };
        let c = self.db.locked();
        let mut st = c.prepare(&format!(
            "SELECT {COLS} FROM group_tasks WHERE root_id = ?1 ORDER BY (kind = 'request') DESC, rowid LIMIT 200"
        ))?;
        Ok(st.query_map([root], row_task)?.collect::<rusqlite::Result<_>>()?)
    }

    /// The trees a client shows: every lane's newest request that still has open work.
    pub fn open_group_tasks(&self, group: &str) -> Result<Vec<GroupTask>> {
        let mut out: Vec<GroupTask> = vec![];
        let lanes: Vec<String> = {
            let c = self.db.locked();
            let mut st =
                c.prepare("SELECT DISTINCT lane FROM group_tasks WHERE group_id = ?1 ORDER BY lane LIMIT 20")?;
            st.query_map([group], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?
        };
        for lane in lanes {
            let rows = self.group_tasks(group, &lane)?;
            let open = rows.iter().any(|row| row.state.is_open());
            if open && rows.first().is_some_and(|root| root.state != TaskState::Cancelled) {
                out.extend(rows.into_iter().filter(|row| row.state != TaskState::Cancelled));
            }
        }
        Ok(out)
    }

    fn latest_root(&self, group: &str, lane: &str) -> Result<Option<String>> {
        let c = self.db.locked();
        Ok(c.query_row(
            "SELECT root_id FROM group_tasks WHERE group_id = ?1 AND lane = ?2 AND kind = 'request'
             ORDER BY rowid DESC LIMIT 1",
            params![group, lane],
            |r| r.get(0),
        )
        .optional()?)
    }

    /// Forgets every tree in `lane` but `keep`: a replaced request's rows have nothing left to do.
    pub fn delete_older_group_tasks(&self, group: &str, lane: &str, keep: &str) -> Result<()> {
        self.db.locked().execute(
            "DELETE FROM group_tasks WHERE group_id = ?1 AND lane = ?2 AND root_id <> ?3",
            params![group, lane, keep],
        )?;
        Ok(())
    }
}

/// `id` and everything below it, in the order it was made.
pub fn branch(rows: &[GroupTask], id: &str) -> Vec<String> {
    let mut found: HashSet<String> = HashSet::from([id.to_owned()]);
    let mut out: Vec<String> = rows.iter().filter(|row| row.id == id).map(|row| row.id.clone()).collect();
    let mut growing = true;
    while growing {
        growing = false;
        for row in rows {
            let Some(parent) = &row.parent_id else { continue };
            if found.contains(parent) && found.insert(row.id.clone()) {
                out.push(row.id.clone());
                growing = true;
            }
        }
    }
    out
}

/// A tree row about to start, with its timestamps set.
#[allow(clippy::too_many_arguments, reason = "one plain row, spelled out where it is made")]
pub fn new_task(
    group: &str,
    lane: &str,
    request_id: &str,
    root_id: &str,
    parent_id: Option<&str>,
    owner_id: Option<&str>,
    kind: TaskKind,
    state: TaskState,
    round: i32,
    instruction: &str,
) -> GroupTask {
    let now = now_ms();
    GroupTask {
        id: uuid::Uuid::new_v4().to_string(),
        group_id: group.to_owned(),
        lane: lane.to_owned(),
        request_id: request_id.to_owned(),
        root_id: root_id.to_owned(),
        parent_id: parent_id.map(str::to_owned),
        owner_id: owner_id.map(str::to_owned),
        kind,
        state,
        round,
        instruction: instruction.chars().take(240).collect(),
        note: None,
        created_at: now,
        updated_at: now,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(id: &str, parent: Option<&str>, state: TaskState) -> GroupTask {
        let mut row = new_task("g", "g", "r", "root", parent, Some(id), TaskKind::Assignment, state, 0, "work");
        row.id = id.into();
        row
    }

    #[test]
    fn a_tree_round_trips_and_descendants_follow_its_branches() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let mut root = new_task("g", "g", "r", "root", None, None, TaskKind::Request, TaskState::Running, 0, "ship it");
        root.id = "root".into();
        let mut child = task("child", Some("root"), TaskState::Paused);
        child.root_id = "root".into();
        let mut grandchild = task("grandchild", Some("child"), TaskState::Queued);
        grandchild.root_id = "root".into();
        let mut other = task("other", Some("root"), TaskState::Queued);
        other.root_id = "root".into();
        for row in [&root, &child, &grandchild, &other] {
            store.save_group_task(row).unwrap();
        }
        let rows = store.group_tasks("g", "g").unwrap();
        assert_eq!(rows.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(), ["root", "child", "grandchild", "other"]);
        assert_eq!(branch(&rows, "child"), ["child", "grandchild"], "a branch is the row and its children");
        assert_eq!(branch(&rows, "root").len(), 4);
        child.state = TaskState::Paused;
        store.save_group_task(&child).unwrap();
        assert_eq!(store.group_task("child").unwrap().unwrap().state, TaskState::Paused);
        assert_eq!(store.open_group_tasks("g").unwrap().len(), 4, "the request still has open work");
        root.state = TaskState::Cancelled;
        store.save_group_task(&root).unwrap();
        assert!(store.open_group_tasks("g").unwrap().is_empty(), "a cancelled request is gone from the list");
    }
}
