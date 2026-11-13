//! What a bot is doing, as a board. The columns are views of its routine runs, not data: a card
//! moves on its own when the host changes the run's state, and a run waiting on an approval card
//! shows in "blocked" until the person answers.
//!
//! Idea from agentes.chat's board that moves itself (`packages/engine`, AGPL-3.0-only): the
//! behavior, not the code.

use crate::hub::Hub;
use crate::routines::Status;
use crate::store::EntryKind;
use anyhow::{Context, Result, ensure};
use serde_json::{Map, Value, json};

/// `board {botId}` → `{columns: {queued, running, blocked, done, failed}}`, each a list of cards.
pub fn board(hub: &Hub, b: &Value) -> Result<Value> {
    let bot = b["botId"].as_str().context("`botId` is required")?;
    ensure!(hub.store.bot(bot)?.is_some_and(|row| !row.deleted), "unknown bot");
    let blocked = awaiting_approval(hub, bot);
    let mut columns: [(&str, Vec<Value>); 5] =
        [("queued", vec![]), ("running", vec![]), ("blocked", vec![]), ("done", vec![]), ("failed", vec![])];
    for run in hub.routines.runs_for(bot) {
        let (column, step) = match run.status {
            Status::Pending => (0, None),
            Status::Starting | Status::Running | Status::Recovering => {
                (if blocked { 2 } else { 1 }, run.detail.clone())
            }
            Status::Succeeded => (3, None),
            Status::Failed | Status::Interrupted | Status::Cancelled => (4, run.detail.clone()),
        };
        columns[column].1.push(json!({
            "id": run.id,
            "routineId": run.routine_id,
            "routine": hub.routines.name(bot, &run.routine_id),
            "status": run.status,
            "step": step,
            "since": run.started_at.unwrap_or(run.created_at),
        }));
    }
    let mut out = Map::new();
    for (name, cards) in columns {
        out.insert(name.to_owned(), Value::Array(cards));
    }
    Ok(json!({"columns": out}))
}

/// Whether a card is waiting for the person: the run that asked for it belongs to them until it is
/// answered, so its card doesn't look like it is still working.
fn awaiting_approval(hub: &Hub, bot: &str) -> bool {
    hub.store
        .history(bot, i64::MAX, 20)
        .unwrap_or_default()
        .iter()
        .any(|e| e.kind == EntryKind::Permission.as_str() && e.data["status"] == "pending")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::identity::Identity;
    use crate::store::Store;
    use std::sync::Arc;

    fn hub() -> (Arc<Hub>, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("codync-board-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let identity = Identity::load_or_create(&dir).unwrap();
        (Hub::new(store, "h".into(), identity, "t".into(), 0), dir)
    }

    #[test]
    fn a_queued_run_is_a_card_with_the_name_of_its_routine() {
        let (hub, dir) = hub();
        let config = serde_json::from_value(json!({"id": "b1", "name": "B", "backend": "claude"})).unwrap();
        hub.store.save_bot(&config).unwrap();
        let saved = hub
            .routines
            .save(&hub, "b1", &json!({"name": "Morning report", "instruction": "report", "triggers": [{"type": "interval", "seconds": 3600}]}))
            .unwrap();
        let id = saved["id"].as_str().unwrap();
        hub.routines.enqueue(&hub.store, "b1", id, json!({"source": "schedule"}), None, false).unwrap();

        let board = board(&hub, &json!({"botId": "b1"})).unwrap();
        let queued = board["columns"]["queued"].as_array().unwrap();
        assert_eq!(queued.len(), 1);
        assert_eq!(queued[0]["routine"], "Morning report");
        assert!(board["columns"]["running"].as_array().unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
