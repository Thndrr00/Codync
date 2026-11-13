//! The board moving itself: a card in **Doing** that names a routine follows that routine's next
//! finished run (Done when it succeeds, Waiting when it fails), and a card due within a day says so
//! once in its bot's chat. Both are small reminders, not work: nothing starts a turn.

use crate::agent::bot::NoticeStyle;
use crate::hub::Hub;
use crate::routines::Status;
use crate::store::{EntryKind, Lane};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

const TICK: Duration = Duration::from_secs(60);
const DAY_MS: i64 = 86_400_000;

fn date_of(ms: i64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp_millis(ms).unwrap_or_default().format("%Y-%m-%d").to_string()
}

fn step(hub: &Hub, now: i64) {
    let tasks = match hub.store.board_tasks() {
        Ok(tasks) => tasks,
        Err(error) => {
            tracing::warn!(error = format!("{error:#}"), "couldn't read the board");
            return;
        }
    };
    for task in &tasks {
        // A card in Doing with a routine follows that routine's next run.
        if task.status == "doing"
            && let (Some(bot), Some(routine)) = (task.bot_id.as_deref(), task.routine_id.as_deref())
        {
            let runs = hub.routines.runs_for(bot);
            let finished = runs.iter().find(|run| {
                run.routine_id.as_str() == routine
                    && run.created_at > task.updated_at
                    && matches!(run.status, Status::Succeeded | Status::Failed | Status::Interrupted)
            });
            if let Some(run) = finished {
                let status = if run.status == Status::Succeeded { "done" } else { "waiting" };
                if let Err(error) = hub.store.move_board_task(&task.id, status, now) {
                    tracing::warn!(error = format!("{error:#}"), "couldn't move a board card after its routine ran");
                }
            }
        }
        // A card due today or tomorrow (or late by up to a week) says so once a day.
        if task.status != "done"
            && let (Some(due), Some(bot)) = (task.due_at, task.bot_id.as_deref())
            && due <= now + DAY_MS
            && due > now - 7 * DAY_MS
            && hub.store.bot(bot).is_ok_and(|row| row.is_some_and(|row| !row.deleted))
        {
            let key = format!("board-due:{}:{}", task.id, date_of(now));
            if hub.store.kv_get(&key).is_some() {
                continue;
            }
            let text = if due < now {
                format!("Board card overdue: {}", task.title)
            } else {
                format!("Board card due {}: {}", date_of(due), task.title)
            };
            hub.add_entry(
                &Lane::main(bot),
                EntryKind::Notice,
                hub.store.max_turn(bot),
                &json!({"text": text, "style": NoticeStyle::Info}),
            );
            if let Err(error) = hub.store.kv_set(&key, "1") {
                tracing::warn!(error = format!("{error:#}"), "couldn't remember a board reminder for the day");
            }
        }
    }
}

pub async fn run(hub: Arc<Hub>) {
    let mut tick = tokio::time::interval(TICK);
    loop {
        tick.tick().await;
        let hub = hub.clone();
        if let Err(error) = tokio::task::spawn_blocking(move || step(&hub, crate::store::now_ms())).await {
            tracing::warn!(error = format!("{error:#}"), "board automation stopped for a tick");
        }
    }
}
