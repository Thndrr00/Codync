//! Metadata-only computer activity. Arguments, output and error text never enter this table.

use super::{Store, now_ms};
use crate::LockExt;
use anyhow::Result;
use rusqlite::params;
use serde::{Deserialize, Serialize};

const RETAINED_PER_BOT: i64 = 1_000;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ComputerActivityResult {
    Success,
    Failed,
}

impl ComputerActivityResult {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputerActivity {
    pub id: i64,
    pub bot_id: String,
    pub action: String,
    pub actor_kind: String,
    pub actor_id: String,
    pub result: ComputerActivityResult,
    pub created_at: i64,
}

impl Store {
    pub(crate) fn record_computer_activity(
        &self,
        bot: &str,
        action: &str,
        result: ComputerActivityResult,
    ) -> Result<()> {
        self.record_computer_activity_as(bot, action, result, "bot", bot)
    }

    pub(crate) fn record_computer_activity_as(
        &self,
        bot: &str,
        action: &str,
        result: ComputerActivityResult,
        actor_kind: &str,
        actor_id: &str,
    ) -> Result<()> {
        let mut db = self.db.locked();
        let tx = db.transaction()?;
        // A bot deleted while the tool was running must not regain retained data.
        tx.execute(
            "INSERT INTO computer_activity(bot_id, action, result, created_at)
             SELECT ?1, ?2, ?3, ?4 WHERE EXISTS(SELECT 1 FROM bots WHERE id = ?1 AND deleted = 0)",
            params![bot, action, result.as_str(), now_ms()],
        )?;
        if tx.changes() > 0 {
            tx.execute(
                "INSERT INTO computer_activity_actors VALUES(?1,?2,?3)",
                params![tx.last_insert_rowid(), actor_kind, actor_id],
            )?;
        }
        tx.execute(
            "DELETE FROM computer_activity WHERE bot_id = ?1 AND id NOT IN
             (SELECT id FROM computer_activity WHERE bot_id = ?1 ORDER BY id DESC LIMIT ?2)",
            params![bot, RETAINED_PER_BOT],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Newest first, bounded to the bot's retained history.
    pub fn computer_activity(&self, bot: &str) -> Result<Vec<ComputerActivity>> {
        let db = self.db.locked();
        let mut query = db.prepare(
            "SELECT a.id, a.bot_id, a.action, a.result, a.created_at,
             COALESCE(w.actor_kind,'bot'),COALESCE(w.actor_id,a.bot_id)
             FROM computer_activity a LEFT JOIN computer_activity_actors w ON w.activity_id=a.id
             WHERE a.bot_id = ?1 ORDER BY a.id DESC LIMIT ?2",
        )?;
        let rows = query.query_map(params![bot, RETAINED_PER_BOT], |row| {
            let result: String = row.get(3)?;
            let result = match result.as_str() {
                "success" => ComputerActivityResult::Success,
                "failed" => ComputerActivityResult::Failed,
                _ => return Err(rusqlite::Error::InvalidQuery),
            };
            let bot_id: String = row.get(1)?;
            Ok(ComputerActivity {
                id: row.get(0)?,
                actor_kind: row.get(5)?,
                actor_id: row.get(6)?,
                bot_id,
                action: row.get(2)?,
                result,
                created_at: row.get(4)?,
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
            .execute("INSERT INTO bots(id, rev, config) VALUES(?1, 0, ?2)", params![id, config.to_string()])
            .unwrap();
    }

    #[test]
    fn retention_is_per_bot_and_deleting_a_bot_removes_its_activity() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        bot(&store, "a");
        bot(&store, "b");
        store.record_computer_activity("b", "click", ComputerActivityResult::Failed).unwrap();
        for _ in 0..1_002 {
            store.record_computer_activity("a", "click", ComputerActivityResult::Success).unwrap();
        }
        let items = store.computer_activity("a").unwrap();
        assert_eq!(items.len(), 1_000);
        assert!(items.windows(2).all(|pair| pair[0].id > pair[1].id));
        assert_eq!(store.computer_activity("b").unwrap().len(), 1);
        store.delete_bot("a").unwrap();
        assert!(store.computer_activity("a").unwrap().is_empty());
        store.record_computer_activity("a", "click", ComputerActivityResult::Success).unwrap();
        assert!(store.computer_activity("a").unwrap().is_empty());
        assert_eq!(store.computer_activity("b").unwrap().len(), 1);
    }

    #[test]
    fn activity_survives_reopening_the_database() {
        let dir = std::env::temp_dir().join(format!("codync-activity-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("host.db");
        {
            let store = Store::open(&path).unwrap();
            bot(&store, "a");
            store.record_computer_activity("a", "type_text", ComputerActivityResult::Success).unwrap();
        }
        {
            let store = Store::open(&path).unwrap();
            let rows = store.computer_activity("a").unwrap();
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].actor_id, "a");
            assert_eq!(rows[0].result, ComputerActivityResult::Success);
        }
        std::fs::remove_dir_all(dir).unwrap();
    }
}
