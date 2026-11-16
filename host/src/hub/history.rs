//! A chat's or a group's history actions from its settings: delete it, start a new conversation, or
//! compact what the agents keep. Ported from OpenBot's "Delete history", "New conversation" and
//! "Compact context".
//!
//! Delete removes the messages and turns only; the bot or group row, its setup, memories, routines,
//! artifacts and files stay. A new conversation keeps the messages on screen (and in the chat's
//! history) but the agents start fresh, with a notice marking where. Compaction asks each agent to
//! summarize what it keeps; a group's members share one conversation across their chats, so compacting
//! a group also compacts each member's own chat.

use super::Hub;
use crate::agent::bot::Cmd;
use crate::store::{EntryKind, Lane};
use anyhow::{Result, bail};
use serde_json::{Value, json};
use std::sync::Arc;

impl Hub {
    /// `deleteHistory`: removes the chat's or group's messages and turns, and stops its running work.
    pub fn delete_history(self: &Arc<Self>, chat: &str) -> Result<Value> {
        let row = self.store.bot(chat)?.filter(|row| !row.deleted).ok_or_else(|| anyhow::anyhow!("Unknown chat"))?;
        if row.config.is_group() {
            crate::chat::group::stop(self, chat);
        } else {
            // A running turn would write new lines into the chat being cleared.
            self.stop_bot(chat)?;
            self.store.set_session(chat, None)?;
        }
        let removed = self.store.clear_history(chat)?;
        self.store.set_conversation_start(chat, 0)?;
        let _ = self.events.send(json!({
            "type": "historyCleared",
            "botId": chat,
            "deleted": removed,
            "rev": self.store.current_rev(),
        }));
        self.emit_bot(chat);
        Ok(json!({"deleted": removed, "bot": self.bot_value(chat)}))
    }

    /// `newSession` (the chat's New session, S in the terminal UI): the chat keeps its messages, the
    /// agents start fresh. A group's members see only the room lines after the notice in their next turns.
    pub fn new_conversation(self: &Arc<Self>, chat: &str) -> Result<Value> {
        let row = self.store.bot(chat)?.filter(|row| !row.deleted).ok_or_else(|| anyhow::anyhow!("Unknown chat"))?;
        if crate::external::is_external(&row.config) {
            // A terminal session answers to its own CLI.
            return Ok(json!({"started": false}));
        }
        let notice = self.store.insert_entry(
            &Lane::main(chat),
            EntryKind::Notice,
            0,
            &json!({"text": "New conversation. Earlier messages stay here; the agents start fresh.", "style": "divider"}),
        )?;
        if row.config.is_group() {
            self.store.set_conversation_start(chat, notice.seq)?;
        } else {
            // `NewSession` stops a running turn, forgets the agent's session and flushes its memory review.
            let _ = self.send_cmd(chat, Cmd::NewSession);
            self.store.set_session(chat, None)?;
        }
        self.emit_bot(chat);
        Ok(json!({"started": true, "notice": notice.seq, "bot": self.bot_value(chat)}))
    }

    /// `compactContext`: asks the agent (or every member of a group) to summarize the conversation it keeps.
    pub fn compact_context(self: &Arc<Self>, chat: &str) -> Result<Value> {
        let row = self.store.bot(chat)?.filter(|row| !row.deleted).ok_or_else(|| anyhow::anyhow!("Unknown chat"))?;
        if crate::external::is_external(&row.config) {
            bail!("A terminal session keeps its own context; compact it in its CLI.");
        }
        let targets: Vec<String> =
            if row.config.is_group() { row.config.members.clone() } else { vec![row.config.id.clone()] };
        let mut queued = 0;
        for target in targets {
            let Some(member) = self.store.bot(&target)?.filter(|row| !row.deleted) else { continue };
            if !crate::agent::bot::offers_threshold(&self.store, &member.config) {
                continue;
            }
            if self.send_cmd(&target, Cmd::CompactNow).is_ok() {
                queued += 1;
            }
        }
        if queued == 0 {
            bail!("Codync can't compact this chat's agent.");
        }
        Ok(json!({"queued": queued}))
    }
}
