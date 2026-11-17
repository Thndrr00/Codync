//! The board of what every bot is doing right now: `tasksAgenda`.
//!
//! The apps can only see one bot's state at a time. This is the host's answer for all of them at once — the first
//! half of agentes.chat's ADR 0128 (their task manager), rebuilt here on `Hub::runtime`. Upcoming routine
//! occurrences and run history come next, and the payload already has their slots so a client can be written once.

use crate::hub::{BotStatus, Hub};
use anyhow::Result;
use serde_json::{Value, json};
use std::sync::Arc;

/// The methods this group answers.
pub const METHODS: [&str; 1] = ["tasksAgenda"];

pub async fn call(hub: &Arc<Hub>, method: &str, b: &Value) -> Result<Value> {
    match method {
        "tasksAgenda" => agenda(hub, b),
        other => anyhow::bail!("unknown agenda method {other}"),
    }
}

/// Every bot's current turn, split into what is running and what waits on a person, inside `[from, to]`.
fn agenda(hub: &Hub, b: &Value) -> Result<Value> {
    let from = b["from"].as_i64().unwrap_or(0);
    let to = b["to"].as_i64().unwrap_or(i64::MAX);
    let (mut running, mut attention) = (Vec::new(), Vec::new());
    for row in hub.store.bots()?.into_iter().filter(|bot| !bot.deleted && !bot.config.is_group()) {
        let runtime = hub.runtime(&row.config.id);
        // A bot that isn't in a turn has no start time; nothing to show.
        let Some(started) = runtime.started_at.filter(|at| *at >= from && *at <= to) else { continue };
        let item = json!({
            "botId": row.config.id,
            "name": row.config.name,
            "status": name(runtime.status),
            "activity": runtime.activity,
            "startedAt": started,
            // The person's message that started it; null for a turn a routine, a room or another bot started.
            "askedBy": runtime.message,
            // Who may stop it: this computer's owner, always.
            "canStop": true,
        });
        match runtime.status {
            BotStatus::Working => running.push(item),
            BotStatus::NeedsInput | BotStatus::Error => attention.push(item),
            BotStatus::Idle => {}
        }
    }
    Ok(json!({
        "running": running,
        "attention": attention,
        // The next two land in their own change; the shape is here so clients can be built once.
        "upcoming": [],
        "history": [],
    }))
}

/// The status in the words the apps already use for a bot.
fn name(status: BotStatus) -> &'static str {
    match status {
        BotStatus::Idle => "idle",
        BotStatus::Working => "working",
        BotStatus::NeedsInput => "needsInput",
        BotStatus::Error => "error",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_status_has_a_word() {
        assert_eq!(name(BotStatus::Working), "working");
        assert_eq!(name(BotStatus::NeedsInput), "needsInput");
        assert_eq!(name(BotStatus::Error), "error");
        assert_eq!(name(BotStatus::Idle), "idle");
    }
}
