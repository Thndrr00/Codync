//! A2A over codync-network: a request is an ordinary letter whose `meta.a2a` carries the JSON-RPC
//! envelope, so the same roster, per-bot offering, rules, budgets and reply edges apply as for any
//! bot request. The receiving computer runs the bot through the normal request path, approvals
//! included, and answers with another letter.

use super::{Waiting, card, protocol};
use crate::chat::team;
use crate::hub::Hub;
use crate::network::Recipient;
use crate::store::BotConfig;
use anyhow::{Context, Result, bail};
use codync_network::{Letter, Message};
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;

/// What one A2A request may say, like `message_bot`.
const MAX_MESSAGE_BYTES: usize = 32_000;

/// How long the receiving computer lets one A2A task run.
const RUN_TIMEOUT: Duration = Duration::from_mins(10);

/// A reservation this computer's message budget gave the request; released on every path.
struct Reserved<'a> {
    hub: &'a Hub,
    bot: &'a str,
}

impl Drop for Reserved<'_> {
    fn drop(&mut self) {
        self.hub.team.release_message(self.bot);
    }
}

/// Asks the A2A agent `agent` on another computer. Returns the task result (`kind: "task"`).
pub(super) async fn ask(
    hub: &Arc<Hub>,
    source: &BotConfig,
    agent: &str,
    message: &str,
    timeout: Duration,
    reserve: bool,
) -> Result<Value> {
    anyhow::ensure!(!message.is_empty() && message.len() <= MAX_MESSAGE_BYTES, "The A2A task is empty or too long.");
    let target = match hub.network.recipient(agent)? {
        Recipient::There(target) => target,
        Recipient::Here(_) => {
            bail!("{agent} is on this computer: use ask_bot for it, or name a bot on another computer.")
        }
    };
    let from = hub.network.handle_of(&source.id).context("this bot isn't offered on the network")?;
    let hops = if reserve { hub.team.reserve_message(&source.id)? } else { 0 };
    let _reserved = reserve.then_some(Reserved { hub, bot: &source.id });
    let json_id = uuid::Uuid::new_v4().to_string();
    let request = protocol::Request {
        id: json!(json_id.clone()),
        method: protocol::Method::Send,
        text: message.to_owned(),
        message_id: uuid::Uuid::new_v4().to_string(),
        context_id: uuid::Uuid::new_v4().to_string(),
    };
    let letter = Letter::new(from, target.address.clone(), message).hops(hops).meta(json!({
        "a2a": {
            "jsonrpc": "2.0",
            "id": request.id,
            "method": "message/send",
            "params": {"message": {
                "role": "user",
                "messageId": request.message_id,
                "contextId": request.context_id,
                "parts": [{"kind": "text", "text": request.text}],
            }},
        }
    }));
    let id = json_id;
    let node = hub.network.slot(Some(&target.network))?.node;
    let (sender, receiver) = tokio::sync::oneshot::channel();
    hub.a2a.insert(id.clone(), sender);
    if let Err(error) = node.send(letter).await {
        hub.a2a.take(&id);
        return Err(error).context("the A2A computer refused the request");
    }
    let response = match tokio::time::timeout(timeout, receiver).await {
        Ok(Ok(response)) => response,
        Ok(Err(_)) => {
            hub.a2a.take(&id);
            bail!("the A2A agent stopped before answering");
        }
        Err(_) => {
            hub.a2a.take(&id);
            bail!("the A2A agent did not answer in time; partial work may have happened");
        }
    };
    if let Some(error) = response.get("error") {
        let why = error["message"].as_str().unwrap_or("refused");
        bail!("{agent}: {why}");
    }
    Ok(response.get("result").cloned().unwrap_or(Value::Null))
}

/// Why this computer refuses an A2A request for `bot`: the per-bot network offering is the same
/// switch the network directory uses, so a bot kept off it is not an A2A agent.
pub(super) fn admit(bot: &BotConfig) -> Result<()> {
    anyhow::ensure!(super::agent_available(bot), "that agent isn't offered to the A2A network");
    Ok(())
}

/// Reads an A2A letter before the normal bot-request paths: a reply resolves its waiter, and a
/// request runs the bot with the receiving computer's rules and approvals. Returns whether the
/// letter was A2A traffic (and so is fully handled here).
pub(crate) fn intercept(hub: &Arc<Hub>, network: &str, m: &Message) -> Result<bool> {
    let Some(a2a) = m.meta.as_ref().and_then(|meta| meta.get("a2a")) else { return Ok(false) };
    if a2a.get("result").is_some() || a2a.get("error").is_some() {
        let id = a2a["id"].as_str().unwrap_or_default();
        if let Some(waiter) = hub.a2a.take(id) {
            let _ = waiter.send(a2a.clone());
        }
        return Ok(true);
    }
    let method = a2a["method"].as_str().unwrap_or_default();
    let id = a2a["id"].clone();
    let Some(bot_id) = hub.network.bot_for(&m.to) else {
        refuse(hub, network, m, &id, "that computer has no agent with this address");
        return Ok(true);
    };
    let Some(bot) = hub.store.bot(&bot_id)?.filter(|row| !row.deleted && !row.config.is_group()) else {
        refuse(hub, network, m, &id, "that computer has no agent with this address");
        return Ok(true);
    };
    if let Err(error) = admit(&bot.config) {
        refuse(hub, network, m, &id, &format!("{error:#}"));
        return Ok(true);
    }
    match method {
        "agent/card" => {
            let handle = hub.network.handle_of(&bot_id).unwrap_or_else(|| m.to.clone());
            let computer = hub.network.slot(Some(network)).ok().and_then(|slot| slot.node.name()).unwrap_or_default();
            let value = card::agent_card(&bot.config, &handle, &computer, &format!("codync://{handle}@{computer}"));
            reply(hub, network, m, protocol::success(&id, value));
        }
        "message/send" | "message/stream" => {
            let text = a2a["params"]["message"]
                .as_ref()
                .and_then(protocol::text_of)
                .filter(|text| !text.trim().is_empty())
                .unwrap_or_else(|| m.body.trim().to_owned());
            if text.is_empty() || text.len() > MAX_MESSAGE_BYTES {
                refuse(hub, network, m, &id, "the A2A task is empty or too long");
                return Ok(true);
            }
            let request = protocol::Request {
                id: id.clone(),
                method: protocol::Method::Send,
                text: text.clone(),
                message_id: a2a["params"]["message"]["messageId"]
                    .as_str()
                    .filter(|id| !id.is_empty())
                    .unwrap_or(&m.id)
                    .to_owned(),
                context_id: a2a["params"]["message"]["contextId"]
                    .as_str()
                    .filter(|id| !id.is_empty())
                    .unwrap_or(&m.id)
                    .to_owned(),
            };
            let hub = hub.clone();
            let network = network.to_owned();
            let received = m.clone();
            let peer = m.from.to_string();
            tokio::spawn(async move {
                let outcome = team::a2a_ask(&hub, &bot.config, &peer, &request.text, received.hops, RUN_TIMEOUT).await;
                let answer = match outcome {
                    Ok(text) => protocol::success(
                        &request.id,
                        protocol::task(&request, protocol::State::Completed, Some(&text)),
                    ),
                    Err(error) => protocol::error(&request.id, -32000, &format!("{error:#}")),
                };
                reply(&hub, &network, &received, answer);
            });
            Ok(true)
        }
        _ => {
            refuse(hub, network, m, &id, "only message/send, message/stream and agent/card are served");
            Ok(true)
        }
    }
}

fn refuse(hub: &Arc<Hub>, network: &str, m: &Message, id: &Value, why: &str) {
    reply(hub, network, m, protocol::error(id, -32000, why));
}

/// Sends an A2A answer as a notice back to the asking computer. It names the request, so the
/// receiver's rules let an answer through even when they changed while the task ran.
fn reply(hub: &Arc<Hub>, network: &str, m: &Message, answer: Value) {
    let Ok(slot) = hub.network.slot(Some(network)) else { return };
    let letter =
        Letter::new(m.to.clone(), m.from.clone(), "A2A").notice().reply_to(m.id.clone()).meta(json!({"a2a": answer}));
    tokio::spawn(async move {
        if let Err(error) = slot.node.send(letter).await {
            tracing::warn!(error = format!("{error:#}"), "couldn't send an A2A answer to another computer");
        }
    });
}
