//! Agent tasks: named, long-lived turn containers of an agent, separate from its chat.
//! A task is one row here plus a root notice entry in the bot's chat. Its messages are
//! ordinary entries in that root's thread (`Lane::in_thread`), so they keep the transcript's
//! rev, search, paging and per-thread agent session for free. Unread is the host's mark on the
//! task row (`read_rev`) weighed against those entries; no second store of messages is kept.
//! Origin: OpenBot nightly's `agent-tasks` subsystem, adapted to Codync's thread store.

use super::entries::NOT_REMOVED;
use super::visibility::PUBLIC;
use super::{Entry, Store, logged, next_rev, now_ms};
use crate::LockExt;
use anyhow::{Context, Result, bail, ensure};
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;

/// Longest task name a person can set.
pub const MAX_NAME: usize = 120;
/// How many tasks one bot may hold at once.
pub const MAX_PER_BOT: usize = 100;
/// Largest page of task messages one call can ask for.
pub const MAX_PAGE: i64 = 100;
/// Page returned when the caller gives no `limit`.
pub const DEFAULT_PAGE: i64 = 50;

/// Where a task is in its life. `open` accepts messages, `closed` is paused, `archived` is out of
/// the default list. The wire values are lowercase; an absent or old value reads as `open`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskState {
    #[default]
    Open,
    Closed,
    Archived,
}

impl TaskState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Closed => "closed",
            Self::Archived => "archived",
        }
    }

    /// The status filter of `taskList`; an unknown value is a caller error, not `open`.
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "open" => Ok(Self::Open),
            "closed" => Ok(Self::Closed),
            "archived" => Ok(Self::Archived),
            other => bail!("unknown task status `{other}`"),
        }
    }
}

/// A task as a client shows it: no messages, only its marks and its unread count.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskInfo {
    pub id: String,
    pub bot_id: String,
    pub name: String,
    #[serde(default)]
    pub state: TaskState,
    pub root: String,
    #[serde(default)]
    pub started_at: i64,
    #[serde(default)]
    pub closed_at: i64,
    /// Agent finals in this task newer than `read_rev`.
    #[serde(default)]
    pub unread: i64,
    #[serde(default)]
    pub rev: i64,
    #[serde(default)]
    pub updated_at: i64,
}

/// Trims and bounds a task name. Both create and rename go through this.
pub fn validate_name(name: &str) -> Result<String> {
    let name = name.trim();
    ensure!(!name.is_empty(), "a task needs a name");
    ensure!(name.chars().count() <= MAX_NAME, "a task name is at most {MAX_NAME} characters");
    Ok(name.to_owned())
}

/// A caller's page size, bounded on both ends so one request can't scan the whole thread.
pub fn clamp_limit(limit: Option<i64>) -> i64 {
    limit.unwrap_or(DEFAULT_PAGE).clamp(1, MAX_PAGE)
}

/// How many messages of `readTaskAround` sit on each side of the anchor.
pub fn around_split(limit: i64) -> (i64, i64) {
    let before = (limit / 2).max(0);
    (before, limit - before)
}

/// Idempotent create/alter, for old databases as well as ones built by this version.
/// Called by each operation because the store's central opener is owned by `schema.rs`.
pub(super) fn ready(c: &Connection) -> Result<()> {
    c.execute_batch(
        "CREATE TABLE IF NOT EXISTS tasks (
            id TEXT PRIMARY KEY,
            bot_id TEXT NOT NULL,
            name TEXT NOT NULL DEFAULT '',
            state TEXT NOT NULL DEFAULT 'open',
            root TEXT NOT NULL DEFAULT '',
            started_at INTEGER NOT NULL DEFAULT 0,
            closed_at INTEGER NOT NULL DEFAULT 0,
            read_rev INTEGER NOT NULL DEFAULT 0,
            rev INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL DEFAULT 0,
            updated_at INTEGER NOT NULL DEFAULT 0
         );",
    )?;
    // An earlier shape of this table may miss a column. Adding one that exists is a no-op error.
    // These come before the index, which names some of them.
    let _ = c.execute("ALTER TABLE tasks ADD COLUMN state TEXT NOT NULL DEFAULT 'open'", []);
    let _ = c.execute("ALTER TABLE tasks ADD COLUMN root TEXT NOT NULL DEFAULT ''", []);
    let _ = c.execute("ALTER TABLE tasks ADD COLUMN started_at INTEGER NOT NULL DEFAULT 0", []);
    let _ = c.execute("ALTER TABLE tasks ADD COLUMN closed_at INTEGER NOT NULL DEFAULT 0", []);
    let _ = c.execute("ALTER TABLE tasks ADD COLUMN read_rev INTEGER NOT NULL DEFAULT 0", []);
    let _ = c.execute("ALTER TABLE tasks ADD COLUMN rev INTEGER NOT NULL DEFAULT 0", []);
    let _ = c.execute("ALTER TABLE tasks ADD COLUMN created_at INTEGER NOT NULL DEFAULT 0", []);
    let _ = c.execute("ALTER TABLE tasks ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0", []);
    c.execute_batch("CREATE INDEX IF NOT EXISTS idx_tasks_bot ON tasks(bot_id, state, updated_at);")?;
    Ok(())
}

type TaskRow = (String, String, String, String, String, i64, i64, i64, i64, i64);

fn row(r: &rusqlite::Row) -> rusqlite::Result<TaskRow> {
    Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?, r.get(9)?))
}

/// Unread agent finals (and pending cards) after the task's own mark. Uses the same
/// `contentRev` rule as a chat's unread, so a reaction doesn't make a message new again.
fn unread_after(c: &Connection, root: &str, read_rev: i64) -> i64 {
    let n = c.query_row(
        &format!(
            "SELECT COUNT(*) FROM entries WHERE thread_id = ?1 AND {PUBLIC} AND
               ((kind = 'agent' AND json_extract(data, '$.final') = 1) OR
                (kind = 'permission' AND json_extract(data, '$.status') = 'pending')) AND
               COALESCE(json_extract(data, '$.contentRev'), rev) > ?2"
        ),
        params![root, read_rev],
        |r| r.get(0),
    );
    logged("task unread", n).unwrap_or(0)
}

fn info(c: &Connection, row: TaskRow) -> Result<TaskInfo> {
    let unread = unread_after(c, &row.4, row.7);
    Ok(TaskInfo {
        id: row.0,
        bot_id: row.1,
        name: row.2,
        state: TaskState::parse(&row.3)?,
        root: row.4,
        started_at: row.5,
        closed_at: row.6,
        unread,
        rev: row.8,
        updated_at: row.9,
    })
}

const TASK_COLS: &str = "id, bot_id, name, state, root, started_at, closed_at, read_rev, rev, updated_at";

fn by_id(c: &Connection, id: &str) -> Result<Option<TaskInfo>> {
    let found = c.query_row(&format!("SELECT {TASK_COLS} FROM tasks WHERE id = ?1"), [id], row).optional()?;
    found.map(|r| info(c, r)).transpose()
}

/// One bounded, oldest-first slice of a task's thread. `before` pages older, `after` pages newer,
/// neither gives the newest `limit`. A caller asks for `limit + 1` to learn if there is more.
fn page(c: &Connection, root: &str, limit: i64, before: Option<i64>, after: Option<i64>) -> Result<Vec<Entry>> {
    let (cursor, sql, reverse) = match (before, after) {
        (_, Some(after)) => (after, "AND seq > ?2 ORDER BY seq ASC LIMIT ?3", false),
        (Some(before), None) => (before, "AND seq < ?2 ORDER BY seq DESC LIMIT ?3", true),
        (None, None) => (i64::MAX, "AND seq < ?2 ORDER BY seq DESC LIMIT ?3", true),
    };
    let mut st = c.prepare(&format!(
        "SELECT seq, id, bot_id, rev, kind, turn, data, created_at, updated_at, thread_id
           FROM entries WHERE thread_id = ?1 AND {PUBLIC} AND {NOT_REMOVED} {sql}"
    ))?;
    let mut rows: Vec<Entry> =
        st.query_map(params![root, cursor, limit], super::entries::row_entry)?.collect::<rusqlite::Result<_>>()?;
    if reverse {
        rows.reverse();
    }
    Ok(rows)
}

impl Store {
    /// One task by id, or None when it was deleted.
    pub fn task(&self, id: &str) -> Result<Option<TaskInfo>> {
        let c = self.db.locked();
        ready(&c)?;
        by_id(&c, id)
    }

    /// A bot's tasks, most recently written first. `status` picks one state; None lists them all.
    pub fn tasks(&self, bot_id: &str, status: Option<TaskState>) -> Result<Vec<TaskInfo>> {
        let c = self.db.locked();
        ready(&c)?;
        let mut st = c.prepare(&format!(
            "SELECT {TASK_COLS} FROM tasks WHERE bot_id = ?1 AND (?2 IS NULL OR state = ?2)
              ORDER BY updated_at DESC, id DESC"
        ))?;
        let state = status.map(|s| s.as_str().to_owned());
        let rows = st.query_map(params![bot_id, state], row)?.collect::<rusqlite::Result<Vec<_>>>()?;
        rows.into_iter().map(|r| info(&c, r)).collect()
    }

    /// How many tasks this bot holds, for the per-bot cap before creating a root entry.
    pub fn task_count(&self, bot_id: &str) -> Result<usize> {
        let c = self.db.locked();
        ready(&c)?;
        Ok(c.query_row("SELECT COUNT(*) FROM tasks WHERE bot_id = ?1", [bot_id], |r| r.get::<_, i64>(0))? as usize)
    }

    /// Inserts a new open task. `root` is the chat entry its messages hang off.
    pub fn task_insert(&self, bot_id: &str, name: &str, root: &str) -> Result<TaskInfo> {
        let name = validate_name(name)?;
        let c = self.db.locked();
        ready(&c)?;
        let count =
            c.query_row("SELECT COUNT(*) FROM tasks WHERE bot_id = ?1", [bot_id], |r| r.get::<_, i64>(0))? as usize;
        ensure!(count < MAX_PER_BOT, "this bot already has {MAX_PER_BOT} tasks; delete one first");
        let id = format!("task:{}", uuid::Uuid::new_v4());
        let rev = next_rev(&c)?;
        let now = now_ms();
        c.execute(
            "INSERT INTO tasks(id, bot_id, name, state, root, started_at, closed_at, read_rev, rev, created_at, updated_at)
             VALUES(?1, ?2, ?3, 'open', ?4, ?5, 0, 0, ?6, ?5, ?5)",
            params![id, bot_id, name, root, now, rev],
        )?;
        by_id(&c, &id)?.context("task was not saved")
    }

    /// Moves a task to `state`. Starting re-stamps `started_at` only the first time; closing and
    /// archiving stamp `closed_at`, reopening clears it.
    pub fn task_set_state(&self, id: &str, state: TaskState, start: bool) -> Result<TaskInfo> {
        let c = self.db.locked();
        ready(&c)?;
        let started: i64 = c
            .query_row("SELECT started_at FROM tasks WHERE id = ?1", [id], |r| r.get(0))
            .optional()?
            .context("unknown task")?;
        let rev = next_rev(&c)?;
        let now = now_ms();
        let started_at = if start && started == 0 { now } else { started };
        let closed_at = if matches!(state, TaskState::Closed | TaskState::Archived) { now } else { 0 };
        c.execute(
            "UPDATE tasks SET state = ?2, started_at = ?3, closed_at = ?4, rev = ?5, updated_at = ?5 WHERE id = ?1",
            params![id, state.as_str(), started_at, closed_at, rev],
        )?;
        by_id(&c, id)?.context("unknown task")
    }

    pub fn task_rename(&self, id: &str, name: &str) -> Result<TaskInfo> {
        let name = validate_name(name)?;
        let c = self.db.locked();
        ready(&c)?;
        let rev = next_rev(&c)?;
        ensure!(
            c.execute("UPDATE tasks SET name = ?2, rev = ?3, updated_at = ?3 WHERE id = ?1", params![id, name, rev])
                > 0,
            "unknown task"
        );
        by_id(&c, id)?.context("unknown task")
    }

    /// Marks everything up to `read_rev` read. The caller passes an entry's rev for "read through
    /// that message", or the current rev for "all read".
    pub fn task_mark_read(&self, id: &str, read_rev: i64) -> Result<TaskInfo> {
        let c = self.db.locked();
        ready(&c)?;
        let rev = next_rev(&c)?;
        ensure!(
            c.execute(
                "UPDATE tasks SET read_rev = ?2, rev = ?3, updated_at = ?3 WHERE id = ?1",
                params![id, read_rev, rev]
            ) > 0,
            "unknown task"
        );
        by_id(&c, id)?.context("unknown task")
    }

    /// Removes the row and stamps a rev; the caller deletes the thread's entries first so every
    /// one of them reaches clients as an `entryDeleted`.
    pub fn task_delete(&self, id: &str) -> Result<Option<i64>> {
        let mut c = self.db.locked();
        ready(&c)?;
        let tx = c.transaction()?;
        let root: Option<String> =
            tx.query_row("SELECT root FROM tasks WHERE id = ?1", [id], |r| r.get(0)).optional()?;
        let Some(root) = root else { return Ok(None) };
        tx.execute("DELETE FROM tasks WHERE id = ?1", [id])?;
        // The root doubled as a thread root for the chip; don't leave its read mark behind.
        tx.execute("DELETE FROM thread_reads WHERE thread_id = ?1", [root])?;
        let rev = next_rev(&tx)?;
        tx.commit()?;
        Ok(Some(rev))
    }

    /// Every message id of a task's thread, including its root, oldest first.
    pub fn task_entry_ids(&self, root: &str) -> Result<Vec<String>> {
        let c = self.db.locked();
        ready(&c)?;
        let mut st = c.prepare("SELECT id FROM entries WHERE thread_id = ?1 OR id = ?1 ORDER BY seq")?;
        Ok(st.query_map([root], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?)
    }

    /// A bounded page of task messages, oldest first. Callers ask for `limit + 1` to tell
    /// whether another page exists.
    pub fn task_messages(&self, root: &str, limit: i64, before: Option<i64>, after: Option<i64>) -> Result<Vec<Entry>> {
        let c = self.db.locked();
        ready(&c)?;
        page(&c, root, limit, before, after)
    }

    /// Up to `limit` messages centred on `seq`: half before it, half from it onwards.
    pub fn task_messages_around(&self, root: &str, seq: i64, limit: i64) -> Result<Vec<Entry>> {
        let c = self.db.locked();
        ready(&c)?;
        let (back, forward) = around_split(limit);
        let mut rows: Vec<Entry> = Vec::new();
        if back > 0 {
            let mut st = c.prepare(&format!(
                "SELECT seq, id, bot_id, rev, kind, turn, data, created_at, updated_at, thread_id
                   FROM entries WHERE thread_id = ?1 AND {PUBLIC} AND {NOT_REMOVED} AND seq < ?2
                  ORDER BY seq DESC LIMIT ?3"
            ))?;
            let mut older: Vec<Entry> =
                st.query_map(params![root, seq, back], super::entries::row_entry)?.collect::<rusqlite::Result<_>>()?;
            older.reverse();
            rows.append(&mut older);
        }
        if forward > 0 {
            let mut st = c.prepare(&format!(
                "SELECT seq, id, bot_id, rev, kind, turn, data, created_at, updated_at, thread_id
                   FROM entries WHERE thread_id = ?1 AND {PUBLIC} AND {NOT_REMOVED} AND seq >= ?2
                  ORDER BY seq ASC LIMIT ?3"
            ))?;
            let mut newer: Vec<Entry> = st
                .query_map(params![root, seq, forward], super::entries::row_entry)?
                .collect::<rusqlite::Result<_>>()?;
            rows.append(&mut newer);
        }
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::model::{EntryKind, Lane};
    use crate::store::tests::temp_store;
    use serde_json::json;

    #[test]
    fn a_task_name_is_trimmed_and_bounded_on_create_and_rename() {
        assert_eq!(validate_name("  Review launch  ").unwrap(), "Review launch");
        assert!(validate_name("   ").is_err());
        assert!(validate_name(&"x".repeat(MAX_NAME)).is_ok());
        let too_long = "x".repeat(MAX_NAME + 1);
        assert!(validate_name(&too_long).is_err());
        let s = temp_store();
        let task = s.task_insert("b", "Draft", "task:r1").unwrap();
        assert!(s.task_rename(&task.id, &too_long).is_err());
        assert_eq!(s.task(&task.id).unwrap().unwrap().name, "Draft");
        assert_eq!(s.task_rename(&task.id, "  Final  ").unwrap().name, "Final");
    }

    #[test]
    fn page_sizes_and_around_splits_stay_bounded() {
        assert_eq!(clamp_limit(None), DEFAULT_PAGE);
        assert_eq!(clamp_limit(Some(0)), 1);
        assert_eq!(clamp_limit(Some(MAX_PAGE + 50)), MAX_PAGE);
        assert_eq!(clamp_limit(Some(7)), 7);
        assert_eq!(around_split(9), (4, 5));
        assert_eq!(around_split(1), (0, 1));
        assert_eq!(around_split(0), (0, 0));
    }

    #[test]
    fn status_filters_are_exact_and_old_values_default_to_open() {
        assert_eq!(TaskState::parse("open").unwrap(), TaskState::Open);
        assert_eq!(TaskState::parse("closed").unwrap(), TaskState::Closed);
        assert_eq!(TaskState::parse("archived").unwrap(), TaskState::Archived);
        assert!(TaskState::parse("running").is_err());
        assert_eq!(TaskState::default(), TaskState::Open);
        assert_eq!(serde_json::to_string(&TaskState::Archived).unwrap(), "\"archived\"");
    }

    #[test]
    fn a_task_counts_only_unread_finals_after_its_own_mark() {
        let s = temp_store();
        let task = s.task_insert("b", "Review", "task:r1").unwrap();
        assert_eq!(task.unread, 0);
        let lane = Lane::in_thread("b", &task.root);
        let a = s.insert_entry(&lane, EntryKind::Agent, 1, &json!({"text": "one", "final": true})).unwrap();
        s.insert_entry(&lane, EntryKind::Agent, 1, &json!({"text": "thinking", "final": false})).unwrap();
        assert_eq!(s.task(&task.id).unwrap().unwrap().unread, 1, "a draft doesn't count");
        let marked = s.task_mark_read(&task.id, a.rev).unwrap();
        assert_eq!(marked.unread, 0, "a read through that message hides it");
        s.insert_entry(&lane, EntryKind::Agent, 2, &json!({"text": "two", "final": true})).unwrap();
        assert_eq!(s.task(&task.id).unwrap().unwrap().unread, 1);
    }

    #[test]
    fn task_pages_are_bounded_and_walk_both_directions() {
        let s = temp_store();
        let task = s.task_insert("b", "Log", "task:r1").unwrap();
        let lane = Lane::in_thread("b", &task.root);
        let seqs: Vec<i64> = (1..=7)
            .map(|i| s.insert_entry(&lane, EntryKind::User, i, &json!({"text": format!("m{i}")})).unwrap().seq)
            .collect();
        let newest = s.task_messages(&task.root, 3, None, None).unwrap();
        assert_eq!(newest.len(), 3);
        assert_eq!(newest[0].seq, seqs[4], "no cursor gives the newest page, oldest first");
        let older = s.task_messages(&task.root, 3, Some(seqs[4]), None).unwrap();
        assert_eq!(older.len(), 3);
        assert_eq!(older[2].seq, seqs[3], "before pages towards the start");
        let newer = s.task_messages(&task.root, 3, None, Some(seqs[5])).unwrap();
        assert_eq!(newer.len(), 2);
        assert_eq!(newer[0].seq, seqs[6], "after pages towards the end");
        let around = s.task_messages_around(&task.root, seqs[3], 5).unwrap();
        assert_eq!(around.len(), 5);
        assert_eq!(around[2].seq, seqs[3], "the anchor sits in the middle");
    }

    #[test]
    fn the_cap_keeps_one_bot_bounded_but_not_another() {
        let s = temp_store();
        for i in 0..MAX_PER_BOT {
            s.task_insert("b", &format!("t{i}"), &format!("task:r{i}")).unwrap();
        }
        assert!(s.task_insert("b", "one too many", "task:extra").is_err());
        assert!(s.task_insert("other", "fine", "task:other").is_ok());
    }

    #[test]
    fn the_table_is_idempotent_when_old_rows_lack_newer_columns() {
        let dir = std::env::temp_dir().join(format!("codync-task-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("t.db");
        {
            let c = rusqlite::Connection::open(&db).unwrap();
            c.execute_batch(
                "CREATE TABLE kv(k TEXT PRIMARY KEY, v TEXT NOT NULL);
                 CREATE TABLE tasks(id TEXT PRIMARY KEY, bot_id TEXT NOT NULL, name TEXT NOT NULL);
                 INSERT INTO tasks(id, bot_id, name) VALUES('task:old', 'b', 'Old');",
            )
            .unwrap();
        }
        let c = rusqlite::Connection::open(&db).unwrap();
        ready(&c).unwrap();
        ready(&c).unwrap(); // running it twice is harmless
        let row = c.query_row(&format!("SELECT {TASK_COLS} FROM tasks WHERE id = 'task:old'"), [], row).unwrap();
        let old = info(&c, row).unwrap();
        assert_eq!(old.state, TaskState::Open, "an old row reads as open");
        assert_eq!(old.unread, 0);
        assert_eq!(old.root, "");
        let mut st = c.prepare(&format!("SELECT {TASK_COLS} FROM tasks WHERE bot_id = 'b'")).unwrap();
        let rows: Vec<TaskRow> = st.query_map([], row).unwrap().collect::<rusqlite::Result<_>>().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows.into_iter().filter(|r| r.3 == TaskState::Closed.as_str()).count(), 0);
    }
}
