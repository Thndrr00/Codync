//! A small board of tasks for one computer: three columns of work in progress and a fourth of what is
//! done. It belongs to the host, so every client sees the same board and it survives restarts.
//!
//! The board does not start turns by itself: a card can name the bot that owns it, and that chat is
//! one click away. Moving a card is the person's action.

use super::Store;
use crate::LockExt;
use anyhow::{Result, bail, ensure};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

const KEY: &str = "board.v1";
const LIMIT: usize = 500;
const MAX_TITLE_CHARS: usize = 200;
const MAX_DETAIL_CHARS: usize = 4_000;
pub const STATUSES: [&str; 4] = ["todo", "doing", "waiting", "done"];

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BoardTask {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub detail: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bot_id: Option<String>,
    /// A routine whose next finished run moves the card (`chat::board_auto`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub routine_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_at: Option<i64>,
    /// `daily` or `weekly`: finishing the card creates the next one with the due date moved on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeat: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

fn read(c: &rusqlite::Connection) -> Result<Vec<BoardTask>> {
    let value: Option<String> = c.query_row("SELECT v FROM kv WHERE k = ?1", [KEY], |row| row.get(0)).optional()?;
    match value {
        Some(value) => Ok(serde_json::from_str(&value)?),
        None => Ok(Vec::new()),
    }
}

fn write(c: &rusqlite::Connection, tasks: &[BoardTask]) -> Result<()> {
    if tasks.is_empty() {
        c.execute("DELETE FROM kv WHERE k = ?1", [KEY])?;
    } else {
        c.execute(
            "INSERT INTO kv(k, v) VALUES(?1, ?2) ON CONFLICT(k) DO UPDATE SET v = excluded.v",
            params![KEY, serde_json::to_string(tasks)?],
        )?;
    }
    Ok(())
}

fn checked_status(status: &str) -> Result<&str> {
    ensure!(STATUSES.contains(&status), "a task is todo, doing, waiting or done");
    Ok(status)
}

fn checked_repeat(repeat: Option<&str>) -> Result<Option<&str>> {
    match repeat.map(str::trim).filter(|repeat| !repeat.is_empty()) {
        Some(repeat) if repeat == "daily" || repeat == "weekly" => Ok(Some(repeat)),
        Some(_) => bail!("a card repeats daily or weekly"),
        None => Ok(None),
    }
}

impl Store {
    /// Every card, in the order it was created.
    pub fn board_tasks(&self) -> Result<Vec<BoardTask>> {
        let mut tasks = read(&self.db.locked())?;
        tasks.sort_by_key(|task| task.created_at);
        Ok(tasks)
    }

    /// Cards whose title or detail contains `query`, newest change first.
    pub fn search_board(&self, query: &str, limit: i64) -> Result<Vec<BoardTask>> {
        let needle = query.to_lowercase();
        let mut tasks: Vec<BoardTask> = read(&self.db.locked())?
            .into_iter()
            .filter(|task| task.title.to_lowercase().contains(&needle) || task.detail.to_lowercase().contains(&needle))
            .collect();
        tasks.sort_by_key(|task| std::cmp::Reverse(task.updated_at));
        tasks.truncate(usize::try_from(limit).unwrap_or(20));
        Ok(tasks)
    }

    /// Creates a card (no `id`) or changes one. A changed card keeps its creation time.
    #[allow(clippy::too_many_arguments)]
    pub fn save_board_task(
        &self,
        id: Option<&str>,
        title: &str,
        detail: &str,
        status: &str,
        bot_id: Option<&str>,
        routine_id: Option<&str>,
        due_at: Option<i64>,
        repeat: Option<&str>,
        now: i64,
    ) -> Result<BoardTask> {
        let title = title.trim();
        ensure!(!title.is_empty(), "a task needs a title");
        ensure!(title.chars().count() <= MAX_TITLE_CHARS, "a task title is at most {MAX_TITLE_CHARS} characters");
        ensure!(detail.chars().count() <= MAX_DETAIL_CHARS, "a task's detail is at most {MAX_DETAIL_CHARS} characters");
        let status = checked_status(status)?;
        let repeat = checked_repeat(repeat)?;
        let c = self.db.locked();
        let mut tasks = read(&c)?;
        let task = match id {
            Some(id) => {
                let task =
                    tasks.iter_mut().find(|task| task.id == id).ok_or_else(|| anyhow::anyhow!("no such task"))?;
                task.title = title.to_owned();
                task.detail = detail.to_owned();
                task.status = status.to_owned();
                task.bot_id = bot_id.map(str::to_owned);
                task.routine_id = routine_id.map(str::to_owned);
                task.due_at = due_at;
                task.repeat = repeat.map(str::to_owned);
                task.updated_at = now;
                task.clone()
            }
            None => {
                ensure!(tasks.len() < LIMIT, "the board holds at most {LIMIT} tasks; delete one first");
                let task = BoardTask {
                    id: uuid::Uuid::new_v4().to_string(),
                    title: title.to_owned(),
                    detail: detail.to_owned(),
                    status: status.to_owned(),
                    bot_id: bot_id.map(str::to_owned),
                    routine_id: routine_id.map(str::to_owned),
                    due_at,
                    repeat: repeat.map(str::to_owned),
                    created_at: now,
                    updated_at: now,
                };
                tasks.push(task.clone());
                task
            }
        };
        write(&c, &tasks)?;
        Ok(task)
    }

    /// Moves a card between columns. `false` when the card is gone. Finishing a repeating card
    /// creates the next one in To do, with its due date moved on.
    pub fn move_board_task(&self, id: &str, status: &str, now: i64) -> Result<bool> {
        let status = checked_status(status)?;
        let c = self.db.locked();
        let mut tasks = read(&c)?;
        let Some(task) = tasks.iter_mut().find(|task| task.id == id) else { return Ok(false) };
        let finished = status == "done" && task.status != "done";
        task.status = status.to_owned();
        task.updated_at = now;
        let next = if finished {
            task.repeat.as_deref().map(|repeat| {
                let step = if repeat == "weekly" { 7 * 86_400_000 } else { 86_400_000 };
                BoardTask {
                    id: uuid::Uuid::new_v4().to_string(),
                    status: "todo".into(),
                    due_at: task.due_at.map(|due| due + step),
                    created_at: now,
                    updated_at: now,
                    ..task.clone()
                }
            })
        } else {
            None
        };
        if let Some(next) = next {
            tasks.push(next);
        }
        write(&c, &tasks)?;
        Ok(true)
    }

    pub fn delete_board_task(&self, id: &str) -> Result<bool> {
        let c = self.db.locked();
        let mut tasks = read(&c)?;
        let before = tasks.len();
        tasks.retain(|task| task.id != id);
        let removed = tasks.len() != before;
        if removed {
            write(&c, &tasks)?;
        }
        Ok(removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_767_225_600_000;

    #[test]
    fn a_card_can_be_created_changed_moved_and_deleted() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let task = store
            .save_board_task(None, "  review the PR ", "", "todo", Some("b1"), None, None, None, None, NOW)
            .expect("created");
        assert_eq!(task.title, "review the PR");
        assert_eq!(store.board_tasks().unwrap().len(), 1);
        let changed = store
            .save_board_task(
                Some(&task.id),
                "review the PR",
                "failing test",
                "doing",
                Some("b1"),
                None,
                None,
                None,
                NOW + 10,
            )
            .expect("changed");
        assert_eq!((changed.status.as_str(), changed.created_at), ("doing", NOW));
        assert!(store.move_board_task(&task.id, "done", NOW + 20).unwrap());
        assert_eq!(store.board_tasks().unwrap()[0].status, "done");
        assert!(!store.move_board_task("missing", "done", NOW).unwrap());
        assert!(store.delete_board_task(&task.id).unwrap());
        assert!(!store.delete_board_task(&task.id).unwrap());
        assert!(store.board_tasks().unwrap().is_empty());
    }

    #[test]
    fn titles_and_columns_are_checked_and_a_known_card_is_required() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        assert!(store.save_board_task(None, "  ", "", "todo", None, None, None, None, NOW).is_err());
        assert!(store.save_board_task(None, "x", "", "someday", None, None, None, None, NOW).is_err());
        assert!(store.save_board_task(Some("nope"), "x", "", "todo", None, None, None, None, NOW).is_err());
        let task = store.save_board_task(None, "x", "", "waiting", None, None, None, None, NOW).unwrap();
        assert!(store.move_board_task(&task.id, "later", NOW).is_err());
        assert_eq!(store.board_tasks().unwrap().len(), 1, "a refused move changes nothing");
    }

    #[test]
    fn finishing_a_repeating_card_creates_the_next_one_once() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let task = store
            .save_board_task(None, "water the plants", "", "doing", Some("b1"), None, Some(NOW), Some("daily"), NOW)
            .unwrap();
        assert!(store.move_board_task(&task.id, "done", NOW + 1_000).unwrap());
        let all = store.board_tasks().unwrap();
        assert_eq!(all.len(), 2);
        let next = all.iter().find(|item| item.id != task.id).unwrap();
        assert_eq!((next.status.as_str(), next.title.as_str()), ("todo", "water the plants"));
        assert_eq!(next.due_at, Some(NOW + 86_400_000));
        // The finished card is already done: moving it to done again adds nothing.
        assert!(store.move_board_task(&task.id, "done", NOW + 2_000).unwrap());
        assert_eq!(store.board_tasks().unwrap().len(), 2);
    }
}
