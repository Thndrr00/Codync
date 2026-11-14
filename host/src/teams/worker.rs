use super::{Arc, Config, Hub, LockExt, Result, activity, client, ingress, load};
use crate::{
    agent::bot::Cmd,
    channels::{
        approval::{self, Approval},
        render,
    },
    store::{EntryKind, Lane, teams::TeamsIncoming},
};
use serde_json::{Value, json};
use std::time::Duration;

pub(super) async fn run(hub: Arc<Hub>) {
    tokio::join!(sending(hub.clone()), watching(hub));
}

async fn sending(hub: Arc<Hub>) {
    loop {
        if let Err(error) = tick(&hub).await {
            tracing::warn!(%error, "Teams channel needs attention");
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

async fn watching(hub: Arc<Hub>) {
    let mut events = hub.events.subscribe();
    loop {
        let event = match events.recv().await {
            Ok(event) => event,
            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
            Err(tokio::sync::broadcast::error::RecvError::Closed) => return,
        };
        if event["type"] != "entry" || event["entry"]["kind"] != "permission" {
            continue;
        }
        if let Err(error) = card(&hub, &event["entry"]).await {
            tracing::warn!(error = format!("{error:#}"), "Teams approval card was not shown");
        }
    }
}

async fn card(hub: &Arc<Hub>, entry: &Value) -> Result<()> {
    if hub.store.kv_read(super::SLOT)?.is_none() {
        return Ok(());
    }
    let Some(id) = entry["id"].as_str() else { return Ok(()) };
    let Some(thread) = entry["threadId"].as_str() else { return Ok(()) };
    let Some(root) = hub.store.entry(thread) else { return Ok(()) };
    let Some(conversation) = root.data["teamsConversation"].as_str() else { return Ok(()) };
    let Some(bot) = entry["botId"].as_str() else { return Ok(()) };
    crate::market::vault::unlock(hub.clone()).await?;
    let Some(c) = load(&hub.store)? else { return Ok(()) };
    if c.bot_id != bot || !activity_allows_root(&c, &root) {
        return Ok(());
    }
    let Some(reference) = hub.store.teams_ref(&c.generation, conversation)? else { return Ok(()) };
    let shown = entry["data"]["teamsCard"].as_str().is_some();
    let settled = entry["data"]["teamsSettled"] == true;
    match (entry["data"]["status"].as_str(), shown, settled) {
        (Some("pending"), false, _) => {
            let Some(model) = Approval::from_entry(entry) else { return Ok(()) };
            let blocks = model.blocks(|option| approval::action(entry, option));
            let card = render::teams(&blocks);
            let activity = client::send_card(&c, &reference.service_url, conversation, &card).await?;
            if activity.is_empty() {
                return Ok(());
            }
            hub.store.map_teams_card(&c.generation, conversation, &activity, id)?;
            let mut data = entry["data"].clone();
            data["teamsCard"] = activity.into();
            hub.set_entry(id, &data);
        }
        (Some(status), true, false) if status != "pending" => {
            let selected = entry["data"]["selected"].as_str();
            let text = Approval::from_entry(entry)
                .map_or_else(|| format!("Approval {status}\n(closed)"), |model| model.settled(selected, status));
            let card = render::teams(&[crate::channels::render::Block::Text(text)]);
            let shown = entry["data"]["teamsCard"].as_str().unwrap_or_default();
            client::update_card(&c, &reference.service_url, conversation, shown, &card).await?;
            let mut data = entry["data"].clone();
            data["teamsSettled"] = true.into();
            hub.set_entry(id, &data);
        }
        _ => {}
    }
    Ok(())
}

fn activity_allows_root(c: &Config, root: &crate::store::Entry) -> bool {
    root.data["teamsGeneration"] == c.generation
        && (c.allowed_users.is_empty()
            || root.data["teamsUser"].as_str().is_some_and(|user| {
                c.allowed_users.iter().any(|allowed| allowed.as_str() == "*" || allowed.as_str() == user)
            }))
}

pub(crate) fn claim_prompt(hub: &Hub, entry: &str) -> Result<bool> {
    let _configuration = hub.store.connector_lock.locked();
    let Some(c) = load(&hub.store)? else { return Ok(false) };
    let Some(e) = hub.store.entry_checked(entry)? else { return Ok(false) };
    let user = e.data["teamsUser"].as_str().unwrap_or_default();
    let allowed = c.allowed_users.is_empty()
        || c.allowed_users.iter().any(|allowed| allowed.as_str() == "*" || allowed.as_str() == user);
    if e.data["teamsGeneration"] != c.generation || e.bot_id != c.bot_id || !allowed {
        return Ok(false);
    }
    hub.store.claim_teams_prompt(entry, &c.generation)
}

async fn tick(hub: &Arc<Hub>) -> Result<()> {
    if hub.store.kv_read(super::SLOT)?.is_none() {
        return Ok(());
    }
    crate::market::vault::unlock(hub.clone()).await?;
    let Some(c) = load(&hub.store)? else { return Ok(()) };
    for event in hub.store.pending_teams()? {
        deliver(hub, &c, &event)?;
    }
    for reply in hub.store.pending_teams_replies(&c.generation)? {
        {
            let _configuration = hub.store.connector_lock.locked();
            if load(&hub.store)?.is_none_or(|current| current.generation != c.generation) {
                return Ok(());
            }
            if hub.store.bot(&reply.bot)?.is_none_or(|bot| bot.deleted || bot.config.is_group()) {
                continue;
            }
            if !hub.store.claim_teams_reply(&reply.entry)? {
                continue;
            }
        }
        let sent = send(hub, &c, &reply).await;
        match sent {
            Ok(activity) => hub.store.finish_teams_reply(&reply.entry, "sent", activity.as_deref())?,
            Err(error) => {
                tracing::warn!(%error, entry = %reply.entry, "Teams reply was not sent");
                hub.store.finish_teams_reply(&reply.entry, "failed", None)?;
            }
        }
    }
    Ok(())
}

async fn send(hub: &Hub, c: &Config, reply: &crate::store::teams::TeamsOutgoing) -> Result<Option<String>> {
    let Some(reference) = hub.store.teams_ref(&c.generation, &reply.conversation)? else {
        anyhow::bail!("the Teams conversation reference is gone");
    };
    let mut text = reply.text.clone();
    if let Some(entry) = hub.store.entry(&reply.entry) {
        let names: Vec<&str> =
            entry.data["files"].as_array().into_iter().flatten().filter_map(|file| file["name"].as_str()).collect();
        if !names.is_empty() {
            text.push_str(&format!("\n\n[Files are not sent to Teams yet: {}]", names.join(", ")));
        }
    }
    let activity = client::send_text(c, &reference.service_url, &reply.conversation, &reply.reply_to, &text).await?;
    Ok((!activity.is_empty()).then_some(activity))
}

fn deliver(hub: &Hub, c: &Config, event: &TeamsIncoming) -> Result<()> {
    let _configuration = hub.store.connector_lock.locked();
    let allowed = c.generation == event.generation
        && c.bot_id == event.bot
        && (c.allowed_users.is_empty()
            || c.allowed_users.iter().any(|user| user.as_str() == "*" || user.as_str() == event.user));
    if !allowed {
        hub.store.reject_teams(&event.activity)?;
        return Ok(());
    }
    let text =
        crate::channels::with_notes(&c.memory, &crate::channels::notes(&hub.store, "teams", "teams"), &event.text);
    let Some(bot) = hub.store.bot(&event.bot)?.filter(|b| !b.deleted && !b.config.is_group()) else {
        hub.store.reject_teams(&event.activity)?;
        return Ok(());
    };
    let root = ingress::root_id(&c.generation, &event.conversation);
    hub.add_entry_once(
        &root,
        &Lane::main(&event.bot),
        EntryKind::User,
        hub.store.max_turn(&event.bot),
        &json!({"text":format!("Teams conversation \u{00b7} {}", event.conversation.chars().take(24).collect::<String>()),
        "source":"teams","status":"sent","teamsConversation":event.conversation,"teamsUser":event.user}),
    )?;
    let lane = Lane::in_thread(&event.bot, &root);
    let (entry, _) = hub.add_entry_once(
        &event.entry,
        &lane,
        EntryKind::User,
        hub.store.max_turn(&event.bot) + 1,
        &json!({"text":text,"status":"queued","source":"teams",
        "teamsGeneration":event.generation,"teamsConversation":event.conversation,"teamsUser":event.user}),
    )?;
    if entry.data["status"] != "queued" {
        hub.store.reject_teams(&event.activity)?;
        return Ok(());
    }
    let text = crate::chat::prompt::of_entry(hub, &bot.config, &entry.data);
    hub.send_cmd(&event.bot, Cmd::Send { lane, entry_id: entry.id, text })?;
    Ok(())
}
