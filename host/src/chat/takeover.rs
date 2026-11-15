//! Human takeover requests (the `request_handoff` tool): the bot stops at a step only the person can do
//! (a CAPTCHA, a login, a payment) and posts a card. The card tells the person once, and lapses after 30
//! minutes (`expired`), so a forgotten request neither stays "waiting" forever nor looks answerable long
//! after anyone remembers what it was.
//!
//! A request the person picks up is `taken`: the card names who has it and since when, and the bot's agent
//! starts nothing new (a queued message, a routine) until the takeover is done or its own 30 minutes from
//! being taken run out, so work never walks into the middle of the person's step. The screen's own takeover
//! (`screenTakeover`) is still the person's choice from the remote screen and is not touched here.

use crate::hub::{BotStatus, Hold, Hub};
use crate::remote::push::{self, AlertKind};
use crate::store::{Store, now_ms};
use anyhow::Result;
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;

/// How long a request waits for the person.
pub const TTL_MS: i64 = 30 * 60 * 1000;
/// How often waiting requests are checked.
const SWEEP_EVERY: Duration = Duration::from_secs(60);

/// The data of a new request's card at `now`. `notified` is set because the person is told the moment it is posted.
pub fn card(bot_id: &str, reason: &str, url: Option<&str>, now: i64) -> Value {
    json!({
        "text": format!("Human takeover requested: {reason}"),
        "style": "info",
        "author": bot_id,
        "takeoverRequest": {
            "reason": reason,
            "url": url,
            "status": "waiting",
            "requestedAt": now,
            "expiresAt": now + TTL_MS,
            "notified": true,
        }
    })
}

/// The card's data once its time is up at `now`; `None` while it still waits or is being taken (or
/// already ended).
pub fn lapsed(data: &Value, now: i64) -> Option<Value> {
    let request = data.get("takeoverRequest")?;
    if !matches!(request["status"].as_str(), Some("waiting" | "taken"))
        || request["expiresAt"].as_i64().is_none_or(|at| at > now)
    {
        return None;
    }
    let mut next = data.clone();
    next["takeoverRequest"]["status"] = "expired".into();
    let reason = request["reason"].as_str().unwrap_or("a step for you");
    next["text"] = match request["takenBy"].as_str() {
        Some(by) => format!("Takeover request expired (taken by {by}): {reason}").into(),
        None => format!("Takeover request expired: {reason}").into(),
    };
    Some(next)
}

/// The card's data once `who` picks the request up: the agent holds until it is done, and the deadline
/// counts from now, not from the ask. `None` when it isn't waiting (already taken, done or lapsed).
pub fn taken(data: &Value, who: &str, now: i64) -> Option<Value> {
    let request = data.get("takeoverRequest")?;
    if request["status"] != "waiting" {
        return None;
    }
    let who = who.trim();
    let mut next = data.clone();
    next["takeoverRequest"]["status"] = "taken".into();
    next["takeoverRequest"]["takenBy"] = who.into();
    next["takeoverRequest"]["takenAt"] = now.into();
    next["takeoverRequest"]["expiresAt"] = (now + TTL_MS).into();
    let reason = request["reason"].as_str().unwrap_or("a step for you");
    next["text"] = format!("Takeover taken by {who}: {reason}").into();
    Some(next)
}

/// The card's data once the person says they did it, with what they did: the agent goes on from that
/// note, whether they had taken it themselves or not. `None` when the request isn't waiting or taken
/// (already done, or lapsed).
pub fn resolved(data: &Value, note: &str, by: &str, now: i64) -> Option<Value> {
    let request = data.get("takeoverRequest")?;
    if !matches!(request["status"].as_str(), Some("waiting" | "taken")) {
        return None;
    }
    let note = note.trim();
    let mut next = data.clone();
    next["takeoverRequest"]["status"] = "resolved".into();
    next["takeoverRequest"]["note"] = note.into();
    next["takeoverRequest"]["resolvedBy"] = by.trim().into();
    next["takeoverRequest"]["resolvedAt"] = now.into();
    let reason = request["reason"].as_str().unwrap_or("a step for you");
    next["text"] = if note.is_empty() {
        format!("Takeover done: {reason}").into()
    } else {
        format!("Takeover done: {reason} — {note}").into()
    };
    Some(next)
}

/// Every live request's card id and data.
fn waiting(store: &Store) -> Result<Vec<(String, Value)>> {
    store.with_connection(|c| {
        let mut st = c.prepare(
            "SELECT id, data FROM entries WHERE kind = 'notice'
               AND json_extract(data, '$.takeoverRequest.status') IN ('waiting', 'taken')",
        )?;
        let rows = st
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows.into_iter().filter_map(|(id, data)| Some((id, serde_json::from_str(&data).ok()?))).collect())
    })
}

/// The live requests whose time is up at `now`, with their bot and their lapsed data.
pub fn due(store: &Store, now: i64) -> Result<Vec<(String, String, Value)>> {
    Ok(waiting(store)?
        .into_iter()
        .filter_map(|(id, data)| {
            let bot = data["author"].as_str().unwrap_or_default().to_owned();
            Some((id, bot, lapsed(&data, now)?))
        })
        .collect())
}

/// Holds `bot`'s agent while `who` has this request in hand: its next turn waits.
pub fn take(hub: &Hub, bot: &str, entry_id: &str, who: &str, reason: &str) {
    hub.set_runtime(bot, |r| {
        r.hold = Some(Hold { entry_id: entry_id.to_owned() });
        if r.status != BotStatus::Working {
            r.status = BotStatus::NeedsInput;
        }
        r.activity = format!("Taken by {who}: {reason}");
    });
}

/// Lets `bot`'s agent go again once the takeover `entry_id` is over: the turns that waited start now.
pub fn release(hub: &Hub, bot: &str, entry_id: &str) {
    let mut released = false;
    hub.set_runtime(bot, |r| {
        if r.hold.as_ref().is_some_and(|hold| hold.entry_id == entry_id) {
            r.hold = None;
            released = true;
            if r.status == BotStatus::NeedsInput {
                r.status = BotStatus::Idle;
            }
            if r.activity.starts_with("Taken by ") {
                r.activity = String::new();
            }
        }
    });
    if released {
        let _ = hub.send_cmd(bot, crate::agent::bot::Cmd::ResumeQueued);
    }
}

/// Tells the person a bot needs them, once, as the card is posted.
pub fn notify(hub: &Hub, bot_id: &str, reason: &str) {
    let Some(config) = hub.store.bot(bot_id).ok().flatten().map(|row| row.config) else { return };
    push::notify(hub, &config, Some(bot_id), &format!("{} needs you", config.name), reason, AlertKind::NeedsInput);
}

/// Closes the requests nobody answered, for as long as the host runs.
pub async fn sweep_loop(hub: Arc<Hub>) {
    let mut tick = tokio::time::interval(SWEEP_EVERY);
    loop {
        tick.tick().await;
        let found = hub.clone();
        match tokio::task::spawn_blocking(move || due(&found.store, now_ms())).await {
            Ok(Ok(lapsed)) => {
                for (id, bot, data) in lapsed {
                    hub.set_entry(&id, &data);
                    release(&hub, &bot, &id);
                }
            }
            Ok(Err(error)) => {
                tracing::warn!(error = format!("{error:#}"), "couldn't look for lapsed takeover requests");
            }
            Err(error) => tracing::warn!(error = %error, "the takeover sweep stopped"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{EntryKind, Lane};

    #[test]
    fn a_request_lapses_after_thirty_minutes_and_only_once() {
        let data = card("b1", "log in", Some("https://example.com"), 1_000);
        assert_eq!(data["takeoverRequest"]["notified"], true);
        assert!(lapsed(&data, 1_000 + TTL_MS - 1).is_none(), "still waiting just before the limit");
        let next = lapsed(&data, 1_000 + TTL_MS).expect("lapses at the limit");
        assert_eq!(next["takeoverRequest"]["status"], "expired");
        assert!(next["text"].as_str().is_some_and(|t| t.contains("log in")));
        assert!(lapsed(&next, 9_999_999_999).is_none(), "an expired request does not lapse again");
    }

    #[test]
    fn a_done_request_carries_the_note_and_never_lapses() {
        let data = card("b1", "log in", None, 1_000);
        let done = resolved(&data, "  I logged in  ", "iPad", 2_000).expect("done");
        assert_eq!(done["takeoverRequest"]["status"], "resolved");
        assert_eq!(done["takeoverRequest"]["note"], "I logged in");
        assert_eq!(done["takeoverRequest"]["resolvedBy"], "iPad");
        assert!(done["text"].as_str().is_some_and(|t| t.contains("I logged in")));
        assert!(lapsed(&done, i64::MAX).is_none(), "a done request never lapses");
        assert!(resolved(&done, "again", "iPad", 3_000).is_none(), "it is only done once");
    }

    #[test]
    fn a_taken_request_names_who_has_it_and_lapses_from_then() {
        let data = card("b1", "log in", None, 1_000);
        let picked = taken(&data, " Mac ", 2_000).expect("taken");
        assert_eq!(picked["takeoverRequest"]["status"], "taken");
        assert_eq!(picked["takeoverRequest"]["takenBy"], "Mac");
        assert_eq!(picked["takeoverRequest"]["takenAt"], 2_000);
        assert!(picked["text"].as_str().is_some_and(|t| t.contains("Mac") && t.contains("log in")));
        assert!(taken(&picked, "iPad", 3_000).is_none(), "only one place has it");
        assert!(lapsed(&picked, 2_000 + TTL_MS - 1).is_none(), "the deadline moved to the takeover");
        let gone = lapsed(&picked, 2_000 + TTL_MS).expect("lapses");
        assert_eq!(gone["takeoverRequest"]["status"], "expired");
        assert!(gone["text"].as_str().is_some_and(|t| t.contains("Mac")), "it says who had it");
        let done = resolved(&picked, "", "Mac", 2_100).expect("a taken request can be done");
        assert_eq!(done["takeoverRequest"]["resolvedBy"], "Mac");
    }

    #[test]
    fn a_resolved_request_or_one_without_a_deadline_never_lapses() {
        let mut data = card("b1", "x", None, 0);
        data["takeoverRequest"]["status"] = "resolved".into();
        assert!(lapsed(&data, i64::MAX).is_none());
        let old = json!({"takeoverRequest": {"reason": "x", "status": "waiting"}});
        assert!(lapsed(&old, i64::MAX).is_none(), "a card from before deadlines is left alone");
    }

    #[test]
    fn taking_a_request_holds_the_agent_and_done_lets_it_go() {
        let dir = std::env::temp_dir().join(format!("takeover-hold-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let identity = crate::remote::identity::Identity::load_or_create(&dir).unwrap();
        let hub = Hub::new(store, "h".into(), identity, "t".into(), 0);
        take(&hub, "b1", "e1", "Mac", "log in");
        let held = hub.runtime("b1");
        assert_eq!(held.hold.as_ref().map(|h| h.entry_id.as_str()), Some("e1"));
        assert_eq!(held.status, BotStatus::NeedsInput);
        assert_eq!(held.activity, "Taken by Mac: log in");
        release(&hub, "b1", "another card");
        assert!(hub.runtime("b1").hold.is_some(), "another card's release leaves this hold");
        release(&hub, "b1", "e1");
        assert!(hub.runtime("b1").hold.is_none(), "done lets the agent go");
        assert_eq!(hub.runtime("b1").status, BotStatus::Idle);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_sweep_finds_only_the_waiting_requests_that_are_due() {
        let path = std::env::temp_dir().join(format!("takeover-{}.db", uuid::Uuid::new_v4()));
        let store = Store::open(&path).expect("store");
        let config = serde_json::from_value(json!({"id": "b1", "name": "B", "backend": "claude"})).expect("bot");
        store.save_bot(&config).expect("saved");
        let lane = Lane::main("b1");
        store.insert_entry(&lane, EntryKind::Notice, 1, &card("b1", "old", None, 0)).expect("old card");
        store.insert_entry(&lane, EntryKind::Notice, 1, &card("b1", "new", None, TTL_MS)).expect("new card");
        let found = due(&store, TTL_MS + 1).expect("due");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].1, "b1", "the sweep knows whose agent to let go");
        assert_eq!(found[0].2["takeoverRequest"]["reason"], "old");
    }
}
