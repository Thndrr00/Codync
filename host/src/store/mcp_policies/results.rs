//! Server-observed annotations and immutable execution receipts; never infer success from dispatch.
use super::{Store, get, now_ms};
use crate::LockExt;
use anyhow::{Context, Result, bail, ensure};
use rusqlite::{OptionalExtension, params};
use serde_json::Value;

impl Store {
    pub fn claim_mcp_execution(&self, id: &str) -> Result<()> {
        let mut db = self.db.locked();
        let tx = db.transaction()?;
        let request = super::get(&tx, id)?.context("Unknown tool request.")?;
        if request.status != "dispatched" {
            bail!("Tool request is not dispatched.");
        }
        tx.execute("INSERT INTO mcp_execution_claims VALUES(?1,?2)", params![id, now_ms()])
            .context("Execution was already attempted. Read its receipt; do not retry it.")?;
        tx.commit()?;
        Ok(())
    }
    pub fn observe_mcp_tools(&self, connector: &str, tools: &[Value]) -> Result<Vec<String>> {
        ensure!(tools.len() <= 1000, "Tool catalog page exceeds 1000 tools.");
        let mut db = self.db.locked();
        let tx = db.transaction()?;
        let mut revoked = vec![];
        for tool in tools {
            let name = tool["name"]
                .as_str()
                .filter(|name| !name.is_empty() && name.len() <= 256)
                .context("Invalid catalog tool name.")?;
            let read_only = tool["annotations"]["readOnlyHint"] == true;
            let old: Option<bool> = tx
                .query_row(
                    "SELECT read_only FROM connector_tool_catalog WHERE connector_id=?1 AND tool_name=?2",
                    params![connector, name],
                    |row| row.get(0),
                )
                .optional()?;
            if old.is_some_and(|old| old != read_only) {
                revoked.extend(super::management::active_ids(&tx, connector, Some(name))?);
                tx.execute("UPDATE pending_tool_calls SET status='revoked' WHERE connector_id=?1 AND tool_name=?2 AND status IN ('pending','approved','dispatched')",params![connector,name])?;
            }
            tx.execute("INSERT INTO connector_tool_catalog VALUES(?1,?2,?3) ON CONFLICT(connector_id,tool_name) DO UPDATE SET read_only=excluded.read_only",
                params![connector,name,read_only])?;
        }
        tx.commit()?;
        Ok(revoked)
    }

    pub fn complete_mcp_call(&self, id: &str, reply: &Value) -> Result<()> {
        let encoded = serde_json::to_string(reply)?;
        ensure!(encoded.len() <= 512 * 1024, "Execution receipt exceeds 512 KiB.");
        let mut db = self.db.locked();
        let tx = db.transaction()?;
        let call = get(&tx, id)?.context("Unknown tool request.")?;
        ensure!(
            reply["jsonrpc"] == "2.0"
                && reply["id"] == call.message["id"]
                && (reply.get("result").is_some() ^ reply.get("error").is_some()),
            "Receipt must be the original JSON-RPC response."
        );
        // Revocation/expiry cannot erase an outcome that already reached the external server.
        let dispatched: bool = tx.query_row(
            "SELECT status IN ('dispatched','revoked') FROM pending_tool_calls WHERE id=?1",
            [id],
            |row| row.get(0),
        )?;
        ensure!(dispatched, "A request must be dispatched before recording its result.");
        let old: Option<String> =
            tx.query_row("SELECT result FROM mcp_call_results WHERE call_id=?1", [id], |row| row.get(0)).optional()?;
        if let Some(old) = old {
            if old != encoded {
                bail!("The immutable execution receipt already exists.");
            }
        } else {
            tx.execute("INSERT INTO mcp_call_results VALUES(?1,?2,?3)", params![id, encoded, now_ms()])?;
        }
        tx.commit()?;
        Ok(())
    }
    pub fn mcp_result(&self, id: &str) -> Result<Option<Value>> {
        let encoded: Option<String> = self
            .db
            .locked()
            .query_row("SELECT result FROM mcp_call_results WHERE call_id=?1", [id], |row| row.get(0))
            .optional()?;
        encoded.map(|encoded| Ok(serde_json::from_str(&encoded)?)).transpose()
    }
}
