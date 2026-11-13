//! Sends the messages people scheduled: every half minute the host looks for messages whose time
//! has come, posts each one as the person's own message in its chat and lets the bot answer it.
//!
//! The host's own clock is the only clock: a message scheduled on another computer is stored with
//! an absolute time, so turning that computer off does not move it. A message whose time passed
//! while the host was stopped goes out on the first tick after it starts.

use crate::api::apps_runtime::dispatch_bot_turn;
use crate::hub::Hub;
use crate::store::ScheduledMessage;
use std::sync::Arc;
use std::time::Duration;

const TICK: Duration = Duration::from_secs(30);

/// Posts one scheduled message and starts its turn. `false` means it should be tried again.
fn send(hub: &Hub, message: &ScheduledMessage) -> bool {
    let row = match hub.store.bot(&message.bot_id) {
        Ok(Some(row)) => row,
        Ok(None) => {
            tracing::warn!(bot = %message.bot_id, "a scheduled message's chat is gone; dropping it");
            return true;
        }
        Err(error) => {
            tracing::warn!(bot = %message.bot_id, error = format!("{error:#}"), "couldn't read a scheduled message's chat");
            return false;
        }
    };
    if row.deleted || row.config.hidden {
        tracing::info!(bot = %message.bot_id, "a scheduled message's chat is hidden; dropping it");
        return true;
    }
    match dispatch_bot_turn(hub, &message.bot_id, &message.text, "scheduled") {
        Ok(()) => true,
        Err(error) => {
            tracing::warn!(bot = %message.bot_id, error = format!("{error:#}"), "couldn't send a scheduled message");
            false
        }
    }
}

fn send_due(hub: &Hub, now: i64) {
    let due = match hub.store.take_due_scheduled(now) {
        Ok(due) => due,
        Err(error) => {
            tracing::warn!(error = format!("{error:#}"), "couldn't read scheduled messages");
            return;
        }
    };
    let failed: Vec<ScheduledMessage> = due.into_iter().filter(|message| !send(hub, message)).collect();
    if let Err(error) = hub.store.restore_scheduled(failed) {
        tracing::warn!(error = format!("{error:#}"), "couldn't put unsent scheduled messages back");
    }
}

pub async fn run(hub: Arc<Hub>) {
    let mut tick = tokio::time::interval(TICK);
    loop {
        tick.tick().await;
        let hub = hub.clone();
        if let Err(error) = tokio::task::spawn_blocking(move || send_due(&hub, crate::store::now_ms())).await {
            tracing::warn!(error = format!("{error:#}"), "scheduled messages stopped for a tick");
        }
    }
}
