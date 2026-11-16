//! `send_to_channel`: a bot posts work a person gives one of its channels into that channel, from its own chat.
//!
//! The channel shows the request as the asking bot's message and starts a room turn there: the members the call
//! names answer it, and with none its lead routes it. The caller's chat gets a notice. Under Ask the post waits on
//! the approval card the other team changes use. Work a teammate asked for goes through `message_bot` or an
//! @-mention instead.

use crate::chat::manage::{agent_bot, approve, notice, text};
use crate::hub::Hub;
use crate::store::{BotConfig, EntryKind, Lane};
use anyhow::{Context, Result, anyhow, bail};
use serde_json::{Value, json};
use std::sync::Arc;

pub const TOOL: &str = "send_to_channel";
/// Longest request one call posts.
const MAX_REQUEST_CHARS: usize = 8_000;

pub fn tool() -> Value {
    json!({
        "name": TOOL,
        "description": format!(
            "Post work the user gives a channel into it, from your own chat, and return its id. The channel shows \
             it as your message and its members answer there. Name the members that should get it; with none, its \
             lead routes it. At most {MAX_REQUEST_CHARS} characters. Use it only for work the user gives a channel: \
             a teammate's request goes through message_bot, or an @-mention in a channel you answer in."
        ),
        "inputSchema": {
            "type": "object",
            "properties": {
                "channel": {"type": "string", "description": "The channel's id, or its name."},
                "request": {"type": "string", "description": "The work, self-contained: what to do and what result the channel should give the user."},
                "members": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "The members that should get it, by id or name: the ones whose role fits the work. Leave it out and the channel's lead routes it.",
                },
            },
            "required": ["channel", "request"],
        },
        "annotations": {"readOnlyHint": false, "idempotentHint": false},
    })
}

pub async fn call(hub: &Arc<Hub>, bot: &str, args: &Value) -> Result<Value> {
    let source = agent_bot(hub, bot)?;
    // The tool posts from the bot's own chat; in a channel its own @-mention is how it brings a member in.
    if let Some(lane) = hub.runtime(&source.id).lane
        && lane.chat != source.id
    {
        bail!(
            "send_to_channel works in your own chat: in a channel, @-mention the member you need in your reply, or \
             post the request before you start answering there"
        );
    }
    let key = text(args, "channel")?.context("channel is required: the channel's id or name")?;
    let group = crate::chat::group::tools::find_channel(hub, &source, &key)?;
    if group.archived {
        bail!("{} is archived: ask the user to bring it back in its settings, then send the request", group.name);
    }
    let request = text(args, "request")?.context("request is required: the work for the channel")?;
    if request.is_empty() {
        bail!("request is required: the work for the channel");
    }
    let chars = request.chars().count();
    if chars > MAX_REQUEST_CHARS {
        bail!("request holds at most {MAX_REQUEST_CHARS} characters, and this one has {chars}: shorten it");
    }
    let named = requested(hub, &group, args)?;
    let lead = group.chief_bot_id.as_deref().or_else(|| group.members.first().map(String::as_str));
    let detail = describe(hub, &group, &request, &named, lead);
    approve(hub, &source, format!("Send to {}", group.name), detail).await?;
    let ids: Vec<String> = named.iter().map(|target| target.id.clone()).collect();
    let posted = json!({
        "text": request,
        "author": source.id,
        "final": true,
        "channelRequest": {"from": source.id, "members": ids},
    });
    let lane = Lane::main(&group.id);
    let turn = hub.store.max_turn(&group.id) + 1;
    let entry = hub
        .add_entry(&lane, EntryKind::Agent, turn, &posted)
        .ok_or_else(|| anyhow!("couldn't save the request in the channel"))?;
    // The room answers it as it answers the user's own request: a room turn, with the request's task tree.
    crate::chat::group::start(hub, &group, lane);
    let routed = if ids.is_empty() {
        lead.map(|id| hub.bot_name(id)).map_or_else(|| "its lead".to_owned(), |name| format!("{name}, its lead"))
    } else {
        ids.iter().map(|id| hub.bot_name(id)).collect::<Vec<_>>().join(", ")
    };
    notice(hub, &source.id, &format!("Sent a request to {} ({routed})", group.name));
    Ok(json!({
        "channelId": group.id,
        "channel": {"id": group.id, "name": group.name},
        "members": named.iter().map(|target| json!({"agentId": target.id, "name": target.name})).collect::<Vec<_>>(),
        "leadRouted": ids.is_empty(),
        "entryId": entry.id,
    }))
}

/// The members the call names, by id or name; none when it names nobody (the channel's lead routes it).
fn requested(hub: &Hub, group: &BotConfig, args: &Value) -> Result<Vec<BotConfig>> {
    let list = match &args["members"] {
        Value::Null => return Ok(vec![]),
        Value::Array(list) => list,
        _ => bail!("members must be a list of ids or names"),
    };
    let members = super::members(hub, group);
    let mut out: Vec<BotConfig> = vec![];
    for value in list {
        let Some(raw) = value.as_str() else { bail!("members must be a list of ids or names") };
        let wanted = raw.trim();
        if wanted.is_empty() {
            continue;
        }
        let found =
            members.iter().find(|member| member.id == wanted || hub.bot_name(&member.id).eq_ignore_ascii_case(wanted));
        let Some(found) = found else {
            let known: Vec<String> = members.iter().map(|m| format!("{} ({})", m.name, m.id)).collect();
            bail!(
                "{wanted} isn't a member of {}: name one of {} or leave members out and its lead routes it",
                group.name,
                known.join(", ")
            );
        };
        if !out.iter().any(|member| member.id == found.id) {
            out.push(found.clone());
        }
    }
    Ok(out)
}

/// The approval card's detail: where the request goes and what it asks.
fn describe(hub: &Hub, group: &BotConfig, request: &str, named: &[BotConfig], lead: Option<&str>) -> String {
    let routed = if named.is_empty() {
        lead.map(|id| hub.bot_name(id)).map_or_else(|| "its lead".to_owned(), |name| format!("{name} (lead)"))
    } else {
        named.iter().map(|member| member.name.clone()).collect::<Vec<_>>().join(", ")
    };
    format!("Channel: {}\nMembers asked: {routed}\nRequest:\n{request}", group.name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::identity::Identity;
    use crate::store::Store;

    fn setup(name: &str) -> (Arc<Hub>, std::path::PathBuf, BotConfig) {
        let dir = std::env::temp_dir().join(format!("codync-send-{}-{name}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::open(&dir.join("t.db")).unwrap();
        let caller: BotConfig = serde_json::from_value(json!({
            "id": "5f0c7a52-0000-4000-8000-0000000000c1", "name": "Ann", "backend": "claude", "cwd": "/tmp",
            "permission": "auto",
        }))
        .unwrap();
        let peer: BotConfig = serde_json::from_value(json!({
            "id": "5f0c7a52-0000-4000-8000-0000000000c2", "name": "Ben", "backend": "claude", "cwd": "/tmp",
        }))
        .unwrap();
        let group: BotConfig = serde_json::from_value(json!({
            "id": "5f0c7a52-0000-4000-8000-0000000000c3", "kind": "group", "name": "Ship room",
            "members": [caller.id, peer.id], "chiefBotId": caller.id,
        }))
        .unwrap();
        for cfg in [&caller, &peer, &group] {
            store.save_bot(cfg).unwrap();
        }
        let hub = Hub::new(store, "test".into(), Identity::load_or_create(&dir).unwrap(), "token".into(), 0);
        (hub, dir, caller)
    }

    #[tokio::test]
    async fn the_request_lands_as_the_bot_message_and_its_chat_gets_a_notice() {
        let (hub, dir, caller) = setup("post");
        let reply = call(&hub, &caller.id, &json!({"channel": "Ship room", "request": "Check the release"}))
            .await
            .expect("a channel the bot can see");
        assert_eq!(reply["channelId"], "5f0c7a52-0000-4000-8000-0000000000c3");
        let entry = hub.store.entry(reply["entryId"].as_str().unwrap()).expect("the request");
        assert_eq!(entry.kind, "agent", "the room shows it as the asking bot's message");
        assert_eq!(entry.data["text"], "Check the release");
        assert_eq!(entry.data["author"], caller.id);
        let notice = hub.store.last_of_kind(&Lane::main(&caller.id), EntryKind::Notice, 0).expect("a notice");
        assert!(notice.data["text"].as_str().is_some_and(|t| t.contains("Ship room")));
        crate::chat::group::stop(&hub, "5f0c7a52-0000-4000-8000-0000000000c3");
        hub.shutdown().await;
        std::fs::remove_dir_all(dir).ok();
    }

    #[tokio::test]
    async fn a_channel_the_bot_cannot_see_is_refused_with_what_to_do() {
        let (hub, dir, caller) = setup("refuse");
        let error = call(&hub, &caller.id, &json!({"channel": "Another room", "request": "Look"}))
            .await
            .unwrap_err()
            .to_string();
        assert!(error.contains("no channel") && error.contains("ask the user"), "{error}");
        std::fs::remove_dir_all(dir).ok();
    }
}
