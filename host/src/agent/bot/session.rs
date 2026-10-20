//! The bot's ACP sessions: starting its agent, new/load/fork, thread sessions.

use super::fast_mode::Listing;
use super::launch::{claude_meta, launch_commands, select_model, start_agent, system_prompt};
use super::{Actor, Conn, Instructions, NoticeStyle};
use crate::agent::acp::{self, Incoming};
use crate::agent::own_memory;
use crate::chat::{context, memory};
use crate::store::{Entry, EntryKind, Lane, lane_key};
use anyhow::{Result, anyhow, bail};
use serde_json::{Value, json};
use std::time::{Duration, Instant};

impl Actor {
    /// Which session a lane's turns use: a thread of the bot's own chat has its own
    /// (by root); its chat and every group lane use the main one.
    pub(super) fn slot(&self, lane: &Lane) -> Option<String> {
        (lane.chat == self.cfg.id).then(|| lane.thread.clone()).flatten()
    }

    pub(super) fn session_for(&mut self, slot: Option<&str>) -> Option<String> {
        let Some(root) = slot else { return self.session_id.clone() };
        if let Some(sid) = self.thread_sessions.get(root) {
            return Some(sid.clone());
        }
        let sid = self.hub.store.kv_get(&lane_key("session", &self.cfg.id, &Lane::in_thread(&self.cfg.id, root)));
        let sid = sid.filter(|s| !s.is_empty())?;
        self.thread_sessions.insert(root.to_owned(), sid.clone());
        Some(sid)
    }

    pub(super) fn set_session(&mut self, slot: Option<&str>, sid: Option<&str>) {
        let Some(root) = slot else {
            self.session_id = sid.map(str::to_owned);
            if let Err(error) = self.hub.store.set_session(&self.cfg.id, sid) {
                tracing::warn!(bot = %self.cfg.id, error = format!("{error:#}"), "couldn't store the session");
            }
            return;
        };
        match sid {
            Some(s) => self.thread_sessions.insert(root.to_owned(), s.to_owned()),
            None => self.thread_sessions.remove(root),
        };
        let key = lane_key("session", &self.cfg.id, &Lane::in_thread(&self.cfg.id, root));
        if let Err(error) = self.hub.store.kv_set(&key, sid.unwrap_or_default()) {
            tracing::warn!(bot = %self.cfg.id, error = format!("{error:#}"), "couldn't store the thread session");
        }
    }

    /// Roots of the threads that have a session (stored or loaded).
    pub(super) fn thread_roots(&self) -> Vec<String> {
        let prefix = lane_key("session", &self.cfg.id, &Lane::main(&self.cfg.id)) + "/";
        let mut roots: Vec<String> = self.thread_sessions.keys().cloned().collect();
        roots.extend(
            self.hub.store.kv_prefix(&prefix).into_iter().filter_map(|k| k.strip_prefix(&prefix).map(str::to_owned)),
        );
        roots.sort();
        roots.dedup();
        roots
    }

    /// What a new thread session is told first: where the thread branches off.
    pub(super) fn thread_intro(&self, root: &str, forked: bool) -> String {
        let quote = |e: &Entry| {
            let who = if e.kind == EntryKind::User.as_str() { "the user" } else { "you" };
            format!("{who}: \"{}\"", acp::truncate(e.data["text"].as_str().unwrap_or_default(), 1500))
        };
        let Some(root_entry) = self.hub.store.entry(root) else {
            return "[Thread] The user opened a thread off your main chat. It is a separate branch of the conversation.".into();
        };
        if forked {
            return format!(
                "[Thread] The user opened a thread on this message from your main chat — {}. This thread is its own \
                 branch: it starts from everything said so far, and nothing said here reaches the main chat. \
                 Just continue the conversation; there's no need to mention this.",
                quote(&root_entry)
            );
        }
        // A fresh session: bring the lines leading up to the message along.
        let before = self.hub.store.history(&self.cfg.id, root_entry.seq, 12).unwrap_or_default();
        let lines: Vec<String> = before
            .iter()
            .chain(std::iter::once(&root_entry))
            .filter(|e| {
                e.kind == EntryKind::User.as_str() || (e.kind == EntryKind::Agent.as_str() && e.data["final"] == true)
            })
            .map(quote)
            .collect();
        format!(
            "[Thread] The user opened a thread on the last of these messages from your main chat (oldest first):\n{}\n\
             This thread is its own branch of the conversation; nothing said here reaches the main chat. \
             Just continue the conversation; there's no need to mention this.",
            lines.join("\n")
        )
    }

    /// What every session request carries: the folder and MCP servers, and the memory folders for an agent that
    /// takes folders besides its own.
    fn session_params(&self, servers: &Value, session: Option<&str>) -> Value {
        let mut params = json!({"cwd": self.cfg.cwd, "mcpServers": servers});
        if let Some(sid) = session {
            params["sessionId"] = sid.into();
        }
        if self.conn.as_ref().is_some_and(|c| c.sessions.additional_directories) {
            params["additionalDirectories"] = json!(memory::files::folders(&self.cfg));
        }
        params
    }

    /// ACP connectors, team collaboration, and optional computer control.
    pub(super) fn mcp_servers(&self) -> Result<Value> {
        let Ok(exe) = std::env::current_exe() else {
            tracing::warn!("can't locate codync-host for MCP servers");
            return Ok(json!([]));
        };
        let port = self.hub.port;
        let mut servers: Vec<Value> =
            (if self.cfg.connectors.is_empty() { Vec::new() } else { crate::market::connectors(&self.hub.store)? })
                .iter()
                .filter(|c| self.cfg.connectors.contains(&c.id))
                .filter_map(|c| c.acp(&exe, port))
                .collect();
        let composio = crate::market::composio::enabled_for(&self.hub.store, &self.cfg.connectors)?;
        let builtin = [
            Some("connectors"),
            Some("chat"),
            Some("team"),
            Some("memory"),
            Some("routines"),
            self.cfg.computer.then_some("computer"),
            composio.then_some("composio"),
        ];
        for name in builtin.into_iter().flatten() {
            let env = if name == "memory" {
                json!([{"name":"CODYNC_MEMORY_LANE", "value":self.slot(&self.lane).unwrap_or_default()}])
            } else {
                json!([])
            };
            servers.push(json!({
                "name": name, "command": exe,
                "args": ["mcp", name, "--bot", self.cfg.id, "--port", port.to_string()],
                "env": env,
            }));
        }
        Ok(Value::Array(servers))
    }

    /// Makes sure the agent runs and `slot`'s session (main, or a thread's) is live;
    /// sets `turn_session`. A thread without one forks the main session when it can.
    pub(super) async fn ensure_session(&mut self, slot: Option<&str>) -> Result<()> {
        // A cancelled allocation may have started tools without returning a session ID.
        // Resumable agents can be replaced without losing their confirmed conversations.
        if self.conn.as_ref().is_some_and(|conn| conn.allocation_uncertain && conn.sessions.load)
            && let Some(conn) = self.conn.take()
        {
            conn.acp.kill().await;
        }
        if !self.cfg.connectors.is_empty()
            || self.hub.store.kv_read(&format!("agent-env:{}", self.cfg.backend))?.is_some()
        {
            crate::market::vault::unlock(self.hub.clone()).await?;
        }
        // An agent that can't resume keeps its session; its next new session gets the new tools.
        if std::mem::take(&mut self.tools_changed)
            && self.conn.as_ref().is_some_and(|c| c.sessions.load)
            && let Some(c) = self.conn.take()
        {
            c.acp.kill().await;
        }
        if self.conn.is_none() {
            let (hub, id) = (self.hub.clone(), self.cfg.id.clone());
            let mut candidates = launch_commands(&self.cfg, move |msg: &str| {
                let msg = msg.to_owned();
                hub.set_runtime(&id, |r| r.activity = msg);
            })
            .await?;
            self.hub.set_runtime(&self.id(), |r| r.activity = "Starting agent…".into());
            // Keys saved from an "environment variable" sign-in.
            let mut env = crate::agent::auth::env(&self.hub.store, &self.cfg.backend)?;
            if let Some(ref account_id) = self.cfg.account {
                let acc_env = crate::agent::account::env(account_id, &self.cfg.backend)?;
                for (k, v) in acc_env {
                    env.retain(|(ek, _)| ek != &k);
                    env.push((k, v));
                }
            }
            own_memory::apply(&self.cfg, &mut env, &mut candidates).await;
            if self.cfg.backend == "codex" && !env.iter().any(|(k, _)| k == "CODEX_CONFIG") {
                // Codex's `/compact` takes no instructions: the keep-list is its config's `compact_prompt`.
                let config = json!({"compact_prompt": codync_mem::compaction::KEEP});
                env.push(("CODEX_CONFIG".into(), config.to_string()));
            }
            let mut last_err = None;
            for (i, command) in candidates.iter().enumerate() {
                let fallback_left = i + 1 < candidates.len();
                // A local CLI too old for ACP just hangs; don't make the user wait long before the fallback.
                let budget = Duration::from_secs(if fallback_left { 20 } else { 180 });
                match start_agent(command, &self.cfg.cwd, &env, budget).await {
                    Ok(conn) => {
                        self.conn = Some(conn);
                        break;
                    }
                    Err(e) => {
                        tracing::info!(command, error = format!("{e:#}"), "agent failed to start");
                        last_err = Some(e);
                    }
                }
            }
            if self.conn.is_none() {
                return Err(last_err.unwrap_or_else(|| anyhow!("agent failed to start")));
            }
        }
        let servers = self.mcp_servers()?;
        let instructions = self.conn.as_ref().map_or(Instructions::FirstMessage, Conn::instructions);
        if slot.is_some()
            && self.session_for(slot).is_none()
            && let Some(main) = self.session_id.clone()
            && self.conn.as_ref().is_some_and(|c| c.sessions.fork)
        {
            self.hub.set_runtime(&self.cfg.id, |r| r.activity = "Opening the thread…".into());
            let mut params = self.session_params(&servers, Some(&main));
            // The fork keeps the instructions its main session runs with, so it shares its cached prompt.
            let parent = self.snapshot(&main).await?;
            match instructions {
                Instructions::ClaudeMeta => params["_meta"] = claude_meta(&self.cfg, &parent.system),
                Instructions::SystemPrompt => params["systemPrompt"] = system_prompt(&parent.system),
                Instructions::FirstMessage => {}
            }
            let res = self.session_request("session/fork", params).await.ok();
            if let Some(res) = res
                && let Some(sid) =
                    res["sessionId"].as_str().filter(|sid| !sid.is_empty() && **sid != main).map(str::to_owned)
                && let Some(conn) = self.conn.as_mut()
            {
                conn.loaded.insert(sid.clone(), Instant::now());
                conn.allocation_uncertain = false;
                conn.pending_models.insert(sid.clone(), res.clone());
                self.set_session(slot, Some(&sid));
                self.note_options(&sid, &res, Listing::Session);
                context::fork(&self.hub.store, &self.cfg, &parent, &sid)?;
                memory::opened(&self.hub.store, &self.cfg.id, &sid, &parent.memory)?;
                if instructions == Instructions::ClaudeMeta {
                    self.applied_system.insert(sid, parent.system);
                }
            } else {
                tracing::warn!(bot = %self.cfg.id, "session/fork failed; the thread starts fresh");
            }
        }
        // Claude gets the frozen instructions as its system prompt. When a compaction
        // re-rendered them, loading the session again swaps the prompt in (the adapter
        // rebuilds its query and resumes the same conversation).
        let current = self.session_for(slot);
        let snapshot = match (&current, instructions) {
            (Some(sid), Instructions::ClaudeMeta) => Some(self.snapshot(sid).await?),
            _ => None,
        };
        let Some(conn) = self.conn.as_ref() else { bail!("agent not running") };
        if let Some(sid) = current {
            let prompt_current =
                snapshot.as_ref().is_none_or(|s| self.applied_system.get(&sid).is_some_and(|t| *t == s.system));
            if conn.loaded.contains_key(&sid) && prompt_current {
                self.finish_session_model(&sid).await?;
                self.turn_session = Some(sid);
                return Ok(());
            }
            if !conn.sessions.load {
                self.notice(
                    "This agent can't resume sessions after a restart — starting a fresh one.",
                    NoticeStyle::Divider,
                );
                self.set_session(slot, None);
                return Box::pin(self.ensure_session(slot)).await;
            }
            self.hub.set_runtime(&self.cfg.id, |r| r.activity = "Resuming session…".into());
            let mut params = self.session_params(&servers, Some(&sid));
            if let Some(s) = &snapshot {
                params["_meta"] = claude_meta(&self.cfg, &s.system);
            }
            if instructions == Instructions::SystemPrompt {
                params["systemPrompt"] = system_prompt(&self.snapshot(&sid).await?.system);
            }
            let res = self.session_request("session/load", params).await;
            let Some(conn) = self.conn.as_mut() else { return res.map(drop) };
            // The agent replays history as session/update before answering; drop it.
            while let Ok(inc) = conn.rx.try_recv() {
                if let Incoming::Request { id, .. } = inc {
                    let _ = conn.acp.respond_error(id, -32601, "not supported").await;
                }
            }
            match res {
                Ok(res) => {
                    // A resumed session can come back on the agent's last-used model (Cursor
                    // keeps one for all its sessions), so the bot's own is set again.
                    conn.loaded.insert(sid.clone(), Instant::now());
                    conn.allocation_uncertain = false;
                    if res.get("configOptions").is_some() || res.get("models").is_some() {
                        conn.pending_models.insert(sid.clone(), res.clone());
                    }
                    self.note_options(&sid, &res, Listing::Session);
                    if let Some(s) = snapshot {
                        // Its system prompt now names the bot and holds memory as rendered.
                        context::mark_announced(&self.hub.store, &self.cfg.id, &s, s.system_identity.clone())?;
                        memory::opened(&self.hub.store, &self.cfg.id, &sid, &s.memory)?;
                        self.applied_system.insert(sid.clone(), s.system);
                    }
                    self.finish_session_model(&sid).await?;
                    self.turn_session = Some(sid);
                    return Ok(());
                }
                Err(e) => {
                    tracing::warn!(error = format!("{e:#}"), "session/load failed; starting fresh");
                    self.thread_intro = None;
                    self.notice("Couldn't resume the previous session — starting a fresh one.", NoticeStyle::Divider);
                }
            }
        }
        let (revision, system) = {
            let (hub, cfg) = (self.hub.clone(), self.cfg.clone());
            tokio::task::spawn_blocking(move || {
                (memory::current_revision(&hub.store, &cfg.id), context::render(&hub.store, &cfg))
            })
            .await?
        };
        let mut params = self.session_params(&servers, None);
        match instructions {
            Instructions::ClaudeMeta => params["_meta"] = claude_meta(&self.cfg, &system),
            Instructions::SystemPrompt => params["systemPrompt"] = system_prompt(&system),
            Instructions::FirstMessage => {}
        }
        let res = self.session_request("session/new", params).await?;
        let Some(conn) = self.conn.as_mut() else { bail!("agent not running") };
        let sid = res["sessionId"]
            .as_str()
            .filter(|sid| !sid.is_empty())
            .ok_or_else(|| anyhow!("agent returned no sessionId"))?
            .to_owned();
        conn.loaded.insert(sid.clone(), Instant::now());
        conn.allocation_uncertain = false;
        conn.pending_models.insert(sid.clone(), res);
        self.set_session(slot, Some(&sid));
        context::adopt_new(&self.hub.store, &self.cfg, &sid, system.clone(), revision)?;
        if instructions == Instructions::ClaudeMeta {
            self.applied_system.insert(sid.clone(), system);
        }
        self.finish_session_model(&sid).await?;
        self.turn_session = Some(sid);
        Ok(())
    }

    /// Only allocation requests can leave tools behind without a confirmed session identity.
    pub(super) async fn session_request(&mut self, method: &str, params: Value) -> Result<Value> {
        let conn = self.conn.as_mut().ok_or_else(|| anyhow!("agent not running"))?;
        if conn.allocation_uncertain {
            bail!(
                "The agent did not confirm its previous allocation. Existing conversations are preserved; reset the affected session before opening another."
            );
        }
        conn.allocation_uncertain = true;
        let result = self.request_with_timeout(method, params).await;
        if result.is_err()
            && let Some(conn) = self.conn.as_mut()
        {
            conn.allocation_uncertain = false;
        }
        result
    }

    /// A hung request stops the agent so it cannot hold the bot indefinitely.
    pub(super) async fn request_with_timeout(&mut self, method: &str, params: Value) -> Result<Value> {
        let acp = self.conn.as_ref().ok_or_else(|| anyhow!("agent not running"))?.acp.clone();
        if let Ok(result) = tokio::time::timeout(SESSION_TIMEOUT, acp.request(method, params)).await {
            return result;
        }
        if let Some(c) = self.conn.take() {
            c.acp.kill().await;
        }
        let message = format!("the agent didn't answer {method} within {}s", SESSION_TIMEOUT.as_secs());
        Err(acp::Failure::error(acp::FailureKind::Timeout, message))
    }

    /// Retry model setup without allocating a second session after a partial setup failure.
    async fn finish_session_model(&mut self, sid: &str) -> Result<()> {
        let Some(conn) = self.conn.as_ref() else { bail!("agent not running") };
        let Some(response) = conn.pending_models.get(sid) else { return Ok(()) };
        let options = select_model(&conn.acp, response, sid, &self.cfg).await?;
        self.note_options(sid, &options, Listing::Session);
        if let Some(conn) = self.conn.as_mut() {
            conn.pending_models.remove(sid);
        }
        Ok(())
    }

    /// Drops the main ACP session so the next turn starts a fresh one.
    pub(super) fn forget_session(&mut self) {
        if self.conn.as_ref().is_some_and(|conn| conn.allocation_uncertain) {
            self.conn = None;
        }
        self.set_session(None, None);
    }
}

/// How long `session/new`, `session/load` and `session/fork` may take.
const SESSION_TIMEOUT: Duration = Duration::from_secs(180);
