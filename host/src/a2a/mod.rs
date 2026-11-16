//! A2A (Agent2Agent) between the user's computers: every bot offered to the network is an A2A
//! agent, and a bot can ask another computer's agent a bounded task.
//!
//! The public protocol surface is the local API: an agent card at
//! `GET /a2a/{bot}/.well-known/agent-card.json` and a JSON-RPC endpoint at `POST /a2a/{bot}` that
//! speaks `message/send` and `message/stream` (Server-Sent Events), authenticated like every other
//! local API route. Between computers the same JSON-RPC request travels as a codync-network letter:
//! the receiving computer's roster, per-bot "offered to the network" switch, rules and budgets
//! decide first, exactly as they do for `ask_bot`, and the recipient bot's own approval cards stay
//! in force. Nothing listens on a public port for A2A.

pub(crate) mod card;
mod client;
mod protocol;
pub(crate) mod server;
#[cfg(test)]
mod tests;

pub(crate) use client::intercept;

use crate::{hub::Hub, store::BotConfig};
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;

/// Replies another computer's A2A agent sent, by JSON-RPC request id, until the caller reads them.
#[derive(Default)]
pub struct Waiting(Mutex<HashMap<String, oneshot::Sender<Value>>>);

impl Waiting {
    pub(crate) fn insert(&self, id: String, reply: oneshot::Sender<Value>) {
        if let Ok(mut waiting) = self.0.lock() {
            waiting.insert(id, reply);
        }
    }

    pub(crate) fn take(&self, id: &str) -> Option<oneshot::Sender<Value>> {
        self.0.lock().ok().and_then(|mut waiting| waiting.remove(id))
    }
}

/// `a2aAgents`, `a2aCard` and `a2aSend`, for this computer's apps.
pub const METHODS: &[&str] = &["a2aAgents", "a2aCard", "a2aSend"];

/// The A2A agents the other computers offer, as cards. Only Codync bots that are offered to the
/// network appear; a bot kept off it is not an A2A agent.
pub fn agents(hub: &Hub) -> Vec<Value> {
    let mut cards = Vec::new();
    for slot in hub.network.slots() {
        for peer in slot.node.directory() {
            if peer.me {
                continue;
            }
            for agent in peer.agents.iter().filter(|agent| agent.kind == codync_network::AgentKind::Codync) {
                cards.push(card::remote_card(&agent.id, &agent.name, &agent.about, &peer.name, &slot.name));
            }
        }
    }
    cards.sort_by(|a, b| {
        a["metadata"]["agent"].as_str().unwrap_or_default().cmp(b["metadata"]["agent"].as_str().unwrap_or_default())
    });
    cards
}

pub(crate) async fn call(hub: &Arc<Hub>, method: &str, body: &Value) -> Result<Value> {
    match method {
        "a2aAgents" => Ok(json!({"agents": agents(hub)})),
        "a2aCard" => {
            let agent = body["agent"].as_str().context("agent is required")?;
            agents(hub)
                .into_iter()
                .find(|card| {
                    let handle = card["metadata"]["agent"].as_str().unwrap_or_default();
                    let computer = card["metadata"]["computer"].as_str().unwrap_or_default();
                    agent == handle || agent == format!("{handle}@{computer}")
                })
                .map(|card| json!({"card": card}))
                .ok_or_else(|| anyhow::anyhow!("No computer offers the A2A agent `{agent}`."))
        }
        "a2aSend" => {
            let source = visible_bot(hub, body["botId"].as_str().context("botId is required")?)?;
            let agent = body["agent"].as_str().context("agent is required")?;
            let message = body["message"].as_str().unwrap_or_default().trim();
            let timeout = timeout(body)?;
            client::ask(hub, &source, agent, message, timeout, false).await
        }
        other => bail!("Unknown A2A method `{other}`."),
    }
}

/// The team tool `a2a_ask`, called by one of this computer's bots.
pub(crate) async fn ask_tool(hub: &Arc<Hub>, from: &str, args: &Value) -> Result<Value> {
    let source = visible_bot(hub, from)?;
    let agent = args["agent"].as_str().context("agent is required")?;
    let message = args["message"].as_str().unwrap_or_default().trim();
    client::ask(hub, &source, agent, message, timeout(args)?, true).await
}

fn timeout(body: &Value) -> Result<std::time::Duration> {
    let seconds = body.get("timeoutSeconds").and_then(Value::as_u64).unwrap_or(600);
    anyhow::ensure!((1..=600).contains(&seconds), "timeoutSeconds must be between 1 and 600.");
    Ok(std::time::Duration::from_secs(seconds))
}

fn visible_bot(hub: &Hub, id: &str) -> Result<BotConfig> {
    hub.store
        .bot(id)?
        .filter(|row| !row.deleted && !row.config.hidden && !row.config.is_group())
        .map(|row| row.config)
        .ok_or_else(|| anyhow::anyhow!("unknown or hidden bot"))
}

/// Whether `bot` is exposed as an A2A agent: the same per-bot switch that offers it to the network.
pub(crate) fn agent_available(bot: &BotConfig) -> bool {
    bot.offered_to_network && !bot.hidden && !bot.is_group()
}

/// The tool description, added to the team tool set.
pub fn tool() -> Value {
    json!({
        "name": "a2a_ask",
        "description": "Ask an A2A agent on another of the user's computers and wait for its result (up to 10 minutes). The agent is one of that computer's Codync bots offered to the network; the receiving computer's network rules, budgets and that bot's own approvals apply, exactly as for ask_bot. Its reply is the task's artifact. Stopping you cancels the wait; partial work may have happened.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "agent": {"type": "string", "description": "The agent's address from list_bots or a2a agents, such as reviewer@desk."},
                "message": {"type": "string", "description": "Self-contained task, context and expected result."},
                "timeoutSeconds": {"type": "integer", "minimum": 1, "maximum": 600, "description": "Deadline including queue time (default 600)."},
            },
            "required": ["agent", "message"],
        },
        "annotations": {"readOnlyHint": false, "idempotentHint": false},
    })
}
