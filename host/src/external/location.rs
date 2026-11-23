//! A voice call asking for the user's location (OpenBot nightly v0.77.0, "an agent can now ask for your
//! location"): a card in the chat through the approval-card path, and nothing is sent until the person answers.
//! The same nightly's browser/CAPTCHA fix is the rule here too: a card that stops being waited on stays
//! answerable, and a late answer becomes the person's next message to the agent.

use crate::LockExt;
use crate::hub::Hub;
use crate::store::{BotConfig, EntryKind, Lane};
use serde_json::{Value, json};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use tokio::sync::broadcast;

/// The option settled on a shared location card.
const SHARE: &str = "share";

/// Longest shared place accepted; a longer paste cannot become a tool result or a chat message.
const MAX_SHARED: usize = 500;

/// Deterministic card id: a repeated call for the same purpose reuses the one card. Only a hash of the
/// purpose goes into the id, so the reason is never written into an id or an analytics property.
fn card_key(call: &str, purpose: &str) -> String {
    let mut h = DefaultHasher::new();
    call.hash(&mut h);
    purpose.hash(&mut h);
    format!("location-{call}-{hash:016x}", hash = h.finish())
}

/// The card the person sees. The reason is the bounded text from the tool's `validate_request`. Neither
/// `callTool`'s arguments nor `shareLocation`'s body is an analytics event, so the reason stays local.
fn card(purpose: &str, bot: &str, cwd: &str) -> Value {
    json!({
        "title": purpose,
        "toolKind": "location",
        "command": Value::Null,
        "detail": "",
        "diffs": [],
        "cwd": cwd,
        "options": [],
        "status": "pending",
        "selected": Value::Null,
        "author": bot,
        "locationCard": true,
    })
}

/// Pure: whether an entry is one of our location cards.
fn is_location_card(kind: &str, data: &Value) -> bool {
    kind == EntryKind::Permission.as_str() && data["locationCard"] == true
}

/// Pure: is this text safe to share? Bounded, and control characters other than line breaks are refused.
fn shareable(text: &str) -> bool {
    let text = text.trim();
    !text.is_empty()
        && text.chars().count() <= MAX_SHARED
        && !text.chars().any(|c| c.is_control() && !c.is_whitespace())
}

/// Where a tap on a location card goes. Pure, tested.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Delivery {
    /// A tool call is still waiting: the text is its result.
    ToolResult,
    /// No call is waiting (it timed out or was dropped): the text is the user's next message to the agent.
    NextMessage,
    /// The card is already answered or closed: ignore the tap.
    AlreadyAnswered,
}

/// `expired` is still answerable (OpenBot nightly's CAPTCHA-card fix): without a waiter the text goes on
/// to the agent as the user's next message instead of being lost.
fn delivery(status: Option<&str>, waiting: bool) -> Delivery {
    match status {
        Some("pending") | Some("expired") if waiting => Delivery::ToolResult,
        Some("pending") | Some("expired") => Delivery::NextMessage,
        _ => Delivery::AlreadyAnswered,
    }
}

/// Closing the wait leaves the card answerable. Marking it `expired` is what keeps the desktop's share
/// button alive after the agent moved on (the card's own status, not the waiter, decides that).
struct Expire {
    hub: Arc<Hub>,
    entry_id: String,
}

impl Drop for Expire {
    fn drop(&mut self) {
        if let Some(mut entry) = self.hub.store.entry(&self.entry_id)
            && entry.data["status"] == "pending"
        {
            entry.data["status"] = "expired".into();
            self.hub.set_entry(&entry.id, &entry.data);
        }
    }
}

/// What the tool call holds while it waits. Dropping it (timeout, call ended) expires the card but does not
/// close it to a later answer, nor cancel another retry's wait on the same card.
pub struct Pending {
    pub entry_id: String,
    pub answer: broadcast::Receiver<String>,
    _expire: Expire,
}

/// Creates the one card for `call`'s purpose and returns the answer to wait on. `None` when the same card
/// was already asked for and settled, so a repeated call never adds a second one.
pub fn request(hub: &Arc<Hub>, call: &str, purpose: &str, cfg: &BotConfig) -> Option<Pending> {
    let data = card(purpose, &cfg.id, &cfg.cwd);
    let key = card_key(call, purpose);
    let turn = hub.store.max_turn(&cfg.id) + 1;
    let (entry, added) = hub
        .add_entry_once(&key, &Lane::main(&cfg.id), EntryKind::Permission, turn, &data)
        .inspect_err(|e| tracing::warn!(%e, "couldn't save the location card"))
        .ok()?;
    if !added && entry.data["status"] != "pending" {
        return None;
    }
    // One broadcaster per card: each retry subscribes on its own, so one call ending cannot cancel another.
    let rx = {
        let mut state = hub.external.state.locked();
        let tx = state.location_waiting.entry(entry.id.clone()).or_insert_with(|| {
            let (tx, first) = broadcast::channel(1);
            drop(first);
            tx
        });
        tx.subscribe()
    };
    Some(Pending { entry_id: entry.id.clone(), answer: rx, _expire: Expire { hub: hub.clone(), entry_id: entry.id } })
}

/// Answers the location card. The text reaches the waiting call, or — once nobody waits — goes to the
/// agent as the user's next message, through the same message-posting path a typed message uses.
pub fn share(hub: &Arc<Hub>, entry_id: &str, text: &str) -> bool {
    if !shareable(text) {
        return false;
    }
    let Some(mut entry) = hub.store.entry(entry_id) else { return false };
    if !is_location_card(&entry.kind, &entry.data) {
        return false;
    }
    let waiting = hub.external.state.locked().location_waiting.get(entry_id).is_some_and(|tx| tx.receiver_count() > 0);
    let where_it_goes = delivery(entry.data["status"].as_str(), waiting);
    if where_it_goes == Delivery::AlreadyAnswered {
        return false;
    }
    let broadcaster = hub.external.state.locked().location_waiting.remove(entry_id);
    let text = text.trim().to_owned();
    let bot = entry.data["author"].as_str().unwrap_or(&entry.bot_id).to_owned();

    entry.data["status"] = "answered".into();
    entry.data["selected"] = SHARE.into();
    hub.set_entry(entry_id, &entry.data);

    // If the wait vanished between the check and here, the place still reaches the agent as a message.
    let delivered = broadcaster.is_some_and(|tx| tx.send(text.clone()).is_ok());
    if where_it_goes == Delivery::NextMessage || !delivered {
        let Some(row) = hub.store.bot(&bot).ok().flatten().filter(|r| !r.deleted) else { return true };
        let data = json!({"text": format!("My location: {text}"), "attachments": []});
        if let Err(e) = crate::chat::messages::post(hub, &row.config, data) {
            tracing::warn!(%e, bot = %bot, "couldn't post a late location answer");
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_id_is_stable_for_a_repeated_call_and_distinct_for_another_purpose() {
        let a = card_key("c1", "find restaurants");
        assert_eq!(a, card_key("c1", "find restaurants"), "a retry reuses the card");
        assert_ne!(a, card_key("c1", "find a hotel"), "a different reason is a different card");
        assert_ne!(a, card_key("c2", "find restaurants"), "another call is another card");
        assert!(a.starts_with("location-c1-"));
    }

    #[test]
    fn a_location_card_is_recognised_by_its_kind_and_flag() {
        let c = card("find restaurants", "b1", "/work");
        assert!(is_location_card(EntryKind::Permission.as_str(), &c));
        assert_eq!(c["title"], "find restaurants");
        assert_eq!(c["toolKind"], "location");
        assert_eq!(c["status"], "pending");
        assert!(c["options"].as_array().is_some_and(Vec::is_empty), "there is no fixed option to tap");
        assert!(!is_location_card(EntryKind::User.as_str(), &c));
        let mut other = c;
        other["locationCard"] = Value::Null;
        assert!(!is_location_card(EntryKind::Permission.as_str(), &other));
    }

    #[test]
    fn only_a_bounded_place_is_accepted() {
        assert!(shareable("  1 Main Street  "));
        assert!(!shareable("   "));
        assert!(!shareable(&"x".repeat(MAX_SHARED + 1)));
        assert!(!shareable("here\u{0}there"), "a control character is not an address");
        assert!(shareable("1 Main Street\nSpringfield"), "a line break is fine");
    }

    #[test]
    fn an_expired_card_is_still_answerable_as_the_next_message() {
        assert_eq!(delivery(Some("pending"), true), Delivery::ToolResult);
        assert_eq!(delivery(Some("pending"), false), Delivery::NextMessage);
        assert_eq!(delivery(Some("expired"), false), Delivery::NextMessage, "the CAPTCHA-card fix");
        assert_eq!(delivery(Some("expired"), true), Delivery::ToolResult);
        for status in [Some("answered"), Some("cancelled"), None] {
            assert_eq!(delivery(status, true), Delivery::AlreadyAnswered, "{status:?}");
            assert_eq!(delivery(status, false), Delivery::AlreadyAnswered, "{status:?}");
        }
    }
}
