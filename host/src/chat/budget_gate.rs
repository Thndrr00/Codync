//! The daily token budget as a gate on user turns. A message always lands in its chat, so nothing
//! is lost; what does not happen is the agent's turn, which would spend tokens past the limit. The
//! chat says why once a day instead of once a message, and raising the limit lets the next message
//! through. Routines already pause or skip at the limit (`routines::check`).

use crate::agent::bot::NoticeStyle;
use crate::hub::Hub;
use crate::store::{EntryKind, Lane};
use serde_json::json;

/// Whether this chat is over its budget, or the whole fleet is over its own. When it is, the
/// person gets one notice that day and the turn is not started.
pub fn blocks(hub: &Hub, bot: &str) -> bool {
    let now = crate::store::now_ms();
    if !hub.budget.exceeded(bot, now) {
        return false;
    }
    let day = crate::budget::day_of(now);
    let key = format!("budget-gate:{bot}");
    if hub.store.kv_get(&key).and_then(|value| value.parse::<i64>().ok()) == Some(day) {
        return true;
    }
    let text = "Today's token limit is reached, so new turns in this chat wait until tomorrow. \
        Raise the limit in Settings › Usage, or change it for all bots together.";
    let lane = Lane::main(bot);
    hub.add_entry(
        &lane,
        EntryKind::Notice,
        hub.store.max_turn(bot),
        &json!({"text": text, "style": NoticeStyle::Info}),
    );
    if let Err(error) = hub.store.kv_set(&key, &day.to_string()) {
        tracing::warn!(bot, error = format!("{error:#}"), "couldn't remember the budget notice for the day");
    }
    true
}
