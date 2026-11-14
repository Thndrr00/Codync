use super::{Arc, Binding, Config, Hub, LockExt, Result, SLOT, client, ingress, json, load, render};
use crate::{
    agent::bot::Cmd,
    channels::approval::Approval,
    store::{EntryKind, Lane, matrix::MatrixIncoming},
};
use std::time::Duration;

pub(crate) async fn run(hub: Arc<Hub>) {
    tokio::join!(sending(hub.clone()), watching(hub));
}

/// Prompt and reply queues, durable across restarts.
async fn sending(hub: Arc<Hub>) {
    loop {
        if let Err(error) = tick(&hub).await {
            tracing::warn!(%error, "Matrix channel needs attention");
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

/// Approval cards: posted in the room the bot was asked in, settled once anyone answers.
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
            tracing::warn!(error = format!("{error:#}"), "Matrix approval card was not shown");
        }
    }
}

async fn card(hub: &Arc<Hub>, entry: &serde_json::Value) -> Result<()> {
    if hub.store.kv_read(SLOT)?.is_none() {
        return Ok(());
    }
    let Some(id) = entry["id"].as_str() else { return Ok(()) };
    let Some(thread) = entry["threadId"].as_str() else { return Ok(()) };
    let Some(root) = hub.store.entry(thread) else { return Ok(()) };
    let Some(room) = root.data["matrixRoom"].as_str() else { return Ok(()) };
    let Some(user) = root.data["matrixUser"].as_str() else { return Ok(()) };
    let Some(bot) = entry["botId"].as_str() else { return Ok(()) };
    crate::market::vault::unlock(hub.clone()).await?;
    let Some(c) = load(&hub.store)? else { return Ok(()) };
    let Some(binding) = c.bindings.iter().find(|binding| binding.room_id == room && binding.bot_id == bot) else {
        return Ok(());
    };
    if !binding.allowed_users.iter().any(|allowed| allowed == user) {
        return Ok(());
    }
    if hub.store.bot(bot)?.is_none_or(|row| row.deleted || row.config.is_group()) {
        return Ok(());
    }
    let shown = entry["data"]["matrixCard"].as_str().is_some();
    let settled = entry["data"]["matrixSettled"] == true;
    match (entry["data"]["status"].as_str(), shown, settled) {
        (Some("pending"), false, _) => {
            let Some(model) = Approval::from_entry(entry) else { return Ok(()) };
            let blocks = model.blocks(|_| None);
            let content = render::card(&blocks);
            let event =
                client::send_content(&c.homeserver, &c.access_token, room, &format!("card.{id}"), &content).await?;
            for (index, key) in ingress::keys(model.options.len()).iter().enumerate() {
                if let Err(error) =
                    client::react(&c.homeserver, &c.access_token, room, &event, *key, &format!("card.{id}.r{index}"))
                        .await
                {
                    tracing::warn!(error = format!("{error:#}"), "Matrix approval reaction was not added");
                }
            }
            hub.store.map_matrix_card(&c.generation, room, &event, id)?;
            let mut data = entry["data"].clone();
            data["matrixCard"] = event.into();
            hub.set_entry(id, &data);
        }
        (Some(status), true, false) if status != "pending" => {
            let Some(event) = entry["data"]["matrixCard"].as_str() else { return Ok(()) };
            let selected = entry["data"]["selected"].as_str();
            let text = Approval::from_entry(entry)
                .map_or_else(|| format!("{status}\n(closed)"), |model| model.settled(selected, status));
            client::replace(&c.homeserver, &c.access_token, room, event, &text, &format!("settle.{id}")).await?;
            let mut data = entry["data"].clone();
            data["matrixSettled"] = true.into();
            hub.set_entry(id, &data);
        }
        _ => {}
    }
    Ok(())
}

pub(crate) fn claim_prompt(hub: &Hub, entry: &str) -> Result<bool> {
    let _configuration = hub.store.connector_lock.locked();
    let Some(c) = load(&hub.store)? else { return Ok(false) };
    let Some(e) = hub.store.entry_checked(entry)? else { return Ok(false) };
    if e.data["matrixGeneration"] != c.generation
        || !c.bindings.iter().any(|b| {
            b.bot_id == e.bot_id
                && e.data["matrixRoom"] == b.room_id
                && e.data["matrixUser"].as_str().is_some_and(|user| b.allowed_users.iter().any(|u| u == user))
        })
    {
        return Ok(false);
    }
    hub.store.claim_matrix_prompt(entry, &c.generation)
}

async fn tick(hub: &Arc<Hub>) -> Result<()> {
    if hub.store.kv_read(SLOT)?.is_none() {
        return Ok(());
    }
    crate::market::vault::unlock(hub.clone()).await?;
    let Some(c) = load(&hub.store)? else { return Ok(()) };
    for event in hub.store.pending_matrix()? {
        deliver(hub, &event)?;
    }
    for reply in hub.store.pending_matrix_replies(&c.generation)? {
        {
            let _configuration = hub.store.connector_lock.locked();
            if load(&hub.store)?.is_none_or(|current| current.generation != c.generation) {
                return Ok(());
            }
            if hub.store.bot(&reply.bot)?.is_none_or(|bot| bot.deleted || bot.config.is_group()) {
                continue;
            }
            if !hub.store.claim_matrix_reply(&reply.entry)? {
                continue;
            }
        }
        let sent = send(&c, &reply).await;
        match sent {
            Ok(event) => hub.store.finish_matrix_reply(&reply.entry, "sent", event.as_deref())?,
            Err(error) => {
                tracing::warn!(%error, entry = %reply.entry, "Matrix reply was not sent");
                hub.store.finish_matrix_reply(&reply.entry, "failed", None)?;
            }
        }
    }
    Ok(())
}

/// Sends a reply's text and files; returns the last event id, when there was one.
async fn send(c: &Config, reply: &crate::store::matrix::MatrixOutgoing) -> Result<Option<String>> {
    let mut last = None;
    if !reply.text.is_empty() {
        let event =
            client::send_text(&c.homeserver, &c.access_token, &reply.room, &reply.reply_to, &reply.text, &reply.entry)
                .await?;
        last = Some(event);
    }
    if !reply.files.is_empty() {
        let ids = client::send_files(&c.homeserver, &c.access_token, &reply.room, &reply.files, &reply.entry).await?;
        last = ids.last().cloned().or(last);
    }
    Ok(last)
}

fn deliver(hub: &Hub, event: &MatrixIncoming) -> Result<()> {
    let _configuration = hub.store.connector_lock.locked();
    let Some(c) = load(&hub.store)? else {
        hub.store.reject_matrix(&event.event)?;
        return Ok(());
    };
    let allowed = c.bindings.iter().any(|binding| {
        binding.room_id == event.room
            && binding.bot_id == event.bot
            && binding.allowed_users.iter().any(|user| *user == event.user)
    });
    if c.generation != event.generation || !allowed {
        hub.store.reject_matrix(&event.event)?;
        return Ok(());
    }
    let Some(binding) = binding_for(&c, event) else {
        hub.store.reject_matrix(&event.event)?;
        return Ok(());
    };
    let text = crate::channels::with_notes(
        &binding.memory,
        &crate::channels::notes(&hub.store, "matrix", &event.room),
        &event.text,
    );
    let Some(bot) = hub.store.bot(&event.bot)?.filter(|b| !b.deleted && !b.config.is_group()) else {
        hub.store.reject_matrix(&event.event)?;
        return Ok(());
    };
    let root = ingress::root_id(&c.generation, &event.room);
    hub.add_entry_once(
        &root,
        &Lane::main(&event.bot),
        EntryKind::User,
        hub.store.max_turn(&event.bot),
        &json!({"text":format!("Matrix conversation · {}", event.room),
        "source":"matrix","status":"sent","matrixRoom":event.room,"matrixUser":event.user}),
    )?;
    let lane = Lane::in_thread(&event.bot, &root);
    let (entry, _) = hub.add_entry_once(
        &event.entry,
        &lane,
        EntryKind::User,
        hub.store.max_turn(&event.bot) + 1,
        &json!({"text":text,"status":"queued","source":"matrix",
        "matrixGeneration":event.generation,"matrixRoom":event.room,"matrixUser":event.user}),
    )?;
    if entry.data["status"] != "queued" {
        hub.store.reject_matrix(&event.event)?;
        return Ok(());
    }
    let text = crate::chat::prompt::of_entry(hub, &bot.config, &entry.data);
    hub.send_cmd(&event.bot, Cmd::Send { lane, entry_id: entry.id, text })?;
    Ok(())
}

fn binding_for<'a>(c: &'a Config, event: &MatrixIncoming) -> Option<&'a Binding> {
    c.bindings.iter().find(|binding| {
        binding.room_id == event.room
            && binding.bot_id == event.bot
            && binding.allowed_users.iter().any(|user| *user == event.user)
    })
}
