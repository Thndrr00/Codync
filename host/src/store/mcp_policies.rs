//! Durable, immutable remote-MCP requests and single-dispatch approval claims.
use super::{Store, now_ms};
use crate::LockExt;
use anyhow::{Result, bail};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;
mod management;
mod results;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Allow,
    Ask,
    Deny,
}

impl Mode {
    fn text(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Ask => "ask",
            Self::Deny => "deny",
        }
    }
}

pub struct Pending {
    pub bot: String,
    pub connector: String,
    pub tool: String,
    pub message: Value,
    pub status: String,
}

pub(super) fn open(db: &Connection) -> Result<()> {
    db.execute_batch(include_str!("mcp_policies.sql"))?;
    Ok(())
}

fn mode(db: &Connection, connector: &str, tool: &str) -> Result<Mode> {
    let value: Option<String> = db
        .query_row(
            "SELECT mode FROM connector_tool_policies WHERE connector_id=?1 AND tool_name=?2",
            params![connector, tool],
            |row| row.get(0),
        )
        .optional()?;
    Ok(match value.as_deref() {
        Some("allow") => Mode::Allow,
        Some("deny") => Mode::Deny,
        Some("ask") => Mode::Ask,
        _ => {
            let read_only: bool = db.query_row("SELECT COALESCE((SELECT read_only FROM connector_tool_catalog WHERE connector_id=?1 AND tool_name=?2),0)", params![connector,tool], |row| row.get(0))?;
            if read_only { Mode::Allow } else { Mode::Ask }
        }
    })
}

fn get(db: &Connection, id: &str) -> Result<Option<Pending>> {
    Ok(db
        .query_row(
            "SELECT bot_id,connector_id,tool_name,message,
        CASE WHEN expires_at<=?2 AND status IN ('pending','approved','dispatched') THEN 'expired' ELSE status END
        FROM pending_tool_calls WHERE id=?1",
            params![id, now_ms()],
            |row| {
                let message: String = row.get(3)?;
                let message = serde_json::from_str(&message).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, Box::new(error))
                })?;
                Ok(Pending {
                    bot: row.get(0)?,
                    connector: row.get(1)?,
                    tool: row.get(2)?,
                    message,
                    status: row.get(4)?,
                })
            },
        )
        .optional()?)
}

impl Store {
    pub fn revoke_mcp_connector(&self, bot: Option<&str>, connector: &str) -> Result<()> {
        let mut db = self.db.locked();
        let tx = db.transaction()?;
        tx.execute(
            "UPDATE pending_tool_calls SET status='revoked' WHERE connector_id=?1
            AND (?2 IS NULL OR bot_id=?2) AND status IN ('pending','approved','dispatched')",
            params![connector, bot],
        )?;
        if bot.is_none() {
            tx.execute("DELETE FROM connector_tool_policies WHERE connector_id=?1", [connector])?;
            tx.execute("DELETE FROM connector_tool_catalog WHERE connector_id=?1", [connector])?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn cancel_mcp_call(&self, id: &str) -> Result<()> {
        self.db.locked().execute(
            "UPDATE pending_tool_calls SET status='revoked' WHERE id=?1
            AND status IN ('pending','approved','dispatched')",
            [id],
        )?;
        Ok(())
    }
    pub fn mcp_policy(&self, connector: &str, tool: &str) -> Result<Mode> {
        mode(&self.db.locked(), connector, tool)
    }

    pub fn set_mcp_policy(&self, connector: &str, tool: &str, policy: Mode) -> Result<Vec<String>> {
        if tool.is_empty() || tool.len() > 256 {
            bail!("tool name must contain 1–256 bytes");
        }
        let mut db = self.db.locked();
        let tx = db.transaction()?;
        let ids = management::active_ids(&tx, connector, Some(tool))?;
        tx.execute(
            "INSERT INTO connector_tool_policies VALUES(?1,?2,?3)
            ON CONFLICT(connector_id,tool_name) DO UPDATE SET mode=excluded.mode",
            params![connector, tool, policy.text()],
        )?;
        // A configuration change never inherits an earlier approval, even after re-enabling.
        tx.execute(
            "UPDATE pending_tool_calls SET status='revoked' WHERE connector_id=?1 AND tool_name=?2
            AND status IN ('pending','approved','dispatched')",
            params![connector, tool],
        )?;
        tx.commit()?;
        Ok(ids)
    }

    pub fn create_mcp_call(&self, bot: &str, connector: &str, message: &Value) -> Result<String> {
        let tool = message["params"]["name"]
            .as_str()
            .filter(|name| !name.is_empty() && name.len() <= 256)
            .ok_or_else(|| anyhow::anyhow!("tool name is required"))?;
        if message["method"] != "tools/call"
            || message.get("id").is_none_or(Value::is_null)
            || !message["params"]["arguments"].is_object()
        {
            bail!("invalid tool call");
        }
        let encoded = serde_json::to_string(message)?;
        if encoded.len() > 128 * 1024 {
            bail!("tool request exceeds 128 KiB");
        }
        let mut db = self.db.locked();
        let tx = db.transaction()?;
        let policy = mode(&tx, connector, tool)?;
        if policy == Mode::Deny {
            bail!("this tool is disabled by its host policy");
        }
        let count: i64 = tx.query_row(
            "SELECT COUNT(*) FROM pending_tool_calls WHERE bot_id=?1
            AND status IN ('pending','approved') AND expires_at>?2",
            params![bot, now_ms()],
            |row| row.get(0),
        )?;
        if count >= 100 {
            bail!("too many pending tool requests");
        }
        let id = uuid::Uuid::new_v4().to_string();
        let now = now_ms();
        tx.execute(
            "INSERT INTO pending_tool_calls VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
            params![
                id,
                bot,
                connector,
                tool,
                encoded,
                if policy == Mode::Allow { "approved" } else { "pending" },
                now,
                now + 3_600_000
            ],
        )?;
        tx.commit()?;
        Ok(id)
    }

    pub fn mcp_call(&self, id: &str) -> Result<Option<Pending>> {
        get(&self.db.locked(), id)
    }

    pub fn decide_mcp_call(&self, id: &str, approve: bool) -> Result<()> {
        self.db.locked().execute(
            "UPDATE pending_tool_calls SET status=?2 WHERE id=?1 AND status='pending'
            AND expires_at>?3",
            params![id, if approve { "approved" } else { "declined" }, now_ms()],
        )?;
        Ok(())
    }

    pub fn claim_mcp_call(&self, id: &str) -> Result<Option<Value>> {
        let mut db = self.db.locked();
        let tx = db.transaction()?;
        let call = get(&tx, id)?.ok_or_else(|| anyhow::anyhow!("unknown tool request"))?;
        if call.status != "approved" {
            return Ok(None);
        }
        if mode(&tx, &call.connector, &call.tool)? == Mode::Deny {
            bail!("tool is disabled");
        }
        let count = tx.execute(
            "UPDATE pending_tool_calls SET status='dispatched' WHERE id=?1
            AND status='approved' AND expires_at>?2",
            params![id, now_ms()],
        )?;
        tx.commit()?;
        Ok((count == 1).then_some(call.message))
    }
}

#[cfg(test)]
mod tests;
