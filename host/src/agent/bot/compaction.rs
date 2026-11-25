//! The bot's conversations as they fill up ([design](../../../../docs/design/bot-memory.md#compaction)): what the
//! agent reports about its context and its own compactions, and the compactions and rollovers the host starts. The
//! rules are `codync_mem::compaction`; this records what it needs and acts on what it decides.

use super::{Actor, Adapter, Done, Instructions, Queued};
use crate::chat::{context, handoff, memory};
use crate::store::{BotConfig, EntryKind, Lane, Store, now_ms, run_session_key};
use anyhow::{Result, anyhow};
use codync_mem::compaction::{self as rules, BeforeExpiry, CacheLife, Conversation, Decision, Threshold};
use serde_json::{Value, json};
use std::time::Duration;

/// Context use below which a fall isn't read as a compaction: a short session has little to summarize.
const COMPACTED_FROM: u64 = 20_000;

/// How long a compaction that could not start keeps being retried before the messages waiting
/// behind it run (OpenBot triage-15426).
const COMPACTION_RETRY_CAP: Duration = Duration::from_mins(10);
/// The plain words for the chat when a compaction fails; no jargon about epochs or agents.
pub(super) const COMPACTION_FAILED: &str = "Compaction failed, retrying.";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CompactionRetry {
    /// Still inside the ten-minute cap: let a later turn or idle tick try again.
    Retry,
    /// The cap passed: give up, report the failure and let queued messages run.
    GiveUp,
}

pub(super) fn compaction_retry(elapsed: Duration) -> CompactionRetry {
    if elapsed < COMPACTION_RETRY_CAP { CompactionRetry::Retry } else { CompactionRetry::GiveUp }
}

/// The per-bot "Compact before the cache expires" switch (OpenBot 0.77.0). On unless explicitly turned off.
pub fn before_expiry_key(bot: &str) -> String {
    format!("compact.beforeExpiry.{bot}")
}

/// Only an explicit `false` turns it off; anything else (an older host, a typo) leaves the default on.
fn parse_before_expiry(raw: Option<String>) -> bool {
    match raw.as_deref().map(str::trim) {
        Some("true") => true,
        Some("false") => false,
        _ => true,
    }
}

pub fn load_before_expiry(store: &Store, bot: &str) -> bool {
    parse_before_expiry(store.kv_get(&before_expiry_key(bot)))
}

pub fn save_before_expiry(store: &Store, bot: &str, enabled: bool) -> Result<()> {
    store.kv_set(&before_expiry_key(bot), &enabled.to_string())
}

/// Where the provider's one-hour prompt-cache setting lives (OpenBot 0.77.0: Claude and Pi, not just Claude).
fn provider_cache_key(backend: &str) -> String {
    format!("provider.promptCache.{backend}")
}

/// The setting wins over what a turn taught us; "providerDefault" and unknown values keep that learned life.
fn requested_life(setting: Option<&str>, fallback: CacheLife) -> CacheLife {
    if setting.is_some_and(|value| value.trim() == "oneHour") { CacheLife::OneHour } else { fallback }
}

/// The two choices the provider page offers; anything else reads as the default.
fn valid_prompt_cache(value: &str) -> bool {
    matches!(value.trim(), "oneHour" | "providerDefault")
}

pub fn load_prompt_cache(store: &Store, backend: &str) -> String {
    store
        .kv_get(&provider_cache_key(backend))
        .filter(|value| valid_prompt_cache(value))
        .unwrap_or_else(|| "providerDefault".to_owned())
}

pub fn save_prompt_cache(store: &Store, backend: &str, value: &str) -> Result<()> {
    if !valid_prompt_cache(value) {
        return Err(anyhow!("prompt cache setting must be oneHour or providerDefault"));
    }
    store.kv_set(&provider_cache_key(backend), value.trim())
}

/// Kinds of conversation the idle rule looks at (OpenBot 0.77.0: a chat, task or channel thread).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LaneKind {
    Chat,
    Task,
    Channel,
    /// A room turn or another bot's request: not a conversation the user comes back to.
    Busy,
}

/// Idle compaction belongs to a chat, task or channel thread. A chat or task waits for its queued message, but a
/// channel thread compacts while a message waits for its turn (OpenBot 0.77.0): a slow channel must not hold its
/// conversation's context open until the queue drains.
pub(super) fn lane_compactable(kind: LaneKind, queued: bool) -> bool {
    match kind {
        LaneKind::Channel => true,
        LaneKind::Chat | LaneKind::Task => !queued,
        LaneKind::Busy => false,
    }
}

/// A side conversation's own session, which `session::side_slot` also gives it.
fn channel_slot(slot: &str) -> bool {
    slot.starts_with("doc:")
        || ["slack-", "discord-", "whatsapp-", "native-channel-"].iter().any(|prefix| slot.starts_with(prefix))
}

/// What the chat says a compaction did (OpenBot 0.77.0 names the cache-expiry one).
fn compaction_notice_text(idle: bool) -> &'static str {
    if idle {
        "Context compacted before the prompt cache expired"
    } else {
        "Earlier context was summarized to make room."
    }
}

/// Pi has no `/compact`, so the keep-list goes as a raw summary prompt (OpenBot 0.77.0). Asking for a short line
/// instead of an error when there is nothing new keeps the no-op quiet.
fn pi_compaction_prompt() -> String {
    format!(
        "Summarize the conversation now so we can keep working with a shorter context. If there is nothing new to summarize, just say so briefly; that is not an error. {}",
        rules::KEEP
    )
}

/// Pi answers an unneeded compaction this way; it is a quiet success, not a failed or empty compaction
/// (OpenBot 0.77.0).
fn nothing_new(text: &str) -> bool {
    let text = text.trim().to_ascii_lowercase();
    text.contains("nothing to compact") || text.contains("already compacted")
}

/// One conversation the idle rule may compact, with the slot that routes a hidden turn back to its session.
struct IdleTarget {
    session: String,
    lane: Lane,
    slot: Option<String>,
    kind: LaneKind,
}

/// Whether Codync summarizes the bot's chat ahead of time, so the apps offer the user a [`Threshold`] for it: its
/// agent is Claude Code, Codex or Pi, or a custom command turned out to run one of their adapters.
pub fn offers_threshold(store: &Store, cfg: &BotConfig) -> bool {
    !cfg.is_group()
        && match harness_adapter(&cfg.backend) {
            Adapter::Other => cfg.backend == "custom" && context_window(store, &cfg.id).is_some(),
            _ => true,
        }
}

/// The adapter a backend runs, before its agent has answered. Pi compacts too (OpenBot 0.77.0).
fn harness_adapter(backend: &str) -> Adapter {
    match backend {
        "claude" => Adapter::Claude,
        "codex" => Adapter::Codex,
        "pi" => Adapter::Pi,
        _ => Adapter::Other,
    }
}

/// The user's threshold where Codync compacts the bot's chat: by the adapter running it, or with none running (a
/// custom command released or not started yet) by what its agent last reported. Elsewhere the default: the agent's
/// own compaction, and no rollovers.
fn threshold(store: &Store, cfg: &BotConfig, running: Option<Adapter>) -> Threshold {
    let compacts = running.map_or_else(|| offers_threshold(store, cfg), |adapter| adapter != Adapter::Other);
    if compacts { cfg.compaction } else { Threshold::default() }
}

fn window_key(bot: &str) -> String {
    format!("context.window.{bot}")
}

/// The context window of the bot's own chat, as its agent last reported it; kept only for an agent Codync compacts.
pub fn context_window(store: &Store, bot: &str) -> Option<u64> {
    store.kv_get(&window_key(bot)).and_then(|v| v.parse().ok())
}

impl Actor {
    fn conversation_key(&self, session: &str) -> String {
        format!("compaction.{}.{session}", self.cfg.id)
    }

    fn conversation(&self, session: &str) -> Conversation {
        self.hub
            .store
            .kv_get(&self.conversation_key(session))
            .and_then(|v| serde_json::from_str(&v).ok())
            .unwrap_or_default()
    }

    fn save_conversation(&self, session: &str, c: &Conversation) {
        let saved = serde_json::to_string(c)
            .map_err(anyhow::Error::from)
            .and_then(|v| self.hub.store.kv_set(&self.conversation_key(session), &v));
        if let Err(error) = saved {
            tracing::warn!(bot = %self.cfg.id, error = format!("{error:#}"), "couldn't record the conversation");
        }
    }

    fn cache_life_key(&self) -> String {
        format!("cache.life.{}", self.cfg.id)
    }

    /// How long the provider keeps this bot's prompts cached: the provider's one-hour setting first (OpenBot
    /// 0.77.0, Pi as well as Claude), then what a turn taught us, then a guess from Claude's sign-in.
    fn cache_life(&self) -> CacheLife {
        let learned = self.hub.store.kv_get(&self.cache_life_key()).and_then(|v| serde_json::from_str(&v).ok());
        let guessed = learned.unwrap_or_else(|| {
            let api_key = crate::agent::auth::saved_env(&self.hub.store, &self.cfg.backend, Some(&self.cfg.id))
                .unwrap_or_default()
                .iter()
                .any(|k| k == "ANTHROPIC_API_KEY")
                || std::env::var_os("ANTHROPIC_API_KEY").is_some();
            if self.adapter() == Adapter::Claude && !api_key { CacheLife::OneHour } else { CacheLife::FiveMinutes }
        });
        requested_life(Some(&load_prompt_cache(&self.hub.store, &self.cfg.backend)), guessed)
    }

    /// The "Compact before the cache expires" switch this bot was saved with.
    fn before_expiry(&self) -> BeforeExpiry {
        BeforeExpiry { enabled: load_before_expiry(&self.hub.store, &self.cfg.id) }
    }

    /// Whether a message waits for this bot's turn, including one steered into the running turn.
    fn message_waiting(&self) -> bool {
        !self.queue.is_empty() || !self.steers_in_flight.is_empty()
    }

    /// The conversation the running turn belongs to, for the idle rule.
    fn lane_kind(&self) -> LaneKind {
        if self.active_group.is_some() || self.active_request.is_some() || !self.perms.is_empty() {
            LaneKind::Busy
        } else if self.active_routine.is_some() {
            LaneKind::Task
        } else if self.lane.thread.is_some() {
            if self.side_slot(&self.lane).is_some() { LaneKind::Channel } else { LaneKind::Task }
        } else {
            LaneKind::Chat
        }
    }

    /// The main session is shared by the bot's chat and its task threads; only there do turn-end compactions
    /// and handoff cards belong.
    fn in_main_session(&self) -> bool {
        self.turn_session.is_some() && self.turn_session == self.session_id
    }

    /// Sessions with a recorded conversation: the main chat (and task threads in it), a routine run's session
    /// and each side conversation still open (OpenBot 0.77.0: chat, task or channel thread).
    fn idle_targets(&self) -> Vec<IdleTarget> {
        let mut targets = Vec::new();
        if let Some(session) = self.session_id.clone() {
            targets.push(IdleTarget { session, lane: Lane::main(&self.cfg.id), slot: None, kind: LaneKind::Chat });
        }
        let Some(conn) = &self.conn else { return targets };
        let prefix = run_session_key(&self.cfg.id, "");
        for key in self.hub.store.kv_prefix(&prefix) {
            let Some(slot) = key.strip_prefix(&prefix).map(str::to_owned) else { continue };
            let (kind, lane) = if channel_slot(&slot) {
                (LaneKind::Channel, Lane::in_thread(&self.cfg.id, &slot))
            } else if slot.starts_with("routine-") {
                (LaneKind::Task, Lane::main(&self.cfg.id))
            } else {
                continue;
            };
            let Some(session) = self.hub.store.kv_get(&key).filter(|sid| !sid.is_empty()) else { continue };
            if !conn.usage.contains_key(&session) {
                continue;
            }
            targets.push(IdleTarget { session, lane, slot: Some(slot), kind });
        }
        targets
    }

    /// The first conversation due for an idle compaction, re-decided when the hidden turn starts too.
    fn idle_due(&self) -> Option<IdleTarget> {
        let (queued, now) = (self.message_waiting(), now_ms());
        let (life, at, switch) = (self.cache_life(), self.threshold(), self.before_expiry());
        if self.compaction_prompt().is_none() {
            return None;
        }
        for target in self.idle_targets() {
            if !lane_compactable(target.kind, queued) {
                continue;
            }
            let c = self.settle_kept(&target.session);
            if c.last_turn > 0 && rules::while_idle(&c, at, life, switch, now) == Decision::Compact {
                return Some(target);
            }
        }
        None
    }

    /// The adapter the bot's agent runs; before it runs, its harness's.
    fn adapter(&self) -> Adapter {
        match self.conn.as_ref().map(|c| c.adapter) {
            // pi-acp still says `Other` at initialize; the backend tells us it can compact (OpenBot 0.77.0).
            Some(Adapter::Other) if self.cfg.backend == "pi" => Adapter::Pi,
            Some(adapter) => adapter,
            None => harness_adapter(&self.cfg.backend),
        }
    }

    /// The hidden turn's text: `/compact` where the harness takes one; Pi gets the same keep-list as a prompt
    /// (OpenBot 0.77.0). `None` for harnesses that compact on their own.
    fn compaction_prompt(&self) -> Option<String> {
        match self.adapter() {
            Adapter::Claude => Some(format!("/compact {}", rules::KEEP)),
            Adapter::Codex => Some("/compact".into()),
            Adapter::Pi => Some(pi_compaction_prompt()),
            Adapter::Other => None,
        }
    }

    fn threshold(&self) -> Threshold {
        threshold(&self.hub.store, &self.cfg, self.conn.as_ref().map(|c| c.adapter))
    }

    /// Keeps the main chat's window for the apps, which offer sizes up to what it holds; none for an agent Codync
    /// doesn't compact. Only the bot's own agent tells it: the sizes are for that agent's window, not a fallback's.
    fn note_window(&self, size: u64) {
        if self.cfg.backend != self.base.backend {
            return;
        }
        let window = match (self.adapter(), size) {
            (Adapter::Other, _) => None,
            (_, 0) => return,
            (_, size) => Some(size),
        };
        if context_window(&self.hub.store, &self.cfg.id) == window {
            return;
        }
        let key = window_key(&self.cfg.id);
        let saved = match window {
            Some(size) => self.hub.store.kv_set(&key, &size.to_string()),
            None => self.hub.store.kv_delete(&key),
        };
        if let Err(error) = saved {
            tracing::warn!(bot = %self.cfg.id, error = format!("{error:#}"), "couldn't record the window");
            return;
        }
        self.hub.emit_bot(&self.cfg.id);
    }

    /// The agent reported its context use.
    pub(super) fn track_usage(&mut self, used: u64, size: u64) {
        let Some(sid) = self.turn_session.clone() else { return };
        let Some(conn) = self.conn.as_mut() else { return };
        let before = conn.usage.insert(sid, (used, size)).map(|(used, _)| used);
        if !conn.reports_compaction && compacted_by_usage(before, used) {
            self.compacted(&format!("usage-{}", now_ms()), None);
        }
    }

    /// The agent summarized its context: the next turn gets freshly rendered instructions and memory (see
    /// `context`), a harness that got them in its first message gets them again, and the chat shows the summary.
    pub(super) fn compacted(&mut self, id: &str, summary: Option<String>) {
        let Some(sid) = self.turn_session.clone() else { return };
        match context::bump_epoch(&self.hub.store, &self.cfg.id, &sid, id) {
            Ok(true) => {}
            // The same compaction reported again; Claude's adapter carries the summary on one of its reports.
            Ok(false) => {
                if let Some(summary) = summary {
                    self.keep_summary(&sid, &summary);
                }
                return;
            }
            Err(error) => {
                tracing::warn!(bot = %self.cfg.id, error = format!("{error:#}"), "couldn't record the compaction");
                return;
            }
        }
        if let Err(error) = memory::compacted(&self.hub.store, &self.cfg.id, &sid) {
            tracing::warn!(bot = %self.cfg.id, error = format!("{error:#}"), "couldn't record the compaction");
        }
        if self.conn.as_ref().is_some_and(|c| c.instructions() == Instructions::FirstMessage) {
            self.profile_resend.insert(sid.clone());
        }
        let mut c = self.conversation(&sid);
        c.compacted(self.compacting.is_some(), self.compacting == Some(true), now_ms());
        self.save_conversation(&sid, &c);
        let _ = self.dreamer.send(memory::DreamEvent::Flush);
        // OpenBot 0.77.0: "Nothing to compact" is a quiet success, not a compaction line in the chat.
        if summary.as_deref().is_some_and(nothing_new) {
            return;
        }
        let data = json!({
            "text": compaction_notice_text(self.compacting == Some(true)),
            "style": "info",
            "compaction": {"summary": Value::Null, "byHost": self.compacting.is_some()},
        });
        self.compaction_notice = self.add(EntryKind::Notice, self.turn.unwrap_or(0), data).map(|e| e.id);
        if let Some(summary) = summary {
            self.keep_summary(&sid, &summary);
        }
    }

    /// The harness's summary: shown on the compaction's line in the chat, and kept for a later handoff.
    fn keep_summary(&mut self, session: &str, summary: &str) {
        if let Err(error) = self.hub.store.kv_set(&handoff::summary_key(&self.cfg.id, session), summary) {
            tracing::warn!(bot = %self.cfg.id, error = format!("{error:#}"), "couldn't keep the summary");
        }
        if let Some(id) = &self.compaction_notice
            && let Some(mut entry) = self.hub.store.entry(id)
        {
            entry.data["compaction"]["summary"] = summary.into();
            self.hub.set_entry(id, &entry.data);
        }
    }

    /// After a turn in a chat, task thread or channel thread: record it, learn the cache's life from it, and compact
    /// or prepare a rollover in the bot's own session when that's due.
    pub(super) fn after_turn(&mut self, done: &Result<Value>) {
        let compacting = self.compacting.take();
        if let Ok(answer) = done
            && answer["usage"].is_object()
        {
            // How well the prompt cache held: the first thing to look at when a turn costs more than it should.
            let usage = &answer["usage"];
            self.count_spend(usage);
            // The turn trace: "Tokens" is the fresh spend, not cache reads; those
            // are reported apart as "Cached input" (OpenBot triage-7101).
            let counts = crate::usage::tokens::split(usage);
            tracing::info!(
                bot = %self.cfg.id,
                session = self.turn_session.as_deref().unwrap_or_default(),
                tokens = counts.new,
                cached_input = counts.cached_input,
                cached_read = usage["cachedReadTokens"].as_u64().unwrap_or(0),
                cached_write = usage["cachedWriteTokens"].as_u64().unwrap_or(0),
                input = usage["inputTokens"].as_u64().unwrap_or(0),
                "turn usage"
            );
        }
        let Some(sid) = self.turn_session.clone() else { return };
        let now = now_ms();
        let mut c = self.conversation(&sid);
        if let Some(&(used, size)) = self.conn.as_ref().and_then(|conn| conn.usage.get(&sid)) {
            c.usage(used, size);
            self.note_window(size);
        }
        // The compaction turn itself is no turn of the conversation: it neither counts nor resets the idle clock.
        // Mark an idle one done even when the harness never reports its compaction, so it isn't queued again.
        if compacting.is_some() {
            c.idle_compacted |= compacting == Some(true);
            self.save_conversation(&sid, &c);
            return;
        }
        if !lane_compactable(self.lane_kind(), self.message_waiting()) {
            return;
        }
        if let Ok(answer) = done
            && let Some(life) = learned_life(answer, now - c.last_turn)
            && let Err(error) = self.hub.store.kv_set(&self.cache_life_key(), &json!(life).to_string())
        {
            tracing::warn!(bot = %self.cfg.id, error = format!("{error:#}"), "couldn't record the cache's life");
        }
        c.turn_ended(now);
        self.save_conversation(&sid, &c);
        match rules::at_turn_end(&c, self.threshold(), now) {
            Decision::Compact if self.in_main_session() && self.compaction_prompt().is_some() => {
                self.queue.push_back(Queued::Compact { idle: false });
            }
            Decision::Rollover if self.in_main_session() => self.draft_handoff(&sid, handoff::Reason::Full),
            _ => {}
        }
    }

    /// While the bot is at rest: compact a chat, task or channel thread before an hour-long cache expires, and put
    /// a handoff in the main chat for the user to review when the next message should start a new conversation.
    pub(super) fn tend_idle(&mut self) {
        if self.idle_due().is_some() {
            self.queue.push_back(Queued::Compact { idle: true });
            return;
        }
        let Some(sid) = self.session_id.clone() else { return };
        let c = self.settle_kept(&sid);
        if c.last_turn == 0 || !lane_compactable(LaneKind::Chat, self.message_waiting()) {
            return;
        }
        let (now, life, at) = (now_ms(), self.cache_life(), self.threshold());
        if rules::rollover_due(&c, at, life, now) && handoff::pending(&self.hub.store, &self.cfg.id).is_none() {
            let reason = if now - c.last_turn >= life.ms() { handoff::Reason::Cold } else { handoff::Reason::Away };
            self.draft_handoff(&sid, reason);
        }
    }

    /// The conversation, with the user's choice to carry on over its last handoff card recorded.
    fn settle_kept(&self, session: &str) -> Conversation {
        let mut c = self.conversation(session);
        if handoff::kept(&self.hub.store, &self.cfg.id) {
            c.rollover_declined = true;
            self.save_conversation(session, &c);
        }
        c
    }

    /// The user changed when the chat is summarized: a fresh conversation the new setting doesn't call for is
    /// withdrawn, and the old one goes on.
    pub(super) fn threshold_changed(&self) {
        let Some(sid) = &self.session_id else { return };
        let Some((card, _)) = handoff::pending(&self.hub.store, &self.cfg.id) else { return };
        let (c, at, now) = (self.conversation(sid), self.threshold(), now_ms());
        if !rules::rollover_due(&c, at, self.cache_life(), now) && rules::at_turn_end(&c, at, now) != Decision::Rollover
        {
            handoff::withdraw(&self.hub, &self.cfg.id, &card);
        }
    }

    fn draft_handoff(&mut self, session: &str, reason: handoff::Reason) {
        if handoff::pending(&self.hub.store, &self.cfg.id).is_some() {
            return;
        }
        if let Err(error) = handoff::draft(&self.hub, &self.cfg, session, reason) {
            tracing::warn!(bot = %self.cfg.id, error = format!("{error:#}"), "couldn't prepare the handoff");
        }
    }

    /// Runs a compaction the host decided on as a hidden turn in the conversation that needs it. Dreaming goes first,
    /// and the opening is rendered again, so the summary keeps current memory (render, load, compact).
    pub(super) async fn start_compaction(&mut self, idle: bool, done_tx: &tokio::sync::mpsc::UnboundedSender<Done>) {
        let Some(prompt) = self.compaction_prompt() else { return };
        let target = if idle {
            let Some(target) = self.idle_due() else { return };
            target
        } else {
            let Some(session) = self.session_id.clone() else { return };
            IdleTarget { session, lane: Lane::main(&self.cfg.id), slot: None, kind: LaneKind::Chat }
        };
        // Decided when it was queued; another turn or a change of the setting may have come first.
        let (c, at, now) = (self.conversation(&target.session), self.threshold(), now_ms());
        let decision = if idle {
            rules::while_idle(&c, at, self.cache_life(), self.before_expiry(), now)
        } else {
            rules::at_turn_end(&c, at, now)
        };
        if decision != Decision::Compact {
            return;
        }
        crate::analytics::context_compacted(&self.hub, &self.cfg.backend, idle, at);
        let _ = self.dreamer.send(memory::DreamEvent::Flush);
        if let Err(error) = context::refresh(&self.hub.store, &self.cfg.id, &target.session) {
            tracing::warn!(bot = %self.cfg.id, error = format!("{error:#}"), "couldn't refresh the opening");
        }
        // A routine run's session is chosen by its slot; side conversations by their lane. Restore the actor's own
        // turn state as soon as the hidden turn is on its way (OpenBot 0.77.0: task and channel threads compact too).
        let routine = target.slot.as_deref().and_then(|slot| slot.strip_prefix("routine-")).map(str::to_owned);
        let previous = self.active_routine.take();
        if let Some(id) = routine {
            self.active_routine = Some(id);
        }
        self.compacting = Some(idle);
        self.raw_prompt = true;
        let started = self.start_turn(target.lane, &[], &prompt, true, done_tx).await;
        self.active_routine = previous;
        if let Err(e) = started {
            self.compacting = None;
            // A failed compaction does not record itself as done, so it stays due. Say so plainly and
            // let a later turn or idle tick try again, but only for ten minutes (triage-15426).
            let retry_key = format!("compaction.retry.{}", self.cfg.id);
            let first = self.hub.store.kv_get(&retry_key).and_then(|v| v.parse::<i64>().ok());
            let began = first.unwrap_or_else(now_ms);
            let _ = self.hub.store.kv_set(&retry_key, &began.to_string());
            let elapsed = Duration::from_millis(u64::try_from(now_ms().saturating_sub(began)).unwrap_or(u64::MAX));
            if compaction_retry(elapsed) == CompactionRetry::Retry {
                if first.is_none() {
                    let _ = self.add(
                        EntryKind::Notice,
                        self.turn.unwrap_or(0),
                        json!({"text": COMPACTION_FAILED, "style": "info"}),
                    );
                }
            } else {
                let _ = self.hub.store.kv_delete(&retry_key);
                self.start_failed(&e);
            }
        }
    }

    /// Before the user's message in the main chat: a handoff waiting for it starts a new conversation. Returns the
    /// handoff to put before the message.
    pub(super) fn take_handoff(&mut self) -> Option<String> {
        // A rollover that came due since the last tick gets its card now, without the chance to edit it.
        if let Some(sid) = self.session_id.clone()
            && handoff::pending(&self.hub.store, &self.cfg.id).is_none()
        {
            let c = self.settle_kept(&sid);
            let life = self.cache_life();
            if rules::rollover_due(&c, self.threshold(), life, now_ms()) {
                let reason =
                    if now_ms() - c.last_turn >= life.ms() { handoff::Reason::Cold } else { handoff::Reason::Away };
                self.draft_handoff(&sid, reason);
            }
        }
        let (id, text) = handoff::pending(&self.hub.store, &self.cfg.id)?;
        handoff::sent(&self.hub, &self.cfg.id, &id);
        self.forget_session();
        Some(text)
    }
}

/// What a turn's prompt usage says about the cache's life, `gap_ms` after the previous turn.
fn learned_life(answer: &Value, gap_ms: i64) -> Option<CacheLife> {
    let usage = &answer["usage"];
    let read = usage["cachedReadTokens"].as_u64()?;
    let input = read + usage["cachedWriteTokens"].as_u64().unwrap_or(0) + usage["inputTokens"].as_u64().unwrap_or(0);
    CacheLife::observed(gap_ms, read, input)
}

/// Whether a session's context use fell the way it does when the agent summarizes it: by more than half.
fn compacted_by_usage(before: Option<u64>, used: u64) -> bool {
    before.is_some_and(|before| before >= COMPACTED_FROM && used < before / 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_halved_context_reads_as_a_compaction() {
        assert!(compacted_by_usage(Some(90_000), 12_000));
        assert!(!compacted_by_usage(Some(90_000), 60_000), "ordinary churn");
        assert!(!compacted_by_usage(Some(8_000), 1_000), "too little to summarize");
        assert!(!compacted_by_usage(None, 1_000), "the session's first report");
    }

    #[test]
    fn a_custom_command_keeps_its_threshold_while_no_agent_runs() {
        let path = std::env::temp_dir().join(format!("codync-threshold-{}.db", uuid::Uuid::new_v4()));
        let store = Store::open(&path).unwrap();
        let cfg: BotConfig =
            serde_json::from_value(json!({"id": "b", "name": "B", "backend": "custom", "compaction": "nearLimit"}))
                .unwrap();
        assert_eq!(threshold(&store, &cfg, None), Threshold::Off, "not known to run Claude's or Codex's adapter yet");
        store.kv_set(&window_key("b"), "1000000").unwrap();
        assert_eq!(threshold(&store, &cfg, None), Threshold::NearLimit, "released after it reported its window");
        assert_eq!(threshold(&store, &cfg, Some(Adapter::Claude)), Threshold::NearLimit);
        assert_eq!(threshold(&store, &cfg, Some(Adapter::Other)), Threshold::Off, "now running another adapter");
        let cold = Conversation { used: 150_000, size: 1_000_000, last_turn: 1, ..Conversation::default() };
        assert!(!rules::rollover_due(&cold, threshold(&store, &cfg, None), CacheLife::FiveMinutes, 2 * 3_600_000));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn a_turn_after_a_pause_tells_the_cache_life() {
        let warm = json!({"usage": {"inputTokens": 2, "cachedReadTokens": 40_000, "cachedWriteTokens": 300}});
        assert_eq!(learned_life(&warm, 20 * 60_000), Some(CacheLife::OneHour));
        let cold = json!({"usage": {"inputTokens": 2, "cachedReadTokens": 0, "cachedWriteTokens": 40_000}});
        assert_eq!(learned_life(&cold, 20 * 60_000), Some(CacheLife::FiveMinutes));
        assert_eq!(learned_life(&json!({"stopReason": "end_turn"}), 20 * 60_000), None, "no usage reported");
    }

    #[test]
    fn pi_is_a_compactor_and_the_provider_one_hour_setting_wins() {
        assert_eq!(harness_adapter("pi"), Adapter::Pi);
        let prompt = pi_compaction_prompt();
        assert!(prompt.contains(rules::KEEP), "the same keep-list as the slash commands");
        assert!(prompt.contains("not an error"), "nothing new stays quiet");
        assert_eq!(requested_life(Some("oneHour"), CacheLife::FiveMinutes), CacheLife::OneHour);
        assert_eq!(requested_life(Some("providerDefault"), CacheLife::FiveMinutes), CacheLife::FiveMinutes);
        assert_eq!(requested_life(None, CacheLife::OneHour), CacheLife::OneHour, "learned behavior stays");
        assert_eq!(provider_cache_key("pi"), "provider.promptCache.pi");
        assert_eq!(provider_cache_key("claude"), "provider.promptCache.claude");
        let path = std::env::temp_dir().join(format!("codync-prompt-cache-{}.db", uuid::Uuid::new_v4()));
        let store = Store::open(&path).unwrap();
        assert_eq!(load_prompt_cache(&store, "pi"), "providerDefault");
        save_prompt_cache(&store, "pi", "oneHour").unwrap();
        assert_eq!(load_prompt_cache(&store, "pi"), "oneHour");
        assert!(save_prompt_cache(&store, "pi", "later").is_err(), "only the two choices are valid");
        assert_eq!(requested_life(Some(&load_prompt_cache(&store, "pi")), CacheLife::FiveMinutes), CacheLife::OneHour);
        assert_eq!(load_prompt_cache(&store, "claude"), "providerDefault", "settings are per provider");
        std::fs::remove_file(&path).ok();
        assert!(channel_slot("slack-abc") && channel_slot("doc:d1"));
        assert!(!channel_slot("routine-1"), "a routine run is a task lane, not a channel thread");
    }

    #[test]
    fn only_a_chat_task_or_channel_thread_compacts_while_idle() {
        for kind in [LaneKind::Chat, LaneKind::Task, LaneKind::Channel] {
            assert!(lane_compactable(kind, false), "{kind:?}");
        }
        assert!(!lane_compactable(LaneKind::Chat, true), "a waiting chat message goes first");
        assert!(!lane_compactable(LaneKind::Task, true), "a waiting task goes first");
        assert!(lane_compactable(LaneKind::Channel, true), "a channel compacts while a message waits");
        assert!(!lane_compactable(LaneKind::Busy, false), "a room or a request is not our conversation");
    }

    #[test]
    fn the_switch_is_on_unless_it_is_explicitly_off() {
        assert!(parse_before_expiry(None));
        assert!(parse_before_expiry(Some("true".into())));
        assert!(!parse_before_expiry(Some("false".into())));
        assert!(parse_before_expiry(Some("later".into())), "a value we don't know keeps the default");
        let path = std::env::temp_dir().join(format!("codync-before-expiry-{}.db", uuid::Uuid::new_v4()));
        let store = Store::open(&path).unwrap();
        assert_eq!(before_expiry_key("b"), "compact.beforeExpiry.b");
        assert!(load_before_expiry(&store, "b"));
        save_before_expiry(&store, "b", false).unwrap();
        assert!(!load_before_expiry(&store, "b"));
        save_before_expiry(&store, "b", true).unwrap();
        assert!(load_before_expiry(&store, "b"));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn nothing_new_to_compact_is_a_quiet_success() {
        assert!(nothing_new("Nothing to compact (session too small)"));
        assert!(nothing_new(" Already compacted "));
        assert!(!nothing_new("Summarized the open tasks."));
        assert_eq!(compaction_notice_text(true), "Context compacted before the prompt cache expired");
        assert_eq!(compaction_notice_text(false), "Earlier context was summarized to make room.");
    }

    #[test]
    fn a_failed_compaction_retries_for_ten_minutes_then_reports_it() {
        assert_eq!(compaction_retry(Duration::ZERO), CompactionRetry::Retry);
        assert_eq!(compaction_retry(COMPACTION_RETRY_CAP - Duration::from_millis(1)), CompactionRetry::Retry);
        assert_eq!(compaction_retry(COMPACTION_RETRY_CAP), CompactionRetry::GiveUp);
        assert_eq!(compaction_retry(COMPACTION_RETRY_CAP * 2), CompactionRetry::GiveUp);
        assert_eq!(COMPACTION_FAILED, "Compaction failed, retrying.");
    }
}
