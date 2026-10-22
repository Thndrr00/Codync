//! A terminal session's life in Codync: the bot standing for it, its status and its transcript.

use super::event::Event;
use super::{BACKEND, STALE_AFTER, tool};
use crate::LockExt;
use crate::agent::acp::truncate;
use crate::hub::{BotStatus, Hub};
use crate::remote::push::{self, AlertKind};
use crate::store::{BotConfig, EntryKind, Lane, now_ms};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::time::Instant;

/// Tool calls remembered per session while they run; a session never has this many at once.
const MAX_RUNNING_TOOLS: usize = 64;

pub struct Session {
    pub bot: String,
    turn: i64,
    /// The agent's tool call id → the transcript entry showing it.
    tools: HashMap<String, String>,
    seen: Instant,
}

fn label(agent: &str) -> &str {
    match agent {
        "" | "claude-desktop" => "Claude Code",
        "gemini" => "Gemini CLI",
        "antigravity" => "Antigravity",
        "codex" => "Codex",
        "copilot" => "Copilot CLI",
        "cursor" => "Cursor",
        _ => "Agent",
    }
}

fn color(agent: &str) -> &'static str {
    match agent {
        "" | "claude-desktop" => "orange",
        "gemini" | "antigravity" => "blue",
        "codex" => "green",
        _ => "violet",
    }
}

fn config(ev: &Event) -> BotConfig {
    let project = ev.project();
    let name =
        if project.is_empty() { label(&ev.agent).to_owned() } else { format!("{} · {project}", label(&ev.agent)) };
    let config = json!({
        "id": "", "name": name, "description": format!("{} in a terminal", label(&ev.agent)),
        "avatarColor": color(&ev.agent), "avatarShape": "squircle", "backend": BACKEND, "cwd": ev.cwd,
        "permission": "ask", "notify": true,
    });
    // Every other setting is the default of a bot that never runs an agent of its own.
    serde_json::from_value(config).expect("a fixed shape always deserializes")
}

/// The bot standing for the event's session, created on first sight. Idle sessions of terminals that were
/// closed without a goodbye are cleaned away on the way.
pub(super) fn bot_for(hub: &Hub, ev: &Event) -> Option<String> {
    let gone: Vec<String> = {
        let mut state = hub.external.state.locked();
        let expired: Vec<String> = state
            .sessions
            .iter()
            .filter(|(key, s)| **key != ev.session && s.seen.elapsed() > STALE_AFTER)
            .map(|(key, _)| key.clone())
            .collect();
        expired.iter().filter_map(|key| state.sessions.remove(key)).map(|s| s.bot).collect()
    };
    for bot in gone {
        forget(hub, &bot);
    }
    let mut state = hub.external.state.locked();
    // A bot the person deleted from the roster stays gone for this session: the terminal has to start a new one.
    if state.dismissed.contains(&ev.session) {
        return None;
    }
    let live = state
        .sessions
        .get_mut(&ev.session)
        .filter(|s| hub.store.bot(&s.bot).ok().flatten().is_some_and(|r| !r.deleted));
    if let Some(session) = live {
        session.seen = Instant::now();
        return Some(session.bot.clone());
    }
    state.sessions.remove(&ev.session);
    match hub.add_external_bot(config(ev)) {
        Ok(bot) => {
            let turn = hub.store.max_turn(&bot);
            state.sessions.insert(
                ev.session.clone(),
                Session { bot: bot.clone(), turn, tools: HashMap::new(), seen: Instant::now() },
            );
            Some(bot)
        }
        Err(error) => {
            tracing::warn!(error = format!("{error:#}"), "couldn't add a terminal session");
            None
        }
    }
}

fn forget(hub: &Hub, bot: &str) {
    if let Err(error) = hub.delete_bot(bot) {
        tracing::warn!(error = format!("{error:#}"), bot, "couldn't remove a finished terminal session");
    }
}

pub(super) fn turn_of(hub: &Hub, session: &str) -> i64 {
    hub.external.state.locked().sessions.get(session).map_or(0, |s| s.turn)
}

fn working(hub: &Hub, bot: &str, activity: &str, fresh: bool) {
    hub.set_runtime(bot, |r| {
        r.status = BotStatus::Working;
        activity.clone_into(&mut r.activity);
        r.lane = Some(Lane::main(bot));
        if fresh || r.started_at.is_none() {
            r.started_at = Some(now_ms());
        }
    });
}

fn idle(hub: &Hub, bot: &str) {
    hub.set_runtime(bot, |r| {
        r.status = BotStatus::Idle;
        r.activity.clear();
        r.started_at = None;
        r.lane = None;
    });
}

pub(super) fn record(hub: &Hub, ev: &Event) {
    if ev.name == "SessionEnd" {
        let session = {
            let mut state = hub.external.state.locked();
            // A new session with the same key is a new terminal session: it starts over.
            state.dismissed.remove(&ev.session);
            state.sessions.remove(&ev.session)
        };
        if let Some(session) = session {
            forget(hub, &session.bot);
        }
        return;
    }
    let Some(bot) = bot_for(hub, ev) else { return };
    match ev.name.as_str() {
        "UserPromptSubmit" => prompt(hub, ev, &bot),
        "PreToolUse" => tool_started(hub, ev, &bot),
        "PostToolUse" | "PostToolUseFailure" => tool_finished(hub, ev, &bot),
        "Stop" | "StopFailure" | "Interrupt" => stopped(hub, ev, &bot),
        _ => {}
    }
}

fn prompt(hub: &Hub, ev: &Event, bot: &str) {
    let turn = {
        let mut state = hub.external.state.locked();
        let Some(session) = state.sessions.get_mut(&ev.session) else { return };
        session.turn += 1;
        session.turn
    };
    if !ev.prompt.is_empty() {
        hub.add_entry(
            &Lane::main(bot),
            EntryKind::User,
            turn,
            &json!({"text": ev.prompt, "status": "sent", "attachments": []}),
        );
    }
    working(hub, bot, "Thinking…", true);
}

fn tool_started(hub: &Hub, ev: &Event, bot: &str) {
    let id = if ev.tool_use_id.is_empty() { uuid::Uuid::new_v4().to_string() } else { ev.tool_use_id.clone() };
    let data = tool::entry_data(&id, &ev.tool, &ev.input, bot);
    let turn = turn_of(hub, &ev.session);
    if let Some(entry) = hub.add_entry(&Lane::main(bot), EntryKind::Tool, turn, &data) {
        let mut state = hub.external.state.locked();
        if let Some(session) = state.sessions.get_mut(&ev.session)
            && session.tools.len() < MAX_RUNNING_TOOLS
        {
            session.tools.insert(id, entry.id);
        }
    }
    working(hub, bot, tool_activity(&data), false);
}

fn tool_activity(data: &Value) -> &str {
    data["title"].as_str().unwrap_or("Working…")
}

fn tool_finished(hub: &Hub, ev: &Event, bot: &str) {
    let status = if ev.name == "PostToolUseFailure" { "failed" } else { "completed" };
    let known = hub.external.state.locked().sessions.get_mut(&ev.session).and_then(|s| s.tools.remove(&ev.tool_use_id));
    let diffs = ev.diffs();
    let started = known.and_then(|id| hub.store.entry(&id));
    if let Some(mut entry) = started {
        entry.data["status"] = status.into();
        if !diffs.is_empty() {
            entry.data["diffs"] = diffs.into();
        }
        hub.set_entry(&entry.id, &entry.data);
    } else {
        // A tool call whose start this host never saw (it was closed, or the hook came late): shown whole.
        let mut data = tool::entry_data(&ev.tool_use_id, &ev.tool, &ev.input, bot);
        data["status"] = status.into();
        data["diffs"] = diffs.into();
        hub.add_entry(&Lane::main(bot), EntryKind::Tool, turn_of(hub, &ev.session), &data);
    }
    working(hub, bot, "Working…", false);
}

fn stopped(hub: &Hub, ev: &Event, bot: &str) {
    idle(hub, bot);
    let failed = ev.name == "StopFailure";
    if !ev.message.is_empty() {
        let data = json!({"text": ev.message, "final": true, "author": bot});
        hub.add_entry(&Lane::main(bot), EntryKind::Agent, turn_of(hub, &ev.session), &data);
    }
    if ev.name == "Interrupt" {
        return;
    }
    let Ok(Some(row)) = hub.store.bot(bot) else { return };
    let (title, kind) = if failed {
        (format!("{} stopped with an error", row.config.name), AlertKind::Failed)
    } else {
        (format!("{} finished", row.config.name), AlertKind::Done)
    };
    push::notify(hub, &row.config, Some(bot), &title, &truncate(&ev.message, 120), kind);
}
