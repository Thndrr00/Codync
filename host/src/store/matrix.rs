use super::{Store, now_ms};
use crate::LockExt;
use anyhow::Result;
use rusqlite::{OptionalExtension, params};
use serde::Serialize;

pub(super) fn open(db: &rusqlite::Connection) -> Result<()> {
    db.execute_batch(include_str!("matrix.sql"))?;
    Ok(())
}

pub struct MatrixIncoming {
    pub event: String,
    pub generation: String,
    pub bot: String,
    pub room: String,
    pub entry: String,
    pub text: String,
    pub user: String,
}

pub struct MatrixOutgoing {
    pub entry: String,
    pub room: String,
    pub reply_to: String,
    pub text: String,
    pub bot: String,
    pub files: Vec<serde_json::Value>,
}

#[derive(Serialize)]
pub struct MatrixDelivery {
    pub id: String,
    pub state: String,
}

impl Store {
    pub fn reject_stale_matrix(&self, generation: Option<&str>) -> Result<Vec<String>> {
        let mut db = self.db.locked();
        let tx = db.transaction()?;
        let ids = {
            let mut q = tx
                .prepare("SELECT entry_id FROM matrix_inbox WHERE state='queued' AND (?1 IS NULL OR generation<>?1)")?;
            q.query_map([generation], |r| r.get(0))?.collect::<rusqlite::Result<Vec<String>>>()?
        };
        tx.execute(
            "UPDATE matrix_inbox SET state='rejected' WHERE state='queued' AND (?1 IS NULL OR generation<>?1)",
            [generation],
        )?;
        tx.commit()?;
        Ok(ids)
    }

    pub fn enqueue_matrix(&self, event: &MatrixIncoming) -> Result<()> {
        self.db.locked().execute(
            "INSERT INTO matrix_inbox(event_id,generation,bot_id,room_id,entry_id,text,user_id,created_at)
            VALUES(?1,?2,?3,?4,?5,?6,?7,?8) ON CONFLICT DO NOTHING",
            params![
                event.event,
                event.generation,
                event.bot,
                event.room,
                event.entry,
                event.text,
                event.user,
                now_ms()
            ],
        )?;
        Ok(())
    }

    pub fn pending_matrix(&self) -> Result<Vec<MatrixIncoming>> {
        let db = self.db.locked();
        let mut q = db.prepare(
            "SELECT event_id,generation,bot_id,room_id,entry_id,text,user_id
            FROM matrix_inbox WHERE state='queued' ORDER BY created_at LIMIT 20",
        )?;
        Ok(q.query_map([], |r| {
            Ok(MatrixIncoming {
                event: r.get(0)?,
                generation: r.get(1)?,
                bot: r.get(2)?,
                room: r.get(3)?,
                entry: r.get(4)?,
                text: r.get(5)?,
                user: r.get(6)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn matrix_thread_exists(&self, generation: &str, room: &str) -> Result<bool> {
        Ok(self.db.locked().query_row(
            "SELECT EXISTS(SELECT 1 FROM matrix_threads WHERE generation=?1 AND room_id=?2)",
            params![generation, room],
            |r| r.get(0),
        )?)
    }

    pub fn map_matrix_thread(&self, event: &MatrixIncoming, root: &str) -> Result<()> {
        self.db.locked().execute(
            "INSERT INTO matrix_threads VALUES(?1,?2,?3,?4,?5)
            ON CONFLICT(generation,room_id) DO UPDATE SET event_id=excluded.event_id",
            params![event.generation, event.room, event.bot, root, event.event],
        )?;
        Ok(())
    }

    pub fn claim_matrix_prompt(&self, entry: &str, generation: &str) -> Result<bool> {
        Ok(self.db.locked().execute(
            "UPDATE matrix_inbox SET state='started' WHERE entry_id=?1 AND generation=?2 AND state='queued'",
            params![entry, generation],
        )? == 1)
    }

    pub fn reject_matrix(&self, event: &str) -> Result<()> {
        self.db
            .locked()
            .execute("UPDATE matrix_inbox SET state='rejected' WHERE event_id=?1 AND state='queued'", [event])?;
        Ok(())
    }

    pub fn pending_matrix_replies(&self, generation: &str) -> Result<Vec<MatrixOutgoing>> {
        let db = self.db.locked();
        let mut q = db.prepare(
            "SELECT e.id,t.room_id,t.event_id,json_extract(e.data,'$.text'),t.bot_id,COALESCE(json_extract(e.data,'$.files'),'[]')
            FROM entries e JOIN matrix_threads t ON t.bot_id=e.bot_id AND t.root_id=e.thread_id
            WHERE t.generation=?1 AND e.kind='agent' AND json_extract(e.data,'$.final')=1
            AND (COALESCE(json_extract(e.data,'$.text'),'')<>'' OR json_extract(e.data,'$.files') IS NOT NULL)
            AND NOT EXISTS(SELECT 1 FROM matrix_outbox o WHERE o.entry_id=e.id) ORDER BY e.seq LIMIT 20",
        )?;
        Ok(q.query_map([generation], |r| {
            Ok(MatrixOutgoing {
                entry: r.get(0)?,
                room: r.get(1)?,
                reply_to: r.get(2)?,
                text: r.get(3)?,
                bot: r.get(4)?,
                files: serde_json::from_str(&r.get::<_, String>(5)?).unwrap_or_default(),
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn claim_matrix_reply(&self, entry: &str) -> Result<bool> {
        Ok(self.db.locked().execute(
            "INSERT INTO matrix_outbox(entry_id,state,updated_at) VALUES(?1,'sending',?2) ON CONFLICT DO NOTHING",
            params![entry, now_ms()],
        )? == 1)
    }

    pub fn finish_matrix_reply(&self, entry: &str, state: &str, event: Option<&str>) -> Result<()> {
        self.db.locked().execute(
            "UPDATE matrix_outbox SET state=?2,event_id=?3,updated_at=?4 WHERE entry_id=?1 AND state='sending'",
            params![entry, state, event, now_ms()],
        )?;
        Ok(())
    }

    pub fn matrix_deliveries(&self) -> Result<Vec<MatrixDelivery>> {
        let db = self.db.locked();
        let mut q = db.prepare(
            "SELECT entry_id,CASE WHEN state='sending' THEN 'unknown' ELSE state END AS state
            FROM matrix_outbox ORDER BY updated_at DESC LIMIT 50",
        )?;
        Ok(q.query_map([], |r| Ok(MatrixDelivery { id: r.get(0)?, state: r.get(1)? }))?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Remembers which approval card a Matrix event shows, so a reply or a reaction finds its entry.
    pub fn map_matrix_card(&self, generation: &str, room: &str, event: &str, entry: &str) -> Result<()> {
        self.db.locked().execute(
            "INSERT INTO matrix_cards(generation,room_id,event_id,entry_id) VALUES(?1,?2,?3,?4)
            ON CONFLICT(generation,room_id,event_id) DO UPDATE SET entry_id=excluded.entry_id",
            params![generation, room, event, entry],
        )?;
        Ok(())
    }

    /// The permission entry a Matrix event shows, if it is a card of this generation.
    pub fn matrix_card_entry(&self, generation: &str, room: &str, event: &str) -> Result<Option<String>> {
        Ok(self
            .db
            .locked()
            .query_row(
                "SELECT entry_id FROM matrix_cards WHERE generation=?1 AND room_id=?2 AND event_id=?3",
                params![generation, room, event],
                |r| r.get(0),
            )
            .optional()?)
    }
}
