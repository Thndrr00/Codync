//! Durable receipts for the Microsoft Teams transport: one inbox, one conversation per Teams
//! conversation, the conversation references a reply needs, and the outbox.

use super::{Store, now_ms};
use crate::LockExt;
use anyhow::Result;
use rusqlite::{OptionalExtension, params};
use serde::Serialize;

pub(super) fn open(db: &rusqlite::Connection) -> Result<()> {
    db.execute_batch(include_str!("teams.sql"))?;
    Ok(())
}

pub struct TeamsIncoming {
    pub activity: String,
    pub generation: String,
    pub bot: String,
    pub conversation: String,
    pub entry: String,
    pub text: String,
    pub user: String,
    pub service_url: String,
}

pub struct TeamsOutgoing {
    pub entry: String,
    pub conversation: String,
    pub reply_to: String,
    pub text: String,
    pub bot: String,
}

pub struct TeamsRef {
    pub conversation: String,
    pub service_url: String,
    pub tenant: String,
}

#[derive(Serialize)]
pub struct TeamsDelivery {
    pub id: String,
    pub state: String,
}

impl Store {
    pub fn reject_stale_teams(&self, generation: Option<&str>) -> Result<Vec<String>> {
        let mut db = self.db.locked();
        let tx = db.transaction()?;
        let ids = {
            let mut q =
                tx.prepare("SELECT entry_id FROM teams_inbox WHERE state='queued' AND (?1 IS NULL OR generation<>?1)")?;
            q.query_map([generation], |r| r.get(0))?.collect::<rusqlite::Result<Vec<String>>>()?
        };
        tx.execute(
            "UPDATE teams_inbox SET state='rejected' WHERE state='queued' AND (?1 IS NULL OR generation<>?1)",
            [generation],
        )?;
        tx.commit()?;
        Ok(ids)
    }

    pub fn enqueue_teams(&self, event: &TeamsIncoming) -> Result<()> {
        self.db.locked().execute(
            "INSERT INTO teams_inbox(activity_id,generation,bot_id,conversation_id,entry_id,text,user_id,service_url,created_at)
            VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9) ON CONFLICT DO NOTHING",
            params![
                event.activity,
                event.generation,
                event.bot,
                event.conversation,
                event.entry,
                event.text,
                event.user,
                event.service_url,
                now_ms()
            ],
        )?;
        Ok(())
    }

    pub fn pending_teams(&self) -> Result<Vec<TeamsIncoming>> {
        let db = self.db.locked();
        let mut q = db.prepare(
            "SELECT activity_id,generation,bot_id,conversation_id,entry_id,text,user_id,service_url
            FROM teams_inbox WHERE state='queued' ORDER BY created_at LIMIT 20",
        )?;
        Ok(q.query_map([], |r| {
            Ok(TeamsIncoming {
                activity: r.get(0)?,
                generation: r.get(1)?,
                bot: r.get(2)?,
                conversation: r.get(3)?,
                entry: r.get(4)?,
                text: r.get(5)?,
                user: r.get(6)?,
                service_url: r.get(7)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn map_teams_thread(&self, event: &TeamsIncoming, root: &str) -> Result<()> {
        self.db.locked().execute(
            "INSERT INTO teams_threads VALUES(?1,?2,?3,?4,?5)
            ON CONFLICT(generation,conversation_id) DO UPDATE SET activity_id=excluded.activity_id",
            params![event.generation, event.conversation, event.bot, root, event.activity],
        )?;
        Ok(())
    }

    pub fn claim_teams_prompt(&self, entry: &str, generation: &str) -> Result<bool> {
        Ok(self.db.locked().execute(
            "UPDATE teams_inbox SET state='started' WHERE entry_id=?1 AND generation=?2 AND state='queued'",
            params![entry, generation],
        )? == 1)
    }

    pub fn reject_teams(&self, activity: &str) -> Result<()> {
        self.db
            .locked()
            .execute("UPDATE teams_inbox SET state='rejected' WHERE activity_id=?1 AND state='queued'", [activity])?;
        Ok(())
    }

    pub fn save_teams_ref(
        &self,
        generation: &str,
        conversation: &str,
        service_url: &str,
        tenant: &str,
        user: &str,
        name: &str,
    ) -> Result<()> {
        self.db.locked().execute(
            "INSERT INTO teams_refs(generation,conversation_id,service_url,tenant_id,user_id,user_name,updated_at)
            VALUES(?1,?2,?3,?4,?5,?6,?7)
            ON CONFLICT(generation,conversation_id) DO UPDATE SET service_url=excluded.service_url,
            tenant_id=excluded.tenant_id,user_id=excluded.user_id,user_name=excluded.user_name,updated_at=excluded.updated_at",
            params![generation, conversation, service_url, tenant, user, name, now_ms()],
        )?;
        Ok(())
    }

    pub fn teams_ref(&self, generation: &str, conversation: &str) -> Result<Option<TeamsRef>> {
        Ok(self
            .db
            .locked()
            .query_row(
                "SELECT conversation_id,service_url,tenant_id FROM teams_refs WHERE generation=?1 AND conversation_id=?2",
                params![generation, conversation],
                |r| Ok(TeamsRef { conversation: r.get(0)?, service_url: r.get(1)?, tenant: r.get(2)? }),
            )
            .optional()?)
    }

    pub fn pending_teams_replies(&self, generation: &str) -> Result<Vec<TeamsOutgoing>> {
        let db = self.db.locked();
        let mut q = db.prepare(
            "SELECT e.id,t.conversation_id,t.activity_id,json_extract(e.data,'$.text'),t.bot_id
            FROM entries e JOIN teams_threads t ON t.bot_id=e.bot_id AND t.root_id=e.thread_id
            WHERE t.generation=?1 AND e.kind='agent' AND json_extract(e.data,'$.final')=1
            AND COALESCE(json_extract(e.data,'$.text'),'')<>''
            AND NOT EXISTS(SELECT 1 FROM teams_outbox o WHERE o.entry_id=e.id) ORDER BY e.seq LIMIT 20",
        )?;
        Ok(q.query_map([generation], |r| {
            Ok(TeamsOutgoing {
                entry: r.get(0)?,
                conversation: r.get(1)?,
                reply_to: r.get(2)?,
                text: r.get(3)?,
                bot: r.get(4)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn claim_teams_reply(&self, entry: &str) -> Result<bool> {
        Ok(self.db.locked().execute(
            "INSERT INTO teams_outbox(entry_id,state,updated_at) VALUES(?1,'sending',?2) ON CONFLICT DO NOTHING",
            params![entry, now_ms()],
        )? == 1)
    }

    pub fn finish_teams_reply(&self, entry: &str, state: &str, activity: Option<&str>) -> Result<()> {
        self.db.locked().execute(
            "UPDATE teams_outbox SET state=?2,activity_id=?3,updated_at=?4 WHERE entry_id=?1 AND state='sending'",
            params![entry, state, activity, now_ms()],
        )?;
        Ok(())
    }

    pub fn teams_deliveries(&self) -> Result<Vec<TeamsDelivery>> {
        let db = self.db.locked();
        let mut q = db.prepare(
            "SELECT entry_id,CASE WHEN state='sending' THEN 'unknown' ELSE state END AS state
            FROM teams_outbox ORDER BY updated_at DESC LIMIT 50",
        )?;
        Ok(q.query_map([], |r| Ok(TeamsDelivery { id: r.get(0)?, state: r.get(1)? }))?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn map_teams_card(&self, generation: &str, conversation: &str, activity: &str, entry: &str) -> Result<()> {
        self.db.locked().execute(
            "INSERT INTO teams_cards(generation,conversation_id,activity_id,entry_id) VALUES(?1,?2,?3,?4)
            ON CONFLICT(generation,conversation_id,activity_id) DO UPDATE SET entry_id=excluded.entry_id",
            params![generation, conversation, activity, entry],
        )?;
        Ok(())
    }

    pub fn teams_card_entry(&self, generation: &str, conversation: &str, activity: &str) -> Result<Option<String>> {
        Ok(self
            .db
            .locked()
            .query_row(
                "SELECT entry_id FROM teams_cards WHERE generation=?1 AND conversation_id=?2 AND activity_id=?3",
                params![generation, conversation, activity],
                |r| r.get(0),
            )
            .optional()?)
    }
}
