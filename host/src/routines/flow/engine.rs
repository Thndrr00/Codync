//! Running a flow: after a routine's run ends, level by level from the routine's own bot, each
//! bot with the replies of the bots linked to it.

use super::{answer_paths, answers, graph, nodes};
use crate::chat::team::ModelOverride;
use crate::chat::team::flow::{FlowCancelled, Handoff, handoff, recipient};
use crate::hub::Hub;
use crate::routines::{Routine, Run, Status, Trigger, local_zone};
use crate::store::{BotConfig, EntryKind, FlowLink, FlowStep, Lane, NewStep, StepStatus, Store, flow_steps};
use answers::ValueProblem;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use tokio::sync::{oneshot, watch};
use tokio::task::JoinSet;

/// Why a step after a cancelled flow never ran.
const CANCELLED_BEFORE: &str = "The flow was cancelled before this step started";

/// Approval steps wait here for `decideWorkflowStep`. The card is the durable record; this map only
/// holds the in-flight waiter, so a restart leaves the step to recovery like any other running step.
static APPROVALS: OnceLock<Mutex<HashMap<String, oneshot::Sender<nodes::ApprovalOutcome>>>> = OnceLock::new();

fn approvals() -> &'static Mutex<HashMap<String, oneshot::Sender<nodes::ApprovalOutcome>>> {
    APPROVALS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Registers the waiter for one approval step and returns the decision channel.
pub(super) fn wait_approval(step_id: &str) -> oneshot::Receiver<nodes::ApprovalOutcome> {
    let (tx, rx) = oneshot::channel();
    approvals().lock().expect("an approval map").insert(step_id.to_owned(), tx);
    rx
}

/// Wakes an approval step with the person's decision. False means no engine is waiting (a run that
/// stopped or restarted); the card and the step still carry the decision.
pub(super) fn decide_approval(step_id: &str, outcome: nodes::ApprovalOutcome) -> bool {
    approvals().lock().expect("an approval map").remove(step_id).is_some_and(|tx| tx.send(outcome).is_ok())
}

/// OpenBot 0.77.0: a loop runs each step once per round. Stored flows are acyclic, so a run here is
/// a single round; the pure round decisions still guard the answers a step reads and how often it
/// runs if a loop graph ever reaches the engine.
const RUN_ROUND: u32 = 0;

/// OpenBot 0.77.0: an agent step waits `queued` while its message reaches the agent, and only the
/// hand-off's delivery moves it to `running`. A media step makes its own file, so it starts running.
fn initial_step_status(node: &str) -> StepStatus {
    if node.starts_with(crate::store::flow_media::PREFIX) { StepStatus::Running } else { StepStatus::Queued }
}

/// OpenBot 0.74.0: a routine result that waits on a teammate's answer is posted with that answer,
/// and never waits longer than a day for it. In Codync a teammate the routine handed work to is a
/// flow step: the post below waits for the last step's reply, and each handoff is bounded by this
/// day limit as well as the run's own timeout.
pub const TEAMMATE_ANSWER_WAIT: Duration = Duration::from_secs(24 * 60 * 60);

/// How long to wait for one teammate's reply: the run's timeout, capped at a day.
fn teammate_wait(timeout: Duration) -> Duration {
    timeout.min(TEAMMATE_ANSWER_WAIT)
}

/// An agent step that replied with a pass (or nothing at all) has nothing to report: it stays
/// silent, handing nothing to the next step and adding nothing to the run's answer. This is the
/// host's no-update path; the pure decision keeps it identical in the send, handoff and output code.
fn nothing_to_report(reply: &str) -> bool {
    crate::chat::group::is_pass(reply)
}

/// The answer posted to "Send answers to": the final teammates' replies, not the text the routine's
/// own agent wrote before it asked them. Silent replies are left out, so a step with nothing to say
/// doesn't push the real answer away.
fn final_answer(replies: &[&str]) -> String {
    replies
        .iter()
        .map(|reply| reply.trim())
        .filter(|reply| !nothing_to_report(*reply))
        .collect::<Vec<_>>()
        .join("\n\n---\n\n")
}

/// The `{{name}}` values an output template can read: the trigger's fixed fields and the checked
/// answers the steps of this run gave, keyed by field name. A supplier's answer wins over a trigger
/// field of the same name, the same as in a step's instruction.
fn output_values(hub: &Hub, start: &Start) -> BTreeMap<String, String> {
    let mut values = BTreeMap::new();
    super::trigger_fields::merged_values(&mut values, &start.trigger_fields);
    let Ok(steps) = hub.store.flow_run_steps(&start.run_id) else { return values };
    for step in steps {
        let Some(output) = step.output.as_deref() else { continue };
        let Some(fields) = start.answer_specs.get(&step.bot_id).map(|spec| answers::fields_of(spec)) else { continue };
        if fields.is_empty() {
            continue;
        }
        let Some(value) = answers::last_json_object(output) else { continue };
        let (answer, _) = answers::check_answer(&fields, &value, answers::ExtraKeys::Ignore);
        for (name, field) in answer {
            values.insert(name, answers::display_value(&field));
        }
    }
    values
}

/// How the routine's own run ended.
#[derive(Clone, Serialize, Deserialize)]
pub enum Root {
    Replied(String),
    Failed(String),
    Cancelled(String),
}

/// One input a step read, and the link instruction that led to it. Kept with the result so the
/// canvas can show what the step actually saw, even if the link has since changed.
#[derive(Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StepInput {
    #[serde(default)]
    pub from: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub instruction: String,
    /// A picture or clip handed on by a media step, as opposed to a bot's text reply.
    #[serde(default)]
    pub picture: bool,
}

/// The prompt, model and other fields a step ran with, plus the inputs it read. Kept as JSON on the
/// step row, so runs saved before this field existed stay readable with no settings shown.
#[derive(Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StepSettings {
    /// `bot` or `media` (`image`/`video` media steps are distinguished by `kind`).
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub model_options: BTreeMap<String, String>,
    #[serde(default)]
    pub instruction: String,
    #[serde(default)]
    pub prompt: String,
    #[serde(default)]
    pub aspect: String,
    /// How many results a media step makes. Older runs show one.
    #[serde(default = "crate::media::default_batch")]
    pub batch: u32,
    /// One status per result, so a client can show progress and a Retry on a failed one.
    #[serde(default)]
    pub results: Vec<crate::media::BatchResult>,
    #[serde(default)]
    pub duration: Option<u32>,
    #[serde(default)]
    pub resolution: String,
    #[serde(default)]
    pub transparent: Option<bool>,
    #[serde(default)]
    pub voice: Option<String>,
    #[serde(default)]
    pub inputs: Vec<StepInput>,
    /// The card an approval step waits on, so `decideWorkflowStep` can settle it.
    #[serde(default)]
    pub entry_id: Option<String>,
}

/// One stored step of the run a "run from this step" branches from: its answer and its settings.
#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SeededStep {
    #[serde(default)]
    pub output: Option<String>,
    #[serde(default)]
    pub settings: Option<serde_json::Value>,
}

/// One routine run whose flow should run.
#[derive(Clone, Serialize, Deserialize)]
pub struct Start {
    pub run_id: String,
    pub routine_id: String,
    pub routine_name: String,
    pub owner: String,
    /// When the run began: only the links made before apply.
    pub run_at: i64,
    /// What each step may take, inherited from the routine.
    pub timeout: Duration,
    /// How many hops from the routine's own bot the flow may reach.
    pub max_depth: usize,
    /// The routine's instruction, the root step's input.
    pub instruction: String,
    /// OpenBot 0.74.0: text for one case of a one-off workflow run. It goes to the first steps only;
    /// the saved routine and its links never change. Kept with the pending flow across a restart.
    #[serde(default)]
    pub case_text: Option<String>,
    /// Where the last reply goes.
    pub outputs: Vec<crate::routines::Output>,
    /// The delivery that started the run, for outputs that answer it.
    pub event: serde_json::Value,
    pub root: Root,
    /// Set when the run begins at one step instead of at the routine's own bot: that step and what
    /// hangs below it run again; the stored answers of the steps before it are handed on as they are.
    #[serde(default)]
    pub from_step: Option<String>,
    /// Those stored answers and settings, keyed by the step's node key (bot id or `media:<id>`).
    #[serde(default)]
    pub seed: BTreeMap<String, SeededStep>,
    /// The run's start time in UTC and the schedule's zone, for steps that compute a time window.
    #[serde(default)]
    pub run_time_text: String,
    /// The answer specs of this flow's steps, frozen when the run began. A later edit to a link
    /// never changes what a retry of this run checks its answer against.
    #[serde(default)]
    pub answer_specs: BTreeMap<String, answers::NodeSpec>,
    /// The answer fields of the trigger that started this run, also frozen for its whole life.
    #[serde(default)]
    pub run_fields: Vec<answers::AnswerField>,
    /// Fixed fields the trigger gave the steps after it (`title`, `author`, `weekday`…), frozen on
    /// the run so a retry or a run from one step sees the same values as the first attempt.
    #[serde(default)]
    pub trigger_fields: answers::Answer,
    /// Which attempt of this step this is. A retry from a failed step increments it.
    #[serde(default = "first_attempt")]
    pub attempt: u32,
}

fn first_attempt() -> u32 {
    1
}

impl Start {
    /// Loads the specs a run started with and checks a webhook's body against its trigger's fields.
    /// Called once before the flow executes, so a restart replay uses the same rules as the first
    /// attempt. A flow saved before typed answers existed simply has no fields to check.
    fn prepare(&mut self, store: &Store) {
        let specs = answers::load_specs(store, &self.owner);
        let current = answers::fields_of(
            specs.get(&format!("routine:{}", self.routine_id)).unwrap_or(&answers::NodeSpec::default()),
        );
        let frozen = (!self.run_fields.is_empty()).then_some(self.run_fields.as_slice());
        self.run_fields = answer_paths::with_run_spec(&current, frozen).to_vec();
        if self.answer_specs.is_empty() {
            self.answer_specs = specs;
        }
        if !self.run_fields.is_empty()
            && let Some(data) = self.event.get("data")
        {
            let problems = answer_paths::webhook_problems(&self.run_fields, data);
            if !problems.is_empty() {
                self.root = Root::Failed(format!(
                    "The workflow cannot start: the webhook body did not fit its answer fields ({}).",
                    answers::fix_text(&problems)
                ));
            }
        }
    }
}

/// What a bot's step left for the bots after it.
#[derive(Clone)]
pub(super) enum Outcome {
    Replied(String),
    /// Nothing to pass on: the bot had nothing to report, or the step didn't run or finish.
    Nothing,
}

/// A step ready to be put to its bot.
struct Pending {
    step: FlowStep,
    from: crate::store::BotConfig,
    bot: String,
    body: String,
    /// The first incoming link's own model and options, when they name any.
    model: Option<ModelOverride>,
    hops: u8,
    /// The fields this step's answer must give. Empty keeps the old free-text handoff.
    fields: Vec<answers::AnswerField>,
}

/// A media step ready to make its file.
struct MediaPending {
    step: FlowStep,
    node: crate::store::FlowMedia,
    /// What the steps before it handed on.
    body: String,
    /// What the canvas shows while the batch runs; updated after each result.
    settings: StepSettings,
}

/// An approval step paused on its card. The waiter is registered before the card is posted so a
/// quick decision can never be lost; `approve` settles the step and continues the taken branch.
struct ApprovalPending {
    step: FlowStep,
    handoff: String,
    rx: oneshot::Receiver<nodes::ApprovalOutcome>,
}

enum Begin {
    Send(Box<Pending>),
    Make(Box<MediaPending>),
    Approve(Box<ApprovalPending>),
    Done(Outcome),
}

/// The branch a source step took, as saved on its row. Only routing nodes have one; every other
/// node continues along its unbranched links.
fn step_branch(branch: Option<&str>) -> Option<nodes::Branch> {
    match branch {
        Some("yes") => Some(nodes::Branch::Yes),
        Some("no") => Some(nodes::Branch::No),
        _ => None,
    }
}

/// An approval settles its step with the decision, which is the branch the flow continues on.
fn approval_branch(output: Option<&str>) -> Option<nodes::Branch> {
    match output {
        Some("approved") => Some(nodes::Branch::Yes),
        Some("rejected") => Some(nodes::Branch::No),
        _ => None,
    }
}

/// Whether a link leaves its source on the branch that source actually took.
fn link_follows_step(link: &FlowLink, branches: &BTreeMap<String, Option<nodes::Branch>>) -> bool {
    nodes::follows_branch(link.branch.as_deref(), branches.get(&link.from_bot_id).copied().flatten())
}

pub async fn execute(hub: Arc<Hub>, mut start: Start) {
    let (run, routine, owner) = (start.run_id.clone(), start.routine_id.clone(), start.owner.clone());
    start.prepare(&hub.store);
    match run_flow(&hub, Arc::new(start)).await {
        Ok(()) => {
            if let Err(error) = hub.routines.ack_flow(&hub.store, &run) {
                tracing::error!(run, %error, "could not acknowledge routine flow; recovery will replay it idempotently");
            }
            // Freeze the graph this run used, so a person can go back to it later.
            if let Err(error) = super::document::record_run_version(&hub.store, &owner) {
                tracing::warn!(run, %error, "could not record the flow version used by a run");
            }
        }
        Err(error) => tracing::error!(run, routine, error = format!("{error:#}"), "routine flow failed"),
    }
}

/// Saves a step (or finds the one a retry already made: `None`) and tells the clients.
pub(super) fn record(hub: &Hub, new: &NewStep) -> Result<Option<FlowStep>> {
    hub.emit_flow_change(|| {
        Ok(match hub.store.flow_add_step(new)? {
            Some((step, change)) => (Some(step), Some(change)),
            None => (None, None),
        })
    })
}

/// Keeps what a step actually ran with on its row, so "Settings" can show it later. A failure here
/// doesn't undo the step: the settings are a record of the run, not the run itself.
fn record_settings(hub: &Hub, step: &str, settings: &StepSettings) {
    if let Err(error) = hub.store.flow_set_step_settings(step, serde_json::to_value(settings).ok()) {
        tracing::warn!(step, error = format!("{error:#}"), "couldn't save a routine flow step's settings");
    }
}

/// Saves a batch's per-result status and tells clients to reload the canvas. A progress write that
/// fails does not undo the file already made.
fn record_progress(hub: &Hub, step: &str, settings: &StepSettings) {
    let Ok(value) = serde_json::to_value(settings) else { return };
    if let Err(error) = hub.emit_flow_change(|| Ok(((), hub.store.flow_progress_step(step, &value)?))) {
        tracing::warn!(step, error = format!("{error:#}"), "couldn't save a media batch's progress");
    }
}

/// Copies a stored step's settings into the new step of a "run from this step" run, so reused steps
/// show what they ran with earlier too.
fn copy_settings(hub: &Hub, from: Option<&serde_json::Value>, to: &str) {
    if let Some(settings) = from
        && let Err(error) = hub.store.flow_set_step_settings(to, Some(settings.clone()))
    {
        tracing::warn!(step = to, error = format!("{error:#}"), "couldn't copy a routine flow step's settings");
    }
}

fn settle(hub: &Hub, id: &str, status: StepStatus, output: Option<&str>, error: Option<&str>) {
    settle_with(hub, id, status, output, error, &[]);
}

fn settle_with(
    hub: &Hub,
    id: &str,
    status: StepStatus,
    output: Option<&str>,
    error: Option<&str>,
    files: &[crate::chat::files::SharedFile],
) {
    if let Err(e) = hub.emit_flow_change(|| Ok(((), hub.store.flow_settle_step(id, status, output, error, files)?))) {
        tracing::error!(step = id, error = format!("{e:#}"), "couldn't save a routine flow step");
    }
}

fn new_step<'a>(start: &'a Start, bot: &'a str, input: &'a str, status: StepStatus) -> NewStep<'a> {
    let source =
        flow_steps::run_source(start.event.get("source").and_then(serde_json::Value::as_str).unwrap_or("manual"));
    NewStep {
        run_id: &start.run_id,
        routine_id: &start.routine_id,
        owner_bot_id: &start.owner,
        bot_id: bot,
        run_at: start.run_at,
        input,
        status,
        output: None,
        error: None,
        attempt: start.attempt,
        branch: None,
        waiting: matches!(status, StepStatus::Queued),
        source,
    }
}

fn record_root(hub: &Hub, start: &Start) -> Result<()> {
    let mut step = new_step(start, &start.owner, &start.instruction, StepStatus::Succeeded);
    match &start.root {
        Root::Replied(text) => step.output = Some(text),
        Root::Failed(error) => (step.status, step.error) = (StepStatus::Failed, Some(error.as_str())),
        Root::Cancelled(error) => (step.status, step.error) = (StepStatus::Cancelled, Some(error.as_str())),
    }
    record(hub, &step).map(drop)
}

/// Ends a step that never ran, with `reason`.
pub(super) fn record_ended(hub: &Hub, start: &Start, bot: &str, status: StepStatus, reason: &str) -> Result<()> {
    let mut step = new_step(start, bot, "", status);
    step.error = Some(reason);
    record(hub, &step).map(drop)
}

fn skip_all<'a>(hub: &Hub, start: &Start, bots: impl IntoIterator<Item = &'a str>, reason: &str) -> Result<()> {
    bots.into_iter().try_for_each(|bot| record_ended(hub, start, bot, StepStatus::Skipped, reason))
}

async fn run_flow(hub: &Arc<Hub>, start: Arc<Start>) -> Result<()> {
    if !hub.routines.flow_allowed(&start) {
        return Ok(());
    }
    // A paused workflow cannot start from a schedule, webhook, chat mention or GitHub event. A
    // person running it by hand still goes through; an occurrence that came due while paused is
    // skipped rather than made up later. The same rule applies to the flow of an automatic run.
    let paused = super::pause::is_paused(&hub.store, &start.routine_id);
    let manual = super::pause::manual_event(&start.event) || start.from_step.is_some();
    if !super::pause::automatic_start_allowed(paused, manual) {
        if super::pause::occurrence_skipped(paused) && start.event.get("scheduledFor").is_some() {
            tracing::info!(
                run = start.run_id,
                routine = start.routine_id,
                "skipped a schedule occurrence while paused"
            );
        }
        return Ok(());
    }
    let links: Vec<FlowLink> =
        hub.store.flow_links(&start.routine_id)?.into_iter().filter(|l| l.created_at <= start.run_at).collect();
    if links.is_empty() {
        if let Root::Replied(reply) = &start.root {
            hub.routines.queue_flow_outputs(&hub.store, &start, reply)?;
        }
        return Ok(());
    }
    let running = hub.routines.flows.register(&start.run_id, &start.routine_id, &start.owner);
    record_root(hub, &start)?;
    let Root::Replied(reply) = &start.root else {
        let reached = graph::reached(&start.owner, &links);
        return skip_all(
            hub,
            &start,
            reached.iter().map(String::as_str),
            "Skipped: the routine's own run didn't finish",
        );
    };
    let Ok(depths) = graph::depths(&start.owner, &links) else {
        // Connecting refuses cycles, so this means the stored links were changed behind the host's back.
        tracing::error!(
            run = start.run_id,
            routine = start.routine_id,
            "routine flow links form a cycle; nothing was handed on"
        );
        let reached = graph::reached(&start.owner, &links);
        return skip_all(hub, &start, reached.iter().map(String::as_str), "Skipped: the routine's links form a loop");
    };
    // "Run from this step" seeds the answers of the steps before it and re-runs only that step and
    // what hangs below it. A full run starts from the routine's own bot as it always has.
    let rerun = start.from_step.as_deref().map(|step| graph::downstream(step, &links));
    let mut results: HashMap<String, Outcome> = if let Some(rerun) = &rerun {
        start
            .seed
            .iter()
            .map(|(bot, saved)| (bot.clone(), saved_outcome(saved.output.as_deref(), saved.output.is_some())))
            .collect()
    } else {
        HashMap::from([(start.owner.clone(), Outcome::Replied(reply.clone()))])
    };
    if !results.contains_key(&start.owner) {
        results.insert(start.owner.clone(), Outcome::Replied(reply.clone()));
    }
    let order = graph::order(&depths);
    let deepest = order.last().map_or(0, |(_, depth)| *depth);
    for depth in 1..=deepest {
        let mut tasks = JoinSet::new();
        for (bot, _) in order.iter().filter(|(_, d)| *d == depth) {
            // A step before the start keeps its stored answer; its work is never sent again. The
            // new run still gets a step for it, so the canvas shows the whole path it continued.
            if let Some(rerun) = &rerun
                && !rerun.contains(*bot)
            {
                if let Some(saved) = start.seed.get(*bot) {
                    let mut reused = new_step(&start, bot, "", StepStatus::Succeeded);
                    reused.output = saved.output.as_deref();
                    if let Some(step) = record(hub, &reused)? {
                        copy_settings(hub, saved.settings.as_ref(), &step.id);
                    }
                }
                continue;
            }
            match begin(hub, &start, &links, &results, bot, depth, &running.cancel)? {
                Begin::Done(outcome) => {
                    results.insert((*bot).to_owned(), outcome);
                }
                Begin::Send(pending) => {
                    let (hub, start, cancel) = (hub.clone(), start.clone(), running.cancel.clone());
                    tasks.spawn(async move { (pending.bot.clone(), send(&hub, &start, *pending, cancel).await) });
                }
                Begin::Make(pending) => {
                    // Steps are keyed by their node, the same key the links use.
                    let node = crate::store::flow_media::key(&pending.node.id);
                    let (hub, start, cancel) = (hub.clone(), start.clone(), running.cancel.clone());
                    tasks.spawn(async move { (node, make(&hub, &start, *pending, cancel).await) });
                }
                Begin::Approve(pending) => {
                    let node = pending.step.bot_id.clone();
                    let (hub, cancel) = (hub.clone(), running.cancel.clone());
                    tasks.spawn(async move { (node, approve(&hub, *pending, cancel).await) });
                }
            }
        }
        while let Some(joined) = tasks.join_next().await {
            match joined {
                Ok((bot, outcome)) => {
                    results.insert(bot, outcome);
                }
                Err(error) => tracing::error!(error = %error, "a routine flow step's task ended abnormally"),
            }
        }
    }
    // The bots nothing hangs below carry the workflow's result out.
    if !start.outputs.is_empty() && !*running.cancel.borrow() {
        let mut final_bots: Vec<&String> =
            results.keys().filter(|bot| !links.iter().any(|link| &link.from_bot_id == *bot)).collect();
        final_bots.sort();
        let replies = final_bots
            .iter()
            .filter_map(|bot| match results.get(*bot) {
                Some(Outcome::Replied(text)) => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        let template = super::output_template::load(&hub.store, &start.routine_id);
        let text =
            super::output_template::message_or_input(&template, &final_answer(&replies), &output_values(hub, &start));
        if !text.is_empty() {
            hub.routines.queue_flow_outputs(&hub.store, &start, &text)?;
        }
    }
    Ok(())
}

/// Decides one bot's step: its inputs are the replies of the bots linked to it that replied.
#[allow(clippy::too_many_arguments)]
fn begin(
    hub: &Hub,
    start: &Start,
    links: &[FlowLink],
    results: &HashMap<String, Outcome>,
    bot: &str,
    depth: usize,
    cancel: &watch::Receiver<bool>,
) -> Result<Begin> {
    let end = |status, reason: &str| -> Result<Begin> {
        record_ended(hub, start, bot, status, reason)?;
        Ok(Begin::Done(Outcome::Nothing))
    };
    // A completed step is a durable result, independent of whether its bot is still visible or
    // runnable. Resolve it before current recipient/input checks when replaying after a crash, but
    // check a stored answer against the fields this run started with: the workflow may have changed.
    let steps = if *cancel.borrow() { Vec::new() } else { hub.store.flow_run_steps(&start.run_id)? };
    let ran_in_round: BTreeMap<String, u32> = steps.iter().map(|step| (step.bot_id.clone(), RUN_ROUND)).collect();
    if !answer_paths::may_run_in_round(bot, RUN_ROUND, &ran_in_round)
        && let Some(step) = steps.iter().find(|step| step.bot_id == bot).cloned()
    {
        if let Err(problem) = saved_answer_problem(start, bot, &step) {
            return end(StepStatus::Failed, &problem);
        }
        return Ok(Begin::Done(step_outcome(step)));
    }
    // Fail closed when a field this step uses has no supplier that finished, or when a name or
    // check does not match the steps before it. A flow with no typed steps stays literal.
    let graph = answer_graph(
        &hub.store,
        &start.routine_id,
        &start.owner,
        &start.instruction,
        &start.answer_specs,
        &start.run_fields,
        links,
    );
    // A condition records the branch it took on its step; an approval settles it as its output.
    // Both are read here so a downstream step only sees the link of the branch that was taken.
    let branches: BTreeMap<String, Option<nodes::Branch>> = steps
        .iter()
        .map(|step| {
            let taken = step_branch(step.branch.as_deref()).or_else(|| {
                if graph.nodes.get(&step.bot_id).is_some_and(|node| node.kind == answer_paths::NodeKind::Approval) {
                    approval_branch(step.output.as_deref())
                } else {
                    None
                }
            });
            (step.bot_id.clone(), taken)
        })
        .collect();
    let inputs: Vec<(&FlowLink, &str)> = links
        .iter()
        .filter(|l| l.to_bot_id == bot)
        .filter(|l| link_follows_step(l, &branches))
        .filter_map(|l| match results.get(&l.from_bot_id)? {
            Outcome::Replied(text) => Some((l, text.as_str())),
            Outcome::Nothing => None,
        })
        .collect();
    if depth > start.max_depth {
        return end(StepStatus::Skipped, &format!("Skipped: the flow reaches at most {} bots deep", start.max_depth));
    }
    // A cancelled flow stops here, unless nothing reaches the step at all: then the bot before it
    // was the one that got cancelled, and there is nothing left to hand on.
    let cancelled = *cancel.borrow();
    // A media step makes its file from its own description; whatever was handed on only adds to it.
    if let Some(id) = bot.strip_prefix(crate::store::flow_media::PREFIX) {
        return if cancelled {
            end(StepStatus::Cancelled, CANCELLED_BEFORE)
        } else {
            begin_media(hub, start, id, &inputs)
        };
    }
    let Some((first, _)) = inputs.first() else {
        return end(StepStatus::Skipped, "Skipped: no bot before this one had anything to hand on");
    };
    if cancelled {
        return end(StepStatus::Cancelled, CANCELLED_BEFORE);
    }
    // Routing nodes are decided below, before a bot is asked to take the step.
    let statuses: BTreeMap<String, StepStatus> = steps.iter().map(|step| (step.bot_id.clone(), step.status)).collect();
    if let Some(gap) = answer_paths::step_answer_gap(&graph, bot, &statuses) {
        return end(StepStatus::Failed, &gap.text());
    }
    let fields = graph.nodes.get(bot).map(|node| answers::fields_of(&node.spec)).unwrap_or_default();
    let sources = answer_paths::readable_sources_in_round(&graph, bot, RUN_ROUND, &ran_in_round);
    let mut outputs: BTreeMap<String, String> = steps
        .iter()
        .filter(|step| step.status == StepStatus::Succeeded)
        .filter_map(|step| step.output.as_ref().map(|text| (step.bot_id.clone(), text.clone())))
        .collect();
    outputs.entry(start.owner.clone()).or_insert_with(|| match &start.root {
        Root::Replied(text) => text.clone(),
        _ => String::new(),
    });
    let by_name = answer_paths::answers_from(&sources, &outputs);
    let mut answer: answers::Answer = by_name.iter().map(|(name, (_, value))| (name.clone(), value.clone())).collect();
    // Trigger fields are fixed answer values, so a check such as `author = ana` works in a step
    // with no model before it. A step's own answer wins if it gave the same field a value.
    super::trigger_fields::merged_answer(&mut answer, &start.trigger_fields);
    let node_kind = graph.nodes.get(bot).map(|node| node.kind);
    // A condition has no bot to ask: it routes the answer it just read to one of its two edges. The
    // branch is saved on the step, so the links of the other side carry nothing downstream.
    if node_kind == Some(answer_paths::NodeKind::Condition) {
        let check = graph.nodes.get(bot).map(|node| node.spec.check.as_str()).unwrap_or_default();
        let taken = nodes::branch_for(check, &answer);
        let handoff = handoff_of(inputs);
        let mut fresh = new_step(start, bot, &handoff, StepStatus::Succeeded);
        fresh.branch = Some(taken.as_str().to_owned());
        if let Some(step) = record(hub, &fresh)? {
            record_settings(hub, &step.id, &StepSettings { kind: "condition".to_owned(), ..StepSettings::default() });
        }
        return Ok(Begin::Done(if handoff.is_empty() { Outcome::Nothing } else { Outcome::Replied(handoff) }));
    }
    // An approval shows one card and pauses. `decideWorkflowStep` settles the card and wakes the
    // waiter; the decision is saved as the step's output so the right branch continues the graph.
    if node_kind == Some(answer_paths::NodeKind::Approval) {
        return begin_approval(hub, start, bot, &inputs);
    }
    // A check on an ordinary step still skips it when it does not match; only a routing condition
    // uses the branch edges.
    if let Some(check) = graph.nodes.get(bot).and_then(answer_paths::node_check)
        && !answers::check_passes(&check, &answer)
    {
        return end(StepStatus::Skipped, "Skipped: the condition did not match");
    }
    if let Err(error) = recipient(hub, bot) {
        return end(StepStatus::Failed, &format!("{error:#}"));
    }
    let Some(from) = hub.store.bot(from_id(&first.from_bot_id, &start.owner))?.filter(|b| !b.deleted) else {
        return end(StepStatus::Failed, "The bot that handed work on is gone");
    };
    let mut values = answer_paths::template_values(&by_name, false);
    super::trigger_fields::merged_values(&mut values, &start.trigger_fields);
    let mut names: Vec<StepInput> = inputs
        .iter()
        .map(|(link, text)| {
            let source = start.answer_specs.get(&link.from_bot_id).map(answers::fields_of).unwrap_or_default();
            let plain = answer_paths::readable_input(&source, text).unwrap_or_else(|| (*text).to_owned());
            StepInput {
                from: node_name(hub, &start.routine_id, &link.from_bot_id),
                text: answers::fill_template(&plain, &values),
                instruction: answers::fill_template(&link.instruction, &values),
                picture: is_picture_node(&link.from_bot_id),
            }
        })
        .collect();
    let include_run_time = depth == 1 || start.from_step.is_some();
    let mut body = answers::fill_template(
        &handoff_body(
            &start.routine_name,
            &names,
            start.case_text.as_deref(),
            depth,
            include_run_time.then_some(start.run_time_text.as_str()).filter(|text| !text.is_empty()),
        ),
        &values,
    );
    // A fan-in is one step: the first incoming link's model and options win, as its instruction leads the task.
    let model = inputs.iter().find_map(|(link, _)| override_of(link));
    if !fields.is_empty() {
        let claude = model
            .as_ref()
            .and_then(|override_| override_.model.as_deref())
            .is_some_and(|id| id.to_ascii_lowercase().starts_with("claude"));
        let ask = if claude { answers::claude_ask(&fields) } else { answers::agent_ask(&fields) };
        body.push_str("\n\n");
        body.push_str(&ask);
    }
    let instruction = inputs.first().map(|(link, _)| link.instruction.clone()).unwrap_or_default();
    let settings = StepSettings {
        kind: "bot".to_owned(),
        model: model.as_ref().and_then(|override_| override_.model.clone()),
        model_options: model.as_ref().map(|override_| override_.options.clone()).unwrap_or_default(),
        instruction: answers::fill_template(&instruction, &values),
        inputs: names,
        ..StepSettings::default()
    };
    let mut fresh = new_step(start, bot, &body, initial_step_status(bot));
    fresh.branch = inputs.first().and_then(|(link, _)| link.branch.as_deref());
    match record(hub, &fresh)? {
        Some(step) => {
            record_settings(hub, &step.id, &settings);
            let hops = u8::try_from(depth).unwrap_or(u8::MAX);
            Ok(Begin::Send(Box::new(Pending {
                step,
                from: from.config,
                bot: bot.to_owned(),
                body,
                model,
                hops,
                fields,
            })))
        }
        // A retry of the run finds the step the first attempt made, and doesn't send it twice.
        None => Ok(Begin::Done(earlier_outcome(hub, &start.run_id, bot)?)),
    }
}

/// The answer a routing node passes to the step on the branch it took.
fn handoff_of(inputs: &[(&FlowLink, &str)]) -> String {
    inputs
        .iter()
        .map(|(_, text)| (*text).trim())
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n---\n\n")
}

/// Starts an approval step: one permission card in the owner's chat, its entry id saved on the step
/// for `decideWorkflowStep`, and the waiter that decision will wake.
fn begin_approval(hub: &Hub, start: &Start, bot: &str, inputs: &[(&FlowLink, &str)]) -> Result<Begin> {
    let handoff = handoff_of(inputs);
    let question = bot
        .strip_prefix("approval:")
        .and_then(|id| hub.store.flow_node(&start.routine_id, id).ok().flatten())
        .and_then(|node| {
            node.fields
                .get("question")
                .or_else(|| node.fields.get("prompt"))
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_default();
    let mut fresh = new_step(start, bot, &handoff, StepStatus::Queued);
    fresh.branch = inputs.first().and_then(|(link, _)| link.branch.as_deref());
    let Some(step) = record(hub, &fresh)? else {
        return Ok(Begin::Done(earlier_outcome(hub, &start.run_id, bot)?));
    };
    // Register before the card is visible: a decision can arrive as soon as clients see it.
    let rx = wait_approval(&step.id);
    let mut card = nodes::approval_card(&question, &step.id, &start.owner);
    let cwd = hub.store.bot(&start.owner).ok().flatten().map(|bot| bot.config.cwd).unwrap_or_default();
    card["cwd"] = serde_json::Value::String(cwd);
    let entry = hub.add_entry(&Lane::main(&start.owner), EntryKind::Permission, 0, &card);
    let settings = StepSettings {
        kind: "approval".to_owned(),
        entry_id: entry.as_ref().map(|entry| entry.id.clone()),
        ..StepSettings::default()
    };
    record_settings(hub, &step.id, &settings);
    Ok(Begin::Approve(Box::new(ApprovalPending { step, handoff, rx })))
}

/// Waits for the person's decision, settles the card's step and hands the answer to the branch they
/// chose. A cancellation settles the step as cancelled and leaves the card answered by the API.
async fn approve(hub: &Arc<Hub>, pending: ApprovalPending, mut cancel: watch::Receiver<bool>) -> Outcome {
    let step = pending.step.id.clone();
    let decision = tokio::select! {
        decision = pending.rx => decision.ok(),
        () = cancel.wait_for(|stopped| *stopped) => None,
    };
    let Some(decision) = decision else {
        settle(hub, &step, StepStatus::Cancelled, None, Some(CANCELLED_BEFORE));
        return Outcome::Nothing;
    };
    settle(hub, &step, StepStatus::Succeeded, Some(decision.as_str()), None);
    if pending.handoff.is_empty() { Outcome::Nothing } else { Outcome::Replied(pending.handoff) }
}

/// The graph of answer fields a run started with. The routine's own bot is the trigger that gives
/// the run input; each link end is a step whose spec was frozen when the run began.
pub(super) fn answer_graph(
    store: &Store,
    routine_id: &str,
    owner: &str,
    instruction: &str,
    specs: &BTreeMap<String, answers::NodeSpec>,
    run_fields: &[answers::AnswerField],
    links: &[FlowLink],
) -> answer_paths::Graph {
    let mut graph = answer_paths::Graph::default();
    let root_spec = specs
        .get(&format!("routine:{routine_id}"))
        .cloned()
        .unwrap_or_else(|| answers::NodeSpec { answer: answers::spec_text(run_fields), check: String::new() });
    graph.nodes.insert(
        owner.to_owned(),
        answer_paths::GraphNode {
            id: owner.to_owned(),
            kind: answer_paths::NodeKind::Trigger,
            spec: root_spec,
            instruction: instruction.to_owned(),
        },
    );
    for link in links {
        for id in [&link.from_bot_id, &link.to_bot_id] {
            if graph.nodes.contains_key(id) {
                continue;
            }
            let media = id.starts_with(crate::store::flow_media::PREFIX);
            graph.nodes.insert(
                id.clone(),
                answer_paths::GraphNode {
                    id: id.clone(),
                    kind: if media { answer_paths::NodeKind::Media } else { answer_paths::NodeKind::Agent },
                    spec: specs.get(id).cloned().unwrap_or_default(),
                    instruction: link.instruction.clone(),
                },
            );
        }
        graph.edges.push(answer_paths::GraphEdge {
            from: link.from_bot_id.clone(),
            to: link.to_bot_id.clone(),
            branch: link.branch.clone(),
            is_loop: false,
        });
    }
    // Routed nodes carry their answer/check fields themselves rather than through a link's spec.
    // Loading them here keeps the graph of a run identical to the one the canvas editor checks.
    if let Ok(saved) = store.flow_nodes(routine_id) {
        for node in saved {
            let check = node.fields.get("check").and_then(Value::as_str).unwrap_or_default().to_owned();
            let answer = node.fields.get("answer").and_then(Value::as_str).unwrap_or_default().to_owned();
            let instruction = node.fields.get("prompt").and_then(Value::as_str).unwrap_or_default().to_owned();
            let kind = match node.kind.as_str() {
                "llm" => answer_paths::NodeKind::Model,
                "condition" => answer_paths::NodeKind::Condition,
                "approval" => answer_paths::NodeKind::Approval,
                "trigger" => answer_paths::NodeKind::Trigger,
                _ => answer_paths::NodeKind::Other,
            };
            let key = node.key();
            graph.nodes.insert(
                key.clone(),
                answer_paths::GraphNode { id: key, kind, spec: answers::NodeSpec { answer, check }, instruction },
            );
        }
    }
    graph
}

/// A retry reuses a step that already ran in this run, but not when its stored answer no longer
/// fits the fields the run started with: a new spec must not validate an old answer.
fn saved_answer_problem(start: &Start, bot: &str, step: &FlowStep) -> std::result::Result<(), String> {
    let fields = answer_spec_fields(start, bot);
    if fields.is_empty() {
        return Ok(());
    }
    let Some(output) = step.output.as_deref() else {
        return Err(format!(
            "The step cannot be reused: it has no answer for {}",
            answers::fix_text(&[ValueProblem {
                name: fields.iter().map(|field| field.name.as_str()).collect::<Vec<_>>().join(", "),
                problem: answers::Problem::Missing,
            }])
        ));
    };
    let Some(value) = answers::last_json_object(output) else {
        return Err("The step cannot be reused: its saved answer has no JSON object".to_owned());
    };
    let (_, problems) = answers::check_answer(&fields, &value, answers::ExtraKeys::Refuse);
    if problems.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "The step cannot be reused with the answer fields this run started with: {}",
            answers::fix_text(&problems)
        ))
    }
}

fn answer_spec_fields(start: &Start, bot: &str) -> Vec<answers::AnswerField> {
    start.answer_specs.get(bot).map(answers::fields_of).unwrap_or_default()
}

/// Whether a link's start is a media step, whose file the next step should read as a picture.
fn is_picture_node(id: &str) -> bool {
    id.starts_with(crate::store::flow_media::PREFIX)
}

/// The bot a step's sender stands for: a media step speaks as the routine's own bot.
fn from_id<'a>(node: &'a str, owner: &'a str) -> &'a str {
    if node.starts_with(crate::store::flow_media::PREFIX) { owner } else { node }
}

/// What a step is called in the text handed to the next bot: its name when the person gave one,
/// else its kind (a media step) or the bot's name. Agents reading the flow therefore see the names.
fn node_name(hub: &Hub, routine_id: &str, id: &str) -> String {
    if let Some(media) = id.strip_prefix(crate::store::flow_media::PREFIX) {
        return match hub.store.flow_media_node(media).ok().flatten() {
            Some(node) if !node.name.trim().is_empty() => node.name,
            Some(node) if node.kind == "video" => "the video step".to_owned(),
            _ => "the picture step".to_owned(),
        };
    }
    // A bot step's name lives on the link that reaches it in this routine.
    if let Some(name) = hub
        .store
        .flow_links(routine_id)
        .ok()
        .and_then(|links| links.into_iter().find(|link| link.to_bot_id == id))
        .map(|link| link.name)
        .filter(|name| !name.trim().is_empty())
    {
        return name;
    }
    hub.bot_name(id)
}

fn earlier_outcome(hub: &Hub, run_id: &str, bot: &str) -> Result<Outcome> {
    let step = hub.store.flow_run_steps(run_id)?.into_iter().find(|s| s.bot_id == bot).context("the step is gone")?;
    Ok(step_outcome(step))
}

fn saved_outcome(output: Option<&str>, succeeded: bool) -> Outcome {
    match (succeeded, output) {
        (true, Some(text)) => Outcome::Replied(text.to_owned()),
        _ => Outcome::Nothing,
    }
}

fn step_outcome(step: FlowStep) -> Outcome {
    saved_outcome(step.output.as_deref(), step.status == StepStatus::Succeeded)
}

/// Decides a media step: the description it was saved with, plus whatever was handed on.
fn begin_media(hub: &Hub, start: &Start, id: &str, inputs: &[(&FlowLink, &str)]) -> Result<Begin> {
    let Some(node) = hub.store.flow_media_in(id, &start.routine_id)? else {
        record_ended(hub, start, &crate::store::flow_media::key(id), StepStatus::Failed, "The media step is gone")?;
        return Ok(Begin::Done(Outcome::Nothing));
    };
    // Stop before a step is recorded as running when its model is not one anyone can make. A live
    // Sunra list is checked again in `make`, before any chargeable call.
    let available = if crate::media::sunra::is_model(&node.model) {
        true
    } else {
        crate::media::model_available(crate::media::catalog::available_ids(), &node.model)
    };
    if !available {
        record_ended(
            hub,
            start,
            &crate::store::flow_media::key(id),
            StepStatus::Failed,
            crate::media::MODEL_UNAVAILABLE,
        )?;
        return Ok(Begin::Done(Outcome::Nothing));
    }
    let read: Vec<StepInput> = inputs
        .iter()
        .map(|(link, text)| StepInput {
            from: node_name(hub, &start.routine_id, &link.from_bot_id),
            text: (*text).to_owned(),
            instruction: link.instruction.clone(),
            picture: is_picture_node(&link.from_bot_id),
        })
        .collect();
    let mut body = inputs
        .iter()
        .map(|(_, text)| text.trim())
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n---\n\n");
    body.push_str(&picture_note(&read));
    let batch = crate::media::media_batch(node.batch);
    let settings = StepSettings {
        kind: node.kind.clone(),
        model: Some(node.model.clone()),
        prompt: node.prompt.clone(),
        aspect: node.aspect.clone(),
        batch,
        results: crate::media::batch_results(batch, &BTreeSet::new(), &BTreeMap::new()),
        duration: Some(node.duration),
        resolution: node.resolution.clone(),
        transparent: Some(node.transparent),
        voice: node.voice.clone(),
        inputs: read,
        ..StepSettings::default()
    };
    let key = crate::store::flow_media::key(id);
    let mut fresh = new_step(start, &key, &body, initial_step_status(&key));
    fresh.branch = inputs.first().and_then(|(link, _)| link.branch.as_deref());
    match record(hub, &fresh)? {
        Some(step) => {
            record_settings(hub, &step.id, &settings);
            Ok(Begin::Make(Box::new(MediaPending { step, node, body, settings })))
        }
        None => Ok(Begin::Done(earlier_outcome(hub, &start.run_id, &key)?)),
    }
}

/// Pictures connected to a Sunra step, read from the file paths earlier steps handed on in their
/// result text (`name (path)` lines). The first becomes the start frame, the last the end frame of a
/// video, and the rest are references; Sunra still works with none when the step only has a prompt.
fn sunra_frames(body: &str, kind: crate::media::Kind) -> crate::media::sunra::Frames {
    let mut images: Vec<crate::media::sunra::Image> = Vec::new();
    for line in body.lines() {
        let Some(open) = line.rfind('(') else { continue };
        let Some(path) = line.rfind(')').filter(|close| *close > open).map(|close| &line[open + 1..close]) else {
            continue;
        };
        if !path.contains(std::path::MAIN_SEPARATOR) && !path.starts_with('/') {
            continue;
        }
        let mime = match path.rsplit('.').next().unwrap_or_default().to_ascii_lowercase().as_str() {
            "jpg" | "jpeg" => "image/jpeg",
            "png" => "image/png",
            "gif" => "image/gif",
            "webp" => "image/webp",
            _ => continue,
        };
        let path = std::path::Path::new(path);
        if !path.is_file() {
            continue;
        }
        images
            .push(crate::media::sunra::Image { path: path.to_string_lossy().into_owned(), mime_type: mime.to_owned() });
    }
    let mut frames = crate::media::sunra::Frames::default();
    if !images.is_empty() {
        let last = images.pop().expect("images is not empty");
        let first = images.pop().unwrap_or_else(|| last.clone());
        frames.first = Some(first);
        if kind == crate::media::Kind::Video {
            frames.last = Some(last);
            frames.references = images;
        } else {
            frames.references.insert(0, last);
            frames.references.extend(images);
        }
    }
    frames
}

/// The one provider path a media result takes: the installed Sunra connector or the bot's own
/// agent CLI. Both are one call, so a batch repeats the same call, bounded by the step's batch.
enum MediaRequest {
    Sunra(crate::media::sunra::Request),
    Agent(crate::media::command::Request),
}

impl MediaRequest {
    fn prompt(&self) -> &str {
        match self {
            Self::Sunra(request) => &request.prompt,
            Self::Agent(request) => &request.prompt,
        }
    }

    async fn generate(
        &self,
        hub: &Arc<Hub>,
        owner: &BotConfig,
        handed: &str,
        timeout: Duration,
    ) -> anyhow::Result<Vec<crate::chat::files::SharedFile>> {
        match self {
            Self::Sunra(request) => crate::media::sunra::generate(hub, owner, request, timeout).await,
            Self::Agent(request) => crate::media::flow::generate(hub, owner, request, handed, timeout).await,
        }
    }
}

/// Makes a media step's results, keeps them in the routine's own bot and hands them all on. A batch
/// loops over the same provider call, checks cancellation between results, and gives every failed
/// result its own status so the client can offer a Retry for just that one.
async fn make(hub: &Arc<Hub>, start: &Start, pending: MediaPending, cancel: watch::Receiver<bool>) -> Outcome {
    let step = pending.step.id.clone();
    let Some(owner) = hub.store.bot(&start.owner).ok().flatten().filter(|bot| !bot.deleted) else {
        settle(hub, &step, StepStatus::Failed, None, Some("The routine's bot is gone"));
        return Outcome::Nothing;
    };
    let mut cancel = cancel;
    // A model the service cannot make now stops here: no provider call and no credits used. The
    // live Sunra list is the catalog for a Sunra id; the fixed catalog is the one for an agent tool.
    let sunra = crate::media::sunra::is_model(&pending.node.model);
    let available = if sunra {
        // Signed out is its own error, and `generate` gives that message before any call. Only a
        // signed-in service has a live catalog to check the model against.
        if !crate::media::sunra::available(&hub.store) {
            true
        } else {
            let wanted = pending.node.model.strip_prefix("sunra-").unwrap_or(&pending.node.model);
            let id = format!("sunra-{wanted}");
            match tokio::time::timeout(start.timeout, crate::media::sunra::list_models(&hub.store)).await {
                Ok(Ok(listed)) => crate::media::model_available(listed.iter().map(|model| model.id.as_str()), &id),
                _ => false,
            }
        }
    } else {
        crate::media::model_available(crate::media::catalog::available_ids(), &pending.node.model)
    };
    if !available {
        settle(hub, &step, StepStatus::Failed, None, Some(crate::media::MODEL_UNAVAILABLE));
        return Outcome::Nothing;
    }
    // A Sunra model is generated through the installed Sunra MCP connector, not the user's own
    // agent CLI (OpenBot nightly's Sunra media provider). Everything else keeps the CLI path.
    let request = if sunra {
        let mut request = match crate::media::sunra::Request::from_node(&pending.node) {
            Ok(request) => request,
            Err(error) => {
                let detail = format!("{error:#}");
                let text = media_failure_text(&pending.node.kind, &detail);
                settle(hub, &step, StepStatus::Failed, None, Some(&text));
                return Outcome::Nothing;
            }
        };
        request.frames = sunra_frames(&pending.body, request.kind);
        MediaRequest::Sunra(request)
    } else {
        match crate::media::request(&pending.node) {
            Ok(request) => MediaRequest::Agent(request),
            Err(error) => {
                let detail = format!("{error:#}");
                let text = media_failure_text(&pending.node.kind, &detail);
                settle(hub, &step, StepStatus::Failed, None, Some(&text));
                return Outcome::Nothing;
            }
        }
    };
    let batch = crate::media::media_batch(pending.node.batch);
    let mut settings = pending.settings;
    let mut succeeded = BTreeSet::new();
    let mut failed: BTreeMap<u32, String> = BTreeMap::new();
    let mut files = Vec::new();
    let mut cancelled = false;
    for index in 0..batch {
        if *cancel.borrow() {
            cancelled = true;
            break;
        }
        let attempt = tokio::select! {
            result = request.generate(hub, &owner.config, &pending.body, start.timeout) => Some(result),
            _ = cancel.wait_for(|stopped| *stopped) => None,
        };
        let Some(result) = attempt else {
            cancelled = true;
            break;
        };
        match result {
            Ok(made) => {
                succeeded.insert(index);
                files.extend(made);
            }
            Err(error) => {
                let detail = format!("{error:#}");
                failed.insert(index, media_failure_text(&pending.node.kind, &detail));
            }
        }
        settings.results = crate::media::batch_results(batch, &succeeded, &failed);
        record_progress(hub, &step, &settings);
    }
    if cancelled {
        settle(hub, &step, StepStatus::Cancelled, None, Some("The flow was cancelled while the files were being made"));
        return Outcome::Nothing;
    }
    let files = crate::media::kept_files(files);
    if files.is_empty() {
        let reason = failed.values().next().cloned().unwrap_or_else(|| "The media provider made no file".to_owned());
        settle(hub, &step, StepStatus::Failed, None, Some(&reason));
        return Outcome::Nothing;
    }
    let text = made_text(request.prompt(), &files, &start.owner);
    settle_with(hub, &step, StepStatus::Succeeded, Some(&text), None, &files);
    Outcome::Replied(text)
}

/// What a made file hands on: what it is and where the next bot finds it.
fn made_text(prompt: &str, files: &[crate::chat::files::SharedFile], owner: &str) -> String {
    let root = crate::chat::files::root(owner);
    let listing: Vec<String> =
        files.iter().map(|file| format!("{} ({})", file.name, root.join(&file.id).display())).collect();
    format!("Made for \"{prompt}\":\n{}", listing.join("\n"))
}

/// Why a media step could not make its file. Triage 13612: the step's error must say which of these
/// happened instead of a generic "could not make the image". The model's own words are the exception
/// — they are kept in full, because they are the only thing that explains a reply with no file.
#[derive(Clone, Debug, PartialEq, Eq)]
enum MediaFailure {
    SignInExpired,
    LimitUsedUp,
    ProviderBusy,
    NoInternet,
    ModelAnswer(String),
    Other(String),
}

/// Classifies a provider error by its words. Pure so every reason can be tested without a provider;
/// the order matters — a rate limit is a busy provider, not a used-up quota, and a model that answered
/// without a file is its own case ahead of the network ones.
fn classify_media_failure(error: &str) -> MediaFailure {
    let lower = error.to_lowercase();
    if lower.contains("made no file") {
        let answer = error.split_once(": ").map_or("", |(_, answer)| answer.trim());
        return MediaFailure::ModelAnswer(answer.to_owned());
    }
    if ["sign-in", "sign in", "unauthor", "expired", "401", "re-author", "log in again"]
        .iter()
        .any(|word| lower.contains(word))
    {
        return MediaFailure::SignInExpired;
    }
    if ["insufficient balance", "out of credit", "no credit", "used up", "quota", "balance"]
        .iter()
        .any(|word| lower.contains(word))
    {
        return MediaFailure::LimitUsedUp;
    }
    if ["rate limit", "busy", "overload", "try again", "temporarily", "server answered 5", "429"]
        .iter()
        .any(|word| lower.contains(word))
    {
        return MediaFailure::ProviderBusy;
    }
    if [
        "can't reach",
        "could not reach",
        "did not answer in time",
        "took too long",
        "timed out",
        "network",
        "offline",
        "connection",
    ]
    .iter()
    .any(|word| lower.contains(word))
    {
        return MediaFailure::NoInternet;
    }
    MediaFailure::Other(error.to_owned())
}

/// The step's error text for a failed media request, naming the reason the person can act on.
fn media_failure_text(kind: &str, error: &str) -> String {
    let what = if kind == "video" { "clip" } else { "picture" };
    match classify_media_failure(error) {
        MediaFailure::SignInExpired => {
            format!("The {what} could not be made because the sign-in expired. Sign in again and rerun this step.")
        }
        MediaFailure::LimitUsedUp => format!(
            "The {what} could not be made because the account's limit is used up. Add credits or use another model, then rerun this step."
        ),
        MediaFailure::ProviderBusy => {
            format!("The {what} could not be made because the provider is busy. Try again shortly.")
        }
        MediaFailure::NoInternet => format!(
            "The {what} could not be made because this computer has no internet connection. Reconnect and rerun this step."
        ),
        MediaFailure::ModelAnswer(answer) if !answer.is_empty() => {
            format!("The {what} could not be made: the model answered \"{answer}\" instead of making a file.")
        }
        MediaFailure::ModelAnswer(_) => {
            format!("The {what} could not be made: the model answered without making a file.")
        }
        MediaFailure::Other(detail) => format!("The {what} could not be made: {detail}"),
    }
}

/// Reads and checks the JSON object a step ended its reply with. A reply without a JSON object is a
/// problem the step gets to correct in one more turn.
fn checked_answer(fields: &[answers::AnswerField], text: &str) -> std::result::Result<String, Vec<ValueProblem>> {
    let problems = match answers::last_json_object(text) {
        None => vec![ValueProblem {
            name: fields.iter().map(|field| field.name.as_str()).collect::<Vec<_>>().join(", "),
            problem: answers::Problem::Json,
        }],
        Some(value) => answers::check_answer(fields, &value, answers::ExtraKeys::Refuse).1,
    };
    if !problems.is_empty() {
        return Err(problems);
    }
    let Some(value) = answers::last_json_object(text) else { return Err(Vec::new()) };
    let (answer, _) = answers::check_answer(fields, &value, answers::ExtraKeys::Refuse);
    Ok(serde_json::to_string(&answer).unwrap_or_default())
}

fn attach_delivery(hub: &Hub, step: &str, delivery: &str) {
    if let Err(e) = hub.store.flow_attach_delivery(step, delivery) {
        tracing::warn!(step, error = format!("{e:#}"), "couldn't record a routine flow step's request");
    }
}

/// OpenBot 0.77.0: the hand-off has put the message in the agent's queue, so a queued step starts
/// running now, not when it was first recorded. The delivery still goes on the step either way.
fn handoff_sent(hub: &Hub, step: &str, delivery: &str) {
    if let Err(e) = hub.emit_flow_change(|| Ok(((), hub.store.flow_start_queued_step(step, delivery)?))) {
        tracing::warn!(step, error = format!("{e:#}"), "couldn't start a queued routine flow step");
    }
    attach_delivery(hub, step, delivery);
}

async fn send(hub: &Arc<Hub>, start: &Start, pending: Pending, cancel: watch::Receiver<bool>) -> Outcome {
    let (step, fields) = (pending.step.id.clone(), pending.fields);
    let request = Handoff {
        routine_name: &start.routine_name,
        from: &pending.from,
        to: &pending.bot,
        body: &pending.body,
        model: pending.model.clone(),
        hops: pending.hops,
        timeout: teammate_wait(start.timeout),
        cancel: cancel.clone(),
    };
    let first = handoff(hub, request, |delivery| handoff_sent(hub, &step, delivery)).await;
    match first {
        Ok(text) if fields.is_empty() || nothing_to_report(&text) => {
            if nothing_to_report(&text) {
                settle(hub, &step, StepStatus::Succeeded, None, None);
                Outcome::Nothing
            } else {
                settle(hub, &step, StepStatus::Succeeded, Some(&text), None);
                Outcome::Replied(text)
            }
        }
        Ok(text) => {
            // An agent that used the `workflow_answer` tool already got its answer checked when it
            // called the tool, so it does not need another turn.
            if let Some(answer) = answers::take_tool_answer(&hub.store, &start.run_id, &step) {
                let output = serde_json::to_string(&answer).unwrap_or_default();
                settle(hub, &step, StepStatus::Succeeded, Some(&output), None);
                return Outcome::Replied(output);
            }
            match checked_answer(&fields, &text) {
                Ok(output) => {
                    settle(hub, &step, StepStatus::Succeeded, Some(&output), None);
                    Outcome::Replied(output)
                }
                Err(problems) => {
                    let retry_body = format!("{}\n\n{}", pending.body, answers::answer_fix_ask(&fields, &problems));
                    let retry = Handoff {
                        routine_name: &start.routine_name,
                        from: &pending.from,
                        to: &pending.bot,
                        body: &retry_body,
                        model: pending.model.clone(),
                        hops: pending.hops,
                        timeout: teammate_wait(start.timeout),
                        cancel,
                    };
                    match handoff(hub, retry, |delivery| attach_delivery(hub, &step, delivery)).await {
                        Ok(reply) => match checked_answer(&fields, &reply) {
                            Ok(output) => {
                                settle(hub, &step, StepStatus::Succeeded, Some(&output), None);
                                Outcome::Replied(output)
                            }
                            Err(problems) => {
                                settle(
                                    hub,
                                    &step,
                                    StepStatus::Failed,
                                    None,
                                    Some(&format!(
                                        "The answer did not fit after another try: {}",
                                        answers::fix_text(&problems)
                                    )),
                                );
                                Outcome::Nothing
                            }
                        },
                        Err(e) if e.is::<FlowCancelled>() => {
                            settle(hub, &step, StepStatus::Cancelled, None, Some(&e.to_string()));
                            Outcome::Nothing
                        }
                        Err(e) => {
                            settle(hub, &step, StepStatus::Failed, None, Some(&format!("{e:#}")));
                            Outcome::Nothing
                        }
                    }
                }
            }
        }
        Err(e) if e.is::<FlowCancelled>() => {
            settle(hub, &step, StepStatus::Cancelled, None, Some(&e.to_string()));
            Outcome::Nothing
        }
        Err(e) => {
            settle(hub, &step, StepStatus::Failed, None, Some(&format!("{e:#}")));
            Outcome::Nothing
        }
    }
}

/// The model and options a step takes from a link; none when it names neither.
pub(super) fn override_of(link: &FlowLink) -> Option<ModelOverride> {
    (link.model.is_some() || !link.model_options.is_empty())
        .then(|| ModelOverride { model: link.model.clone(), options: link.model_options.clone() })
}

/// What a bot is asked: the replies before it and what the links say to do with them. A picture
/// handed on by a media step is passed with its file path and an instruction to use it as given;
/// a model that can't read pictures is told to stop with a clear message instead of inventing one.
fn handoff_text(routine: &str, inputs: &[StepInput]) -> String {
    let answers: Vec<String> = inputs
        .iter()
        .map(|input| {
            let answer: &str = if nothing_to_report(&input.text) { "(nothing to report)" } else { input.text.as_str() };
            format!("{} answered:\n\n{}", input.from, answer)
        })
        .collect();
    let mut instructions: Vec<&str> =
        inputs.iter().map(|input| input.instruction.trim()).filter(|i| !i.is_empty()).collect();
    instructions.dedup();
    let task = if instructions.is_empty() {
        "Do your part of the routine with this.".to_owned()
    } else {
        instructions.join("\n\n")
    };
    format!(
        "The routine \"{routine}\" continues with you.\n\n{}\n{}\n{task}",
        answers.join("\n\n---\n\n"),
        picture_note(inputs),
    )
}

/// The extra instruction a step gets when pictures reach it through a picture connection. Empty
/// when no picture is connected, so a text-only flow reads exactly as it did before.
fn picture_note(inputs: &[StepInput]) -> String {
    let pictures: Vec<&str> = inputs
        .iter()
        .filter(|input| input.picture)
        .map(|input| input.text.trim())
        .filter(|text| !text.is_empty())
        .collect();
    if pictures.is_empty() {
        return String::new();
    }
    format!(
        "Pictures connected to this step (use them as given, together, and don't invent a substitute subject):\n{}\n\
         If your model cannot read pictures, stop here and reply that this step needs a model that can read pictures, \
         or that the picture connection must be removed.",
        pictures.join("\n"),
    )
}

/// OpenBot 0.74.0: a one-off run can carry the text of the one case it is for (a document, an
/// order…). Only the first steps, the ones next to the routine's own bot, get it; later steps read
/// only what those steps handed on. The routine's saved instruction and links stay as they are.
fn handoff_body(
    routine: &str,
    inputs: &[StepInput],
    case: Option<&str>,
    depth: usize,
    run_time: Option<&str>,
) -> String {
    let body = handoff_text(routine, inputs);
    let case = case.map(str::trim).filter(|case| !case.is_empty());
    let when = |body: String| match run_time {
        Some(time) if !time.is_empty() => format!("{time}\n\n{body}"),
        _ => body,
    };
    match (depth, case) {
        (1, Some(case)) => when(format!("Case for this run:\n\n{case}\n\n{body}")),
        _ => when(body),
    }
}

/// A schedule trigger hands its run time to the following steps: the instant in UTC and the IANA
/// zone of the trigger, so a model can compute a window ("the 24 hours before this run"). A manual
/// run has no cron zone, so it uses this computer's zone.
pub fn run_time_text(run_at: i64, triggers: &[Trigger]) -> String {
    let fallback = local_zone();
    let zone = triggers
        .iter()
        .find_map(|trigger| match trigger {
            Trigger::Cron { time_zone, .. } => Some(time_zone.as_str()),
            _ => None,
        })
        .unwrap_or(fallback.as_str());
    let utc = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(run_at)
        .map(|at| at.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| "an unknown time".to_owned());
    format!("Run time: {utc} (trigger time zone: {zone}).")
}

/// The case text an agent gave the `run_routine` call for one run, if any.
fn run_case_text(event: &serde_json::Value) -> Option<String> {
    event["caseText"].as_str().map(str::trim).filter(|text| !text.is_empty()).map(str::to_owned)
}

/// The flow to run for a routine run that just ended: none when the routine is gone or the run
/// replied with nothing to pass on.
pub fn start_for(
    routine: Option<&Routine>,
    run: &Run,
    status: Status,
    detail: Option<&str>,
    text: Option<&str>,
) -> Option<Start> {
    let routine = routine?;
    let root = match status {
        Status::Succeeded => Root::Replied(text.filter(|t| !crate::chat::group::is_pass(t))?.to_owned()),
        Status::Failed | Status::Interrupted => Root::Failed(detail.unwrap_or("The routine's run failed").to_owned()),
        Status::Cancelled => Root::Cancelled("The routine's run was cancelled".to_owned()),
        Status::Pending | Status::Starting | Status::Running | Status::Recovering => return None,
    };
    Some(Start {
        run_id: run.id.clone(),
        routine_id: routine.id.clone(),
        routine_name: routine.name.clone(),
        owner: routine.bot_id.clone(),
        run_at: run.created_at,
        timeout: Duration::from_secs(routine.timeout_seconds),
        max_depth: super::max_depth(),
        instruction: routine.instruction.clone(),
        case_text: run_case_text(&run.event),
        outputs: routine.outputs.clone(),
        event: run.event.clone(),
        root,
        from_step: None,
        seed: BTreeMap::new(),
        run_time_text: run_time_text(run.created_at, &routine.triggers),
        answer_specs: BTreeMap::new(),
        run_fields: Vec::new(),
        trigger_fields: super::trigger_fields::fixed_answers(run.created_at, &routine.triggers, &run.event),
        attempt: first_attempt(),
    })
}

/// Runs the flow in the background; a run that ended outside a runtime (a unit test) has none.
pub fn spawn(hub: &Arc<Hub>, start: Start) {
    if tokio::runtime::Handle::try_current().is_ok() {
        tokio::spawn(execute(hub.clone(), start));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_old_step_reads_as_one_result_and_a_batch_saves_one_status_each() {
        let old: StepSettings = serde_json::from_value(serde_json::json!({})).unwrap();
        assert_eq!(old.batch, 1);
        assert!(old.results.is_empty());
        let saved = StepSettings {
            batch: 3,
            results: crate::media::batch_results(3, &BTreeSet::new(), &BTreeMap::new()),
            ..StepSettings::default()
        };
        let written = serde_json::to_value(&saved).unwrap();
        assert_eq!(written["batch"], serde_json::json!(3));
        assert_eq!(written["results"].as_array().unwrap().len(), 3);
        assert_eq!(written["results"][0]["status"], serde_json::json!("pending"));
    }

    #[test]
    fn an_agent_step_waits_queued_while_a_media_step_starts_running() {
        assert_eq!(initial_step_status("ada"), StepStatus::Queued);
        assert_eq!(initial_step_status(&crate::store::flow_media::key("m1")), StepStatus::Running);
    }

    #[test]
    fn a_one_case_run_gives_its_case_text_to_the_first_steps_only() {
        let inputs = [StepInput {
            from: "a".to_owned(),
            text: "a's answer".to_owned(),
            instruction: "Do the next part".to_owned(),
            picture: false,
        }];
        let first = handoff_body("Digest", &inputs, Some(" order 42 "), 1, None);
        assert!(first.contains("Case for this run:\n\norder 42"), "{first}");
        assert!(first.contains("a answered:\n\na's answer"), "the saved handoff is still there");
        assert!(first.contains("Do the next part"), "the link's instruction is unchanged");

        // Later steps see only what the first step handed on, never the case text again.
        let later = handoff_body("Digest", &inputs, Some("order 42"), 2, None);
        assert_eq!(later, handoff_text("Digest", &inputs));
        assert!(!later.contains("order 42"));
        // No case text, or a blank one, changes nothing at any depth.
        assert_eq!(handoff_body("Digest", &inputs, None, 1, None), handoff_text("Digest", &inputs));
        assert_eq!(handoff_body("Digest", &inputs, Some("  "), 1, None), handoff_text("Digest", &inputs));
        // The schedule's time goes to the following steps, ahead of the case text and the handoff.
        let with_time = handoff_body("Digest", &inputs, Some("order 42"), 1, Some("Run time: 10:00 UTC."));
        assert!(with_time.starts_with("Run time: 10:00 UTC.\n\nCase for this run:"), "{with_time}");
    }

    #[test]
    fn a_connected_picture_is_passed_with_an_instruction_to_use_it_as_given() {
        let inputs = [
            StepInput {
                from: "the picture step".to_owned(),
                text: "pic.png (/files/pic.png)".to_owned(),
                picture: true,
                ..StepInput::default()
            },
            StepInput { from: "a".to_owned(), text: "a's answer".to_owned(), picture: false, ..StepInput::default() },
        ];
        let text = handoff_text("Digest", &inputs);
        assert!(text.contains("pic.png (/files/pic.png)"), "the connected picture is handed on");
        assert!(text.contains("use them as given"), "{text}");
        assert!(text.contains("don't invent a substitute subject"), "{text}");
        assert!(text.contains("cannot read pictures, stop here"), "{text}");
        // A text-only step gets no picture instruction at all.
        let plain = handoff_text("Digest", &inputs[1..]);
        assert!(!plain.contains("Pictures connected to this step"), "{plain}");
    }

    #[test]
    fn a_schedule_step_gets_the_run_time_in_utc_and_the_triggers_zone() {
        // 2026-01-02 03:04:05 UTC.
        let at = 1_767_323_045_000;
        let cron = Trigger::Cron { expression: "0 9 * * *".to_owned(), time_zone: "Asia/Taipei".to_owned() };
        let text = run_time_text(at, &[cron]);
        assert!(text.contains("2026-01-02 03:04:05 UTC"), "{text}");
        assert!(text.contains("trigger time zone: Asia/Taipei"), "{text}");
        // A manual run has no cron trigger, so it names this computer's zone instead.
        let manual = run_time_text(at, &[]);
        assert!(manual.contains(&format!("trigger time zone: {}", local_zone())), "{manual}");
    }

    #[test]
    fn the_post_waits_for_the_teammates_answer_at_most_a_day() {
        // The posted answer is what the last teammate replied, not the routine's own earlier text.
        assert_eq!(final_answer(&["  ", "the teammate's final answer", " "]), "the teammate's final answer");
        assert_eq!(final_answer(&["first", "second"]), "first\n\n---\n\nsecond");
        assert_eq!(final_answer(&[]), "");
        // A run may ask for less than a day, but never more.
        assert_eq!(teammate_wait(Duration::from_secs(30)), Duration::from_secs(30));
        assert_eq!(teammate_wait(Duration::from_secs(48 * 60 * 60)), TEAMMATE_ANSWER_WAIT);
        assert_eq!(TEAMMATE_ANSWER_WAIT, Duration::from_secs(24 * 60 * 60));
    }

    #[test]
    fn the_case_text_is_read_from_the_one_off_run_event() {
        let event = serde_json::json!({"source": "test", "caseText": " invoice 7 "});
        assert_eq!(run_case_text(&event).as_deref(), Some("invoice 7"));
        assert_eq!(run_case_text(&serde_json::json!({"source": "test"})), None);
        assert_eq!(run_case_text(&serde_json::json!({"caseText": "   "})), None);
    }

    #[test]
    fn a_failed_media_step_says_why_instead_of_a_generic_error() {
        assert_eq!(classify_media_failure("Sunra's sign-in headers are invalid"), MediaFailure::SignInExpired);
        assert_eq!(classify_media_failure("401 unauthorized: token expired"), MediaFailure::SignInExpired);
        assert_eq!(classify_media_failure("Insufficient balance"), MediaFailure::LimitUsedUp);
        assert_eq!(classify_media_failure("the account quota is spent"), MediaFailure::LimitUsedUp);
        assert_eq!(classify_media_failure("Sunra's server answered 503"), MediaFailure::ProviderBusy);
        assert_eq!(classify_media_failure("rate limit reached, try again"), MediaFailure::ProviderBusy);
        assert_eq!(classify_media_failure("can't reach Sunra"), MediaFailure::NoInternet);
        assert_eq!(classify_media_failure("making the image took too long"), MediaFailure::NoInternet);
        assert_eq!(
            classify_media_failure("the Image model made no file: I can only write text"),
            MediaFailure::ModelAnswer("I can only write text".to_owned())
        );
        assert_eq!(classify_media_failure("a new error"), MediaFailure::Other("a new error".to_owned()));

        let expired = media_failure_text("image", "sign-in expired");
        assert!(expired.contains("sign-in expired"), "{expired}");
        assert!(expired.starts_with("The picture could not be made"), "{expired}");
        let limit = media_failure_text("video", "Insufficient balance");
        assert!(limit.contains("limit is used up"), "{limit}");
        assert!(limit.starts_with("The clip could not be made"), "{limit}");
        assert!(media_failure_text("image", "server answered 503").contains("provider is busy"));
        assert!(media_failure_text("image", "can't reach Sunra").contains("no internet connection"));
        let answered = media_failure_text("image", "the Image model made no file: I can only write text");
        assert!(answered.contains("the model answered \"I can only write text\""), "{answered}");
        assert!(media_failure_text("image", "a new error").contains("a new error"));
    }

    fn routed_link(branch: Option<&str>) -> FlowLink {
        FlowLink {
            id: "l".to_owned(),
            routine_id: "r".to_owned(),
            owner_bot_id: "a".to_owned(),
            from_bot_id: "c".to_owned(),
            to_bot_id: "b".to_owned(),
            name: String::new(),
            instruction: String::new(),
            model: None,
            model_options: BTreeMap::new(),
            branch: branch.map(str::to_owned),
            port: crate::store::flows::PortType::Text,
            created_at: 0,
        }
    }

    #[test]
    fn a_routing_node_only_follows_the_edge_of_the_branch_it_took() {
        let (yes, no, plain) = (routed_link(Some("yes")), routed_link(Some("no")), routed_link(None));
        let took_yes = BTreeMap::from([("c".to_owned(), Some(nodes::Branch::Yes))]);
        assert!(link_follows_step(&yes, &took_yes));
        assert!(!link_follows_step(&no, &took_yes), "the other branch carries nothing onward");
        assert!(!link_follows_step(&plain, &took_yes), "a routed node has no plain hand-off");
        let took_no = BTreeMap::from([("c".to_owned(), Some(nodes::Branch::No))]);
        assert!(link_follows_step(&no, &took_no));
        assert!(!link_follows_step(&yes, &took_no));
        let plain_node = BTreeMap::from([("c".to_owned(), None)]);
        assert!(link_follows_step(&plain, &plain_node));
        assert!(!link_follows_step(&yes, &plain_node));
        assert_eq!(step_branch(Some("yes")), Some(nodes::Branch::Yes));
        assert_eq!(step_branch(Some("no")), Some(nodes::Branch::No));
        assert_eq!(step_branch(Some("maybe")), None);
    }

    #[test]
    fn an_approval_decision_continues_on_the_branch_it_chose() {
        assert_eq!(approval_branch(Some(nodes::ApprovalOutcome::Approved.as_str())), Some(nodes::Branch::Yes));
        assert_eq!(approval_branch(Some(nodes::ApprovalOutcome::Rejected.as_str())), Some(nodes::Branch::No));
        assert_eq!(approval_branch(None), None);
        assert_eq!(nodes::approval_outcome(true).as_str(), "approved");
        assert_eq!(nodes::approval_outcome(false).as_str(), "rejected");
        let took_no = BTreeMap::from([("c".to_owned(), approval_branch(Some("rejected")))]);
        assert!(link_follows_step(&routed_link(Some("no")), &took_no));
        assert!(!link_follows_step(&routed_link(Some("yes")), &took_no));
    }

    fn answer_start() -> Start {
        Start {
            run_id: "run".to_owned(),
            routine_id: "r".to_owned(),
            routine_name: "Digest".to_owned(),
            owner: "a".to_owned(),
            run_at: 0,
            timeout: Duration::from_secs(30),
            max_depth: 5,
            instruction: String::new(),
            case_text: None,
            outputs: Vec::new(),
            event: serde_json::json!({"source": "test"}),
            root: Root::Replied("seed".to_owned()),
            from_step: None,
            seed: BTreeMap::new(),
            run_time_text: String::new(),
            answer_specs: BTreeMap::new(),
            run_fields: Vec::new(),
            trigger_fields: answers::Answer::new(),
            attempt: 1,
        }
    }

    #[test]
    fn a_webhook_body_with_a_wrong_cased_field_fails_the_run_clearly() {
        let store = Store::open(std::path::Path::new(":memory:")).expect("a store");
        answers::save_spec(&store, "a", "routine:r", Some("count: int, name: text"), None).expect("a spec");
        let mut start = answer_start();
        start.event = serde_json::json!({"data": {"Count": 3, "name": "Ada"}});
        start.prepare(&store);
        assert_eq!(start.run_fields.len(), 2, "the trigger's fields are frozen on the run");
        assert!(
            matches!(start.root, Root::Failed(ref text) if text.contains("Count") && text.contains("case-sensitive"))
        );

        let mut good = answer_start();
        good.event = serde_json::json!({"data": {"count": 3, "name": "Ada", "extra": "kept"}});
        good.prepare(&store);
        assert!(matches!(good.root, Root::Replied(_)), "an extra sender key is fine");
    }

    #[test]
    fn a_retry_reuses_a_step_only_when_its_answer_fits_the_fields_of_the_run() {
        let mut start = answer_start();
        start
            .answer_specs
            .insert("b".to_owned(), answers::NodeSpec { answer: "count: int".to_owned(), check: String::new() });
        let good = FlowStep {
            id: "s1".to_owned(),
            run_id: "run".to_owned(),
            bot_id: "b".to_owned(),
            delivery_id: None,
            input: String::new(),
            output: Some("{\"count\":3}".to_owned()),
            status: StepStatus::Succeeded,
            error: None,
            files: Vec::new(),
            results: None,
            created_at: 0,
            updated_at: 0,
            routine_id: "r".to_owned(),
            owner_bot_id: "a".to_owned(),
            run_at: 0,
        };
        assert!(saved_answer_problem(&start, "b", &good).is_ok());
        let bad = FlowStep { output: Some("{\"Count\":3}".to_owned()), ..good.clone() };
        assert!(saved_answer_problem(&start, "b", &bad).is_err(), "a changed spec rejects the old answer");
        let no_json = FlowStep { output: Some("no json here".to_owned()), ..good };
        assert!(saved_answer_problem(&start, "b", &no_json).is_err());
    }

    #[test]
    fn a_step_with_a_bad_answer_gets_one_more_try() {
        let fields = answers::fields_of(&answers::NodeSpec { answer: "count: int".to_owned(), check: String::new() });
        assert!(checked_answer(&fields, "no json").is_err());
        assert!(checked_answer(&fields, "{\"count\":3}").is_ok());
        let (_, problems) =
            answers::check_answer(&fields, &serde_json::json!({"count":"0x10"}), answers::ExtraKeys::Refuse);
        let retry = answers::answer_fix_ask(&fields, &problems);
        assert!(retry.contains("count has the wrong type"), "{retry}");
        assert!(retry.contains("- count: integer"), "{retry}");
    }

    #[test]
    fn a_claude_model_step_is_given_the_exact_answer_shape() {
        let fields = answers::fields_of(&answers::NodeSpec { answer: "count: int".to_owned(), check: String::new() });
        let schema = answers::answer_schema(&fields);
        assert_eq!(schema["properties"]["count"]["type"], "integer");
        assert!(answers::claude_ask(&fields).contains("\"additionalProperties\""));
    }

    #[test]
    fn a_step_with_nothing_to_report_stays_silent_in_the_handoff_and_the_answer() {
        assert!(nothing_to_report(""));
        assert!(nothing_to_report("   "));
        assert!(nothing_to_report("(pass)"));
        assert!(nothing_to_report("PASS."));
        assert!(!nothing_to_report("the report is ready"));

        // A pass never displaces the real answer, and blank text is left out too.
        assert_eq!(final_answer(&["(pass)", " the real answer ", "  "]), "the real answer");
        assert_eq!(final_answer(&["first", "second"]), "first\n\n---\n\nsecond");
        assert_eq!(final_answer(&[]), "");

        let inputs = [
            StepInput { from: "ada".to_owned(), text: "(pass)".to_owned(), ..StepInput::default() },
            StepInput { from: "bob".to_owned(), text: "the answer".to_owned(), ..StepInput::default() },
        ];
        let text = handoff_text("Digest", &inputs);
        assert!(text.contains("ada answered:\n\n(nothing to report)"), "{text}");
        assert!(text.contains("bob answered:\n\nthe answer"), "{text}");
        assert!(!text.contains("no answer text"), "{text}");
    }
}
