use super::Store;
use crate::LockExt;
use anyhow::Result;
use rusqlite::params;
use serde_json::Value;
impl Store {
    pub fn set_turn_marker(&self, bot: &str, value: &str, sources: Option<&Value>) -> Result<()> {
        let mut db = self.db.locked();
        let tx = db.transaction()?;
        tx.execute(
            "INSERT INTO kv(k,v) VALUES(?1,?2) ON CONFLICT(k) DO UPDATE SET v=excluded.v",
            params![format!("turn.inflight.{bot}"), value],
        )?;
        let source_key = format!("turn.sources.{bot}");
        if let Some(sources) = sources {
            tx.execute(
                "INSERT INTO kv(k,v) VALUES(?1,?2) ON CONFLICT(k) DO UPDATE SET v=excluded.v",
                params![source_key, serde_json::to_string(sources)?],
            )?;
        } else {
            tx.execute("DELETE FROM kv WHERE k=?1", [source_key])?;
        }
        tx.commit()?;
        Ok(())
    }
}
