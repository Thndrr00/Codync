//! Clearing one chat's or group's history: its messages and turns, never the row, the setup, the
//! memories, the routines, the artifacts or the files. Ported from OpenBot's "Delete history".
//!
//! The chat itself is `thread_id IS NULL`; side conversations (`Lane::thread`) and every other chat
//! keep their lines. A group's transcript is the group's own lane, so its members see nothing of it in
//! their next room turn once it is gone.

use super::{Store, next_rev};
use crate::LockExt;
use anyhow::Result;

impl Store {
    /// Removes the messages and turns of `chat`'s own conversation and returns how many lines went. The
    /// chat's side threads, its bot or group row, its memories, routines, artifacts and files stay.
    pub fn clear_history(&self, chat: &str) -> Result<usize> {
        let mut c = self.db.locked();
        let tx = c.transaction()?;
        let removed = tx.execute("DELETE FROM entries WHERE bot_id = ?1 AND thread_id IS NULL", [chat])?;
        if removed > 0 {
            next_rev(&tx)?;
        }
        tx.commit()?;
        Ok(removed)
    }

    /// The group chats `bot` answers in, in roster order.
    pub fn groups_with(&self, bot: &str) -> Result<Vec<String>> {
        Ok(self
            .bots()?
            .into_iter()
            .filter(|row| row.config.is_group() && row.config.members.iter().any(|m| m == bot))
            .map(|row| row.config.id)
            .collect())
    }

    /// The sequence of the notice that started `chat`'s current conversation, or 0 for its beginning.
    /// A group's members see only the room lines after it.
    pub fn conversation_start(&self, chat: &str) -> i64 {
        self.kv_get(&start_key(chat)).and_then(|v| v.parse().ok()).unwrap_or(0)
    }

    pub fn set_conversation_start(&self, chat: &str, seq: i64) -> Result<()> {
        if seq <= 0 { self.kv_delete(&start_key(chat)) } else { self.kv_set(&start_key(chat), &seq.to_string()) }
    }
}

fn start_key(chat: &str) -> String {
    format!("conversation.start.{chat}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{BotConfig, EntryKind, Lane};
    use serde_json::json;

    fn bot(value: serde_json::Value) -> BotConfig {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn clearing_a_chat_keeps_its_side_threads_and_every_other_chat() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        store.save_bot(&bot(json!({"id": "a", "name": "A", "cwd": "/tmp"}))).unwrap();
        store.save_bot(&bot(json!({"id": "b", "name": "B", "cwd": "/tmp"}))).unwrap();
        let chat = Lane::main("a");
        let thread = Lane::in_thread("a", "root");
        store.insert_entry(&chat, EntryKind::User, 1, &json!({"text": "hello"})).unwrap();
        store.insert_entry(&chat, EntryKind::Agent, 1, &json!({"text": "hi"})).unwrap();
        let side = store.insert_entry(&thread, EntryKind::User, 2, &json!({"text": "side"})).unwrap();
        store.insert_entry(&Lane::main("b"), EntryKind::User, 1, &json!({"text": "other"})).unwrap();

        assert_eq!(store.clear_history("a").unwrap(), 2);
        assert!(store.history("a", i64::MAX, 10).unwrap().is_empty());
        assert_eq!(store.history("b", i64::MAX, 10).unwrap().len(), 1, "another chat keeps its messages");
        assert!(store.entry(&side.id).is_some(), "a side thread keeps its lines");
        assert!(store.bot("a").unwrap().is_some(), "the bot row stays");
        assert_eq!(store.clear_history("a").unwrap(), 0, "clearing twice removes nothing more");
    }

    #[test]
    fn a_group_lists_the_bots_that_answer_in_it() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        store.save_bot(&bot(json!({"id": "a", "name": "A", "cwd": "/tmp"}))).unwrap();
        store.save_bot(&bot(json!({"id": "b", "name": "B", "cwd": "/tmp"}))).unwrap();
        store.save_bot(&bot(json!({"id": "room", "kind": "group", "name": "Room", "members": ["a"]}))).unwrap();
        assert_eq!(store.groups_with("a").unwrap(), vec!["room".to_owned()]);
        assert!(store.groups_with("b").unwrap().is_empty());
    }

    #[test]
    fn a_new_conversation_hides_the_lines_before_it_from_a_member() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let lane = Lane::main("room");
        let before = store.insert_entry(&lane, EntryKind::User, 1, &json!({"text": "before"})).unwrap();
        let start = store.insert_entry(&lane, EntryKind::Notice, 0, &json!({"text": "New conversation"})).unwrap();
        store.insert_entry(&lane, EntryKind::User, 2, &json!({"text": "after"})).unwrap();
        store.set_conversation_start("room", start.seq).unwrap();
        assert_eq!(store.conversation_start("room"), start.seq);
        let seen = store.messages_after(&lane, store.conversation_start("room"), 24).unwrap();
        assert_eq!(seen.len(), 1, "only the line after the divider");
        assert!(seen[0].seq > before.seq);
        store.set_conversation_start("room", 0).unwrap();
        assert_eq!(store.conversation_start("room"), 0, "clearing the history drops the marker too");
    }
}
