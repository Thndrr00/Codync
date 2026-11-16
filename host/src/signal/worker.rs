use super::{Arc, Binding, Config, Hub, LockExt, Result, SLOT, client, ingress, load, render};
use crate::{
    agent::bot::Cmd,
    channels::approval::Approval,
    store::{EntryKind, Lane, signal::SignalIncoming},
};
use std::time::Duration;

pub(crate) async fn run(hub: Arc<Hub>) {
    tokio::join!(sending(hub.clone()), watching(hub));
}

/// Prompt and reply queues, durable across restarts.
async fn sending(hub: Arc<Hub>) {
    loop {
        if let Err(error) = tick(&hub).await {
            tracing::warn!(%error, "Signal channel needs attention");
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

/// Approval cards: posted in the chat the bot was asked in, answered by a quoted number.
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
            tracing::warn!(error = format!("{error:#}"), "Signal approval card was not shown");
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
    let Some(chat) = root.data["signalChat"].as_str() else { return Ok(()) };
    let Some(user) = root.data["signalUser"].as_str() else { return Ok(()) };
    let Some(bot) = entry["botId"].as_str() else { return Ok(()) };
    crate::market::vault::unlock(hub.clone()).await?;
    let Some(c) = load(&hub.store)? else { return Ok(()) };
    let Some(binding) = c.bindings.iter().find(|binding| binding.chat == chat && binding.bot_id == bot) else {
        return Ok(());
    };
    if !binding.allowed_users.iter().any(|allowed| allowed.as_str() == user) {
        return Ok(());
    }
    if hub.store.bot(bot)?.is_none_or(|row| row.deleted || row.config.is_group()) {
        return Ok(());
    }
    let shown = entry["data"]["signalCard"].as_str().is_some();
    let settled = entry["data"]["signalSettled"] == true;
    match (entry["data"]["status"].as_str(), shown, settled) {
        (Some("pending"), false, _) => {
            let Some(model) = Approval::from_entry(entry) else { return Ok(()) };
            let text = render::approval_card(&model);
            let message = client::send_text(&c.daemon, &c.number, chat, &text, &[]).await?;
            hub.store.map_signal_card(&c.generation, chat, &message, id)?;
            let mut data = entry["data"].clone();
            data["signalCard"] = message.into();
            hub.set_entry(id, &data);
        }
        (Some(status), true, false) if status != "pending" => {
            let text = render::settled(entry);
            client::send_text(&c.daemon, &c.number, chat, &text, &[]).await?;
            let mut data = entry["data"].clone();
            data["signalSettled"] = true.into();
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
    if e.data["signalGeneration"] != c.generation
        || !c.bindings.iter().any(|b| {
            b.bot_id == e.bot_id
                && e.data["signalChat"] == b.chat
                && e.data["signalUser"].as_str().is_some_and(|user| b.allowed_users.iter().any(|u| u == user))
        })
    {
        return Ok(false);
    }
    hub.store.claim_signal_prompt(entry, &c.generation)
}

async fn tick(hub: &Arc<Hub>) -> Result<()> {
    if hub.store.kv_read(SLOT)?.is_none() {
        return Ok(());
    }
    crate::market::vault::unlock(hub.clone()).await?;
    let Some(c) = load(&hub.store)? else { return Ok(()) };
    for event in hub.store.pending_signal()? {
        deliver(hub, &event)?;
    }
    for reply in hub.store.pending_signal_replies(&c.generation)? {
        {
            let _configuration = hub.store.connector_lock.locked();
            if load(&hub.store)?.is_none_or(|current| current.generation != c.generation) {
                return Ok(());
            }
            if hub.store.bot(&reply.bot)?.is_none_or(|bot| bot.deleted || bot.config.is_group()) {
                continue;
            }
            if !hub.store.claim_signal_reply(&reply.entry)? {
                continue;
            }
        }
        let sent = send(&c, &reply).await;
        match sent {
            Ok(message) => hub.store.finish_signal_reply(&reply.entry, "sent", message.as_deref())?,
            Err(error) => {
                tracing::warn!(%error, entry = %reply.entry, "Signal reply was not sent");
                hub.store.finish_signal_reply(&reply.entry, "failed", None)?;
            }
        }
    }
    Ok(())
}

/// Sends a reply's files and text. signal-cli carries the files with the message, so they go in
/// one request, after which the text is the message's own body.
async fn send(c: &Config, reply: &crate::store::signal::SignalOutgoing) -> Result<Option<String>> {
    let mut paths = Vec::new();
    for file in reply.files.iter().take(10) {
        let Some(path) = file["path"].as_str() else { continue };
        let Ok(metadata) = tokio::fs::metadata(path).await else { continue };
        if metadata.len() > 25 * 1024 * 1024 {
            tracing::warn!(path, "Signal file is larger than 25 MiB and was not sent");
            continue;
        }
        paths.push(path.to_owned());
    }
    if reply.text.trim().is_empty() && paths.is_empty() {
        return Ok(None);
    }
    let text = if reply.text.trim().is_empty() { "Sent a file.".to_owned() } else { reply.text.clone() };
    client::send_text(&c.daemon, &c.number, &reply.chat, &text, &paths).await.map(Some)
}

fn deliver(hub: &Hub, event: &SignalIncoming) -> Result<()> {
    let _configuration = hub.store.connector_lock.locked();
    let Some(c) = load(&hub.store)? else {
        hub.store.reject_signal(&event.message)?;
        return Ok(());
    };
    let allowed = c.bindings.iter().any(|binding| {
        binding.chat == event.chat
            && binding.bot_id == event.bot
            && binding.allowed_users.iter().any(|user| *user == event.user)
    });
    if c.generation != event.generation || !allowed {
        hub.store.reject_signal(&event.message)?;
        return Ok(());
    }
    let Some(binding) = binding_for(&c, event) else {
        hub.store.reject_signal(&event.message)?;
        return Ok(());
    };
    let text = crate::channels::with_notes(
        &binding.memory,
        &crate::channels::notes(&hub.store, "signal", &event.chat),
        &event.text,
    );
    let Some(bot) = hub.store.bot(&event.bot)?.filter(|b| !b.deleted && !b.config.is_group()) else {
        hub.store.reject_signal(&event.message)?;
        return Ok(());
    };
    let root = ingress::root_id(&c.generation, &event.chat);
    hub.add_entry_once(
        &root,
        &Lane::main(&event.bot),
        EntryKind::User,
        hub.store.max_turn(&event.bot),
        &serde_json::json!({"text":format!("Signal conversation \u{00b7} {}", ingress::label(&event.chat)),
        "source":"signal","status":"sent","signalChat":event.chat,"signalUser":event.user}),
    )?;
    let lane = Lane::in_thread(&event.bot, &root);
    let (entry, _) = hub.add_entry_once(
        &event.entry,
        &lane,
        EntryKind::User,
        hub.store.max_turn(&event.bot) + 1,
        &serde_json::json!({"text":text,"status":"queued","source":"signal","files":event.files.clone(),
        "signalGeneration":event.generation,"signalChat":event.chat,"signalUser":event.user}),
    )?;
    if entry.data["status"] != "queued" {
        hub.store.reject_signal(&event.message)?;
        return Ok(());
    }
    let text = crate::chat::prompt::of_entry(hub, &bot.config, &entry.data);
    hub.send_cmd(&event.bot, Cmd::Send { lane, entry_id: entry.id, text })?;
    Ok(())
}

fn binding_for<'a>(c: &'a Config, event: &SignalIncoming) -> Option<&'a Binding> {
    c.bindings.iter().find(|binding| {
        binding.chat == event.chat
            && binding.bot_id == event.bot
            && binding.allowed_users.iter().any(|user| *user == event.user)
    })
}
