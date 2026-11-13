//! Messages the person writes now and sends later: one list, ordered by when each one goes out.
//!
//! A scheduled message is a plain message with a time: when it is due the host posts it as the
//! person's own message in that chat and the bot answers it, exactly as if they had sent it then.
//! Nothing runs while the host is stopped; a message whose time passed meanwhile goes out on the
//! first tick after it starts again.

use super::Store;
use crate::LockExt;
use anyhow::{Result, ensure};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

const KEY: &str = "scheduled-messages.v1";
/// Most messages waiting at once; enough for any normal use and keeps one bad client bounded.
const LIMIT: usize = 200;
const MAX_TEXT_CHARS: usize = 8_000;
/// How far ahead a message may be scheduled.
const MAX_AHEAD_MS: i64 = 366 * 24 * 3_600_000;
/// A message may be scheduled this much in the past and still go out at the next tick.
const GRACE_MS: i64 = 60_000;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScheduledMessage {
    pub id: String,
    pub bot_id: String,
    pub text: String,
    /// When it goes out, in Unix milliseconds.
    pub send_at: i64,
    pub created_at: i64,
}

/// Where a scheduled message's time comes from: `in_minutes` (relative, the common case) or `send_at`.
#[derive(Clone, Copy, Debug)]
pub enum When {
    At(i64),
    InMinutes(i64),
}

impl When {
    fn resolve(self, now: i64) -> i64 {
        match self {
            When::At(ms) => ms,
            When::InMinutes(minutes) => now + minutes * 60_000,
        }
    }
}

fn read(c: &rusqlite::Connection) -> Result<Vec<ScheduledMessage>> {
    let value: Option<String> = c.query_row("SELECT v FROM kv WHERE k = ?1", [KEY], |row| row.get(0)).optional()?;
    match value {
        Some(value) => Ok(serde_json::from_str(&value)?),
        None => Ok(Vec::new()),
    }
}

fn write(c: &rusqlite::Connection, messages: &[ScheduledMessage]) -> Result<()> {
    if messages.is_empty() {
        c.execute("DELETE FROM kv WHERE k = ?1", [KEY])?;
    } else {
        c.execute(
            "INSERT INTO kv(k, v) VALUES(?1, ?2) ON CONFLICT(k) DO UPDATE SET v = excluded.v",
            params![KEY, serde_json::to_string(messages)?],
        )?;
    }
    Ok(())
}

impl Store {
    /// Every message still waiting, soonest first.
    pub fn scheduled_messages(&self) -> Result<Vec<ScheduledMessage>> {
        let mut messages = read(&self.db.locked())?;
        messages.sort_by_key(|m| m.send_at);
        Ok(messages)
    }

    pub fn scheduled_for_bot(&self, bot_id: &str) -> Result<Vec<ScheduledMessage>> {
        Ok(self.scheduled_messages()?.into_iter().filter(|m| m.bot_id == bot_id).collect())
    }

    /// Schedules `text` for `when`. A time that already passed (within the grace window) goes out
    /// on the next tick; anything further in the past or past the limit is refused.
    pub fn schedule_message(&self, bot_id: &str, text: &str, when: When, now: i64) -> Result<ScheduledMessage> {
        let text = text.trim();
        ensure!(!text.is_empty(), "a scheduled message needs text");
        ensure!(text.chars().count() <= MAX_TEXT_CHARS, "a scheduled message is at most {MAX_TEXT_CHARS} characters");
        let send_at = when.resolve(now);
        ensure!(send_at >= now - GRACE_MS, "that time has already passed");
        ensure!(send_at <= now + MAX_AHEAD_MS, "a message can be scheduled at most a year ahead");
        let message = ScheduledMessage {
            id: uuid::Uuid::new_v4().to_string(),
            bot_id: bot_id.to_owned(),
            text: text.to_owned(),
            send_at,
            created_at: now,
        };
        let c = self.db.locked();
        let mut messages = read(&c)?;
        ensure!(messages.len() < LIMIT, "there are already {LIMIT} messages waiting; cancel one first");
        messages.push(message.clone());
        messages.sort_by_key(|m| m.send_at);
        write(&c, &messages)?;
        Ok(message)
    }

    /// Removes one message; `true` when it was still waiting.
    pub fn cancel_scheduled(&self, id: &str) -> Result<bool> {
        let c = self.db.locked();
        let mut messages = read(&c)?;
        let before = messages.len();
        messages.retain(|m| m.id != id);
        let removed = messages.len() != before;
        if removed {
            write(&c, &messages)?;
        }
        Ok(removed)
    }

    /// Removes every message for a bot (its chat is being deleted).
    pub fn clear_scheduled_for_bot(&self, bot_id: &str) -> Result<()> {
        let c = self.db.locked();
        let mut messages = read(&c)?;
        messages.retain(|m| m.bot_id != bot_id);
        write(&c, &messages)
    }

    /// Takes the messages due at `now` out of the list, so each one is sent once.
    pub fn take_due_scheduled(&self, now: i64) -> Result<Vec<ScheduledMessage>> {
        let c = self.db.locked();
        let mut messages = read(&c)?;
        let (due, waiting): (Vec<_>, Vec<_>) = messages.drain(..).partition(|m| m.send_at <= now);
        write(&c, &waiting)?;
        Ok(due)
    }

    /// Puts messages back after a send failed, keeping the order and the original times.
    pub fn restore_scheduled(&self, failed: Vec<ScheduledMessage>) -> Result<()> {
        if failed.is_empty() {
            return Ok(());
        }
        let c = self.db.locked();
        let mut messages = read(&c)?;
        for message in failed {
            if !messages.iter().any(|m| m.id == message.id) {
                messages.push(message);
            }
        }
        messages.sort_by_key(|m| m.send_at);
        write(&c, &messages)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_767_225_600_000;

    #[test]
    fn a_message_keeps_its_text_and_time_and_comes_back_soonest_first() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let late = store.schedule_message("b1", "  later  ", When::InMinutes(60), NOW).unwrap();
        let early = store.schedule_message("b1", "early", When::InMinutes(5), NOW).unwrap();
        assert_eq!(late.text, "later");
        assert_eq!(late.send_at, NOW + 60 * 60_000);
        let waiting = store.scheduled_for_bot("b1").unwrap();
        assert_eq!(waiting.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(), vec![early.id, late.id]);
        assert!(store.scheduled_for_bot("b2").unwrap().is_empty());
    }

    #[test]
    fn times_in_the_past_or_too_far_ahead_are_refused() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        assert!(store.schedule_message("b1", "x", When::At(NOW - 5 * 60_000), NOW).is_err());
        assert!(store.schedule_message("b1", "x", When::At(NOW + MAX_AHEAD_MS + 1), NOW).is_err());
        assert!(store.schedule_message("b1", "  ", When::InMinutes(5), NOW).is_err());
        assert!(store.schedule_message("b1", "x", When::At(NOW - 30_000), NOW).is_ok(), "a minute late still goes out");
    }

    #[test]
    fn due_messages_leave_the_list_once_and_a_failure_puts_them_back() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let due = store.schedule_message("b1", "now", When::InMinutes(1), NOW).unwrap();
        store.schedule_message("b1", "later", When::InMinutes(120), NOW).unwrap();
        let taken = store.take_due_scheduled(NOW + 61_000).unwrap();
        assert_eq!(taken.len(), 1);
        assert!(store.take_due_scheduled(NOW + 61_000).unwrap().is_empty(), "sent once");
        assert_eq!(store.scheduled_messages().unwrap().len(), 1);
        store.restore_scheduled(taken).unwrap();
        assert_eq!(store.scheduled_messages().unwrap().len(), 2);
        assert_eq!(store.scheduled_messages().unwrap()[0].id, due.id);
    }

    #[test]
    fn cancelling_removes_only_that_message_and_a_chat_takes_its_own_away() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let one = store.schedule_message("b1", "one", When::InMinutes(5), NOW).unwrap();
        let two = store.schedule_message("b1", "two", When::InMinutes(6), NOW).unwrap();
        store.schedule_message("b2", "other", When::InMinutes(7), NOW).unwrap();
        assert!(store.cancel_scheduled(&one.id).unwrap());
        assert!(!store.cancel_scheduled(&one.id).unwrap());
        assert_eq!(store.scheduled_messages().unwrap().len(), 2);
        store.clear_scheduled_for_bot("b1").unwrap();
        let left = store.scheduled_messages().unwrap();
        assert_eq!(left.len(), 1);
        assert_ne!(left[0].id, two.id);
    }
}
