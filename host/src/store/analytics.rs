use super::Store;
use crate::LockExt;
use anyhow::Result;
use rusqlite::{Connection, params};
use serde_json::Value;

pub(super) fn open(c: &Connection) -> Result<()> {
    c.execute_batch(include_str!("analytics.sql"))?;
    Ok(())
}
impl Store {
    /// Consent, deduplication and the durable bounded queue share one transaction.
    pub fn enqueue_analytics(&self, payload: &Value, client_event: bool) -> Result<bool> {
        let id = payload["uuid"].as_str().ok_or_else(|| anyhow::anyhow!("Missing analytics event identity"))?;
        let encoded = serde_json::to_string(payload)?;
        anyhow::ensure!(encoded.len() <= 65536, "Analytics event exceeds its size limit");
        let mut c = self.db.locked();
        let tx = c.transaction()?;
        let enabled: bool =
            tx.query_row("SELECT COALESCE((SELECT v FROM kv WHERE k='analytics_enabled'),'1') != '0'", [], |r| {
                r.get(0)
            })?;
        if !enabled {
            return Ok(false);
        }
        if client_event
            && tx.execute(
                "INSERT INTO analytics_client_events(id,created_at) VALUES(?1,?2) ON CONFLICT DO NOTHING",
                params![id, super::now_ms()],
            )? == 0
        {
            return Ok(false);
        }
        let added = tx.execute(
            "INSERT INTO analytics_outbox(id,payload) VALUES(?1,?2) ON CONFLICT DO NOTHING",
            params![id, encoded],
        )? == 1;
        tx.execute("DELETE FROM analytics_outbox WHERE seq NOT IN (SELECT seq FROM analytics_outbox ORDER BY seq DESC LIMIT 500)", [])?;
        tx.execute("DELETE FROM analytics_client_events WHERE id NOT IN (SELECT id FROM analytics_client_events ORDER BY created_at DESC,rowid DESC LIMIT 2000)", [])?;
        tx.commit()?;
        Ok(added)
    }
    pub fn pending_analytics(&self) -> Result<Vec<Value>> {
        let c = self.db.locked();
        let mut st = c.prepare("SELECT payload FROM analytics_outbox ORDER BY seq LIMIT 50")?;
        let encoded = st.query_map([], |row| row.get::<_, String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
        encoded.into_iter().map(|text| Ok(serde_json::from_str(&text)?)).collect()
    }
    pub fn acknowledge_analytics(&self, batch: &[Value]) -> Result<()> {
        let mut c = self.db.locked();
        let tx = c.transaction()?;
        for event in batch {
            tx.execute("DELETE FROM analytics_outbox WHERE id=?1", [event["uuid"].as_str().unwrap_or_default()])?;
        }
        tx.commit()?;
        Ok(())
    }
    pub fn set_analytics_enabled(&self, on: bool) -> Result<()> {
        let mut c = self.db.locked();
        let tx = c.transaction()?;
        tx.execute(
            "INSERT INTO kv(k,v) VALUES('analytics_enabled',?1) ON CONFLICT(k) DO UPDATE SET v=excluded.v",
            [if on { "1" } else { "0" }],
        )?;
        if !on {
            tx.execute("DELETE FROM analytics_outbox", [])?;
        }
        tx.commit()?;
        Ok(())
    }
}
