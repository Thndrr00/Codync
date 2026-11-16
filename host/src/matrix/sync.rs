//! The Matrix long poll: `GET /sync` with the token the last page left, the invites the bot accepts,
//! and the events of bound rooms handed to [`super::ingress`]. The next token is saved only after the
//! events of a page are queued, so a crash replays them and the inbox drops what it already has.

use super::{Config, Hub, client, ingress, load};
use crate::LockExt;
use anyhow::{Context, Result};
use serde_json::Value;
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(30);
const MAX_ROOMS: usize = 200;
const MAX_EVENTS: usize = 60;

pub(super) struct Page {
    pub next_batch: String,
    pub joined: Vec<Joined>,
    pub invites: Vec<Invite>,
}

pub(super) struct Joined {
    pub room: String,
    pub events: Vec<Value>,
}

pub(super) struct Invite {
    pub room: String,
    /// Who invited the bot, when the invite state names them.
    pub inviter: Option<String>,
}

/// One page as the host reads it. Anything malformed is dropped rather than trusted; a page without
/// `next_batch` cannot be resumed and is an error.
pub(super) fn parse(value: &Value) -> Result<Page> {
    let next_batch = value["next_batch"]
        .as_str()
        .filter(|token| !token.is_empty() && token.len() <= 1024)
        .context("Matrix sync sent no next token.")?
        .to_owned();
    let mut joined = Vec::new();
    for (room, body) in value["rooms"]["join"].as_object().into_iter().flatten().take(MAX_ROOMS) {
        let events = body["timeline"]["events"].as_array().cloned().unwrap_or_default();
        joined.push(Joined { room: room.clone(), events: events.into_iter().take(MAX_EVENTS).collect() });
    }
    let mut invites = Vec::new();
    for (room, body) in value["rooms"]["invite"].as_object().into_iter().flatten().take(MAX_ROOMS) {
        let inviter = body["invite_state"]["events"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|event| event["type"] == "m.room.member" && event["content"]["membership"] == "invite")
            .and_then(|event| event["sender"].as_str())
            .map(str::to_owned);
        invites.push(Invite { room: room.clone(), inviter });
    }
    Ok(Page { next_batch, joined, invites })
}

fn set_state(hub: &Hub, state: &str) {
    if let Err(error) = hub.store.kv_set("matrix.state", state) {
        tracing::warn!(error = format!("{error:#}"), "Matrix state was not saved");
    }
}

pub(super) async fn run(hub: std::sync::Arc<Hub>) {
    let mut wait = Duration::from_secs(1);
    loop {
        match serve(&hub).await {
            Ok(()) => wait = Duration::from_secs(1),
            Err(error) => {
                set_state(&hub, &format!("error: {}", format!("{error:#}").chars().take(160).collect::<String>()));
                tracing::warn!(error = format!("{error:#}"), "Matrix lost its sync");
                wait = (wait * 2).min(Duration::from_mins(1));
            }
        }
        tokio::time::sleep(wait).await;
    }
}

/// The long poll until the configuration is removed or replaced.
async fn serve(hub: &std::sync::Arc<Hub>) -> Result<()> {
    loop {
        if hub.store.kv_read(super::SLOT)?.is_none() {
            tokio::time::sleep(Duration::from_secs(5)).await;
            return Ok(());
        }
        crate::market::vault::unlock(hub.clone()).await?;
        let Some(c) = load(&hub.store)? else {
            tokio::time::sleep(Duration::from_secs(5)).await;
            return Ok(());
        };
        if current(hub).as_ref().is_none_or(|now| now.generation != c.generation) {
            return Ok(());
        }
        let key = format!("matrix.since.{}", c.generation);
        let since = hub.store.kv_read(&key)?.unwrap_or_default();
        let first = since.is_empty();
        if first {
            set_state(hub, "starting");
        }
        let value = client::sync(&c.homeserver, &c.access_token, &since, TIMEOUT).await?;
        // A configuration written while the poll was open wins: this page belongs to the old one.
        let Some(now) = load(&hub.store)?.filter(|now| now.generation == c.generation) else { return Ok(()) };
        let page = parse(&value)?;
        for invite in &page.invites {
            join(hub, &now, invite).await;
        }
        // The first page is the baseline: its room history predates the connection, so it is not
        // replayed to the bot. Invites above still count, which is how a fresh bot accepts one.
        if !first && let Err(error) = ingress::accept(hub, &now, &page.joined) {
            tracing::warn!(error = format!("{error:#}"), "Matrix events were not queued");
        }
        if current(hub).as_ref().is_none_or(|latest| latest.generation != now.generation) {
            return Ok(());
        }
        hub.store.kv_set(&key, &page.next_batch)?;
        set_state(hub, "ok");
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

fn current(hub: &Hub) -> Option<Config> {
    let _configuration = hub.store.connector_lock.locked();
    load(&hub.store).ok().flatten()
}

/// Accepts an invite when the owner bound that room, or when an allowed person sent it.
async fn join(hub: &Hub, c: &Config, invite: &Invite) {
    let bound = c.bindings.iter().any(|binding| binding.room_id == invite.room);
    let trusted = invite
        .inviter
        .as_deref()
        .is_some_and(|inviter| c.bindings.iter().any(|binding| binding.allowed_users.iter().any(|u| u == inviter)));
    if !bound && !trusted {
        return;
    }
    let configuration = hub.store.connector_lock.locked();
    let now = load(&hub.store).ok().flatten().filter(|now| now.generation == c.generation);
    drop(configuration);
    let Some(now) = now else { return };
    if let Err(error) = client::join(&now.homeserver, &now.access_token, &invite.room).await {
        tracing::warn!(
            room = super::room_label(&invite.room),
            error = format!("{error:#}"),
            "Matrix invite was not accepted"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_page_keeps_the_token_rooms_and_inviters() {
        let page = parse(&json!({
            "next_batch": "s2_1",
            "rooms": {
                "join": {"!a:example.org": {"timeline": {"events": [{"type": "m.room.message"}, {"type": "m.reaction"}]}}},
                "invite": {"!b:example.org": {"invite_state": {"events": [
                    {"type": "m.room.member", "sender": "@ana:example.org", "content": {"membership": "invite"}}
                ]}}},
            },
        }))
        .expect("a page");
        assert_eq!(page.next_batch, "s2_1");
        assert_eq!(page.joined[0].room, "!a:example.org");
        assert_eq!(page.joined[0].events.len(), 2);
        assert_eq!(page.invites[0].inviter.as_deref(), Some("@ana:example.org"));
    }

    #[test]
    fn a_page_without_a_token_is_not_resumed() {
        assert!(parse(&json!({"rooms": {}})).is_err());
        assert!(parse(&json!({"next_batch": ""})).is_err());
        assert!(parse(&json!({"next_batch": "s"})).expect("an empty page").joined.is_empty());
    }

    #[test]
    fn a_page_is_bounded() {
        let events: Vec<Value> = (0..(MAX_EVENTS + 10)).map(|_| json!({"type": "m.room.message"})).collect();
        let page = parse(&json!({"next_batch": "s", "rooms": {"join": {"!a:x": {"timeline": {"events": events}}}}}))
            .expect("a page");
        assert_eq!(page.joined[0].events.len(), MAX_EVENTS);
    }
}
