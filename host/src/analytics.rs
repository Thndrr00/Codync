//! Product analytics (`PostHog`), on unless the computer's owner turned it off
//! (`setAnalytics`). Events name the feature that was used and never carry content:
//! no message text, prompts, file names, paths or bot names.
//! What is collected and why: docs/features/analytics.md.

use crate::LockExt as _;
use crate::api::devices::Caller;
use crate::hub::Hub;
use crate::store::Store;
use anyhow::Result;
use serde::Deserialize;
use serde_json::{Map, Value, json};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};

pub mod errors;
pub(crate) mod setup;

const KV_ENABLED: &str = "analytics_enabled";
/// The account the install id was last merged into (one `$identify` per account).
const KV_IDENTIFIED: &str = "analytics_identified";
/// Write-only project token: safe to ship in public builds.
const TOKEN: &str = "phc_rQHvxdSFKnMquPfMjmiMhXhM4tqJ5qCYsWuVEoFxCCWX";
const BATCH_URL: &str = "https://us.i.posthog.com/batch/";
const FLUSH_EVERY: Duration = Duration::from_secs(30);
/// Everything the host reports. Names follow the `object_verb` convention.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Event {
    AppOpened,
    OnboardingCompleted,
    OnboardingStepViewed,
    OnboardingAbandoned,
    SetupReady,
    SetupActivated,
    SetupFailed,
    SignedIn,
    SignedOut,
    BotCreated,
    BotUpdated,
    BotsReordered,
    BotDeleted,
    MessageSent,
    TurnStopped,
    TurnFinished,
    SessionReset,
    PermissionAnswered,
    RoutineSaved,
    RoutineDeleted,
    RoutineRun,
    VoiceCallFinished,
    RemoteScreenOpened,
    ConnectorInstalled,
    SkillInstalled,
    /// Error tracking (`errors`): a panic or an uncaught error.
    Exception,
}

impl Event {
    fn name(self) -> &'static str {
        match self {
            Self::AppOpened => "app_opened",
            Self::OnboardingCompleted => "onboarding_completed",
            Self::OnboardingStepViewed => "onboarding_step_viewed",
            Self::OnboardingAbandoned => "onboarding_abandoned",
            Self::SetupReady => "setup_ready",
            Self::SetupActivated => "setup_activated",
            Self::SetupFailed => "setup_failed",
            Self::SignedIn => "signed_in",
            Self::SignedOut => "signed_out",
            Self::BotCreated => "bot_created",
            Self::BotUpdated => "bot_updated",
            Self::BotsReordered => "bots_reordered",
            Self::BotDeleted => "bot_deleted",
            Self::MessageSent => "message_sent",
            Self::TurnStopped => "turn_stopped",
            Self::TurnFinished => "turn_finished",
            Self::SessionReset => "session_reset",
            Self::PermissionAnswered => "permission_answered",
            Self::RoutineSaved => "routine_saved",
            Self::RoutineDeleted => "routine_deleted",
            Self::RoutineRun => "routine_run",
            Self::VoiceCallFinished => "voice_call_finished",
            Self::RemoteScreenOpened => "remote_screen_opened",
            Self::ConnectorInstalled => "connector_installed",
            Self::SkillInstalled => "skill_installed",
            Self::Exception => "$exception",
        }
    }

    /// Events the computer's own apps report through `track` (the rest come from API calls).
    fn client_reported(self) -> bool {
        matches!(
            self,
            Self::AppOpened
                | Self::OnboardingCompleted
                | Self::OnboardingStepViewed
                | Self::OnboardingAbandoned
                | Self::SignedIn
                | Self::SignedOut
        )
    }
}

pub struct Analytics {
    enabled: Mutex<bool>,
    consent: tokio::sync::watch::Sender<bool>,
    tx: mpsc::UnboundedSender<Msg>,
    rx: Mutex<Option<mpsc::UnboundedReceiver<Msg>>>,
}

enum Msg {
    /// A panic, captured off the panicking thread (`errors::report_panics`).
    Report(errors::Report),
    /// Send what's queued now and answer when done.
    Flush(oneshot::Sender<()>),
}

impl Analytics {
    pub fn load(store: &Store) -> Self {
        let enabled = match store.kv_read(KV_ENABLED) {
            Ok(value) => value.as_deref() != Some("0"),
            Err(error) => {
                tracing::warn!(%error, "Could not read analytics consent; reporting disabled");
                false
            }
        };
        let (tx, rx) = mpsc::unbounded_channel();
        let (consent, _) = tokio::sync::watch::channel(enabled);
        Self { enabled: Mutex::new(enabled), consent, tx, rx: Mutex::new(Some(rx)) }
    }

    pub fn state(&self) -> bool {
        *self.enabled.locked()
    }

    pub fn set_enabled(&self, store: &Store, on: bool) -> Result<()> {
        let mut enabled = self.enabled.locked();
        store.set_analytics_enabled(on)?;
        *enabled = on;
        self.consent.send_replace(on);
        Ok(())
    }
}

/// `track`: an event the computer's own app reports (app opened, signed in, …).
pub fn track(hub: &Hub, b: &Value) -> Result<()> {
    let event = Event::deserialize(&b["event"]).map_err(|_| anyhow::anyhow!("unknown event"))?;
    anyhow::ensure!(event.client_reported(), "unknown event");
    if matches!(event, Event::OnboardingStepViewed | Event::OnboardingAbandoned) {
        let step = b["step"].as_str().unwrap_or_default();
        anyhow::ensure!(
            matches!(step, "welcome" | "account" | "computer_access" | "pairing" | "ready"),
            "Unknown onboarding step"
        );
        let id = uuid::Uuid::parse_str(b["clientEventId"].as_str().unwrap_or_default())?;
        if hub.analytics.state() {
            let account = hub.cloud.status().owner.map(|o| o.user_id);
            queue_identified(
                hub,
                event.name(),
                account.as_deref().unwrap_or(&hub.host_id),
                json!({"step":step}).as_object().cloned().unwrap_or_default(),
                Some(id),
            )?;
        }
    } else {
        capture(hub, event, Map::new());
    }
    Ok(())
}

/// The event an API call stands for, with its properties, read before the call runs.
pub fn observe(caller: &Caller, method: &str, b: &Value) -> Option<(Event, Map<String, Value>)> {
    let mut p = Map::new();
    let event = match method {
        "createBot" => {
            let group = b["kind"] == "group";
            p.insert("kind".into(), (if group { "group" } else { "bot" }).into());
            if group {
                p.insert("members".into(), b["members"].as_array().map_or(0, Vec::len).into());
            } else {
                p.insert("agent".into(), b["backend"].clone());
                p.insert("model".into(), b["model"].clone());
            }
            Event::BotCreated
        }
        "updateBot" => Event::BotUpdated,
        "reorderBots" => Event::BotsReordered,
        "deleteBot" => Event::BotDeleted,
        "send" => {
            let in_thread = b["threadId"].as_str().is_some_and(|t| !t.is_empty());
            p.insert("in_thread".into(), in_thread.into());
            p.insert("attachments".into(), b["attachments"].as_array().map_or(0, Vec::len).into());
            Event::MessageSent
        }
        "stop" => Event::TurnStopped,
        "newSession" => Event::SessionReset,
        "respondPermission" => Event::PermissionAnswered,
        "saveRoutine" => Event::RoutineSaved,
        "deleteRoutine" => Event::RoutineDeleted,
        "runRoutine" => Event::RoutineRun,
        "logCall" => {
            p.insert("seconds".into(), b["seconds"].as_u64().unwrap_or(0).into());
            Event::VoiceCallFinished
        }
        "screenPrepare" => Event::RemoteScreenOpened,
        "installConnector" => {
            // A catalog name is public; a custom connector's URL or command is not.
            let catalog = b["registryName"].as_str().unwrap_or("custom");
            p.insert("connector".into(), catalog.into());
            Event::ConnectorInstalled
        }
        "installSkill" => {
            p.insert("custom".into(), (!b["source"].is_string()).into());
            Event::SkillInstalled
        }
        _ => return None,
    };
    let client = if matches!(caller, Caller::Local) { "local" } else { "remote" };
    p.insert("client".into(), client.into());
    Some((event, p))
}

/// How an agent's turn ended (`turn_finished`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TurnOutcome {
    /// `end_turn`: the agent finished.
    Done,
    /// The user stopped it.
    Stopped,
    /// The agent process or connection failed.
    Failed,
    /// It ended short of `end_turn` (output or step limit, refusal).
    Incomplete,
}

/// What started the turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TurnSource {
    User,
    Routine,
    Group,
    Bot,
}

/// `turn_finished`: the agent, how the turn ended, why it failed and how long it ran;
/// never its text or the error's.
pub fn turn_finished(
    hub: &Hub,
    agent: &str,
    outcome: TurnOutcome,
    stop_reason: &str,
    failure: Option<&anyhow::Error>,
    source: TurnSource,
    ms: i64,
) {
    let outcome = match outcome {
        TurnOutcome::Done => "done",
        TurnOutcome::Stopped => "stopped",
        TurnOutcome::Failed => "failed",
        TurnOutcome::Incomplete => "incomplete",
    };
    // ACP's stop reasons plus ours; anything else an adapter invents is just `other`.
    let stop_reason = match stop_reason {
        "end_turn" | "cancelled" | "max_tokens" | "max_turn_requests" | "refusal" | "error" | "start_failed" => {
            stop_reason
        }
        _ => "other",
    };
    let source = match source {
        TurnSource::User => "user",
        TurnSource::Routine => "routine",
        TurnSource::Group => "group",
        TurnSource::Bot => "bot",
    };
    let mut p = json!({"agent": agent, "outcome": outcome, "stop_reason": stop_reason, "source": source, "seconds": ms.max(0) / 1000});
    if let Some(e) = failure {
        let (kind, rpc) = failure_kind(e);
        p["error"] = kind.into();
        if let Some(rpc) = rpc {
            p["rpc_code"] = rpc.code.into();
            if let Some(reason) = &rpc.kind {
                p["agent_reason"] = reason.as_str().into();
            }
        }
    }
    capture(hub, Event::TurnFinished, p.as_object().cloned().unwrap_or_default());
}

/// Why a turn failed, as a category (plus the agent's JSON-RPC error, for its code and own
/// category): never the error's text, which can hold paths and agent output.
fn failure_kind(e: &anyhow::Error) -> (&'static str, Option<&crate::agent::acp::RpcError>) {
    use crate::agent::acp::{Failure, FailureKind, RpcError};
    if let Some(f) = e.chain().find_map(|c| c.downcast_ref::<Failure>()) {
        let kind = match f.kind {
            FailureKind::NotSetUp => "not_installed",
            FailureKind::Timeout => "timeout",
            FailureKind::Exited => "exited",
        };
        return (kind, None);
    }
    if crate::agent::acp::needs_sign_in(e) {
        return ("sign_in", None);
    }
    for cause in e.chain() {
        if let Some(r) = cause.downcast_ref::<RpcError>() {
            return ("agent_error", Some(r));
        }
        if let Some(io) = cause.downcast_ref::<std::io::Error>() {
            return (if io.kind() == std::io::ErrorKind::NotFound { "not_installed" } else { "io" }, None);
        }
    }
    ("other", None)
}

/// `observe` plus what only the store knows: a resent message (same `clientNonce`) is
/// not a new one, and a message names its bot's agent.
pub fn observe_call(hub: &Hub, caller: &Caller, method: &str, b: &Value) -> Option<(Event, Map<String, Value>)> {
    let (event, mut p) = observe(caller, method, b)?;
    if event == Event::MessageSent {
        let bot = b["botId"].as_str().unwrap_or_default();
        let nonce = b["clientNonce"].as_str().unwrap_or_default();
        if !nonce.is_empty() && hub.store.find_by_nonce(bot, nonce).is_some() {
            return None;
        }
        if let Ok(Some(row)) = hub.store.bot(bot) {
            p.insert("kind".into(), (if row.config.is_group() { "group" } else { "bot" }).into());
            if !row.config.is_group() {
                p.insert("agent".into(), row.config.backend.into());
            }
        }
    }
    Some((event, p))
}

/// Queues one event unless the owner turned analytics off. Signed in to a Codync account,
/// events are linked to that account; otherwise to this install's random id. Both get a
/// person profile.
pub fn capture(hub: &Hub, event: Event, mut properties: Map<String, Value>) {
    if !hub.analytics.state() {
        return;
    }
    let account = hub.cloud.status().owner.map(|o| o.user_id);
    if let Some(user) = &account
        && hub.store.kv_get(KV_IDENTIFIED).as_deref() != Some(user.as_str())
    {
        // Merges this install's earlier, anonymous events into the account.
        let identify = json!({"$anon_distinct_id": hub.host_id, "$process_person_profile": true});
        queue(hub, "$identify", user, identify.as_object().cloned().unwrap_or_default());
        if let Err(e) = hub.store.kv_set(KV_IDENTIFIED, user) {
            tracing::warn!(error = format!("{e:#}"), "couldn't save the analytics identity");
        }
    }
    properties.insert("$process_person_profile".into(), true.into());
    queue(hub, event.name(), account.as_deref().unwrap_or(&hub.host_id), properties);
}

fn queue(hub: &Hub, event: &str, distinct_id: &str, properties: Map<String, Value>) {
    if let Err(error) = queue_identified(hub, event, distinct_id, properties, None) {
        tracing::warn!(%error,"Could not persist analytics event");
    }
}
fn queue_identified(
    hub: &Hub,
    event: &str,
    distinct_id: &str,
    mut properties: Map<String, Value>,
    client_id: Option<uuid::Uuid>,
) -> Result<()> {
    properties.extend([
        ("$lib".into(), "codync-host".into()),
        ("app_version".into(), env!("CARGO_PKG_VERSION").into()),
        ("env".into(), (if option_env!("CODYNC_ENV") == Some("main") { "main" } else { "dev" }).into()),
        ("$os".into(), std::env::consts::OS.into()),
        ("arch".into(), std::env::consts::ARCH.into()),
        // Joins this computer's events across accounts and with its phones' `computer_paired`.
        ("computer".into(), computer_tag(&hub.identity.computer_id()).into()),
    ]);
    let payload = json!({
        "event": event,
        "distinct_id": distinct_id,
        // When it happened, not when its batch left; the uuid makes a resent batch count once.
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "uuid": client_id.unwrap_or_else(uuid::Uuid::new_v4).to_string(),
        "properties": properties,
    });
    hub.store.enqueue_analytics(&payload, client_id.is_some())?;
    Ok(())
}

/// A computer's analytics key: a hash of its id, so a phone's `computer_paired` joins the
/// computer's events without revealing the id the relay knows it by.
pub fn computer_tag(computer_id: &str) -> String {
    use sha2::{Digest as _, Sha256};
    use std::fmt::Write as _;
    let hash = Sha256::digest(format!("codync-analytics:{computer_id}"));
    hash[..8].iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

/// Sends queued events in batches every `FLUSH_EVERY`; turning analytics off drops them.
pub fn start(hub: &Arc<Hub>) {
    let Some(mut rx) = hub.analytics.rx.locked().take() else { return };
    errors::report_panics(hub);
    let hub = hub.clone();
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(FLUSH_EVERY);
        loop {
            tokio::select! {
                msg = rx.recv() => match msg {
                    Some(Msg::Report(r)) => errors::capture_report(&hub, r),
                    Some(Msg::Flush(done)) => {
                        flush(&hub).await;
                        let _ = done.send(());
                    }
                    None => return,
                },
                _ = tick.tick() => flush(&hub).await,
            }
        }
    });
}

async fn flush(hub: &Hub) {
    let mut consent = hub.analytics.consent.subscribe();
    if !*consent.borrow_and_update() {
        return;
    }
    let batch = match hub.store.pending_analytics() {
        Ok(batch) if !batch.is_empty() => batch,
        Ok(_) => return,
        Err(error) => {
            tracing::warn!(%error,"Could not read analytics outbox");
            return;
        }
    };
    let sent = tokio::select! {
        biased;
        _ = consent.changed() => false,
        sent = send(&batch) => sent,
    };
    if sent && let Err(error) = hub.store.acknowledge_analytics(&batch) {
        tracing::warn!(%error,"Could not acknowledge analytics batch");
    }
}

/// Sends what's queued before the host exits, so a restart or update loses nothing.
pub async fn flush_before_exit(hub: &Hub) {
    let (tx, rx) = oneshot::channel();
    if hub.analytics.tx.send(Msg::Flush(tx)).is_ok() {
        let _ = tokio::time::timeout(Duration::from_secs(3), rx).await;
    }
}

async fn send(batch: &[Value]) -> bool {
    // Debug builds (`cargo test`, `cargo run`) are throwaway hosts: each e2e test's fresh
    // home would count as a new user, and one built with `CODYNC_ENV=main` lands in production.
    if cfg!(debug_assertions) {
        return true;
    }
    let body = json!({"api_key": TOKEN, "batch": batch});
    match crate::http().post(BATCH_URL).json(&body).timeout(Duration::from_secs(15)).send().await {
        Ok(r) if r.status().is_success() => true,
        Ok(r) => {
            tracing::debug!(status = %r.status(), "analytics batch refused");
            // Refused for good (bad payload): retrying would only repeat it.
            r.status().is_client_error() && !matches!(r.status().as_u16(), 408 | 429)
        }
        Err(e) => {
            tracing::debug!(error = format!("{e:#}"), "analytics batch not sent");
            false
        }
    }
}

#[cfg(test)]
mod tests;
