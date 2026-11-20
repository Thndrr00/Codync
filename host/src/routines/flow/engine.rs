//! Running a flow: after a routine's run ends, level by level from the routine's own bot, each
//! bot with the replies of the bots linked to it.

use super::graph;
use crate::chat::team::ModelOverride;
use crate::chat::team::flow::{FlowCancelled, Handoff, handoff, recipient};
use crate::hub::Hub;
use crate::routines::{Routine, Run, Status, Trigger, local_zone};
use crate::store::{FlowLink, FlowStep, NewStep, StepStatus};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use tokio::task::JoinSet;

/// Why a step after a cancelled flow never ran.
const CANCELLED_BEFORE: &str = "The flow was cancelled before this step started";

/// OpenBot 0.74.0: a routine result that waits on a teammate's answer is posted with that answer,
/// and never waits longer than a day for it. In Codync a teammate the routine handed work to is a
/// flow step: the post below waits for the last step's reply, and each handoff is bounded by this
/// day limit as well as the run's own timeout.
pub const TEAMMATE_ANSWER_WAIT: Duration = Duration::from_secs(24 * 60 * 60);

/// How long to wait for one teammate's reply: the run's timeout, capped at a day.
fn teammate_wait(timeout: Duration) -> Duration {
    timeout.min(TEAMMATE_ANSWER_WAIT)
}

/// The answer posted to "Send answers to": the final teammates' replies, not the text the routine's
/// own agent wrote before it asked them. Empty replies are left out, so a step with nothing to say
/// doesn't push the real answer away.
fn final_answer(replies: &[&str]) -> String {
    replies.iter().map(|reply| reply.trim()).filter(|reply| !reply.is_empty()).collect::<Vec<_>>().join("\n\n---\n\n")
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
}

/// A media step ready to make its file.
struct MediaPending {
    step: FlowStep,
    node: crate::store::FlowMedia,
    /// What the steps before it handed on.
    body: String,
}

enum Begin {
    Send(Box<Pending>),
    Make(Box<MediaPending>),
    Done(Outcome),
}

pub async fn execute(hub: Arc<Hub>, start: Start) {
    let (run, routine) = (start.run_id.clone(), start.routine_id.clone());
    match run_flow(&hub, Arc::new(start)).await {
        Ok(()) => {
            if let Err(error) = hub.routines.ack_flow(&hub.store, &run) {
                tracing::error!(run, %error, "could not acknowledge routine flow; recovery will replay it idempotently");
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
        let text = final_answer(&replies);
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
    // A completed step is a durable result, independent of whether its bot is still visible or
    // runnable. Resolve it before current recipient/input checks when replaying after a crash.
    if !*cancel.borrow()
        && let Some(step) = hub.store.flow_run_steps(&start.run_id)?.into_iter().find(|step| step.bot_id == bot)
    {
        return Ok(Begin::Done(step_outcome(step)));
    }
    let inputs: Vec<(&FlowLink, &str)> = links
        .iter()
        .filter(|l| l.to_bot_id == bot)
        .filter_map(|l| match results.get(&l.from_bot_id)? {
            Outcome::Replied(text) => Some((l, text.as_str())),
            Outcome::Nothing => None,
        })
        .collect();
    let end = |status, reason: &str| -> Result<Begin> {
        record_ended(hub, start, bot, status, reason)?;
        Ok(Begin::Done(Outcome::Nothing))
    };
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
    if let Err(error) = recipient(hub, bot) {
        return end(StepStatus::Failed, &format!("{error:#}"));
    }
    let Some(from) = hub.store.bot(from_id(&first.from_bot_id, &start.owner))?.filter(|b| !b.deleted) else {
        return end(StepStatus::Failed, "The bot that handed work on is gone");
    };
    let names: Vec<StepInput> = inputs
        .iter()
        .map(|(link, text)| StepInput {
            from: node_name(hub, &link.from_bot_id),
            text: (*text).to_owned(),
            instruction: link.instruction.clone(),
            picture: is_picture_node(&link.from_bot_id),
        })
        .collect();
    let include_run_time = depth == 1 || start.from_step.is_some();
    let body = handoff_body(
        &start.routine_name,
        &names,
        start.case_text.as_deref(),
        depth,
        include_run_time.then_some(start.run_time_text.as_str()).filter(|text| !text.is_empty()),
    );
    // A fan-in is one step: the first incoming link's model and options win, as its instruction leads the task.
    let model = inputs.iter().find_map(|(link, _)| override_of(link));
    let settings = StepSettings {
        kind: "bot".to_owned(),
        model: model.as_ref().and_then(|override_| override_.model.clone()),
        model_options: model.as_ref().map(|override_| override_.options.clone()).unwrap_or_default(),
        instruction: inputs.first().map(|(link, _)| link.instruction.clone()).unwrap_or_default(),
        inputs: names,
        ..StepSettings::default()
    };
    match record(hub, &new_step(start, bot, &body, StepStatus::Running))? {
        Some(step) => {
            record_settings(hub, &step.id, &settings);
            let hops = u8::try_from(depth).unwrap_or(u8::MAX);
            Ok(Begin::Send(Box::new(Pending { step, from: from.config, bot: bot.to_owned(), body, model, hops })))
        }
        // A retry of the run finds the step the first attempt made, and doesn't send it twice.
        None => Ok(Begin::Done(earlier_outcome(hub, &start.run_id, bot)?)),
    }
}

/// Whether a link's start is a media step, whose file the next step should read as a picture.
fn is_picture_node(id: &str) -> bool {
    id.starts_with(crate::store::flow_media::PREFIX)
}

/// The bot a step's sender stands for: a media step speaks as the routine's own bot.
fn from_id<'a>(node: &'a str, owner: &'a str) -> &'a str {
    if node.starts_with(crate::store::flow_media::PREFIX) { owner } else { node }
}

/// What a step is called in the text handed to the next bot.
fn node_name(hub: &Hub, id: &str) -> String {
    if id.starts_with(crate::store::flow_media::PREFIX) {
        return match hub.store.flow_media_node(id).ok().flatten().map(|node| node.kind) {
            Some(kind) if kind == "video" => "the video step".to_owned(),
            _ => "the picture step".to_owned(),
        };
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
    let read: Vec<StepInput> = inputs
        .iter()
        .map(|(link, text)| StepInput {
            from: node_name(hub, &link.from_bot_id),
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
    let settings = StepSettings {
        kind: node.kind.clone(),
        model: Some(node.model.clone()),
        prompt: node.prompt.clone(),
        aspect: node.aspect.clone(),
        duration: Some(node.duration),
        resolution: node.resolution.clone(),
        transparent: Some(node.transparent),
        voice: node.voice.clone(),
        inputs: read,
        ..StepSettings::default()
    };
    let key = crate::store::flow_media::key(id);
    match record(hub, &new_step(start, &key, &body, StepStatus::Running))? {
        Some(step) => {
            record_settings(hub, &step.id, &settings);
            Ok(Begin::Make(Box::new(MediaPending { step, node, body })))
        }
        None => Ok(Begin::Done(earlier_outcome(hub, &start.run_id, &key)?)),
    }
}

/// Makes a media step's file, keeps it in the routine's own bot and hands what was made on.
async fn make(hub: &Arc<Hub>, start: &Start, pending: MediaPending, cancel: watch::Receiver<bool>) -> Outcome {
    let step = pending.step.id.clone();
    let Some(owner) = hub.store.bot(&start.owner).ok().flatten().filter(|bot| !bot.deleted) else {
        settle(hub, &step, StepStatus::Failed, None, Some("The routine's bot is gone"));
        return Outcome::Nothing;
    };
    let request = match crate::media::request(&pending.node) {
        Ok(request) => request,
        Err(error) => {
            settle(hub, &step, StepStatus::Failed, None, Some(&format!("{error:#}")));
            return Outcome::Nothing;
        }
    };
    let mut cancel = cancel;
    let work = crate::media::flow::generate(hub, &owner.config, &request, &pending.body, start.timeout);
    tokio::pin!(work);
    let made = tokio::select! {
        made = &mut work => made,
        _ = cancel.wait_for(|stopped| *stopped) => {
            settle(hub, &step, StepStatus::Cancelled, None, Some("The flow was cancelled while the file was being made"));
            return Outcome::Nothing;
        }
    };
    match made {
        Ok(files) => {
            let text = made_text(&request.prompt, &files, &start.owner);
            settle_with(hub, &step, StepStatus::Succeeded, Some(&text), None, &files);
            Outcome::Replied(text)
        }
        Err(error) => {
            settle(hub, &step, StepStatus::Failed, None, Some(&format!("{error:#}")));
            Outcome::Nothing
        }
    }
}

/// What a made file hands on: what it is and where the next bot finds it.
fn made_text(prompt: &str, files: &[crate::chat::files::SharedFile], owner: &str) -> String {
    let root = crate::chat::files::root(owner);
    let listing: Vec<String> =
        files.iter().map(|file| format!("{} ({})", file.name, root.join(&file.id).display())).collect();
    format!("Made for \"{prompt}\":\n{}", listing.join("\n"))
}

async fn send(hub: &Arc<Hub>, start: &Start, pending: Pending, cancel: watch::Receiver<bool>) -> Outcome {
    let step = pending.step.id.clone();
    let request = Handoff {
        routine_name: &start.routine_name,
        from: &pending.from,
        to: &pending.bot,
        body: &pending.body,
        model: pending.model.clone(),
        hops: pending.hops,
        timeout: teammate_wait(start.timeout),
        cancel,
    };
    let attach = |delivery: &str| {
        if let Err(e) = hub.store.flow_attach_delivery(&step, delivery) {
            tracing::warn!(step, error = format!("{e:#}"), "couldn't record a routine flow step's request");
        }
    };
    match handoff(hub, request, attach).await {
        Ok(text) if crate::chat::group::is_pass(&text) => {
            settle(hub, &step, StepStatus::Succeeded, None, None);
            Outcome::Nothing
        }
        Ok(text) => {
            settle(hub, &step, StepStatus::Succeeded, Some(&text), None);
            Outcome::Replied(text)
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
            format!(
                "{} answered:\n\n{}",
                input.from,
                if input.text.is_empty() { "(no answer text)" } else { &input.text }
            )
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
}
