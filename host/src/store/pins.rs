//! Pinned messages: the `data.pinned` marker an entry carries, and the pins of every chat of a bot.
//!
//! A pin is a person (with their device's name, or none at the computer itself) or the bot's own agent
//! with `pin_message`. The marker rides on the entry, so an existing rev broadcast carries a pin to
//! every client; there is no table of its own.

use super::entries::{ENTRY_COLS, NOT_REMOVED, row_entry};
use super::visibility::PUBLIC;
use super::{Entry, Store};
use crate::LockExt;
use anyhow::Result;
use rusqlite::params;

/// The most pins one chat (its chat itself or one of its side conversations) keeps.
pub const PINS_LIMIT: i64 = 50;
/// The most pins a list carries: the newest across the bot's chats.
pub const LIST_LIMIT: i64 = 200;

/// Messages only: a tool card or a notice has nothing worth coming back to.
const PINNABLE: &str = "json_extract(data, '$.pinned') IS NOT NULL";
/// The chat itself (`?2` null) or that one side conversation.
const SCOPE: &str = "((?2 IS NULL AND thread_id IS NULL) OR thread_id = ?2)";

impl Store {
    /// How many pins `chat` keeps, in `thread` or in the chat itself.
    pub fn pins_in(&self, chat: &str, thread: Option<&str>) -> Result<i64> {
        let c = self.db.locked();
        Ok(c.query_row(
            &format!("SELECT COUNT(*) FROM entries WHERE bot_id = ?1 AND {PUBLIC} AND {PINNABLE} AND {SCOPE}"),
            params![chat, thread],
            |r| r.get(0),
        )?)
    }

    /// The bot's pins, newest first.
    pub fn pins(&self, chat: &str, limit: i64) -> Result<Vec<Entry>> {
        let c = self.db.locked();
        let mut st = c.prepare(&format!(
            "SELECT {ENTRY_COLS} FROM entries WHERE bot_id = ?1 AND {PUBLIC} AND {PINNABLE} AND {NOT_REMOVED}
             ORDER BY json_extract(data, '$.pinned.at') DESC, seq DESC LIMIT ?2"
        ))?;
        Ok(st.query_map(params![chat, limit], row_entry)?.filter_map(Result::ok).collect())
    }

    /// The newest `limit` messages of the chat itself or of `thread`, oldest first: what `pin_message`
    /// looks through for its quote.
    pub fn pin_candidates(&self, chat: &str, thread: Option<&str>, limit: i64) -> Result<Vec<Entry>> {
        let c = self.db.locked();
        let mut st = c.prepare(&format!(
            "SELECT {ENTRY_COLS} FROM entries WHERE bot_id = ?1 AND {PUBLIC} AND {SCOPE} AND {NOT_REMOVED}
             ORDER BY seq DESC LIMIT ?3"
        ))?;
        let mut rows: Vec<Entry> =
            st.query_map(params![chat, thread, limit], row_entry)?.collect::<rusqlite::Result<_>>()?;
        rows.reverse();
        Ok(rows)
    }
}
