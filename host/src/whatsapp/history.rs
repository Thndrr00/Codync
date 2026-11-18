//! What a linked `WhatsApp` number received, kept so an agent can read it back later. Ported from
//! `OpenBot` 0.72.0: a number keeps its messages for 90 days, also the group messages that did not
//! mention it, and unlinking the number deletes them. `OpenBot` keeps the files too; here the text is
//! what a routine summarizes, and the files stay on the messages that reached the bot.
//!
//! The table is made on first use, indexed by time, and pruned on every write, so it never needs a
//! migration or a background sweep.

use crate::store::Store;
use anyhow::Result;
use rusqlite::params;
use serde_json::{Value, json};

/// How long a message is kept.
pub(crate) const KEEP_MS: i64 = 90 * 24 * 60 * 60 * 1000;
/// Most messages one number keeps; the oldest go first.
const MAX_KEPT: i64 = 20_000;

/// The oldest message kept at `at`.
fn cutoff(at: i64) -> i64 {
    at.saturating_sub(KEEP_MS)
}

/// Keeps one message the number received. Empty text (a bare attachment) is not kept.
pub(crate) fn remember(store: &Store, chat: &str, sender: &str, name: &str, text: &str, at: i64) -> Result<()> {
    let text = text.trim();
    if text.is_empty() || chat.is_empty() {
        return Ok(());
    }
    store.with_connection(|c| {
        c.execute_batch(
            "CREATE TABLE IF NOT EXISTS whatsapp_history(
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                chat TEXT NOT NULL,
                sender TEXT NOT NULL,
                name TEXT NOT NULL,
                text TEXT NOT NULL,
                at INTEGER NOT NULL);
             CREATE INDEX IF NOT EXISTS whatsapp_history_at ON whatsapp_history(at);",
        )?;
        c.execute(
            "INSERT INTO whatsapp_history(chat, sender, name, text, at) VALUES(?1, ?2, ?3, ?4, ?5)",
            params![chat, sender, name, text, at],
        )?;
        c.execute("DELETE FROM whatsapp_history WHERE at < ?1", [cutoff(at)])?;
        c.execute(
            "DELETE FROM whatsapp_history WHERE id NOT IN (SELECT id FROM whatsapp_history ORDER BY id DESC LIMIT ?1)",
            [MAX_KEPT],
        )?;
        Ok(())
    })
}

/// The kept messages holding every word of `words` (already lowercased), at or after `since` and at
/// or before `before`, newest first.
pub(crate) fn search(store: &Store, words: &[String], since: i64, before: i64, limit: usize) -> Result<Vec<Value>> {
    store.with_connection(|c| {
        c.execute_batch(
            "CREATE TABLE IF NOT EXISTS whatsapp_history(
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                chat TEXT NOT NULL,
                sender TEXT NOT NULL,
                name TEXT NOT NULL,
                text TEXT NOT NULL,
                at INTEGER NOT NULL);
             CREATE INDEX IF NOT EXISTS whatsapp_history_at ON whatsapp_history(at);",
        )?;
        let mut sql =
            String::from("SELECT chat, sender, name, text, at FROM whatsapp_history WHERE at >= ?1 AND at <= ?2");
        let mut bound: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(since), Box::new(before)];
        for word in words {
            // The word is matched as text: its own `%` and `_` must not become wildcards.
            bound.push(Box::new(format!("%{}%", word.replace('%', "").replace('_', ""))));
            sql.push_str(&format!(" AND lower(text) LIKE ?{}", bound.len()));
        }
        sql.push_str(&format!(" ORDER BY id DESC LIMIT {limit}"));
        let mut statement = c.prepare(&sql)?;
        let refs: Vec<&dyn rusqlite::ToSql> = bound.iter().map(|value| value.as_ref()).collect();
        let rows = statement.query_map(refs.as_slice(), |row| {
            Ok(json!({
                "chat": row.get::<_, String>(0)?,
                "from": row.get::<_, String>(1)?,
                "name": row.get::<_, String>(2)?,
                "text": row.get::<_, String>(3)?,
                "at": row.get::<_, i64>(4)?,
            }))
        })?;
        let mut out = Vec::with_capacity(limit);
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    })
}

/// Deletes everything kept for the number, when it is unlinked.
pub(crate) fn forget(store: &Store) -> Result<()> {
    store.with_connection(|c| {
        c.execute_batch(
            "CREATE TABLE IF NOT EXISTS whatsapp_history(
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                chat TEXT NOT NULL,
                sender TEXT NOT NULL,
                name TEXT NOT NULL,
                text TEXT NOT NULL,
                at INTEGER NOT NULL);",
        )?;
        c.execute("DELETE FROM whatsapp_history", [])?;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_message_is_kept_for_ninety_days_and_not_a_day_more() {
        let at = 1_800_000_000_000;
        assert_eq!(cutoff(at), at - 90 * 24 * 60 * 60 * 1000);
        assert!(cutoff(at) < at, "the cutoff is in the past");
        assert_eq!(cutoff(i64::MIN), i64::MIN, "a clock before the epoch never overflows");
    }
}
