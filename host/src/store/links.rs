//! Durable links between a messaging conversation and a Codync thread. A Slack thread, a Discord
//! channel or a Telegram chat can keep the same root across restarts, and a call can join the
//! conversation it belongs to instead of opening a root of its own.

use super::{Store, now_ms};
use crate::LockExt;
use anyhow::{Result, ensure};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

pub(super) fn open(db: &rusqlite::Connection) -> Result<()> {
    db.execute_batch(include_str!("links.sql"))?;
    Ok(())
}

/// One conversation link: where a provider's conversation writes and which Codync thread it shares.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationLink {
    pub bot_id: String,
    pub source: String,
    pub channel: String,
    pub thread: String,
    pub root_id: String,
    pub created_at: i64,
    pub updated_at: i64,
}

const SOURCES: [&str; 4] = ["slack", "telegram", "discord", "matrix"];

fn source(value: &str) -> Result<&str> {
    let value = value.trim();
    ensure!(SOURCES.contains(&value), "a link belongs to Slack, Telegram, Discord or Matrix");
    Ok(value)
}

fn part(value: &str, what: &str) -> Result<&str> {
    let value = value.trim();
    ensure!(!value.is_empty() && value.chars().count() <= 200, "{what} is required");
    ensure!(!value.chars().any(char::is_control), "{what} can't hold control characters");
    Ok(value)
}

impl Store {
    /// The root a conversation writes into, when the person linked it.
    pub fn conversation_root(&self, bot: &str, source: &str, channel: &str, thread: &str) -> Result<Option<String>> {
        let (source, channel, thread) = (source(source)?, part(channel, "channel")?, thread.trim());
        Ok(self
            .db
            .locked()
            .query_row(
                "SELECT root_id FROM conversation_links WHERE bot_id=?1 AND source=?2 AND channel=?3 AND thread=?4",
                params![bot, source, channel, thread],
                |row| row.get(0),
            )
            .optional()?)
    }

    /// Links a conversation to a Codync thread. Linking again moves it: what it writes from now on shares that root.
    pub fn link_conversation(
        &self,
        bot: &str,
        source: &str,
        channel: &str,
        thread: &str,
        root: &str,
    ) -> Result<ConversationLink> {
        let (source, channel, root) = (source(source)?, part(channel, "channel")?, part(root, "thread")?);
        let thread = thread.trim();
        ensure!(!thread.chars().any(char::is_control), "thread can't hold control characters");
        let now = now_ms();
        self.db.locked().execute(
            "INSERT INTO conversation_links(bot_id,source,channel,thread,root_id,created_at,updated_at)
             VALUES(?1,?2,?3,?4,?5,?6,?6)
             ON CONFLICT(bot_id,source,channel,thread) DO UPDATE SET root_id=excluded.root_id, updated_at=excluded.updated_at",
            params![bot, source, channel, thread, root, now],
        )?;
        self.conversation_links(bot)?
            .into_iter()
            .find(|link| link.source == source && link.channel == channel && link.thread == thread)
            .ok_or_else(|| anyhow::anyhow!("the conversation link wasn't saved"))
    }

    /// Removes a link; its next message opens a conversation of its own again.
    pub fn unlink_conversation(&self, bot: &str, source: &str, channel: &str, thread: &str) -> Result<bool> {
        let (source, channel, thread) = (source(source)?, part(channel, "channel")?, thread.trim());
        Ok(self.db.locked().execute(
            "DELETE FROM conversation_links WHERE bot_id=?1 AND source=?2 AND channel=?3 AND thread=?4",
            params![bot, source, channel, thread],
        )? > 0)
    }

    /// A bot's linked conversations, newest change first.
    pub fn conversation_links(&self, bot: &str) -> Result<Vec<ConversationLink>> {
        let db = self.db.locked();
        let mut query = db.prepare(
            "SELECT bot_id,source,channel,thread,root_id,created_at,updated_at
             FROM conversation_links WHERE bot_id=?1 ORDER BY updated_at DESC",
        )?;
        Ok(query
            .query_map([bot], |row| {
                Ok(ConversationLink {
                    bot_id: row.get(0)?,
                    source: row.get(1)?,
                    channel: row.get(2)?,
                    thread: row.get(3)?,
                    root_id: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_linked_conversation_keeps_its_root_and_never_crosses_bots() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let link = store.link_conversation("b", "slack", "C1", "1700.1", "thread:root").unwrap();
        assert_eq!(link.root_id, "thread:root");
        assert_eq!(store.conversation_root("b", "slack", "C1", "1700.1").unwrap().as_deref(), Some("thread:root"));
        assert_eq!(store.conversation_root("other", "slack", "C1", "1700.1").unwrap(), None);
        // Linking again moves the root without losing the identity.
        let moved = store.link_conversation("b", "slack", "C1", "1700.1", "doc:d1").unwrap();
        assert_eq!(moved.root_id, "doc:d1");
        assert_eq!(store.conversation_links("b").unwrap().len(), 1);
        assert!(store.unlink_conversation("b", "slack", "C1", "1700.1").unwrap());
        assert_eq!(store.conversation_root("b", "slack", "C1", "1700.1").unwrap(), None);
    }

    #[test]
    fn only_known_sources_and_bounded_parts_are_accepted() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        assert!(store.link_conversation("b", "email", "C1", "", "thread:root").is_err());
        assert!(store.link_conversation("b", "slack", "", "", "thread:root").is_err());
        assert!(store.link_conversation("b", "slack", "C1", "", "x".repeat(201).as_str()).is_err());
    }
}
