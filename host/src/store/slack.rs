use super::{Store, now_ms};
use crate::LockExt;
use anyhow::Result;
use rusqlite::params;
use serde::Serialize;

pub(super) fn open(db: &rusqlite::Connection) -> Result<()> {
    db.execute_batch(include_str!("slack.sql"))?;
    Ok(())
}

pub struct SlackIncoming {
    pub event: String,
    pub generation: String,
    pub bot: String,
    pub channel: String,
    pub thread: String,
    pub entry: String,
    pub text: String,
    pub user: String,
}
pub struct SlackOutgoing {
    pub entry: String,
    pub channel: String,
    pub thread: String,
    pub text: String,
    pub bot: String,
}
#[derive(Serialize)]
pub struct SlackDelivery {
    pub id: String,
    pub state: String,
    pub timestamp: Option<String>,
}

impl Store {
    pub fn resume_slack_sign_in(&self, entry: &str) -> Result<bool> {
        Ok(self.db.locked().execute(
            "UPDATE slack_inbox SET state='queued' WHERE entry_id=?1 AND state='started'
            AND EXISTS(SELECT 1 FROM entries e WHERE e.id=?1 AND json_extract(e.data,'$.source')='slack'
                AND json_extract(e.data,'$.turnResult.state')='waitingForSignIn')",
            [entry],
        )? == 1)
    }
    pub fn reject_stale_slack(&self, generation: Option<&str>) -> Result<Vec<String>> {
        let mut db = self.db.locked();
        let tx = db.transaction()?;
        let ids = {
            let mut q =
                tx.prepare("SELECT entry_id FROM slack_inbox WHERE state='queued' AND (?1 IS NULL OR generation<>?1)")?;
            q.query_map([generation], |r| r.get(0))?.collect::<rusqlite::Result<Vec<String>>>()?
        };
        tx.execute(
            "UPDATE slack_inbox SET state='rejected' WHERE state='queued' AND (?1 IS NULL OR generation<>?1)",
            [generation],
        )?;
        tx.commit()?;
        Ok(ids)
    }
    pub fn enqueue_slack(&self, event: &SlackIncoming) -> Result<()> {
        self.db.locked().execute(
            "INSERT INTO slack_inbox(event_id,generation,bot_id,channel_id,thread_ts,entry_id,text,user_id,created_at)
            VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9) ON CONFLICT DO NOTHING",
            params![
                event.event,
                event.generation,
                event.bot,
                event.channel,
                event.thread,
                event.entry,
                event.text,
                event.user,
                now_ms()
            ],
        )?;
        Ok(())
    }
    pub fn pending_slack(&self) -> Result<Vec<SlackIncoming>> {
        let db = self.db.locked();
        let mut q = db.prepare(
            "SELECT event_id,generation,bot_id,channel_id,thread_ts,entry_id,text,user_id
            FROM slack_inbox WHERE state='queued' ORDER BY created_at LIMIT 20",
        )?;
        Ok(q.query_map([], |r| {
            Ok(SlackIncoming {
                event: r.get(0)?,
                generation: r.get(1)?,
                bot: r.get(2)?,
                channel: r.get(3)?,
                thread: r.get(4)?,
                entry: r.get(5)?,
                text: r.get(6)?,
                user: r.get(7)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?)
    }
    pub fn slack_thread_exists(&self, generation: &str, channel: &str, thread: &str) -> Result<bool> {
        Ok(self.db.locked().query_row(
            "SELECT EXISTS(SELECT 1 FROM slack_threads WHERE generation=?1 AND channel_id=?2 AND thread_ts=?3)",
            params![generation, channel, thread],
            |r| r.get(0),
        )?)
    }
    pub fn map_slack_thread(&self, event: &SlackIncoming, root: &str) -> Result<()> {
        self.db.locked().execute(
            "INSERT INTO slack_threads VALUES(?1,?2,?3,?4,?5) ON CONFLICT DO NOTHING",
            params![event.generation, event.channel, event.thread, event.bot, root],
        )?;
        Ok(())
    }
    pub fn claim_slack_prompt(&self, entry: &str, generation: &str) -> Result<bool> {
        Ok(self.db.locked().execute(
            "UPDATE slack_inbox SET state='started' WHERE entry_id=?1 AND generation=?2 AND state='queued'",
            params![entry, generation],
        )? == 1)
    }
    pub fn reject_slack(&self, event: &str) -> Result<()> {
        self.db
            .locked()
            .execute("UPDATE slack_inbox SET state='rejected' WHERE event_id=?1 AND state='queued'", [event])?;
        Ok(())
    }
    pub fn pending_slack_replies(&self, generation: &str) -> Result<Vec<SlackOutgoing>> {
        let db = self.db.locked();
        let mut q = db.prepare(
            "SELECT e.id,t.channel_id,t.thread_ts,json_extract(e.data,'$.text'),t.bot_id
            FROM entries e JOIN slack_threads t ON t.bot_id=e.bot_id AND t.root_id=e.thread_id
            WHERE t.generation=?1 AND e.kind='agent' AND json_extract(e.data,'$.final')=1
            AND COALESCE(json_extract(e.data,'$.text'),'')<>''
            AND NOT EXISTS(SELECT 1 FROM slack_outbox o WHERE o.entry_id=e.id) ORDER BY e.seq LIMIT 20",
        )?;
        Ok(q.query_map([generation], |r| {
            Ok(SlackOutgoing {
                entry: r.get(0)?,
                channel: r.get(1)?,
                thread: r.get(2)?,
                text: r.get(3)?,
                bot: r.get(4)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?)
    }
    pub fn claim_slack_reply(&self, entry: &str) -> Result<bool> {
        Ok(self.db.locked().execute(
            "INSERT INTO slack_outbox VALUES(?1,'sending',NULL,?2) ON CONFLICT DO NOTHING",
            params![entry, now_ms()],
        )? == 1)
    }
    pub fn finish_slack_reply(&self, entry: &str, state: &str, timestamp: Option<&str>) -> Result<()> {
        self.db.locked().execute(
            "UPDATE slack_outbox SET state=?2,external_ts=?3,updated_at=?4 WHERE entry_id=?1 AND state='sending'",
            params![entry, state, timestamp, now_ms()],
        )?;
        Ok(())
    }
    pub fn slack_deliveries(&self) -> Result<Vec<SlackDelivery>> {
        let db = self.db.locked();
        let mut q = db.prepare(
            "SELECT entry_id,CASE WHEN state='sending' THEN 'unknown' ELSE state END AS state,external_ts
            FROM slack_outbox ORDER BY updated_at DESC LIMIT 50",
        )?;
        Ok(q.query_map([], |r| Ok(SlackDelivery { id: r.get(0)?, state: r.get(1)?, timestamp: r.get(2)? }))?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    }
}
