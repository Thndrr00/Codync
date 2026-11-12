//! Last host-recorded context and scoped MCP request metadata for the desktop island.
//! Usage is a completed-turn report, never a guessed live context window or provider bill.

use crate::hub::Hub;
use anyhow::{Context, Result, anyhow, ensure};
use serde_json::{Value, json};
use std::sync::Arc;

const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

pub(super) fn read(hub: &Arc<Hub>, body: &Value) -> Result<Value> {
    let bot_id = body["botId"].as_str().context("botId is required")?;
    let row = hub.store.bot(bot_id)?.filter(|row| !row.deleted).context("Unknown bot")?;
    let mcp = hub.store.island_mcp_calls(bot_id)?;
    // This is the same host-only key written by agent::bot::compaction for this persisted session.
    let stored = row
        .session_id
        .as_deref()
        .map(|session| hub.store.kv_read(&format!("compaction.{bot_id}.{session}")))
        .transpose();
    let context = stored.and_then(|stored| {
        stored
            .flatten()
            .map(|raw| {
                let value: Value =
                    serde_json::from_str(&raw).map_err(|_| anyhow!("Stored context report is invalid."))?;
                project_context(&value)
            })
            .transpose()
    });
    let (context, error) = match context {
        Ok(context) => (context, None),
        Err(_) => (None, Some("The saved context report is unavailable or invalid.")),
    };
    Ok(json!({
        "context": context,
        "cost": {"amount": null, "currency": null, "reason": "notReported"},
        "mcp": mcp,
        "error": error,
    }))
}

fn counter(value: &Value, field: &str, maximum: u64) -> Result<Option<u64>> {
    match value.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value.as_u64().filter(|number| *number <= maximum).map(Some).context("Invalid context counter"),
    }
}

fn project_context(value: &Value) -> Result<Value> {
    ensure!(value.is_object(), "Invalid context report");
    let used = counter(value, "used", MAX_SAFE_INTEGER)?;
    let size = counter(value, "size", MAX_SAFE_INTEGER)?.filter(|size| *size > 0);
    let reported = used.is_some_and(|used| used > 0) || size.is_some();
    let last_turn = counter(value, "lastTurn", MAX_SAFE_INTEGER)?.filter(|at| *at > 0);
    let compactions = counter(value, "compactions", u32::MAX.into())?.unwrap_or_default();
    Ok(json!({
        "used": used.filter(|_| reported), "size": size,
        "compactions": compactions, "lastTurn": last_turn,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_reported_context_counters_are_projected() {
        let projected = project_context(&json!({
            "used": 120_000, "size": 200_000, "lastTurn": 1000, "compactions": 2,
            "summary": "private", "sessionId": "secret-session", "hostCompactions": [1, 2],
        }))
        .expect("valid context");
        assert_eq!(projected, json!({"used":120_000,"size":200_000,"lastTurn":1000,"compactions":2}));
    }

    #[test]
    fn default_or_absent_usage_is_unknown_but_a_reported_zero_is_valid() {
        assert_eq!(
            project_context(&json!({})).expect("absent"),
            json!({"used":null,"size":null,"lastTurn":null,"compactions":0})
        );
        assert_eq!(project_context(&json!({"used":0,"size":0,"lastTurn":0})).expect("default")["used"], Value::Null);
        assert_eq!(project_context(&json!({"used":0,"size":200_000})).expect("reported zero")["used"], 0);
        assert_eq!(project_context(&json!({"used":25,"size":0})).expect("unknown window")["size"], Value::Null);
    }

    #[test]
    fn invalid_or_unrepresentable_counters_are_not_published() {
        for value in [
            json!(null),
            json!([]),
            json!({"used":-1}),
            json!({"used":"12"}),
            json!({"size":1.5}),
            json!({"used":MAX_SAFE_INTEGER+1}),
            json!({"compactions":u64::MAX}),
        ] {
            assert!(project_context(&value).is_err());
        }
    }
}
