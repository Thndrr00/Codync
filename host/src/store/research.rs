use super::Store;
use crate::LockExt;
use anyhow::{Context, Result, ensure};
use rusqlite::{OptionalExtension, params};
use serde_json::Value;
pub(super) fn open(db: &rusqlite::Connection) -> Result<()> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS research_packets(id TEXT PRIMARY KEY,bot_id TEXT NOT NULL,packet TEXT NOT NULL,created_at INTEGER NOT NULL);
        CREATE INDEX IF NOT EXISTS research_packets_bot ON research_packets(bot_id,created_at);")?;
    Ok(())
}
impl Store {
    pub fn save_research_packet(&self, bot: &str, packet: &Value) -> Result<()> {
        let mut db = self.db.locked();
        let tx = db.transaction()?;
        let inserted=tx.execute(
            "INSERT INTO research_packets SELECT ?1,?2,?3,?4 WHERE EXISTS(SELECT 1 FROM bots WHERE id=?2 AND deleted=0)",
            params![
                packet["id"].as_str().context("Missing evidence id")?,
                bot,
                serde_json::to_string(packet)?,
                packet["capturedAt"].as_i64().context("Missing evidence time")?
            ],
        )?;
        ensure!(inserted == 1, "Bot was removed before evidence could be saved.");
        tx.execute(
            "DELETE FROM research_packets WHERE bot_id=?1 AND id NOT IN
            (SELECT id FROM research_packets WHERE bot_id=?1 ORDER BY created_at DESC,rowid DESC LIMIT 200)",
            [bot],
        )?;
        tx.commit()?;
        Ok(())
    }
    pub fn research_packet(&self, bot: &str, id: &str) -> Result<Option<Value>> {
        let packet: Option<String> = self
            .db
            .locked()
            .query_row("SELECT packet FROM research_packets WHERE bot_id=?1 AND id=?2", params![bot, id], |r| r.get(0))
            .optional()?;
        packet.map(|s| serde_json::from_str(&s).map_err(Into::into)).transpose()
    }
}
