//! Durable receipts for the Signal channel: one inbox, one conversation per chat and one outbox.

use super::{Store, now_ms};
use crate::LockExt;
use anyhow::Result;
use rusqlite::{OptionalExtension, params};
use serde::Serialize;

pub(super) fn open(db: &rusqlite::Connection) -> Result<()> {
    db.execute_batch(include_str!("signal.sql"))?;
    Ok(())
}

pub struct SignalIncoming {
    pub message: String,
    pub generation: String,
    pub bot: String,
    pub chat: String,
    pub entry: String,
    pub text: String,
    pub files: Vec<serde_json::Value>,
    pub user: String,
}

pub struct SignalOutgoing {
    pub entry: String,
    pub chat: String,
    pub reply_to: String,
    pub text: String,
    pub bot: String,
    pub files: Vec<serde_json::Value>,
}

#[derive(Serialize)]
pub struct SignalDelivery {
    pub id: String,
    pub state: String,
}

impl Store {
    pub fn reject_stale_signal(&self, generation: Option<&str>) -> Result<Vec<String>> {
        let mut db = self.db.locked();
        let tx = db.transaction()?;
        let ids = {
            let mut q = tx
                .prepare("SELECT entry_id FROM signal_inbox WHERE state='queued' AND (?1 IS NULL OR generation<>?1)")?;
            q.query_map([generation], |r| r.get(0))?.collect::<rusqlite::Result<Vec<String>>>()?
        };
        tx.execute(
            "UPDATE signal_inbox SET state='rejected' WHERE state='queued' AND (?1 IS NULL OR generation<>?1)",
            [generation],
        )?;
        tx.commit()?;
        Ok(ids)
    }

    pub fn enqueue_signal(&self, event: &SignalIncoming) -> Result<()> {
        self.db.locked().execute(
            "INSERT INTO signal_inbox(message_id,generation,bot_id,chat,entry_id,text,files,user_id,created_at)
            VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9) ON CONFLICT DO NOTHING",
            params![
                event.message,
                event.generation,
                event.bot,
                event.chat,
                event.entry,
                event.text,
                serde_json::to_string(&event.files)?,
                event.user,
                now_ms()
            ],
        )?;
        Ok(())
    }

    pub fn pending_signal(&self) -> Result<Vec<SignalIncoming>> {
        let db = self.db.locked();
        let mut q = db.prepare(
            "SELECT message_id,generation,bot_id,chat,entry_id,text,files,user_id
            FROM signal_inbox WHERE state='queued' ORDER BY created_at LIMIT 20",
        )?;
        Ok(q.query_map([], |r| {
            Ok(SignalIncoming {
                message: r.get(0)?,
                generation: r.get(1)?,
                bot: r.get(2)?,
                chat: r.get(3)?,
                entry: r.get(4)?,
                text: r.get(5)?,
                files: serde_json::from_str(&r.get::<_, String>(6)?).unwrap_or_default(),
                user: r.get(7)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn signal_thread_exists(&self, generation: &str, chat: &str) -> Result<bool> {
        Ok(self.db.locked().query_row(
            "SELECT EXISTS(SELECT 1 FROM signal_threads WHERE generation=?1 AND chat=?2)",
            params![generation, chat],
            |r| r.get(0),
        )?)
    }

    pub fn map_signal_thread(&self, event: &SignalIncoming, root: &str) -> Result<()> {
        self.db.locked().execute(
            "INSERT INTO signal_threads VALUES(?1,?2,?3,?4,?5)
            ON CONFLICT(generation,chat) DO UPDATE SET message_id=excluded.message_id",
            params![event.generation, event.chat, event.bot, root, event.message],
        )?;
        Ok(())
    }

    pub fn claim_signal_prompt(&self, entry: &str, generation: &str) -> Result<bool> {
        Ok(self.db.locked().execute(
            "UPDATE signal_inbox SET state='started' WHERE entry_id=?1 AND generation=?2 AND state='queued'",
            params![entry, generation],
        )? == 1)
    }

    pub fn reject_signal(&self, message: &str) -> Result<()> {
        self.db
            .locked()
            .execute("UPDATE signal_inbox SET state='rejected' WHERE message_id=?1 AND state='queued'", [message])?;
        Ok(())
    }

    pub fn pending_signal_replies(&self, generation: &str) -> Result<Vec<SignalOutgoing>> {
        let db = self.db.locked();
        let mut q = db.prepare(
            "SELECT e.id,t.chat,t.message_id,json_extract(e.data,'$.text'),t.bot_id,COALESCE(json_extract(e.data,'$.files'),'[]')
            FROM entries e JOIN signal_threads t ON t.bot_id=e.bot_id AND t.root_id=e.thread_id
            WHERE t.generation=?1 AND e.kind='agent' AND json_extract(e.data,'$.final')=1
            AND (COALESCE(json_extract(e.data,'$.text'),'')<>'' OR json_extract(e.data,'$.files') IS NOT NULL)
            AND NOT EXISTS(SELECT 1 FROM signal_outbox o WHERE o.entry_id=e.id) ORDER BY e.seq LIMIT 20",
        )?;
        Ok(q.query_map([generation], |r| {
            Ok(SignalOutgoing {
                entry: r.get(0)?,
                chat: r.get(1)?,
                reply_to: r.get(2)?,
                text: r.get(3)?,
                bot: r.get(4)?,
                files: serde_json::from_str(&r.get::<_, String>(5)?).unwrap_or_default(),
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn claim_signal_reply(&self, entry: &str) -> Result<bool> {
        Ok(self.db.locked().execute(
            "INSERT INTO signal_outbox(entry_id,state,updated_at) VALUES(?1,'sending',?2) ON CONFLICT DO NOTHING",
            params![entry, now_ms()],
        )? == 1)
    }

    pub fn finish_signal_reply(&self, entry: &str, state: &str, message: Option<&str>) -> Result<()> {
        self.db.locked().execute(
            "UPDATE signal_outbox SET state=?2,message_id=?3,updated_at=?4 WHERE entry_id=?1 AND state='sending'",
            params![entry, state, message, now_ms()],
        )?;
        Ok(())
    }

    pub fn signal_deliveries(&self) -> Result<Vec<SignalDelivery>> {
        let db = self.db.locked();
        let mut q = db.prepare(
            "SELECT entry_id,CASE WHEN state='sending' THEN 'unknown' ELSE state END AS state
            FROM signal_outbox ORDER BY updated_at DESC LIMIT 50",
        )?;
        Ok(q.query_map([], |r| Ok(SignalDelivery { id: r.get(0)?, state: r.get(1)? }))?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Remembers which approval card a Signal message shows, so a reply quote finds its entry.
    pub fn map_signal_card(&self, generation: &str, chat: &str, message: &str, entry: &str) -> Result<()> {
        self.db.locked().execute(
            "INSERT INTO signal_cards(generation,chat,message_id,entry_id) VALUES(?1,?2,?3,?4)
            ON CONFLICT(generation,chat,message_id) DO UPDATE SET entry_id=excluded.entry_id",
            params![generation, chat, message, entry],
        )?;
        Ok(())
    }

    /// The permission entry a Signal message shows, if it is a card of this generation.
    pub fn signal_card_entry(&self, generation: &str, chat: &str, message: &str) -> Result<Option<String>> {
        Ok(self
            .db
            .locked()
            .query_row(
                "SELECT entry_id FROM signal_cards WHERE generation=?1 AND chat=?2 AND message_id=?3",
                params![generation, chat, message],
                |r| r.get(0),
            )
            .optional()?)
    }
}
