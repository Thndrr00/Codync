//! `Cmd::Block`: a declarative UI block from the bot's `chat` MCP server, posted as the approval card
//! every client already knows how to route, with the block beside what an older client falls back to.

use super::Actor;
use super::updates::Answer;
use crate::hub::BotStatus;
use crate::store::EntryKind;
use serde_json::Value;
use std::time::Instant;
use tokio::sync::oneshot;

impl Actor {
    pub(super) fn show_block(&mut self, data: Value, reply: Option<oneshot::Sender<Option<Value>>>) {
        let turn = self.turn.unwrap_or(0);
        let title = data["title"].as_str().unwrap_or("Needs your answer").to_owned();
        self.close_seg();
        let Some(entry) = self.add(EntryKind::Permission, turn, data) else {
            // The chat went away: the tool hears why instead of waiting out its timeout.
            if let Some(reply) = reply {
                let _ = reply.send(None);
            }
            return;
        };
        let Some(reply) = reply else { return };
        self.perms.insert(entry.id.clone(), Answer::Block(reply));
        self.waiting.asked(&entry.id, Instant::now());
        self.hub.set_runtime(&self.cfg.id, |runtime| {
            runtime.status = BotStatus::NeedsInput;
            runtime.activity = format!("Needs your answer: {title}");
        });
    }
}
