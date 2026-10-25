//! Owner-facing policy settings and bounded approval inbox queries.
use super::{Mode, Store, now_ms};
use crate::LockExt;
use anyhow::{Result, bail};
use rusqlite::{Connection, params};
use serde::Serialize;

#[derive(Serialize)]
pub struct ToolPolicy {
    tool: String,
    mode: Mode,
    explicit: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CallSummary {
    pub id: String,
    bot_id: String,
    tool: String,
    status: String,
    created_at: i64,
    expires_at: i64,
}

pub(super) fn active_ids(db: &Connection, connector: &str, tool: Option<&str>) -> Result<Vec<String>> {
    let mut query = db.prepare(
        "SELECT id FROM pending_tool_calls WHERE connector_id=?1
        AND (?2 IS NULL OR tool_name=?2) AND status IN ('pending','approved','dispatched')",
    )?;
    Ok(query
        .query_map(params![connector, tool], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?)
}

impl Store {
    pub fn mcp_tool_settings(&self, connector: &str) -> Result<Vec<ToolPolicy>> {
        let db = self.db.locked();
        let mut query = db.prepare(
            "SELECT names.tool_name,COALESCE(p.mode,CASE WHEN c.read_only=1 THEN 'allow' ELSE 'ask' END) AS mode,p.mode IS NOT NULL
            FROM (SELECT tool_name FROM connector_tool_policies WHERE connector_id=?1
                  UNION SELECT tool_name FROM connector_tool_catalog WHERE connector_id=?1) names
            LEFT JOIN connector_tool_policies p ON p.connector_id=?1 AND p.tool_name=names.tool_name
            LEFT JOIN connector_tool_catalog c ON c.connector_id=?1 AND c.tool_name=names.tool_name ORDER BY names.tool_name")?;
        let rows = query.query_map([connector], |row| {
            let mode: String = row.get(1)?;
            Ok(ToolPolicy {
                tool: row.get(0)?,
                mode: match mode.as_str() {
                    "allow" => Mode::Allow,
                    "deny" => Mode::Deny,
                    _ => Mode::Ask,
                },
                explicit: row.get(2)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn list_mcp_calls(&self, connector: &str, after: &str) -> Result<Vec<CallSummary>> {
        let db = self.db.locked();
        let mut query = db.prepare(
            "SELECT id,bot_id,tool_name,status,created_at,expires_at FROM pending_tool_calls
            WHERE connector_id=?1 AND id>?2 AND expires_at>?3
            AND status IN ('pending','approved','dispatched') ORDER BY id LIMIT 51",
        )?;
        let rows = query.query_map(params![connector, after, now_ms()], |row| {
            Ok(CallSummary {
                id: row.get(0)?,
                bot_id: row.get(1)?,
                tool: row.get(2)?,
                status: row.get(3)?,
                created_at: row.get(4)?,
                expires_at: row.get(5)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Revocation and optional override removal commit together; credentials remain installed.
    pub fn revoke_mcp_calls(&self, connector: &str, tool: Option<&str>, reset: bool) -> Result<Vec<String>> {
        if reset && tool.is_none() {
            bail!("a tool is required to reset its policy");
        }
        let mut db = self.db.locked();
        let tx = db.transaction()?;
        let ids = active_ids(&tx, connector, tool)?;
        tx.execute(
            "UPDATE pending_tool_calls SET status='revoked' WHERE connector_id=?1
            AND (?2 IS NULL OR tool_name=?2) AND status IN ('pending','approved','dispatched')",
            params![connector, tool],
        )?;
        if reset {
            tx.execute(
                "DELETE FROM connector_tool_policies WHERE connector_id=?1 AND tool_name=?2",
                params![connector, tool],
            )?;
        }
        tx.commit()?;
        Ok(ids)
    }
}
