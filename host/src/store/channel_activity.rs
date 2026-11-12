//! Accepted channel receipts contain routing metadata only, never message text or user identifiers.

use super::Store;
use crate::LockExt;
use anyhow::{Result, ensure};
use rusqlite::{OptionalExtension, params};
use serde::Serialize;

const RETAINED: i64 = 200;

pub(super) fn open(db: &rusqlite::Connection) -> Result<()> {
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS channel_activity(
            id TEXT PRIMARY KEY, source TEXT NOT NULL CHECK(source IN ('slack','discord','telegram')),
            bot_id TEXT NOT NULL, thread_id TEXT, channel TEXT NOT NULL,
            mentioned INTEGER NOT NULL CHECK(mentioned IN (0,1)), at INTEGER NOT NULL);
         CREATE INDEX IF NOT EXISTS channel_activity_recent ON channel_activity(at DESC,id DESC);",
    )?;
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelActivity {
    pub id: String,
    pub source: String,
    pub bot_id: String,
    pub thread_id: Option<String>,
    pub channel: String,
    pub mentioned: bool,
    pub at: i64,
}

impl Store {
    /// Only trusted channel ingress records receipts; client message bodies cannot manufacture them.
    pub(crate) fn record_channel_activity(&self, activity: &ChannelActivity) -> Result<()> {
        ensure!(matches!(activity.source.as_str(), "slack" | "discord" | "telegram"), "Unsupported channel source");
        ensure!(!activity.id.is_empty() && activity.id.len() <= 256, "Invalid channel receipt identifier");
        let mut db = self.db.locked();
        let tx = db.transaction()?;
        // Old retries must keep their first accepted timestamp even after their receipt was pruned.
        let inbox = match activity.source.as_str() {
            "slack" => activity
                .id
                .strip_prefix("slack-")
                .map(|entry| ("SELECT created_at FROM slack_inbox WHERE entry_id=?1 AND bot_id=?2", entry)),
            "discord" => activity
                .id
                .strip_prefix("discord-")
                .map(|entry| ("SELECT created_at FROM discord_inbox WHERE entry_id=?1 AND bot_id=?2", entry)),
            _ => None,
        };
        let at = if let Some((sql, entry)) = inbox {
            tx.query_row(sql, params![entry, activity.bot_id], |row| row.get::<_, i64>(0))
                .optional()?
                .unwrap_or(activity.at)
        } else {
            activity.at
        };
        tx.execute(
            "INSERT OR IGNORE INTO channel_activity(id,source,bot_id,thread_id,channel,mentioned,at)
             SELECT ?1,?2,?3,?4,?5,?6,?7
             WHERE EXISTS(SELECT 1 FROM bots WHERE id=?3 AND deleted=0)",
            params![
                activity.id,
                activity.source,
                activity.bot_id,
                activity.thread_id,
                activity.channel,
                activity.mentioned,
                at
            ],
        )?;
        tx.execute(
            "DELETE FROM channel_activity WHERE id NOT IN
             (SELECT id FROM channel_activity ORDER BY at DESC,id DESC LIMIT ?1)",
            [RETAINED],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Newest accepted receipts across visible, live bots, with a bounded page size.
    pub fn channel_activity(&self, limit: usize) -> Result<Vec<ChannelActivity>> {
        self.channel_activity_page(limit, false)
    }

    /// Filter before limiting so a busy channel cannot hide the latest mention.
    pub fn channel_mentions(&self, limit: usize) -> Result<Vec<ChannelActivity>> {
        self.channel_activity_page(limit, true)
    }

    fn channel_activity_page(&self, limit: usize, mentions_only: bool) -> Result<Vec<ChannelActivity>> {
        let db = self.db.locked();
        let mut query = db.prepare(
            "SELECT a.id,a.source,a.bot_id,a.thread_id,a.channel,a.mentioned,a.at
             FROM channel_activity a JOIN bots b ON b.id=a.bot_id
             WHERE b.deleted=0 AND COALESCE(json_extract(b.config,'$.hidden'),0)=0 AND (?2=0 OR a.mentioned=1)
             ORDER BY a.at DESC,a.id DESC LIMIT ?1",
        )?;
        let rows = query.query_map(params![i64::try_from(limit.clamp(1, 50))?, mentions_only], |row| {
            Ok(ChannelActivity {
                id: row.get(0)?,
                source: row.get(1)?,
                bot_id: row.get(2)?,
                thread_id: row.get(3)?,
                channel: row.get(4)?,
                mentioned: row.get(5)?,
                at: row.get(6)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bot(store: &Store, id: &str) {
        let config = serde_json::json!({"id": id, "name": id});
        store
            .db
            .locked()
            .execute("INSERT INTO bots(id,rev,config) VALUES(?1,0,?2)", params![id, config.to_string()])
            .unwrap();
    }

    fn receipt(id: &str, at: i64) -> ChannelActivity {
        ChannelActivity {
            id: id.into(),
            source: "slack".into(),
            bot_id: "a".into(),
            thread_id: Some("thread-root".into()),
            channel: "channel".into(),
            mentioned: true,
            at,
        }
    }

    #[test]
    fn opening_the_store_creates_an_empty_receipt_feed() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        assert!(store.channel_activity(20).unwrap().is_empty());
    }

    #[test]
    fn non_mentions_do_not_displace_the_latest_mention_from_its_page() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        bot(&store, "a");
        store.record_channel_activity(&receipt("mention", 1)).unwrap();
        for at in 2..72 {
            let mut message = receipt(&format!("message-{at}"), at);
            message.mentioned = false;
            store.record_channel_activity(&message).unwrap();
        }
        assert_eq!(store.channel_mentions(1).unwrap()[0].id, "mention");
        assert_eq!(store.channel_activity(1).unwrap()[0].id, "message-71");
    }

    #[test]
    fn receipts_sort_newest_first_with_stable_ties_and_bounded_pages() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        bot(&store, "a");
        for (id, at) in [("a", 10), ("b", 20), ("c", 20)] {
            store.record_channel_activity(&receipt(id, at)).unwrap();
        }
        let page = store.channel_activity(2).unwrap();
        assert_eq!(page.iter().map(|item| item.id.as_str()).collect::<Vec<_>>(), ["c", "b"]);
        assert_eq!(store.channel_activity(0).unwrap().len(), 1);
        for at in 30..240 {
            store.record_channel_activity(&receipt(&format!("r-{at}"), at)).unwrap();
        }
        assert_eq!(store.channel_activity(usize::MAX).unwrap().len(), 50);
        assert_eq!(
            store
                .db
                .locked()
                .query_row("SELECT COUNT(*) FROM channel_activity", [], |row| row.get::<_, i64>(0))
                .unwrap(),
            RETAINED
        );
    }

    #[test]
    fn redelivery_keeps_the_original_receipt_instead_of_fabricating_new_activity() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        bot(&store, "a");
        let original = receipt("same-event", 10);
        store.record_channel_activity(&original).unwrap();
        let mut redelivery = original.clone();
        redelivery.at = 20;
        redelivery.mentioned = false;
        store.record_channel_activity(&redelivery).unwrap();
        assert_eq!(store.channel_activity(20).unwrap(), [original]);
    }

    #[test]
    fn replaying_a_pruned_slack_receipt_does_not_make_an_old_message_new() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        bot(&store, "a");
        store
            .enqueue_slack(&super::super::slack::SlackIncoming {
                event: "event".into(),
                generation: "generation".into(),
                bot: "a".into(),
                channel: "channel".into(),
                thread: "thread".into(),
                entry: "channel-entry".into(),
                text: "message".into(),
                user: "person".into(),
            })
            .unwrap();
        store.db.locked().execute("UPDATE slack_inbox SET created_at=1 WHERE entry_id='channel-entry'", []).unwrap();
        let mut original = receipt("slack-channel-entry", 10);
        store.record_channel_activity(&original).unwrap();
        assert_eq!(store.channel_activity(1).unwrap()[0].at, 1);
        for at in 2..202 {
            store.record_channel_activity(&receipt(&format!("new-{at}"), at)).unwrap();
        }
        original.at = 10_000;
        store.record_channel_activity(&original).unwrap();
        let old_count: i64 = store
            .db
            .locked()
            .query_row("SELECT COUNT(*) FROM channel_activity WHERE id='slack-channel-entry'", [], |row| row.get(0))
            .unwrap();
        assert_eq!(old_count, 0);
        assert_eq!(store.channel_activity(1).unwrap()[0].at, 201);
    }

    #[test]
    fn deleted_hidden_and_missing_bots_cannot_leak_into_the_feed() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let activity = receipt("message", 10);
        store.record_channel_activity(&activity).unwrap();
        assert!(store.channel_activity(20).unwrap().is_empty());
        bot(&store, "a");
        store.record_channel_activity(&activity).unwrap();
        store.db.locked().execute("UPDATE bots SET config=json_set(config,'$.hidden',1) WHERE id='a'", []).unwrap();
        assert!(store.channel_activity(20).unwrap().is_empty());
        store.db.locked().execute("UPDATE bots SET config=json_remove(config,'$.hidden') WHERE id='a'", []).unwrap();
        assert_eq!(store.channel_activity(20).unwrap().len(), 1);
        store.db.locked().execute("UPDATE bots SET deleted=1 WHERE id='a'", []).unwrap();
        store.record_channel_activity(&receipt("late-message", 20)).unwrap();
        assert!(store.channel_activity(20).unwrap().is_empty());
        store.db.locked().execute("DELETE FROM bots WHERE id='a'", []).unwrap();
        assert!(store.channel_activity(20).unwrap().is_empty());
    }

    #[test]
    fn unrecognized_channel_sources_are_rejected() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let mut activity = receipt("message", 10);
        activity.source = "client".into();
        assert!(store.record_channel_activity(&activity).is_err());
    }
}
