//! A short daily digest for a bot: what is due in the task board, what waits for the person's
//! answer, and what the subscribed calendars hold today. Read-only and computed from what the host
//! already has, so it never asks a model to summarize.

use crate::hub::Hub;
use anyhow::{Result, ensure};
use chrono::{Local, TimeZone};
use serde_json::{Value, json};

pub const METHODS: &[&str] = &["dailyDigest"];

pub fn tools() -> Value {
    json!([{
        "name": "daily_digest",
        "description": "Today's brief from this computer: board cards due or overdue, cards in progress, approvals waiting on the user and today's subscribed-calendar events. Read-only.",
        "inputSchema": {"type": "object", "properties": {}},
        "annotations": {"readOnlyHint": true, "idempotentHint": true},
    }])
}

pub async fn call(hub: &Hub, method: &str, _b: &Value) -> Result<Value> {
    ensure!(method == "dailyDigest", "unknown digest method {method}");
    digest(hub).await
}

pub async fn tool_call(hub: &Hub, name: &str) -> Result<Value> {
    ensure!(name == "daily_digest", "unknown digest tool: {name}");
    Ok(json!({"text": digest(hub).await?["text"]}))
}

async fn digest(hub: &Hub) -> Result<Value> {
    let now = Local::now();
    let start = now
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .and_then(|naive| Local.from_local_datetime(&naive).single())
        .map_or(0, |at| at.timestamp_millis());
    let end = start + 86_400_000;
    let tasks = hub.store.board_tasks()?;
    let overdue: Vec<_> =
        tasks.iter().filter(|task| task.status != "done" && task.due_at.is_some_and(|due| due < start)).collect();
    let due_today: Vec<_> = tasks
        .iter()
        .filter(|task| task.status != "done" && task.due_at.is_some_and(|due| (start..end).contains(&due)))
        .collect();
    let doing: Vec<_> = tasks.iter().filter(|task| task.status == "doing").collect();

    let mut approvals = Vec::new();
    for row in hub.store.bots()?.into_iter().filter(|row| !row.deleted && !row.config.is_group()) {
        let cards = hub.store.pending_approvals(&row.config.id)?;
        if let Some(card) = cards.first() {
            approvals.push(json!({
                "bot": row.config.name,
                "title": card.data["title"].as_str().unwrap_or("Approval"),
            }));
        }
    }

    let calendar = crate::calendar_feeds::agenda(hub, &json!({"days": 1, "from": start})).await?;
    let events = calendar["events"].as_array().cloned().unwrap_or_default();

    let mut text = String::new();
    if overdue.is_empty() && due_today.is_empty() && doing.is_empty() {
        text.push_str("The board is clear today.");
    } else {
        text.push_str(&format!(
            "Board: {} overdue, {} due today, {} in progress.",
            overdue.len(),
            due_today.len(),
            doing.len()
        ));
        for task in overdue.iter().take(3) {
            text.push_str(&format!("\n- Overdue: {}", task.title));
        }
        for task in due_today.iter().take(3) {
            text.push_str(&format!("\n- Due today: {}", task.title));
        }
    }
    if approvals.is_empty() {
        text.push_str("\nNothing waits on your answer.");
    } else {
        text.push_str(&format!("\nWaiting on you: {}", approvals.len()));
        for approval in approvals.iter().take(5) {
            text.push_str(&format!(
                "\n- {}: {}",
                approval["bot"].as_str().unwrap_or("A bot"),
                approval["title"].as_str().unwrap_or("Approval")
            ));
        }
    }
    if events.is_empty() {
        text.push_str("\nNo calendar events today.");
    } else {
        text.push_str(&format!("\nCalendar: {}", events.len()));
        for event in events.iter().take(5) {
            text.push_str(&format!("\n- {}", event["title"].as_str().unwrap_or("Untitled")));
        }
    }
    let waiting = crate::waiting::open(&hub.store);
    if !waiting.is_empty() {
        text.push_str(&format!("\nWaiting for: {}", waiting.len()));
        for item in waiting.iter().take(5) {
            text.push_str(&format!("\n- {}", item.title));
        }
    }

    Ok(json!({
        "text": text,
        "overdue": overdue.iter().take(20).map(|task| task.title.clone()).collect::<Vec<_>>(),
        "dueToday": due_today.iter().take(20).map(|task| task.title.clone()).collect::<Vec<_>>(),
        "inProgress": doing.iter().take(20).map(|task| task.title.clone()).collect::<Vec<_>>(),
        "approvals": approvals,
        "events": events,
        "waiting": waiting.iter().take(20).map(|item| item.title.clone()).collect::<Vec<_>>(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tool_is_read_only() {
        let tools = tools();
        assert_eq!(tools[0]["name"], "daily_digest");
        assert_eq!(tools[0]["annotations"]["readOnlyHint"], true);
    }
}
