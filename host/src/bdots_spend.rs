#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Spend aggregation and cursor helpers ported from bdots' `agents/spend.ts`. Pure arithmetic;
//! the host already has its own budget table, so these are used for reporting, not enforcement.

use serde_json::Value;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SpendRow {
    pub day: String,
    pub bot_id: String,
    pub turn: String,
    pub cost: f64,
    pub tokens_in: u64,
    pub tokens_out: u64,
    pub requests: u64,
}

fn meta(log: &Value) -> Value {
    match log.get("metadata") {
        Some(Value::String(raw)) => serde_json::from_str::<Value>(raw).unwrap_or(Value::Null),
        other => other.cloned().unwrap_or(Value::Null),
    }
}

/// Group logs by UTC day + bot + turn kind, summing cost/tokens/requests, dropping foreign bots
/// (`tally`). `day` is the first 10 chars of `created_at`.
pub fn tally(logs: &[Value], bots: &[String]) -> Vec<SpendRow> {
    let mut rows: Vec<SpendRow> = Vec::new();
    for log in logs {
        let m = meta(log);
        let bot = m.get("botId").and_then(Value::as_str).unwrap_or("");
        if !bots.iter().any(|b| b == bot) {
            continue;
        }
        let turn = if m.get("turn").and_then(Value::as_str) == Some("background") { "background" } else { "chat" };
        let created = log.get("created_at").and_then(Value::as_str).unwrap_or("");
        let day = created.chars().take(10).collect::<String>();
        let cost = log.get("cost").and_then(Value::as_f64).unwrap_or(0.0);
        let tin = log.get("tokens_in").and_then(Value::as_u64).unwrap_or(0);
        let tout = log.get("tokens_out").and_then(Value::as_u64).unwrap_or(0);
        if let Some(row) = rows.iter_mut().find(|r| r.day == day && r.bot_id == bot && r.turn == turn) {
            row.cost += cost;
            row.tokens_in += tin;
            row.tokens_out += tout;
            row.requests += 1;
        } else {
            rows.push(SpendRow {
                day,
                bot_id: bot.to_owned(),
                turn: turn.to_owned(),
                cost,
                tokens_in: tin,
                tokens_out: tout,
                requests: 1,
            });
        }
    }
    rows
}

/// Logs with a time newer than the cursor, dropping ids already seen at that time (`newLogs`).
pub fn new_logs<'a>(logs: &'a [Value], cursor: Option<&(String, Vec<String>)>) -> Vec<&'a Value> {
    let Some((at, seen)) = cursor else { return logs.iter().collect() };
    logs.iter()
        .filter(|log| {
            let created = log.get("created_at").and_then(Value::as_str).unwrap_or("");
            if created <= at.as_str() {
                return false;
            }
            let id = log.get("id").and_then(Value::as_str).unwrap_or("");
            !seen.iter().any(|s| s == id)
        })
        .collect()
}

/// Move a sync cursor to the newest log time, keeping the ids seen at that time (`advance`).
pub fn advance(logs: &[Value], cursor: Option<(String, Vec<String>)>) -> Option<(String, Vec<String>)> {
    let newest = logs.iter().filter_map(|l| l.get("created_at").and_then(Value::as_str)).max()?.to_owned();
    let ids: Vec<String> = logs
        .iter()
        .filter(|l| l.get("created_at").and_then(Value::as_str) == Some(newest.as_str()))
        .filter_map(|l| l.get("id").and_then(Value::as_str).map(str::to_owned))
        .collect();
    let mut next = (newest, ids);
    if let Some(old) = cursor
        && old.0 == next.0
    {
        for id in old.1 {
            if !next.1.contains(&id) {
                next.1.push(id);
            }
        }
    }
    Some(next)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn log(id: &str, at: &str, bot: &str, cost: f64, background: bool) -> Value {
        json!({
            "id": id, "created_at": at, "cost": cost, "tokens_in": 10, "tokens_out": 5,
            "metadata": json!({"botId": bot, "turn": if background { "background" } else { "chat" }}).to_string()
        })
    }

    #[test]
    fn logs_tally_by_day_bot_and_turn() {
        let logs = vec![
            log("a", "2026-01-01T10:00:00Z", "b1", 1.5, false),
            log("b", "2026-01-01T11:00:00Z", "b1", 0.5, false),
            log("c", "2026-01-01T12:00:00Z", "b1", 2.0, true),
            log("d", "2026-01-01T13:00:00Z", "other", 9.0, false),
        ];
        let rows = tally(&logs, &["b1".to_owned()]);
        assert_eq!(rows.len(), 2);
        let chat = rows.iter().find(|r| r.turn == "chat").unwrap();
        assert_eq!((chat.requests, chat.cost, chat.tokens_in), (2, 2.0, 20));
        let bg = rows.iter().find(|r| r.turn == "background").unwrap();
        assert_eq!(bg.cost, 2.0);
    }

    #[test]
    fn cursor_filters_and_advances() {
        let logs = vec![
            log("a", "2026-01-01T10:00:00Z", "b1", 1.0, false),
            log("b", "2026-01-02T10:00:00Z", "b1", 1.0, false),
            log("c", "2026-01-02T11:00:00Z", "b1", 1.0, false),
        ];
        let cursor = Some(("2026-01-02T10:00:00Z".to_owned(), vec!["b".to_owned()]));
        let fresh = new_logs(&logs, cursor.as_ref());
        assert_eq!(fresh.len(), 1);
        assert_eq!(fresh[0]["id"], "c");
        let next = advance(&logs, cursor).unwrap();
        assert_eq!(next.0, "2026-01-02T11:00:00Z");
        assert_eq!(next.1, vec!["c"]);
    }
}
