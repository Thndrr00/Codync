//! Durable bot routines, shared by API clients and the built-in MCP server.
mod bindings;
pub mod blueprints;
mod check;
mod delivery;
pub use check::Checked;
mod editor;
pub mod events;
mod failed;
pub mod flow;
mod idle;
pub(crate) use editor::days_in_month;
pub use editor::{ScheduleDraft, preview as schedule_preview};
pub mod hooks;
use hooks::Refused;
mod mcp;
pub use mcp::{INSTRUCTIONS, call, tools};
mod output_text;
mod outputs;
pub use outputs::Output;
mod retry;
mod runner;
mod schedule;
mod schedule_calendar;
mod watch;
pub use runner::start;
pub use schedule::{Trigger, local_zone};
pub use schedule_calendar::occurrences_in_range;

use crate::{
    LockExt,
    hub::Hub,
    market::bot_secrets,
    store::{EntryKind, Lane, Store, now_ms},
};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    str::FromStr,
    sync::{Arc, Mutex},
};

const STATE_KEY: &str = "routines.v1";
const MAX_FINISHED_RUNS: usize = 1000;
/// How many refused deliveries stay in the history; a full log drops its oldest entries.
const MAX_IGNORED_DELIVERIES: usize = 50;
/// Chat and GitHub trigger histories are bounded the same way as OpenBot's 50 deliveries.
const MAX_CHAT_DELIVERIES: usize = 50;
/// Webhook receipts are deduplicated for seven days, then pruned (`events.md`).
const WEBHOOK_RECEIPT_TTL_MS: i64 = 7 * 24 * 60 * 60 * 1000;
const MAX_WEBHOOK_RECEIPTS: usize = 500;
const MAX_LAST_EVENTS: usize = 100;
/// What a routine does with the occurrences that came due while the host was away: record them
/// and drop them (`skip`), or run the newest one when the host comes back (`runOnce`).
pub const MISSED_POLICIES: [&str; 2] = ["skip", "runOnce"];
/// What a due run does while the daily token limit is used up: hold until it resets (`wait`),
/// or drop it (`skip`).
pub const LIMIT_POLICIES: [&str; 2] = ["wait", "skip"];
fn default_timeout() -> u64 {
    3600
}
fn default_missed_policy() -> String {
    "skip".into()
}
fn default_limit_policy() -> String {
    "wait".into()
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Routine {
    pub id: String,
    pub bot_id: String,
    pub name: String,
    pub instruction: String,
    pub triggers: Vec<Trigger>,
    pub enabled: bool,
    pub created_at: i64,
    pub updated_at: i64,
    pub next_runs: Vec<Option<i64>>,
    pub webhook_key: String,
    pub deleted: bool,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    #[serde(default)]
    pub last_error: Option<String>,
    /// What each `watch` trigger last saw, by trigger position.
    #[serde(default)]
    pub watch: Vec<watch::WatchState>,
    /// Finished runs in a row that ran nothing that changes anything (`idle.rs`).
    #[serde(default)]
    pub idle_runs: u32,
    #[serde(default = "default_missed_policy")]
    pub missed_policy: String,
    #[serde(default = "default_limit_policy")]
    pub limit_policy: String,
    /// Where a workflow's last reply goes, if anywhere.
    #[serde(default)]
    pub outputs: Vec<Output>,
    /// Runs post in a thread of the bot's chat instead of its main one.
    #[serde(default)]
    pub thread: bool,
    /// Runs keep their steps in the chat but post no notice of their own and send no alert.
    /// Ported from OpenBot nightly's quiet routine runs; the default keeps old routines as they were.
    #[serde(default)]
    pub quiet: bool,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    Pending,
    Starting,
    Running,
    Recovering,
    Succeeded,
    Failed,
    Interrupted,
    Cancelled,
}

impl Status {
    fn active(self) -> bool {
        matches!(self, Self::Pending | Self::Starting | Self::Running | Self::Recovering)
    }
    /// The run holds the bot's turn: starting, running, or resuming after a restart.
    fn in_turn(self) -> bool {
        matches!(self, Self::Starting | Self::Running | Self::Recovering)
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Run {
    pub id: String,
    pub routine_id: String,
    pub bot_id: String,
    pub status: Status,
    pub created_at: i64,
    pub finished_at: Option<i64>,
    pub detail: Option<String>,
    pub event: Value,
    /// Immutable authorization captured on the first attempt to start this run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    whatsapp_binding: Option<Value>,
    pub delivery_id: Option<String>,
    #[serde(default)]
    pub started_at: Option<i64>,
    /// The thread this run posts in, when its routine asked for one.
    #[serde(default)]
    pub thread_root: Option<String>,
    #[serde(default)]
    report_pending: bool,
    #[serde(default)]
    result: Option<String>,
    /// How many earlier tries of the same work failed before acting (`retry.rs`).
    #[serde(default)]
    pub attempt: u32,
    /// A queued retry waits until then.
    #[serde(default)]
    pub not_before: Option<i64>,
    /// This failed run is tried again automatically then.
    #[serde(default)]
    pub retry_at: Option<i64>,
    /// This failed run can be retried by the user, so it keeps its event until then.
    #[serde(default)]
    pub retryable: bool,
    /// A stable reason for a failure (`timeout`, `usage_limit`, `memory`…), for the history and its filters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_code: Option<String>,
    /// The occurrence this run stands for, when its schedule named one: a run that started well
    /// after it ran late, and a skipped one names the occurrences it dropped.
    #[serde(default)]
    pub scheduled_for: Option<i64>,
    /// Occurrences that came due and did not run; the run's `status` says what happened to them.
    #[serde(default)]
    pub missed: Option<schedule::Missed>,
}

impl Run {
    /// A retry still waiting for its time: it holds nothing up and nothing takes it yet.
    fn waiting(&self, now: i64) -> bool {
        self.status == Status::Pending && self.not_before.is_some_and(|at| at > now)
    }
}

/// A signed delivery that did not start a run, kept for the panel's short history.
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IgnoredDelivery {
    routine_id: String,
    at: i64,
    /// `paused` or `mismatch`: why no run was queued.
    reason: String,
    delivery_id: Option<String>,
}

/// One GitHub delivery in a workflow trigger's bounded history. No body is kept: only the item and
/// the outcome, as in OpenBot's `workflow_github_deliveries` (docs/architecture/events.md).
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GitHubDelivery {
    id: String,
    routine_id: String,
    event: String,
    item: Option<events::GitHubItem>,
    at: i64,
    /// `started`, `waiting` or `skipped`.
    outcome: String,
    reason: Option<String>,
    run_id: Option<String>,
}

/// A run on a GitHub item, so a third one in ten minutes is recorded as a chain.
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GitHubRunMark {
    routine_id: String,
    repository: String,
    number: i64,
    at: i64,
}

/// A chat mention a workflow trigger considered, with no message text in the row.
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChatDelivery {
    routine_id: String,
    source: String,
    connection: String,
    channel: String,
    message_id: String,
    at: i64,
    outcome: String,
    reason: Option<String>,
    run_id: Option<String>,
}

/// A webhook receipt, kept for seven days without its body so a repeated delivery id is a no-op.
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WebhookReceipt {
    routine_id: String,
    delivery_id: String,
    at: i64,
    reason: Option<String>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
struct State {
    routines: Vec<Routine>,
    runs: Vec<Run>,
    /// Written with the terminal run, acknowledged only after its outputs have been queued.
    #[serde(default)]
    pending_flows: Vec<flow::Start>,
    /// A sender's own Standard Webhooks secret, by routine id. Only the host holds these.
    #[serde(default)]
    sender_secrets: BTreeMap<String, String>,
    #[serde(default)]
    ignored: Vec<IgnoredDelivery>,
    #[serde(default)]
    github_deliveries: Vec<GitHubDelivery>,
    #[serde(default)]
    github_runs: Vec<GitHubRunMark>,
    #[serde(default)]
    chat_deliveries: Vec<ChatDelivery>,
    #[serde(default)]
    webhook_receipts: Vec<WebhookReceipt>,
    /// The last event per workflow, used by "run with last event". Ids and fields only.
    #[serde(default)]
    last_events: BTreeMap<String, Value>,
}

pub struct Routines {
    output_workers: Arc<tokio::sync::Semaphore>,
    state: Mutex<State>,
    /// How many runs each bot has in flight (at most `Hub::MAX_SIDE`).
    active: Mutex<std::collections::HashMap<String, usize>>,
    scheduler: Mutex<Option<tokio::task::JoinHandle<()>>>,
    stopping: std::sync::atomic::AtomicBool,
    /// Bumped whenever the webhook set the relay registers (`hooks()`) changes.
    hooks_rev: tokio::sync::watch::Sender<u64>,
    /// The routine flows running now (`flow`).
    flows: flow::Registry,
}

impl Default for Routines {
    fn default() -> Self {
        Self {
            output_workers: Arc::new(tokio::sync::Semaphore::new(4)),
            state: Mutex::default(),
            active: Mutex::default(),
            scheduler: Mutex::default(),
            stopping: std::sync::atomic::AtomicBool::default(),
            hooks_rev: tokio::sync::watch::channel(0).0,
            flows: flow::Registry::default(),
        }
    }
}

/// One routine's public webhook as the cloud stores it.
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct Hook {
    pub id: String,
    pub key: String,
    pub enabled: bool,
}

fn hook_set(state: &State) -> Vec<Hook> {
    state
        .routines
        .iter()
        .filter(|r| !r.deleted && r.triggers.iter().any(|t| matches!(t, Trigger::Webhook | Trigger::Event { .. })))
        .map(|r| Hook { id: r.id.clone(), key: r.webhook_key.clone(), enabled: r.enabled })
        .collect()
}

fn webhook_key() -> String {
    format!("{}{}", uuid::Uuid::new_v4().simple(), uuid::Uuid::new_v4().simple())
}

/// The name of the API key in the routine owner's vault that holds this routine's signing secret.
/// Routine ids are UUIDs, so replacing `-` with `_` keeps the name valid for `bot_secrets`.
pub fn webhook_secret_name(routine_id: &str) -> String {
    format!("webhook_{}", routine_id.replace('-', "_"))
}

impl Routines {
    pub fn restore(&self, store: &Store) -> Result<()> {
        store.recover_routine_outputs()?;
        let mut state = match store.kv_read(STATE_KEY)? {
            Some(raw) => serde_json::from_str::<State>(&raw).context("reading routines")?,
            None => State::default(),
        };
        let mut active = self.active.locked();
        active.clear();
        for run in &mut state.runs {
            match run.status {
                Status::Starting => run.status = Status::Pending,
                Status::Running | Status::Recovering => {
                    run.status = Status::Recovering;
                    run.finished_at = None;
                    *active.entry(run.bot_id.clone()).or_default() += 1;
                }
                _ => {}
            }
        }
        store.kv_set(STATE_KEY, &serde_json::to_string(&state)?)?;
        *self.state.locked() = state;
        self.hooks_rev.send_modify(|v| *v += 1);
        Ok(())
    }

    /// The webhook routines the relay registers with the cloud.
    pub fn hooks(&self) -> Vec<Hook> {
        hook_set(&self.state.locked())
    }

    pub fn subscribe_hooks(&self) -> tokio::sync::watch::Receiver<u64> {
        self.hooks_rev.subscribe()
    }

    /// OpenBot v0.78.0: a correction to a belief that a routine's instruction quotes also updates that
    /// routine, so its next run does not repeat the old path, account or deadline. Pure and bounded.
    pub fn apply_belief(&self, store: &Store, old: &str, new: &str) -> Result<usize> {
        let (old, new) = (old.trim(), new.trim());
        if old.is_empty() || new.is_empty() || old == new {
            return Ok(0);
        }
        let mut changed = 0;
        let ids: Vec<String> = self.state.locked().routines.iter().map(|r| r.id.clone()).collect();
        for id in ids {
            let next = self.change(store, |state| {
                let Some(routine) = state.routines.iter_mut().find(|r| r.id == id) else { return Ok(false) };
                if !routine.instruction.contains(old) {
                    return Ok(false);
                }
                routine.instruction = routine.instruction.replace(old, new);
                Ok(true)
            })?;
            if next {
                changed += 1;
            }
        }
        Ok(changed)
    }

    fn change<T>(&self, store: &Store, update: impl FnOnce(&mut State) -> Result<T>) -> Result<T> {
        let mut guard = self.state.locked();
        let mut next = guard.clone();
        let result = update(&mut next)?;
        let mut kept = 0;
        next.runs.reverse();
        next.runs.retain(|run| {
            // A failure the user can still retry keeps its event (at most one per routine).
            if run.status.active() || run.report_pending || run.retryable {
                return true;
            }
            kept += 1;
            kept <= MAX_FINISHED_RUNS
        });
        next.runs.reverse();
        next.routines
            .retain(|r| !r.deleted || next.runs.iter().any(|run| run.routine_id == r.id && run.status.active()));
        store.kv_set(STATE_KEY, &serde_json::to_string(&next)?)?;
        let hooks_changed = hook_set(&guard) != hook_set(&next);
        *guard = next;
        drop(guard);
        if hooks_changed {
            self.hooks_rev.send_modify(|v| *v += 1);
        }
        Ok(result)
    }

    pub fn list(&self, bot: &str, attention: impl Fn(&str) -> bool) -> Value {
        let state = self.state.locked();
        let runs = state
            .runs
            .iter()
            .rev()
            .filter(|r| r.bot_id == bot)
            .take(200)
            .map(|run| {
                let in_turn = run.status.in_turn();
                let mut run = serde_json::to_value(run).unwrap_or(Value::Null);
                // The bot asks a question mid-run: the run waits on the user until it answers.
                run["needsAttention"] = json!(in_turn && attention(bot));
                run
            })
            .collect::<Vec<_>>();
        json!({"routines": state.routines.iter().filter(|r| r.bot_id == bot && !r.deleted).map(|routine| {
            let mut value = public(routine);
            // The run in flight, so a canvas can offer Stop for it (OpenBot 0.71.0). A queued run counts
            // too: stopping it is how a person takes it back before it starts.
            if let Some(run) = state.runs.iter().rev().find(|run| run.routine_id == routine.id && run.status.active()) {
                value["running"] = json!(run.id);
            }
            value
        }).collect::<Vec<_>>(),
            "runs": runs})
    }

    pub fn owns(&self, bot: &str, id: &str) -> bool {
        self.state.locked().routines.iter().any(|r| r.id == id && r.bot_id == bot && !r.deleted)
    }

    /// The bot's calendar for [from, to]: the runs that stand for moments in the window, and the
    /// clock occurrences of its enabled routines that are still to come.
    pub fn calendar(&self, bot: &str, from: i64, to: i64) -> Result<Value> {
        if to.saturating_sub(from) > 60 * 86_400_000 {
            bail!("the calendar window must be at most 60 days");
        }
        let state = self.state.locked();
        let runs = state
            .runs
            .iter()
            .filter(|r| r.bot_id == bot)
            .filter_map(|run| {
                let at = run.scheduled_for.unwrap_or(run.created_at);
                (from..=to).contains(&at).then(|| {
                    json!({"id": run.id, "routineId": run.routine_id, "at": at, "status": run.status,
                            "skipped": run.status == Status::Cancelled && run.scheduled_for.is_some()})
                })
            })
            .collect::<Vec<_>>();
        let now = now_ms();
        let mut scheduled: Vec<Value> = vec![];
        for r in state.routines.iter().filter(|r| r.bot_id == bot && !r.deleted && r.enabled) {
            for (trigger, next) in r.triggers.iter().zip(&r.next_runs) {
                match trigger {
                    Trigger::Interval { seconds } => {
                        let Some(step) = seconds.checked_mul(1000).filter(|s| *s > 0) else {
                            continue;
                        };
                        let mut at = next.unwrap_or(now);
                        if at <= now {
                            at += (now.saturating_sub(at) / step + 1) * step;
                        }
                        while at <= to && scheduled.len() < 1000 {
                            scheduled.push(json!({"routineId": r.id, "at": at}));
                            at += step;
                        }
                    }
                    Trigger::Once { at } => {
                        if *at > now && *at <= to {
                            scheduled.push(json!({"routineId": r.id, "at": at}));
                        }
                    }
                    Trigger::Cron { expression, time_zone } => {
                        let (Ok(zone), Ok(cron)) =
                            (time_zone.parse::<chrono_tz::Tz>(), croner::Cron::from_str(expression))
                        else {
                            continue;
                        };
                        let Some(mut t) =
                            chrono::DateTime::<chrono::Utc>::from_timestamp_millis(now).map(|d| d.with_timezone(&zone))
                        else {
                            continue;
                        };
                        while let Ok(next) = cron.find_next_occurrence(&t, false) {
                            t = next;
                            let at = t.timestamp_millis();
                            if at > to || scheduled.len() >= 1000 {
                                break;
                            }
                            scheduled.push(json!({"routineId": r.id, "at": at}));
                        }
                    }
                    Trigger::Webhook | Trigger::Event { .. } | Trigger::Watch { .. } => {}
                }
            }
        }
        scheduled.sort_by_key(|e| e["at"].as_i64().unwrap_or(0));
        Ok(json!({
            "routines": state.routines.iter().filter(|r| r.bot_id == bot && !r.deleted).map(public).collect::<Vec<_>>(),
            "runs": runs,
            "scheduled": scheduled,
        }))
    }

    pub fn save(&self, hub: &Hub, bot: &str, body: &Value) -> Result<Value> {
        let row = hub
            .store
            .bot(bot)?
            .filter(|b| !b.deleted && !b.config.is_group())
            .context("routine requires an existing agent bot")?;
        let Checked { name, instruction, mut triggers, timeout } = Checked::parse(body)?;
        let wanted_outputs = output_text::request(body)?;
        // "Keep" means the saved triggers, not the client's copy of them, which an older app
        // may have decoded without the fields of a trigger type it doesn't know.
        if body["schedule"]["kind"] == "keep"
            && let Some(id) = body["id"].as_str()
        {
            let s = self.state.locked();
            if let Some(saved) = s.routines.iter().find(|r| r.id == id && r.bot_id == bot && !r.deleted) {
                triggers.clone_from(&saved.triggers);
            }
        }
        let now = now_ms();
        let id = body["id"].as_str().map_or_else(|| uuid::Uuid::new_v4().to_string(), str::to_owned);
        let updated = body["id"].is_string();
        let routine = self.change(&hub.store, |s| {
            let old = check::replaced(s, bot, body, &triggers, now)?;
            output_text::check_expected(body, old.as_ref())?;
            if !updated && s.routines.iter().filter(|r| !r.deleted).count() >= 1000 {
                bail!("routine limit reached");
            }
            let r = Routine {
                id: id.clone(),
                bot_id: row.config.id.clone(),
                name: name.clone(),
                instruction,
                enabled: body["enabled"].as_bool().unwrap_or_else(|| old.as_ref().is_none_or(|r| r.enabled)),
                created_at: old.as_ref().map_or(now, |r| r.created_at),
                updated_at: now,
                next_runs: if let Some(old) = &old
                    && old.triggers == triggers
                    && (body["enabled"] != true || old.enabled)
                {
                    old.next_runs.clone()
                } else {
                    triggers.iter().map(|t| t.next(now)).collect::<Result<_>>()?
                },
                // Same triggers keep what their watches saw; new ones start a fresh baseline.
                watch: old.as_ref().filter(|o| o.triggers == triggers).map(|o| o.watch.clone()).unwrap_or_default(),
                idle_runs: 0,
                triggers,
                webhook_key: old.as_ref().map_or_else(webhook_key, |r| r.webhook_key.clone()),
                deleted: false,
                timeout_seconds: timeout
                    .unwrap_or_else(|| old.as_ref().map_or_else(default_timeout, |r| r.timeout_seconds)),
                last_error: None,
                missed_policy: policy(
                    body["missedPolicy"].as_str(),
                    &MISSED_POLICIES,
                    old.as_ref(),
                    |r| r.missed_policy.as_str(),
                    "skip",
                ),
                limit_policy: policy(
                    body["limitPolicy"].as_str(),
                    &LIMIT_POLICIES,
                    old.as_ref(),
                    |r| r.limit_policy.as_str(),
                    "wait",
                ),
                outputs: wanted_outputs.or_else(|| old.as_ref().map(|r| r.outputs.clone())).unwrap_or_default(),
                thread: body["thread"].as_bool().unwrap_or_else(|| old.as_ref().is_some_and(|r| r.thread)),
                quiet: body["quiet"].as_bool().unwrap_or_else(|| old.as_ref().is_some_and(|r| r.quiet)),
            };
            if !(1..=86_400).contains(&r.timeout_seconds) {
                bail!("timeoutSeconds must be between 1 and 86400");
            }
            if !r.enabled {
                hub.store.cancel_routine_outputs(&id)?;
                s.pending_flows.retain(|flow| flow.routine_id != id);
                self.flows.cancel_routine(&id);
                cancel_pending(s, &id);
            }
            s.routines.retain(|r| r.id != id);
            s.routines.push(r.clone());
            Ok(r)
        })?;
        hub.add_entry(&Lane::main(bot), EntryKind::Notice, 0, &json!({"text":format!("{} routine: {name}", if updated {"Updated"} else {"Created"}), "routineId": id, "style":"info"}));
        Ok(json!({"routine":public(&routine)}))
    }

    pub fn set_enabled(&self, store: &Store, bot: &str, id: &str, enabled: bool) -> Result<Value> {
        self.change(store, |s| {
            let r = s
                .routines
                .iter_mut()
                .find(|r| r.id == id && r.bot_id == bot && !r.deleted)
                .context("routine not found")?;
            if enabled && !r.enabled {
                r.next_runs = r.triggers.iter().map(|t| t.next(now_ms())).collect::<Result<_>>()?;
                // A watch paused for failing checks gets its full allowance again.
                r.watch.iter_mut().for_each(|w| w.failures = 0);
            }
            r.enabled = enabled;
            r.updated_at = now_ms();
            if !enabled {
                store.cancel_routine_outputs(id)?;
                s.pending_flows.retain(|flow| flow.routine_id != id);
                self.flows.cancel_routine(id);
                for run in &mut s.runs {
                    if run.routine_id == id && run.status == Status::Pending {
                        cancel_run(run);
                    }
                }
            }
            Ok(json!({"routine":public(r)}))
        })
    }

    pub fn remove(&self, hub: &Hub, bot: &str, id: &str) -> Result<Value> {
        let withdrawn = self.change(&hub.store, |s| {
            let r = s
                .routines
                .iter_mut()
                .find(|r| r.id == id && r.bot_id == bot && !r.deleted)
                .context("routine not found")?;
            r.deleted = true;
            r.enabled = false;
            r.webhook_key.clear();
            s.sender_secrets.remove(id);
            cancel_pending(s, id);
            Ok(retry::forget(s, |run| run.routine_id == id))
        })?;
        retry::withdraw(hub, &withdrawn);
        flow::forget_routine(hub, id, bot);
        Ok(json!({"ok":true}))
    }

    pub fn enqueue(
        &self,
        store: &Store,
        bot: &str,
        id: &str,
        event: Value,
        delivery: Option<String>,
        test: bool,
    ) -> Result<Value> {
        self.change(store, |s| {
            let r =
                s.routines.iter().find(|r| r.id == id && r.bot_id == bot && !r.deleted).context("routine not found")?;
            if !r.enabled && !test {
                bail!(Refused::Paused);
            }
            if let Some(key) = &delivery
                && let Some(run) =
                    s.runs.iter().find(|run| run.routine_id == id && run.delivery_id.as_ref() == Some(key))
            {
                return Ok(json!({"run":run}));
            }
            // Asked to run now while a retry waits: that retry runs now.
            if test
                && let Some(waiting) = s
                    .runs
                    .iter_mut()
                    .find(|run| run.routine_id == id && run.status == Status::Pending && run.not_before.is_some())
            {
                waiting.not_before = None;
                return Ok(json!({"run":waiting}));
            }
            if busy(s, id) {
                bail!(Refused::Busy);
            }
            let run = new_run(r, event, delivery);
            let result = json!({"run":run});
            s.runs.push(run);
            Ok(result)
        })
    }

    /// Rotates `id`'s signing secret, cutting off the old one. When the routine's secret has already
    /// been given to `bot` as a vault key, refresh that key too so the agent always uses the current
    /// secret. Ported from OpenBot 0.74.0 ("when you regenerate the secret … the agent gets the new
    /// one too"), in Codync's terms: the agent's key is a `bot_secrets` record.
    fn rotate_webhook(&self, hub: &Hub, bot: &str, id: &str, sync_key: bool) -> Result<String> {
        let had_key = sync_key && bot_secrets::find(&hub.store, bot, &webhook_secret_name(id)).is_ok();
        let key = self.change(&hub.store, |s| {
            let r = s
                .routines
                .iter_mut()
                .find(|r| r.id == id && r.bot_id == bot && !r.deleted)
                .context("routine not found")?;
            r.webhook_key = webhook_key();
            r.updated_at = now_ms();
            Ok(r.webhook_key.clone())
        })?;
        if had_key {
            // The value never goes into a card, log or tool result: only the encrypted vault holds it.
            bot_secrets::save(&hub.store, bot, &webhook_secret_name(id), bot_secrets::Usage::Command, &key)?;
        }
        Ok(key)
    }

    /// Gives the routine's signing secret to its agent as one of the agent's own vault keys, so the
    /// agent can add it to the sender without the value appearing in chat, logs or analytics. The
    /// secret is rotated first: the old one stops working. The agent uses the key with
    /// `run_with_secrets` (as `$WEBHOOK_<ID>`) and `list_secrets` shows only its name and destination.
    /// Ported from OpenBot 0.74.0's "Give the signing secret to <agent>" action.
    pub fn give_webhook_secret(&self, hub: &Hub, bot: &str, id: &str) -> Result<Value> {
        let key = self.rotate_webhook(hub, bot, id, false)?;
        let saved = bot_secrets::save(&hub.store, bot, &webhook_secret_name(id), bot_secrets::Usage::Command, &key)?;
        Ok(saved.public())
    }

    /// The routine's webhook: the public URL through the Codync cloud (when it's on; a private
    /// relay doesn't deliver them), the local one, and the key. `rotate` replaces the key first,
    /// cutting off the old one.
    pub fn credentials(&self, hub: &Hub, bot: &str, id: &str, rotate: bool) -> Result<Value> {
        if rotate {
            self.rotate_webhook(hub, bot, id, true)?;
        }
        let (key, uses_sender_secret) = {
            let s = self.state.locked();
            s.routines
                .iter()
                .find(|r| r.id == id && r.bot_id == bot && !r.deleted)
                .map(|r| (r.webhook_key.clone(), s.sender_secrets.contains_key(id)))
                .context("routine not found")?
        };
        let cloud = hub.cloud.status();
        // Public deliveries go through the relay, for a computer in an account (docs/plans/optional-sign-in.md).
        let url = crate::remote::cloud::codync_url(hub)
            .ok()
            .filter(|_| crate::remote::owner::owner(&hub.store).is_some())
            .map(|base| format!("{}/v1/hooks/{}/{id}", base.trim_end_matches('/'), hub.identity.computer_id()));
        Ok(json!({
            "url": url,
            "needsAccount": crate::remote::owner::can_join(hub),
            "localUrl": format!("http://127.0.0.1:{}/hooks/routines/{id}", hub.port),
            "key": key,
            "connected": url.is_some() && cloud.connected,
            "usesSenderSecret": uses_sender_secret,
        }))
    }

    /// Sets (or clears, with `None`) the sender's own Standard Webhooks secret for `id`. The value
    /// lives only in the host's routine state; neither a reply nor a log ever carries it.
    pub fn set_sender_secret(&self, store: &Store, bot: &str, id: &str, secret: Option<String>) -> Result<Value> {
        let cleaned = secret.map(|s| s.trim().to_owned()).filter(|s| !s.is_empty());
        if cleaned.as_ref().is_some_and(|s| s.len() > 512) {
            bail!("the sender secret is too long");
        }
        self.change(store, |s| {
            s.routines.iter().find(|r| r.id == id && r.bot_id == bot && !r.deleted).context("routine not found")?;
            match cleaned {
                Some(value) => {
                    s.sender_secrets.insert(id.to_owned(), value);
                }
                None => {
                    s.sender_secrets.remove(id);
                }
            }
            Ok(json!({"usesSenderSecret": s.sender_secrets.contains_key(id)}))
        })
    }

    /// The latest deliveries that were signed but did not run, for the routine's webhook panel.
    pub fn ignored(&self, bot: &str, id: &str) -> Value {
        let state = self.state.locked();
        if !state.routines.iter().any(|r| r.id == id && r.bot_id == bot && !r.deleted) {
            return json!({"ignored": []});
        }
        json!({"ignored": state.ignored.iter().filter(|entry| entry.routine_id == id).rev().map(|entry| json!({
            "at": entry.at, "reason": entry.reason, "deliveryId": entry.delivery_id,
        })).collect::<Vec<_>>()})
    }

    /// Builds a sample delivery for the routine's triggers, signs it with the secret its sender
    /// uses, and runs it through the same verification path as a real delivery. The panel's
    /// "Send test event" button calls this; the secret is never returned.
    pub fn test_webhook(&self, hub: &Hub, bot: &str, id: &str) -> Result<Value> {
        let r = {
            let state = self.state.locked();
            state
                .routines
                .iter()
                .find(|r| r.id == id && r.bot_id == bot && !r.deleted)
                .map(|r| (r.clone(), state.sender_secrets.get(id).cloned()))
                .context("routine not found")?
        };
        let (routine, sender_secret) = r;
        let now = now_ms();
        let event = events::test_event(&routine, now);
        let body = serde_json::to_vec(&event)?;
        let message_id = uuid::Uuid::new_v4().simple().to_string();
        let timestamp = (now / 1000).to_string();
        let secret = sender_secret.as_deref().unwrap_or(&routine.webhook_key);
        let signature =
            events::sign_sender(secret, &message_id, &timestamp, &body).context("couldn't sign the test event")?;
        let mut headers = hooks::Headers::new();
        headers.insert("webhook-id".into(), message_id);
        headers.insert("webhook-timestamp".into(), timestamp);
        headers.insert("webhook-signature".into(), signature);
        // A test runs even when the routine is paused, just like `runRoutine`.
        let answer = self.receive_inner(hub, id, &headers, &body, true).map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(json!({"test": true, "event": event, "answer": answer}))
    }

    /// An event from a channel (a Slack or Telegram mention): queues every enabled routine whose event
    /// trigger matches. Returns the routine ids it queued.
    #[allow(clippy::needless_pass_by_value, reason = "cloned for each queued routine")]
    pub fn receive_event(&self, store: &Store, event: &Value, delivery: Option<String>) -> Vec<String> {
        let github = event["source"] == "github";
        let matches: Vec<(String, String)> = self
            .state
            .locked()
            .routines
            .iter()
            .filter(|r| !r.deleted && r.enabled && r.triggers.iter().any(|t| trigger_matches(t, event)))
            .map(|r| (r.id.clone(), r.bot_id.clone()))
            .collect();
        let mut queued = vec![];
        for (id, bot) in matches {
            if github && !self.github_deliverable(store, &id, event, delivery.as_deref()) {
                continue;
            }
            match self.enqueue(store, &bot, &id, event.clone(), delivery.clone(), false) {
                Ok(answer) => {
                    queued.push(id.clone());
                    if github {
                        let run_id = answer["run"]["id"].as_str().map(str::to_owned);
                        self.note_github_delivery(store, &id, event, delivery.as_deref(), "started", None, run_id);
                        self.remember_event(store, &id, event);
                    }
                }
                Err(error) if github => {
                    let reason = error.downcast_ref::<Refused>().map_or("refused", Refused::code);
                    self.note_github_delivery(store, &id, event, delivery.as_deref(), "skipped", Some(reason), None);
                }
                Err(_) => {}
            }
        }
        queued
    }

    /// One delivery from the local endpoint or the cloud: checks the key or signature, then
    /// queues a run when the event matches. Retries of a delivery id return its first run.
    pub fn receive(&self, hub: &Hub, id: &str, headers: &hooks::Headers, body: &[u8]) -> Result<Value, Refused> {
        self.receive_inner(hub, id, headers, body, false)
    }

    /// The real delivery path; a host-originated test is the only caller with `test` set, so it can
    /// run a paused routine without loosening what a sender may do.
    fn receive_inner(
        &self,
        hub: &Hub,
        id: &str,
        headers: &hooks::Headers,
        body: &[u8],
        test: bool,
    ) -> Result<Value, Refused> {
        let (r, sender_secret) = {
            let state = self.state.locked();
            let r = state.routines.iter().find(|r| r.id == id && !r.deleted).cloned();
            (r, state.sender_secrets.get(id).cloned())
        };
        let Some(r) = r else { return Err(Refused::Unauthorized) };
        // One verification path for every workflow webhook. The pure decision reuses the host's
        // bearer/HMAC proof and Standard Webhooks signature check, then parses the event; a sender
        // that keeps its own secret is tried only if the routine's key did not verify.
        let request = events::WebhookBody { headers: headers.clone(), raw: body.to_vec() };
        let now = now_ms();
        let mut decision = events::workflow_webhook(&request, &r.webhook_key, now);
        if decision == events::WebhookDecision::Unauthorized
            && let Some(secret) = sender_secret.as_deref()
        {
            decision = events::workflow_webhook(&request, secret, now);
        }
        let event = match decision {
            events::WebhookDecision::Run(event) => event,
            events::WebhookDecision::Unauthorized => return Err(Refused::Unauthorized),
            events::WebhookDecision::TooLarge => return Err(Refused::Invalid("event too large".into())),
            events::WebhookDecision::Ignored(events::WebhookSkip::Ping) => return Ok(json!({"ping": true})),
            events::WebhookDecision::Ignored(events::WebhookSkip::Invalid) => {
                return Err(Refused::Invalid("event is not JSON".into()));
            }
        };
        let delivery = hooks::delivery_id(headers)?;
        if !test && self.duplicate_receipt(id, delivery.as_deref(), now_ms()) {
            return Ok(json!({"duplicate": true}));
        }
        if !r.enabled && !test {
            self.note_ignored(&hub.store, id, delivery.as_deref(), "paused");
            self.note_receipt(&hub.store, id, delivery.as_deref(), Some("paused"));
            return Err(Refused::Paused);
        }
        if !r.triggers.iter().any(|t| matches!(t, Trigger::Webhook) || trigger_matches(t, &event)) {
            self.note_ignored(&hub.store, id, delivery.as_deref(), "mismatch");
            self.note_receipt(&hub.store, id, delivery.as_deref(), Some("mismatch"));
            return Ok(json!({"ignored": "event does not match this routine", "reason": "mismatch"}));
        }
        if !test
            && event["source"] == "github"
            && r.triggers.iter().any(|t| matches!(t, Trigger::Event { source, .. } if source == "github"))
            && !self.github_deliverable(&hub.store, id, &event, delivery.as_deref())
        {
            self.note_receipt(&hub.store, id, delivery.as_deref(), Some("filter"));
            return Ok(json!({"ignored": "event does not match this routine", "reason": "filter"}));
        }
        let started =
            self.enqueue(&hub.store, &r.bot_id, id, event.clone(), delivery.clone(), test).map_err(|e| match e
                .downcast::<Refused>()
            {
                Ok(refused) => refused,
                Err(e) => Refused::Invalid(format!("{e:#}")),
            })?;
        self.note_receipt(&hub.store, id, delivery.as_deref(), None);
        if event["source"] == "github" {
            let run_id = started["run"]["id"].as_str().map(str::to_owned);
            self.note_github_delivery(&hub.store, id, &event, delivery.as_deref(), "started", None, run_id);
        }
        self.remember_event(&hub.store, id, &event);
        Ok(started)
    }

    /// Records a signed delivery that didn't run, keeping the log small; a store failure must not
    /// change what the delivery answered.
    fn note_ignored(&self, store: &Store, id: &str, delivery: Option<&str>, reason: &str) {
        let saved = self.change(store, |s| {
            s.ignored.push(IgnoredDelivery {
                routine_id: id.to_owned(),
                at: now_ms(),
                reason: reason.to_owned(),
                delivery_id: delivery.map(str::to_owned),
            });
            if s.ignored.len() > MAX_IGNORED_DELIVERIES {
                s.ignored.drain(..s.ignored.len() - MAX_IGNORED_DELIVERIES);
            }
            Ok(())
        });
        if let Err(error) = saved {
            tracing::warn!(error = format!("{error:#}"), "couldn't record an ignored webhook delivery");
        }
    }

    /// The bounded GitHub delivery history of one workflow, newest first. No event bodies are kept.
    pub fn github_history(&self, bot: &str, id: &str) -> Value {
        let state = self.state.locked();
        if !state.routines.iter().any(|r| r.id == id && r.bot_id == bot && !r.deleted) {
            return json!({"deliveries": []});
        }
        json!({"deliveries": state.github_deliveries.iter().filter(|d| d.routine_id == id).rev().map(|d| json!({
            "id": d.id, "event": d.event, "item": d.item, "receivedAt": d.at,
            "outcome": d.outcome, "reason": d.reason, "runId": d.run_id,
        })).collect::<Vec<_>>()})
    }

    /// The chat mention history of one workflow, newest first. Ids only, never message text.
    pub fn chat_history(&self, bot: &str, id: &str) -> Value {
        let state = self.state.locked();
        if !state.routines.iter().any(|r| r.id == id && r.bot_id == bot && !r.deleted) {
            return json!({"deliveries": []});
        }
        json!({"deliveries": state.chat_deliveries.iter().filter(|d| d.routine_id == id).rev().map(|d| json!({
            "source": d.source, "connection": d.connection, "channel": d.channel,
            "messageId": d.message_id, "receivedAt": d.at, "outcome": d.outcome,
            "reason": d.reason, "runId": d.run_id,
        })).collect::<Vec<_>>()})
    }

    /// The seven-day receipts of a workflow webhook, newest first, without their bodies.
    pub fn webhook_receipts(&self, bot: &str, id: &str) -> Value {
        let state = self.state.locked();
        if !state.routines.iter().any(|r| r.id == id && r.bot_id == bot && !r.deleted) {
            return json!({"receipts": []});
        }
        json!({"receipts": state.webhook_receipts.iter().filter(|r| r.routine_id == id).rev().map(|r| json!({
            "deliveryId": r.delivery_id, "receivedAt": r.at, "reason": r.reason,
        })).collect::<Vec<_>>()})
    }

    /// The workflow webhook's public URL as the canvas shows it. The signing secret is never part
    /// of this projection: it is returned once, by [`Self::workflow_webhook_credentials`] on rotation.
    pub fn webhook_status(&self, hub: &Hub, bot: &str, id: &str) -> Value {
        let state = self.state.locked();
        let Some(r) = state.routines.iter().find(|r| r.id == id && r.bot_id == bot && !r.deleted) else {
            return json!({});
        };
        let has_secret = !r.webhook_key.is_empty() || state.sender_secrets.contains_key(id);
        let uses_sender_secret = state.sender_secrets.contains_key(id);
        drop(state);
        let url = crate::remote::cloud::codync_url(hub)
            .ok()
            .filter(|_| crate::remote::owner::owner(&hub.store).is_some())
            .map(|base| format!("{}/v1/hooks/{}/{id}", base.trim_end_matches('/'), hub.identity.computer_id()));
        json!({
            "url": url,
            "localUrl": format!("http://127.0.0.1:{}/hooks/routines/{id}", hub.port),
            "connected": url.is_some() && hub.cloud.status().connected,
            "needsAccount": crate::remote::owner::can_join(hub),
            "hasSecret": has_secret,
            "usesSenderSecret": uses_sender_secret,
        })
    }

    /// The workflow-level webhook credentials. `rotate` makes a new secret and returns it once;
    /// without rotation only the URL and the fact that a secret exists are shown.
    pub fn workflow_webhook_credentials(&self, hub: &Hub, bot: &str, id: &str, rotate: bool) -> Result<Value> {
        if rotate {
            self.rotate_webhook(hub, bot, id, false)?;
        }
        let mut value = self.webhook_status(hub, bot, id);
        if let Some(object) = value.as_object_mut() {
            object.insert(
                "key".into(),
                if rotate { self.credentials(hub, bot, id, false)?["key"].clone() } else { Value::Null },
            );
        }
        Ok(value)
    }

    /// Replays the newest event a workflow trigger saw, into a fresh run. Paused workflows can be
    /// replayed for a test, just as a person's manual run can; a busy one is refused as usual.
    pub fn run_last_event(&self, hub: &Hub, bot: &str, id: &str) -> Result<Value> {
        if !self.owns(bot, id) {
            bail!("routine not found among this bot's routines");
        }
        let event =
            self.state.locked().last_events.get(id).cloned().context("this workflow has no event to run again")?;
        self.enqueue(&hub.store, bot, id, event, None, true)
    }

    /// A mention that matched chat trigger nodes, and the routines that actually started. When
    /// `matched` is non-zero and `queued` is empty the caller adds the platform's failed reaction.
    pub fn receive_chat_event(&self, store: &Store, event: &Value, delivery: Option<String>) -> ChatReceive {
        let source = event["source"].as_str().unwrap_or_default().to_owned();
        let matches: Vec<(String, String)> = self
            .state
            .locked()
            .routines
            .iter()
            .filter(|r| {
                !r.deleted
                    && r.enabled
                    && r.triggers.iter().any(|t| {
                        trigger_matches(t, event)
                            || matches!(t, Trigger::Event { source: s, filters, .. } if s == &source
                                && events::chat_trigger_match(event, &events::chat_node(s, filters)))
                    })
            })
            .map(|r| (r.id.clone(), r.bot_id.clone()))
            .collect();
        let matched = matches.len();
        let mut queued = Vec::new();
        for (id, bot) in matches {
            let message =
                event["messageId"].as_str().or_else(|| event["message_id"].as_str()).unwrap_or_default().to_owned();
            match self.enqueue(store, &bot, &id, event.clone(), delivery.clone(), false) {
                Ok(answer) => {
                    let run_id = answer["run"]["id"].as_str().map(str::to_owned);
                    queued.push(id.clone());
                    self.note_chat_delivery(store, &id, &source, event, "started", None, run_id, &message);
                }
                Err(error) => {
                    let reason = error.downcast_ref::<Refused>().map_or("refused", Refused::code);
                    self.note_chat_delivery(store, &id, &source, event, "skipped", Some(reason), None, &message);
                }
            }
            self.remember_event(store, &id, event);
        }
        ChatReceive { matched, queued }
    }

    /// Keeps the newest event of a workflow so "run with last event" can replay it. Only the
    /// normalized event (ids and fields) is kept, never the raw request body.
    fn remember_event(&self, store: &Store, id: &str, event: &Value) {
        let saved = self.change(store, |s| {
            s.last_events.insert(id.to_owned(), event.clone());
            while s.last_events.len() > MAX_LAST_EVENTS {
                let oldest = s.last_events.keys().next().cloned();
                match oldest {
                    Some(key) => {
                        s.last_events.remove(&key);
                    }
                    None => break,
                }
            }
            Ok(())
        });
        if let Err(error) = saved {
            tracing::warn!(error = format!("{error:#}"), "couldn't keep the workflow's last event");
        }
    }

    /// Records one GitHub delivery. Bounded to 50; a run mark is added for chain detection.
    #[allow(clippy::too_many_arguments, reason = "one row of the delivery history")]
    fn note_github_delivery(
        &self,
        store: &Store,
        id: &str,
        event: &Value,
        delivery: Option<&str>,
        outcome: &str,
        reason: Option<&str>,
        run_id: Option<String>,
    ) {
        let saved = self.change(store, |s| {
            let key = delivery.map(str::to_owned).unwrap_or_else(|| uuid::Uuid::new_v4().simple().to_string());
            if s.github_deliveries.iter().any(|d| d.routine_id == id && d.id == key) {
                return Ok(false);
            }
            let item = events::github_item(event);
            if outcome == "started" {
                if let Some(item) = &item {
                    s.github_runs.push(GitHubRunMark {
                        routine_id: id.to_owned(),
                        repository: item.repository.clone(),
                        number: item.number,
                        at: now_ms(),
                    });
                    let cutoff = now_ms().saturating_sub(events::GITHUB_CHAIN_WINDOW_MS);
                    s.github_runs.retain(|mark| mark.at >= cutoff);
                }
            }
            s.github_deliveries.push(GitHubDelivery {
                id: key,
                routine_id: id.to_owned(),
                event: event["event"].as_str().unwrap_or_default().to_owned(),
                item,
                at: now_ms(),
                outcome: outcome.to_owned(),
                reason: reason.map(str::to_owned),
                run_id,
            });
            if s.github_deliveries.len() > events::GITHUB_DELIVERY_LIMIT {
                s.github_deliveries.drain(..s.github_deliveries.len() - events::GITHUB_DELIVERY_LIMIT);
            }
            Ok(true)
        });
        if let Err(error) = saved {
            tracing::warn!(error = format!("{error:#}"), "couldn't record a GitHub delivery");
        }
    }

    /// Applies the pure skip decision to a GitHub delivery, recording a skipped row when it does
    /// not start. A repeated delivery id is ignored as in OpenBot's history.
    fn github_deliverable(&self, store: &Store, id: &str, event: &Value, delivery: Option<&str>) -> bool {
        let reason = {
            let s = self.state.locked();
            let Some(r) = s.routines.iter().find(|r| r.id == id && !r.deleted) else { return false };
            let Some(filters) = r.triggers.iter().find_map(|t| match t {
                Trigger::Event { source, filters, .. } if source == "github" && trigger_matches(t, event) => {
                    Some(filters.clone())
                }
                _ => None,
            }) else {
                return false;
            };
            if delivery.is_some_and(|key| s.github_deliveries.iter().any(|d| d.routine_id == id && d.id == key)) {
                return false;
            }
            let item = events::github_item(event);
            let now = now_ms();
            let recent = item.as_ref().map_or(0, |item| {
                s.github_runs
                    .iter()
                    .filter(|mark| {
                        mark.routine_id == id
                            && mark.repository == item.repository
                            && mark.number == item.number
                            && now.saturating_sub(mark.at) <= events::GITHUB_CHAIN_WINDOW_MS
                    })
                    .count()
            });
            // Codync receives GitHub events on the same signed webhook path as every other event;
            // there is no relay route to go offline, so the decision gets `online: true`. The
            // `offline` reason stays part of the pure decision and the history contract.
            events::delivery_reason(
                event,
                &filters,
                &events::DeliveryContext {
                    changed_object: None,
                    recent_runs: recent,
                    online: true,
                    can_run: r.enabled && !busy(&s, id),
                },
            )
        };
        if let Some(reason) = reason {
            self.note_github_delivery(store, id, event, delivery, "skipped", Some(reason.as_str()), None);
            false
        } else {
            true
        }
    }

    /// Records one chat mention that a workflow trigger considered. Ids only, never message text.
    #[allow(clippy::too_many_arguments, reason = "one row of the chat history")]
    fn note_chat_delivery(
        &self,
        store: &Store,
        id: &str,
        source: &str,
        event: &Value,
        outcome: &str,
        reason: Option<&str>,
        run_id: Option<String>,
        message: &str,
    ) {
        let saved = self.change(store, |s| {
            s.chat_deliveries.push(ChatDelivery {
                routine_id: id.to_owned(),
                source: source.to_owned(),
                connection: event["connection"].as_str().unwrap_or_default().to_owned(),
                channel: event["channel"].as_str().or_else(|| event["chat"].as_str()).unwrap_or_default().to_owned(),
                message_id: message.to_owned(),
                at: now_ms(),
                outcome: outcome.to_owned(),
                reason: reason.map(str::to_owned),
                run_id,
            });
            if s.chat_deliveries.len() > MAX_CHAT_DELIVERIES {
                s.chat_deliveries.drain(..s.chat_deliveries.len() - MAX_CHAT_DELIVERIES);
            }
            Ok(())
        });
        if let Err(error) = saved {
            tracing::warn!(error = format!("{error:#}"), "couldn't record a chat delivery");
        }
    }

    /// Writes a webhook receipt in the same state change as nothing else: a repeat of the same id
    /// within seven days is a no-op, and old receipts are pruned on write.
    fn note_receipt(&self, store: &Store, id: &str, delivery: Option<&str>, reason: Option<&str>) {
        let Some(key) = delivery else { return };
        let saved = self.change(store, |s| {
            if s.webhook_receipts
                .iter()
                .any(|r| r.routine_id == id && r.delivery_id == key && now_ms() - r.at <= WEBHOOK_RECEIPT_TTL_MS)
            {
                return Ok(false);
            }
            s.webhook_receipts.push(WebhookReceipt {
                routine_id: id.to_owned(),
                delivery_id: key.to_owned(),
                at: now_ms(),
                reason: reason.map(str::to_owned),
            });
            let cutoff = now_ms() - WEBHOOK_RECEIPT_TTL_MS;
            s.webhook_receipts.retain(|r| r.at >= cutoff);
            if s.webhook_receipts.len() > MAX_WEBHOOK_RECEIPTS {
                s.webhook_receipts.drain(..s.webhook_receipts.len() - MAX_WEBHOOK_RECEIPTS);
            }
            Ok(true)
        });
        if let Err(error) = saved {
            tracing::warn!(error = format!("{error:#}"), "couldn't record a webhook receipt");
        }
    }

    fn duplicate_receipt(&self, id: &str, delivery: Option<&str>, now: i64) -> bool {
        let Some(key) = delivery else { return false };
        self.state
            .locked()
            .webhook_receipts
            .iter()
            .any(|r| r.routine_id == id && r.delivery_id == key && now.saturating_sub(r.at) <= WEBHOOK_RECEIPT_TTL_MS)
    }
}

/// The outcome of a chat mention handed to workflow trigger nodes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatReceive {
    pub matched: usize,
    pub queued: Vec<String>,
}

impl ChatReceive {
    /// Every node that matched skipped a run: the caller adds the platform's failed reaction.
    pub fn all_skipped(&self) -> bool {
        self.matched > 0 && self.queued.is_empty()
    }
}

fn required<'a>(body: &'a Value, key: &str) -> Result<&'a str> {
    body[key].as_str().with_context(|| format!("{key} is required"))
}
/// A routine policy as the save body names it: the valid name it gives, else the routine's own,
/// else the default for a new one.
fn policy(
    value: Option<&str>,
    valid: &[&str],
    old: Option<&Routine>,
    of: impl Fn(&Routine) -> &str,
    default: &str,
) -> String {
    match value.filter(|p| valid.contains(p)) {
        Some(name) => name.to_owned(),
        None => old.map_or_else(|| default.to_owned(), |r| of(r).to_owned()),
    }
}
pub(crate) fn public(r: &Routine) -> Value {
    let (output_text, output_text_error) = match output_text::format(&r.outputs) {
        Ok(text) => (Some(text), None),
        Err(error) => (None, Some(error.to_string())),
    };
    json!({"id":r.id,"botId":r.bot_id,"name":r.name,"instruction":r.instruction,"triggers":r.triggers,"enabled":r.enabled,"createdAt":r.created_at,"updatedAt":r.updated_at,
        "timeoutSeconds":r.timeout_seconds,"lastError":r.last_error,"missedPolicy":r.missed_policy,"limitPolicy":r.limit_policy,
        "outputs":r.outputs,"outputDescriptions":r.outputs.iter().map(outputs::describe).collect::<Vec<_>>(),
        "outputText":output_text,"outputTextError":output_text_error,
        "thread":r.thread,"quiet":r.quiet,
        "triggerDescriptions":r.triggers.iter().map(Trigger::description).collect::<Vec<_>>(),"nextRunAt": if r.enabled {r.next_runs.iter().flatten().min().copied()} else {None}})
}

/// Whether a trigger accepts `event`. The generic match keeps old raw GitHub spellings working;
/// OpenBot's canonical names are checked through [`events::github_aliases`] so a workflow saved with
/// `issue.opened` fires on the same `issues.opened` delivery. Pure, with tests in `events.rs`.
pub(crate) fn trigger_matches(trigger: &Trigger, event: &Value) -> bool {
    if trigger.matches(event) {
        return true;
    }
    if let Trigger::Event { source, event: wanted, filters } = trigger
        && source == "github"
        && events::github_trigger(event, filters).is_run()
    {
        return wanted == "*" || events::github_aliases(event).iter().any(|name| name == wanted);
    }
    false
}

fn new_run(r: &Routine, event: Value, delivery_id: Option<String>) -> Run {
    Run {
        id: uuid::Uuid::new_v4().to_string(),
        routine_id: r.id.clone(),
        bot_id: r.bot_id.clone(),
        status: Status::Pending,
        created_at: now_ms(),
        finished_at: None,
        detail: None,
        event,
        whatsapp_binding: None,
        delivery_id,
        started_at: None,
        thread_root: None,
        report_pending: false,
        result: None,
        attempt: 0,
        not_before: None,
        retry_at: None,
        retryable: false,
        failure_code: None,
        scheduled_for: None,
        missed: None,
    }
}

/// A due occurrence the routine will not run: it stays in the history with what it stood for.
pub(crate) fn cancelled_record(r: &Routine, at: i64, missed: Option<schedule::Missed>, reason: Option<String>) -> Run {
    let mut run = new_run(r, Value::Null, None);
    run.status = Status::Cancelled;
    run.finished_at = Some(now_ms());
    run.scheduled_for = Some(at);
    run.missed = missed;
    run.detail = reason;
    run
}

/// A paused routine still takes test runs.
fn may_run(routine: &Routine, event: &Value) -> bool {
    routine.enabled || event["source"] == "test"
}

/// Whether `routine` has a run under way or queued; a retry waiting for its time doesn't count.
fn busy(s: &State, routine: &str) -> bool {
    let now = now_ms();
    s.runs.iter().any(|r| r.routine_id == routine && r.status.active() && !r.waiting(now))
}

fn cancel_run(run: &mut Run) {
    run.status = Status::Cancelled;
    run.finished_at = Some(now_ms());
    run.event = Value::Null;
}
fn cancel_pending(state: &mut State, id: &str) {
    for run in &mut state.runs {
        if run.routine_id == id && run.status == Status::Pending {
            cancel_run(run);
        }
    }
}

/// The name of a run's own agent session, which it resumes after a restart.
pub fn session_slot(run_id: &str) -> String {
    format!("routine-{run_id}")
}

/// Whether a run posts its own chat notices and push alert. A quiet run keeps its steps in the
/// chat — so it stays visible there — but adds no "Routine · …" start notice, result notice or
/// notification of its own. Pure so the decision is testable without a host.
pub(crate) fn announces(quiet: bool) -> bool {
    !quiet
}

/// The notice a run posts in the chat when it begins; the run's turn is the notice's.
pub fn notice_id(run_id: &str) -> String {
    format!("routine-{run_id}")
}

/// The chat notice that reports a run's outcome.
fn result_notice_id(run_id: &str) -> String {
    format!("routine-result-{run_id}")
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod webhook_tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::hub::Hub;
    use crate::remote::identity::Identity;

    fn hub_with(routine: Routine) -> std::sync::Arc<Hub> {
        let dir = std::env::temp_dir().join(format!("codync-webhook-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let hub = Hub::new(store, "webhooks".into(), Identity::load_or_create(&dir).unwrap(), "t".into(), 0);
        hub.routines
            .change(&hub.store, |s| {
                s.routines.push(routine);
                Ok(())
            })
            .unwrap();
        std::sync::Arc::new(hub)
    }

    fn bearer() -> hooks::Headers {
        hooks::Headers::from([("authorization".into(), "Bearer secret".into())])
    }

    #[test]
    fn a_senders_own_secret_authorizes_a_delivery_and_the_test_event_runs() {
        let mut routine = tests::routine();
        routine.triggers = vec![Trigger::Webhook];
        let hub = hub_with(routine);
        hub.routines.set_sender_secret(&hub.store, "bot", "routine", Some("senders-secret".into())).unwrap();
        let body = br#"{"source":"test","text":"hello"}"#;
        let now = now_ms();
        let timestamp = (now / 1000).to_string();
        let signature = events::sign_sender("senders-secret", "msg-1", &timestamp, body).unwrap();
        let headers = hooks::Headers::from([
            ("webhook-id".into(), "msg-1".into()),
            ("webhook-timestamp".into(), timestamp),
            ("webhook-signature".into(), signature),
        ]);
        let answer = hub.routines.receive(&hub, "routine", &headers, body).unwrap();
        assert!(answer["run"]["id"].is_string(), "the sender's own signature started a run");
        let mut tampered = headers.clone();
        tampered.insert("webhook-id".into(), "msg-2".into());
        assert_eq!(hub.routines.receive(&hub, "routine", &tampered, body), Err(Refused::Unauthorized));
        assert!(hub.routines.receive(&hub, "routine", &bearer(), body).is_ok(), "the routine key still works");

        let tested = hub.routines.test_webhook(&hub, "bot", "routine").unwrap();
        assert_eq!(tested["test"], true);
        assert!(tested["event"]["source"] == "test");
        assert!(tested["answer"]["run"]["id"].is_string(), "the signed test went through the real path");
        assert!(!serde_json::to_string(&tested).unwrap().contains("senders-secret"), "no secret is returned");

        hub.routines.set_sender_secret(&hub.store, "bot", "routine", None).unwrap();
        assert_eq!(hub.routines.receive(&hub, "routine", &headers, body), Err(Refused::Unauthorized));
    }

    #[test]
    fn paused_and_non_matching_deliveries_land_in_a_bounded_history() {
        let mut routine = tests::routine();
        routine.triggers =
            vec![Trigger::Event { source: "github".into(), event: "issues.opened".into(), filters: BTreeMap::new() }];
        routine.enabled = false;
        let hub = hub_with(routine);
        let body = br#"{"source":"slack","event":"app_mention"}"#;
        assert_eq!(
            hub.routines.receive(&hub, "routine", &bearer(), body),
            Err(Refused::Paused),
            "a routine created enabled=false is paused"
        );
        hub.routines.set_enabled(&hub.store, "bot", "routine", true).unwrap();
        let answer = hub.routines.receive(&hub, "routine", &bearer(), body).unwrap();
        assert_eq!(answer["reason"], "mismatch");

        let history = hub.routines.ignored("bot", "routine");
        let ignored = history["ignored"].as_array().unwrap();
        assert_eq!(ignored.len(), 2);
        assert_eq!(ignored[0]["reason"], "paused");
        assert_eq!(ignored[1]["reason"], "mismatch");
        assert!(hub.routines.ignored("other", "routine")["ignored"].as_array().unwrap().is_empty());

        for i in 0..MAX_IGNORED_DELIVERIES + 5 {
            hub.routines.note_ignored(&hub.store, "routine", Some(&format!("d-{i}")), "mismatch");
        }
        let kept = hub.routines.ignored("bot", "routine")["ignored"].as_array().unwrap().len();
        assert_eq!(kept, MAX_IGNORED_DELIVERIES, "the history stays bounded");
    }

    #[test]
    fn a_github_trigger_records_deliveries_and_replays_the_last_event() {
        let mut routine = tests::routine();
        routine.triggers =
            vec![Trigger::Event { source: "github".into(), event: "issue.opened".into(), filters: BTreeMap::new() }];
        let hub = hub_with(routine);
        let event = json!({"source":"github","event":"issues.opened","repo":"o/r","payload":{
            "action":"opened","issue":{"id":1,"number":7,"title":"Hi"},
            "sender":{"login":"ana","type":"User"}}});
        let queued = hub.routines.receive_event(&hub.store, &event, Some("g-1".into()));
        assert_eq!(queued, vec!["routine".to_owned()]);
        let history = hub.routines.github_history("bot", "routine");
        assert_eq!(history["deliveries"][0]["outcome"], "started");
        assert_eq!(history["deliveries"][0]["item"]["number"], 7);
        assert!(history["deliveries"][0]["reason"].is_null());
        assert!(hub.routines.github_history("other", "routine")["deliveries"].as_array().unwrap().is_empty());

        // The same delivery id is a no-op; a filter miss is recorded with its skip reason.
        assert!(hub.routines.receive_event(&hub.store, &event, Some("g-1".into())).is_empty());
        hub.routines
            .change(&hub.store, |s| {
                let r = s.routines.iter_mut().find(|r| r.id == "routine").unwrap();
                r.triggers = vec![Trigger::Event {
                    source: "github".into(),
                    event: "issue.opened".into(),
                    filters: BTreeMap::from([("title".into(), "absent".into())]),
                }];
                for run in &mut s.runs {
                    run.status = Status::Succeeded;
                }
                Ok(())
            })
            .unwrap();
        assert!(hub.routines.receive_event(&hub.store, &event, Some("g-2".into())).is_empty());
        let history = hub.routines.github_history("bot", "routine");
        assert_eq!(history["deliveries"][0]["reason"], "filter");

        // The last event is kept for a replay even after the filters changed.
        let replay = hub.routines.run_last_event(&hub, "bot", "routine").unwrap();
        assert!(replay["run"]["id"].is_string());
        assert!(hub.routines.run_last_event(&hub, "other", "routine").is_err());
    }

    #[test]
    fn chat_triggers_queue_a_run_and_report_when_every_node_skips() {
        let mut routine = tests::routine();
        routine.triggers = vec![Trigger::Event {
            source: "discord".into(),
            event: "message.mention".into(),
            filters: BTreeMap::from([("connection".into(), "G1".into()), ("roles".into(), "admin".into())]),
        }];
        let hub = hub_with(routine);
        let event = json!({"source":"discord","event":"message.mention","connection":"G1","channel":"C1",
            "roles":["admin"],"text":"Deploy the fix","messageId":"m1"});
        let first = hub.routines.receive_chat_event(&hub.store, &event, Some("m1".into()));
        assert_eq!(first.matched, 1);
        assert_eq!(first.queued, vec!["routine".to_owned()]);
        assert!(!first.all_skipped());

        let history = hub.routines.chat_history("bot", "routine");
        assert_eq!(history["deliveries"][0]["messageId"], "m1");
        assert_eq!(history["deliveries"][0]["outcome"], "started");
        assert!(!serde_json::to_string(&history).unwrap().contains("Deploy the fix"), "no message text is kept");

        // The queued run holds the workflow, so the next matching mention is skipped by every node.
        let second = hub.routines.receive_chat_event(&hub.store, &event, Some("m2".into()));
        assert_eq!(second.matched, 1);
        assert!(second.all_skipped(), "the caller adds the failed reaction");
        assert_eq!(hub.routines.chat_history("bot", "routine")["deliveries"][0]["reason"], "busy");

        let wrong = json!({"source":"discord","event":"message.mention","connection":"G2","channel":"C1","text":"x"});
        assert_eq!(hub.routines.receive_chat_event(&hub.store, &wrong, None).matched, 0);
    }

    #[test]
    fn webhook_receipts_deduplicate_a_delivery_id() {
        let mut routine = tests::routine();
        routine.triggers = vec![Trigger::Webhook];
        let hub = hub_with(routine);
        let mut headers = bearer();
        headers.insert("x-delivery-id".into(), "delivery-1".into());
        let body = br#"{"source":"test","text":"hello"}"#;
        let first = hub.routines.receive(&hub, "routine", &headers, body).unwrap();
        assert!(first["run"]["id"].is_string());
        let second = hub.routines.receive(&hub, "routine", &headers, body).unwrap();
        assert_eq!(second["duplicate"], true);
        let receipts = hub.routines.webhook_receipts("bot", "routine");
        assert_eq!(receipts["receipts"].as_array().unwrap().len(), 1);
        assert!(receipts["receipts"][0]["reason"].is_null());
        assert!(hub.routines.webhook_receipts("other", "routine")["receipts"].as_array().unwrap().is_empty());
    }
}
