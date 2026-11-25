//! The receivers: one poller and one streamer per configured Telegram bot, kept in step with the configuration.
//!
//! The poller reads updates (`getUpdates`), pairs people, answers commands and hands a message to the assigned bot as
//! if it had been typed in a client. The streamer watches the host's events and shows the reply, and the approvals
//! the bot asks, in the Telegram chat the message came from.

use super::client::{self, Endpoint};
use super::{Arc, Hub, SLOT, TgBot, Value, commands, json, load, pairing};
use crate::api::devices::Caller;
use sha2::{Digest as _, Sha256};
use std::collections::HashMap;
use std::time::Instant;
use tokio::sync::{Mutex, broadcast::error::RecvError};
use tokio::task::JoinHandle;

/// Aborts its tasks when dropped, so a stopped host loop leaves nothing polling.
struct Tasks(Vec<JoinHandle<()>>, super::inbox::Inbox);

static CONFIGURATION: tokio::sync::Notify = tokio::sync::Notify::const_new();

pub(super) fn configuration_changed() {
    CONFIGURATION.notify_waiters();
}

impl Drop for Tasks {
    fn drop(&mut self) {
        self.0.iter().for_each(JoinHandle::abort);
    }
}

/// Where the reply to the last message of a Telegram chat goes.
pub(super) struct Route {
    chat: i64,
    /// Entries of this turn and later belong to it.
    turn: i64,
    live: Option<Live>,
    /// The "Working on it…" line, while it is still on Telegram (OpenBot 15568).
    busy: Option<i64>,
    /// Approval cards shown in the chat, by entry id → Telegram message id.
    cards: HashMap<String, i64>,
}

/// Why Telegram's "Working on it…" line is being taken down.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BusyReason {
    /// The message was an internal teammate result, not a request for this bot.
    NotForBot,
    /// The final answer carries only files, so there is no text to leave the line above.
    FilesOnly,
}

/// Whether the "Working on it…" line should be deleted. OpenBot 15568 removes it when the message
/// wasn't for the bot, or when the answer is files-only. Pure so the decision has a test without a
/// bot or a socket; the caller does the transport delete.
fn drop_busy_post(reason: BusyReason, has_text: bool) -> bool {
    match reason {
        BusyReason::NotForBot => true,
        BusyReason::FilesOnly => !has_text,
    }
}

/// The Telegram message the current reply is being written into.
struct Live {
    entry: String,
    message: i64,
    sent: String,
    at: Instant,
}

pub(super) type Shared = Arc<Mutex<Option<Route>>>;

pub(super) async fn run(hub: Arc<Hub>, ep: Endpoint) {
    let mut running: HashMap<String, (String, Tasks)> = HashMap::new();
    loop {
        let refresh = CONFIGURATION.notified();
        tokio::pin!(refresh);
        refresh.as_mut().enable();
        if let Err(error) = reconcile(&hub, &ep, &mut running).await {
            tracing::warn!(error = format!("{error:#}"), "Telegram channels need attention");
        }
        tokio::select! {
            () = refresh => {},
            () = tokio::time::sleep(ep.reconcile) => {},
        }
    }
}

pub(super) fn fingerprint(bot: &TgBot) -> String {
    let mut allowed = bot.allowed.clone();
    allowed.sort_unstable();
    json!([bot.token, bot.bot_id, bot.username, allowed, bot.generation, bot.memory]).to_string()
}

async fn reconcile(
    hub: &Arc<Hub>,
    ep: &Endpoint,
    running: &mut HashMap<String, (String, Tasks)>,
) -> anyhow::Result<()> {
    let wanted = if hub.store.kv_read(SLOT)?.is_none() {
        Vec::new()
    } else {
        crate::market::vault::unlock(hub.clone()).await?;
        load(&hub.store)?.bots
    };
    running.retain(|name, (print, tasks)| {
        wanted.iter().any(|bot| &bot.name == name)
            && (tasks.1.handling_control()
                || (!tasks.0.iter().any(JoinHandle::is_finished)
                    && wanted.iter().any(|bot| &bot.name == name && &fingerprint(bot) == print)))
    });
    for bot in wanted {
        if running.contains_key(&bot.name) {
            continue;
        }
        let shared = Shared::default();
        let inbox = super::inbox::Inbox::open(hub, &bot.name, fingerprint(&bot))?;
        let monitor = inbox.clone();
        let tasks = Tasks(
            vec![
                tokio::spawn(poll(hub.clone(), ep.clone(), bot.name.clone(), shared.clone(), inbox.clone())),
                tokio::spawn(inbox.run(hub.clone(), ep.clone(), bot.name.clone(), shared.clone())),
                tokio::spawn(stream(hub.clone(), ep.clone(), bot.name.clone(), shared)),
            ],
            monitor,
        );
        running.insert(bot.name.clone(), (fingerprint(&bot), tasks));
    }
    Ok(())
}

pub(super) fn current(hub: &Hub, name: &str) -> Option<TgBot> {
    load(&hub.store).ok()?.bots.into_iter().find(|b| b.name == name)
}

fn set(hub: &Hub, key: &str, name: &str, value: &str) {
    if let Err(error) = hub.store.kv_set(&format!("telegram.{key}.{name}"), value) {
        tracing::warn!(error = format!("{error:#}"), "Telegram state was not saved");
    }
}

pub(super) fn checkpoint(hub: &Hub, name: &str, token: &str, update: &Value) -> anyhow::Result<()> {
    let _guard = crate::LockExt::locked(&hub.store.connector_lock);
    let current = load(&hub.store)?.bots.into_iter().find(|bot| bot.name == name);
    if current.is_none_or(|bot| bot.token != token) {
        return Ok(());
    }
    let Some(id) = update["update_id"].as_i64() else { return Ok(()) };
    let next = id.checked_add(1).ok_or_else(|| anyhow::anyhow!("Invalid Telegram update id"))?;
    let key = format!("telegram.offset.{name}");
    let previous = hub.store.kv_read(&key)?.as_deref().map(str::parse::<i64>).transpose()?.unwrap_or(0);
    hub.store.kv_set(&key, &previous.max(next).to_string())
}

// MARK: receiving

async fn poll(hub: Arc<Hub>, ep: Endpoint, name: String, shared: Shared, inbox: super::inbox::Inbox) {
    loop {
        let Some(bot) = current(&hub, &name) else { return };
        let offset = hub
            .store
            .kv_read(&format!("telegram.offset.{name}"))
            .ok()
            .flatten()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        match client::updates(&ep, &bot.token, offset).await {
            Ok(list) => {
                set(&hub, "state", &name, "ok");
                for update in list {
                    if let Err(error) = inbox.accept(&hub, &ep, &name, &shared, &update).await {
                        tracing::warn!(telegram = name, %error, "Telegram input was not saved; polling will retry");
                        break;
                    }
                    if let Err(error) = checkpoint(&hub, &name, &bot.token, &update) {
                        tracing::warn!(telegram = name, %error, "Telegram acknowledgement will retry");
                        break;
                    }
                }
            }
            Err(error) => {
                let text = format!("{error:#}");
                tracing::warn!(error = text, telegram = name, "Telegram could not be reached");
                set(&hub, "state", &name, &format!("error: {}", text.chars().take(160).collect::<String>()));
                tokio::time::sleep(ep.retry).await;
            }
        }
    }
}

pub(super) async fn say(ep: &Endpoint, bot: &TgBot, chat: i64, text: &str) {
    if let Err(error) = client::message(ep, &bot.token, chat, text, None).await {
        tracing::warn!(error = format!("{error:#}"), telegram = bot.name, "Telegram message was not sent");
    }
}

/// OpenBot 0.70.0: a Reply brings the message it answers into the agent's prompt. A manual quote
/// carries only the part the person selected. Transport text is untrusted, so the context is one
/// bounded line: no sender id is kept and none of its content goes into analytics. Pure so the
/// decision is testable without a bot or a socket.
const REPLY_CHARS: usize = 600;

pub(super) fn reply_context(reply: Option<&str>, quote: Option<&str>) -> Option<String> {
    let selected = quote
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .or_else(|| reply.map(str::trim).filter(|text| !text.is_empty()))?;
    let flat: String = selected.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut text: String = flat.chars().take(REPLY_CHARS).collect();
    if flat.chars().count() > REPLY_CHARS {
        text.push('…');
    }
    Some(format!("Replying to: {text}"))
}

pub(super) async fn handle(hub: &Arc<Hub>, ep: &Endpoint, name: &str, shared: &Shared, update: &Value, expected: &str) {
    let Some(bot) = current(hub, name) else { return };
    if fingerprint(&bot) != expected {
        return;
    }
    if let Some(query) = update.get("callback_query") {
        return answer_button(hub, ep, &bot, query).await;
    }
    let message = &update["message"];
    let group_text = message["text"].as_str().or_else(|| message["caption"].as_str()).unwrap_or_default().trim();
    // A group may not talk to the bot as an agent, but a new mention can start a workflow. That
    // mention is handed to trigger nodes here and never opens an agent conversation.
    if message["chat"]["type"] != "private" {
        if !super::mentions::mentions_bot(message, &bot.username) {
            return;
        }
        let Some(chat) = message["chat"]["id"].as_i64() else { return };
        let user = message["from"]["id"].as_i64().unwrap_or_default();
        let message_id = message["message_id"].as_i64().unwrap_or_default();
        let event = json!({
            "source": "telegram", "event": "mention", "connection": chat.to_string(),
            "chat": chat.to_string(), "chatId": chat.to_string(),
            "user": user.to_string(), "author": user.to_string(), "text": group_text,
            "messageId": message_id.to_string(),
            "thread": message["message_thread_id"].as_i64().map(|id| id.to_string()).unwrap_or_default(),
        });
        let skipped =
            hub.routines.receive_chat_event(&hub.store, &event, Some(format!("tg-{name}-{message_id}"))).all_skipped();
        if skipped {
            let (hub, name) = (hub.clone(), name.to_owned());
            tokio::spawn(async move {
                if let Err(error) = super::react(&hub.store, &name, chat, message_id, "👎").await {
                    tracing::warn!(error = format!("{error:#}"), "couldn't mark a skipped Telegram workflow mention");
                }
            });
        }
        return;
    }
    let (Some(chat), Some(user)) = (message["chat"]["id"].as_i64(), message["from"]["id"].as_i64()) else { return };
    let text = message["text"].as_str().or_else(|| message["caption"].as_str()).unwrap_or_default().trim();
    // OpenBot 0.70.0: a Reply shows the message answered, or only the selected part when quoted.
    let reply = reply_context(
        message["reply_to_message"]["text"].as_str().or_else(|| message["reply_to_message"]["caption"].as_str()),
        message["quote"]["text"].as_str(),
    );
    let voice = message.get("voice").or_else(|| message.get("audio"));
    if text.is_empty() && voice.is_none() {
        return;
    }
    set(hub, "seen", name, &crate::store::now_ms().to_string());
    if !bot.allowed.contains(&user) {
        return pair(hub, ep, &bot, chat, user, text).await;
    }
    let mentioned = super::mentions::mentions_bot(message, &bot.username);
    if mentioned {
        let nonce = format!("tg-{name}-{}", update["update_id"]);
        let event = json!({
            "source": "telegram", "event": "mention", "connection": chat.to_string(),
            "chat": chat.to_string(), "chatId": chat.to_string(), "user": user,
            "author": user, "username": message["from"]["username"], "text": text,
            "messageId": message["message_id"],
        });
        let skipped = hub.routines.receive_chat_event(&hub.store, &event, Some(nonce)).all_skipped();
        if skipped {
            let message_id = message["message_id"].as_i64().unwrap_or_default();
            let (hub, name) = (hub.clone(), name.to_owned());
            tokio::spawn(async move {
                if let Err(error) = super::react(&hub.store, &name, chat, message_id, "👎").await {
                    tracing::warn!(error = format!("{error:#}"), "couldn't mark a skipped Telegram workflow mention");
                }
            });
        }
    }
    if let Some(command) = commands::parse(text, Some(&bot.username)) {
        return say(ep, &bot, chat, &commands::run(hub, &bot.bot_id, &command)).await;
    }
    if text.starts_with("/start") {
        let help = "Write here to talk to your bot. /estado shows what it is doing, /rutinas lists its routines, /detener stops it.";
        return say(ep, &bot, chat, help).await;
    }
    // Authorization and text commands precede audio I/O. A transcript never becomes a transport
    // command such as /detener; it is ordinary message content for this bound bot.
    let from = super::history::sender(message);
    // Keep Telegram's message id on the row so a later edit or delete can find it (OpenBot nightly).
    let message_id = message["message_id"].as_i64().map(|id| id.to_string()).unwrap_or_default();
    let remember = |text: &str, file: Option<&str>| {
        if let Err(error) = super::history::remember_with_id(
            &hub.store,
            &super::history::Incoming {
                connector: name,
                chat,
                group: false,
                sender: from.as_str(),
                title: message["chat"]["title"].as_str(),
                text,
                file,
                at: crate::store::now_ms(),
            },
            &message_id,
        ) {
            tracing::warn!(telegram = name, error = format!("{error:#}"), "Telegram history was not saved");
        }
    };
    let (text, file) = if let Some(voice) = voice {
        let fetched = async {
            let id = voice["file_id"].as_str().ok_or_else(|| anyhow::anyhow!("Telegram voice note has no file id"))?;
            let bytes = client::voice_note(ep, &bot.token, id).await?;
            // OpenBot 0.75.0: the file that arrived is kept beside the text it carried.
            let file = super::history::save_file(&bytes, "ogg").await.ok();
            let transcript = crate::voice::transcription::note(&hub.store, &bytes).await;
            Ok::<_, anyhow::Error>((transcript, file))
        }
        .await;
        let (transcript, file) = match fetched {
            Ok(pair) => pair,
            Err(error) => {
                tracing::warn!(
                    telegram = name,
                    error = format!("{error:#}"),
                    "Telegram voice note could not be transcribed"
                );
                return say(ep, &bot, chat, "Could not transcribe that voice note. Check the speech provider in Codync call settings, or send the message as text.").await;
            }
        };
        match transcript {
            Ok(transcript) => (format!("{text}\n\n[Voice note transcript]\n{transcript}").trim().to_owned(), file),
            Err(error) => {
                tracing::warn!(
                    telegram = name,
                    error = format!("{error:#}"),
                    "Telegram voice note could not be transcribed"
                );
                // The file already arrived: keep it with the text instead of leaving it orphaned.
                remember(text, file.as_deref());
                return say(ep, &bot, chat, "Could not transcribe that voice note. Check the speech provider in Codync call settings, or send the message as text.").await;
            }
        }
    } else {
        (text.to_owned(), None)
    };
    // OpenBot 0.70.0: the same context reaches the kept row and the prompt, so an agent reading the
    // history after a restart sees what the Reply pointed at. A voice note keeps its file too.
    let note =
        reply.as_deref().map_or_else(|| text.to_owned(), |context| format!("{text}\n\n{context}").trim().to_owned());
    // OpenBot 0.75.0: keep what arrived before it becomes a bot event, so an agent can read the
    // conversation back after a restart. A voice note keeps its file with its text.
    remember(&note, file.as_deref());
    if current(hub, name).is_none_or(|active| fingerprint(&active) != expected || !active.allowed.contains(&user)) {
        return;
    }
    // The route exists before the message is sent: a quick reply must not beat it. Entries of earlier turns are not ours.
    let nonce = format!("tg-{name}-{}", update["update_id"]);
    let turn = hub
        .store
        .find_by_nonce(&bot.bot_id, &nonce)
        .map_or_else(|| hub.store.max_turn(&bot.bot_id) + 1, |entry| entry.turn);
    {
        let mut route = shared.lock().await;
        if route.as_ref().is_none_or(|route| route.chat != chat || route.turn != turn) {
            *route = Some(Route { chat, turn, live: None, busy: None, cards: HashMap::new() });
        }
        // OpenBot 15568: acknowledge the request while the bot works. The line is taken down when
        // the request turns out to be internal, or when the answer has no text of its own.
        if let Ok(message) = client::message(ep, &bot.token, chat, "Working on it…", None).await
            && let Some(route) = route.as_mut()
        {
            route.busy = Some(message);
        }
    }
    match super::input::post(hub, name, user, expected, &note, &nonce) {
        Ok(entry) => {
            if let Err(error) = hub.store.record_channel_activity(&crate::store::ChannelActivity {
                id: nonce,
                source: "telegram".into(),
                bot_id: bot.bot_id.clone(),
                thread_id: None,
                channel: chat.to_string(),
                mentioned,
                at: entry.created_at,
            }) {
                tracing::warn!(error = format!("{error:#}"), "Telegram activity receipt was not saved");
            }
        }
        Err(error) => {
            *shared.lock().await = None;
            say(ep, &bot, chat, &format!("Could not reach the bot: {error:#}")).await;
        }
    }
}

/// A stranger gets no answer, unless they send `/start CODE` (or `/pair CODE`) with a live code.
async fn pair(hub: &Arc<Hub>, ep: &Endpoint, bot: &TgBot, chat: i64, user: i64, text: &str) {
    let Some(rest) = text.strip_prefix("/start").or_else(|| text.strip_prefix("/pair")) else { return };
    let Some(code) = rest.split_whitespace().last() else { return };
    match pairing::redeem(&hub.store, &bot.name, code, crate::store::now_ms()) {
        Ok(true) => match allow(hub, bot, user) {
            Ok(()) => say(ep, bot, chat, &format!("Paired. You can talk to {} here.", hub.bot_name(&bot.bot_id))).await,
            Err(error) => tracing::warn!(error = format!("{error:#}"), "A paired person was not saved"),
        },
        Ok(false) => say(ep, bot, chat, "That code didn't work. Make a new one in Codync.").await,
        Err(error) => tracing::warn!(error = format!("{error:#}"), "A pairing code could not be checked"),
    }
}

fn allow(hub: &Hub, expected: &TgBot, user: i64) -> anyhow::Result<()> {
    let _guard = crate::LockExt::locked(&hub.store.connector_lock);
    let mut config = load(&hub.store)?;
    let bot = config
        .bots
        .iter_mut()
        .find(|b| b.name == expected.name)
        .ok_or_else(|| anyhow::anyhow!("The Telegram bot is gone."))?;
    anyhow::ensure!(fingerprint(bot) == fingerprint(expected), "Telegram connection changed before pairing completed");
    if !bot.allowed.contains(&user) {
        bot.allowed.push(user);
        bot.generation = uuid::Uuid::new_v4().to_string();
    }
    super::save(&hub.store, &config)
}

// MARK: approval buttons

/// Identifies what a button was made for, so a button of a card that changed cannot answer another.
fn card_print(card: &Value) -> String {
    let mut digest = Sha256::new();
    digest.update(card["data"]["title"].as_str().unwrap_or_default());
    for option in card["data"]["options"].as_array().into_iter().flatten() {
        digest.update(option["optionId"].as_str().unwrap_or_default());
    }
    crate::chat::files::digest_hex(&digest.finalize()).chars().take(6).collect()
}

fn keyboard(card: &Value) -> Option<Value> {
    let id = card["id"].as_str()?;
    let print = card_print(card);
    let row: Vec<Value> = card["data"]["options"]
        .as_array()?
        .iter()
        .filter_map(|o| {
            let data = format!("p|{id}|{}|{print}", o["optionId"].as_str()?);
            // Telegram refuses callback data over 64 bytes.
            let text = o["name"].as_str()?;
            (data.len() <= 64).then(|| json!({"text": text, "callback_data": data}))
        })
        .collect();
    (!row.is_empty()).then(|| json!(row.chunks(2).collect::<Vec<_>>()))
}

async fn answer_button(hub: &Arc<Hub>, ep: &Endpoint, bot: &TgBot, query: &Value) {
    let reply = |text: String| async move {
        let body = json!({"callback_query_id": query["id"], "text": text});
        if let Err(error) = client::call(ep, &bot.token, "answerCallbackQuery", &body).await {
            tracing::warn!(error = format!("{error:#}"), "A button press was not answered");
        }
    };
    if !query["from"]["id"].as_i64().is_some_and(|u| bot.allowed.contains(&u)) {
        return reply("You are not allowed to answer this.".into()).await;
    }
    let mut parts = query["data"].as_str().unwrap_or_default().split('|');
    let (Some("p"), Some(id), Some(option), Some(print)) = (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return reply("That button is not for this.".into()).await;
    };
    let card = hub.store.entry(id).filter(|e| e.bot_id == bot.bot_id && e.kind == "permission");
    let Some(card) = card.and_then(|e| serde_json::to_value(e).ok()) else {
        return reply("That approval is gone.".into()).await;
    };
    if card_print(&card) != print {
        return reply("That approval changed.".into()).await;
    }
    // The first answer wins, from any client; a later one finds the card settled.
    let answered =
        crate::api::dispatch(hub, &Caller::Local, "respondPermission", json!({"entryId": id, "optionId": option}))
            .await;
    reply(match answered {
        Ok(_) => "Done.".into(),
        Err(error) => format!("{error:#}").chars().take(180).collect(),
    })
    .await;
}

// MARK: replying

async fn stream(hub: Arc<Hub>, ep: Endpoint, name: String, shared: Shared) {
    let mut events = hub.events.subscribe();
    loop {
        let event = match events.recv().await {
            Ok(event) => event,
            Err(RecvError::Lagged(_)) => continue,
            Err(RecvError::Closed) => return,
        };
        let Some(bot) = current(&hub, &name) else { return };
        let entry = &event["entry"];
        if event["type"] != "entry" || entry["botId"] != bot.bot_id.as_str() || entry.get("threadId").is_some() {
            continue;
        }
        let mut route = shared.lock().await;
        let Some(route) = route.as_mut().filter(|r| entry["turn"].as_i64().unwrap_or(0) >= r.turn) else { continue };
        // A teammate's internal result isn't a message for this bot: take the working line down
        // instead of leaving it hanging over a reply nobody in this chat asked for (15568).
        if entry["data"]["internal"] == true
            && let Some(busy) = route.busy.take()
            && drop_busy_post(BusyReason::NotForBot, false)
        {
            if let Err(error) = client::delete(ep, &bot.token, route.chat, busy).await {
                tracing::warn!(error = format!("{error:#}"), telegram = name, "working line was not deleted");
            }
            continue;
        }
        let result = match entry["kind"].as_str() {
            Some("agent") => reply(&ep, &bot, route, entry).await.map(|finished| {
                if finished {
                    route.live = None;
                }
            }),
            Some("permission") => card(&ep, &bot, route, entry).await,
            _ => Ok(()),
        };
        if let Err(error) = result {
            tracing::warn!(error = format!("{error:#}"), telegram = name, "Telegram could not show a reply");
        }
    }
}

/// Writes the agent's text into the chat: one message that grows while it streams. True when the reply is complete.
async fn reply(ep: &Endpoint, bot: &TgBot, route: &mut Route, entry: &Value) -> anyhow::Result<bool> {
    let text = entry["data"]["text"].as_str().unwrap_or_default();
    let finished = entry["data"]["final"] == true;
    let id = entry["id"].as_str().unwrap_or_default();
    let has_text = !text.is_empty();
    if !has_text {
        // A files-only answer has no text to leave above: drop the working line (15568).
        if finished
            && let Some(busy) = route.busy.take()
            && drop_busy_post(BusyReason::FilesOnly, has_text)
        {
            client::delete(ep, &bot.token, route.chat, busy).await?;
        }
        return Ok(false);
    }
    match route.live.as_mut().filter(|l| l.entry == id) {
        None => {
            let message = client::reply(ep, &bot.token, route.chat, text).await?;
            route.live = Some(Live { entry: id.into(), message, sent: text.into(), at: Instant::now() });
        }
        Some(live) if live.sent != text && (finished || live.at.elapsed() >= ep.edit_every) => {
            client::edit_reply(ep, &bot.token, route.chat, live.message, text).await?;
            live.sent = text.into();
            live.at = Instant::now();
        }
        Some(_) => {}
    }
    if finished && text.chars().count() > client::MAX_TEXT {
        client::document(ep, &bot.token, route.chat, "reply.txt", text.as_bytes(), "The whole reply is in this file.")
            .await?;
    }
    Ok(finished)
}

/// An approval the bot asks becomes a message with buttons; once it is settled anywhere, the buttons go.
async fn card(ep: &Endpoint, bot: &TgBot, route: &mut Route, entry: &Value) -> anyhow::Result<()> {
    let id = entry["id"].as_str().unwrap_or_default();
    let title = entry["data"]["title"].as_str().unwrap_or("Approval");
    match (entry["data"]["status"].as_str(), route.cards.get(id)) {
        (Some("pending"), None) => {
            let Some(buttons) = keyboard(entry) else { return Ok(()) };
            let message =
                client::message(ep, &bot.token, route.chat, &format!("Needs you: {title}"), Some(buttons)).await?;
            route.cards.insert(id.into(), message);
        }
        (Some("pending") | None, _) | (Some(_), None) => {}
        (Some(status), Some(&message)) => {
            let chosen = entry["data"]["options"]
                .as_array()
                .into_iter()
                .flatten()
                .find(|o| o["optionId"] == entry["data"]["selected"])
                .and_then(|o| o["name"].as_str());
            let line = match (status, chosen) {
                ("answered", Some(name)) => format!("{title}\n✓ {name}"),
                _ => format!("{title}\n(closed)"),
            };
            client::edit(ep, &bot.token, route.chat, message, &line).await?;
            route.cards.remove(id);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_working_line_goes_when_the_message_isnt_for_the_bot_or_the_answer_is_files_only() {
        assert!(drop_busy_post(BusyReason::NotForBot, false));
        assert!(drop_busy_post(BusyReason::NotForBot, true), "an internal result drops it even with text");
        assert!(drop_busy_post(BusyReason::FilesOnly, false), "files-only has no text to show");
        assert!(!drop_busy_post(BusyReason::FilesOnly, true), "a text answer keeps the line");
    }

    #[test]
    fn a_reply_shows_the_answered_message_and_a_quote_shows_only_the_selection() {
        assert_eq!(
            reply_context(Some("  the whole message\nnext line  "), None),
            Some("Replying to: the whole message next line".into())
        );
        assert_eq!(
            reply_context(Some("the whole message"), Some("  just this part  ")),
            Some("Replying to: just this part".into()),
            "a manual quote wins over the whole message"
        );
        assert_eq!(reply_context(None, Some("  ")), None, "an empty selection is no context");
        assert_eq!(reply_context(None, None), None, "no reply is no context");
    }

    #[test]
    fn reply_context_is_one_bounded_line_without_a_sender() {
        let long = "word ".repeat(REPLY_CHARS);
        let context = reply_context(Some(&long), None).expect("a context");
        assert!(context.starts_with("Replying to: word"));
        assert!(context.chars().count() <= "Replying to: ".chars().count() + REPLY_CHARS + 1);
        assert!(context.ends_with('…'), "the tail is marked as cut");
        assert!(!context.contains('\n'), "transport newlines are flattened");
        assert!(!context.contains("from:"), "no sender id is kept");
    }
}
