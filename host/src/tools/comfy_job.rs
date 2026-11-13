//! ComfyUI jobs: what was queued, how far it got, and what it made. A job is a graph queued with
//! `POST /prompt`, followed through `/queue` and `/history` and, while it runs, through `/ws` for the
//! node and the step. Its outputs go to the chat of the bot that asked, from inside that bot's turn:
//! `send_file` only works while a turn runs, so the bot's own `run_workflow` / `run_template` call
//! waits for the job and attaches them, once.

use super::comfy::results;
use super::comfy::{self, MODELS_URL};
use super::comfy_api::Api;
use crate::LockExt;
use crate::hub::Hub;
use anyhow::{Result, bail, ensure};
use serde_json::{Value, json};
use std::collections::{BTreeMap, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::broadcast;

const POLL_EVERY: Duration = Duration::from_secs(1);
/// A job that runs longer than this is stopped.
const MAX_RUN: Duration = Duration::from_mins(30);
/// How many finished jobs the card and the phone keep.
const KEEP_HISTORY: usize = 20;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    Queued,
    Running,
    Done,
    Cancelled,
    Error,
}

impl Phase {
    fn name(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Done => "done",
            Self::Cancelled => "cancelled",
            Self::Error => "error",
        }
    }

    fn active(self) -> bool {
        matches!(self, Self::Queued | Self::Running)
    }
}

/// The node and the step the engine reported on `/ws`.
#[derive(Clone, Debug, Default)]
struct Progress {
    node: String,
    class: String,
    step: u64,
    total: u64,
}

impl Progress {
    #[allow(
        clippy::cast_precision_loss,
        reason = "a step count never needs the 52 bits a f64 keeps, and it is only shown as a percentage"
    )]
    fn public(&self) -> Value {
        json!({
            "node": self.node,
            "class": self.class,
            "step": self.step,
            "total": self.total,
            "percent": (self.total > 0).then(|| self.step as f64 / self.total as f64),
        })
    }
}

#[derive(Clone)]
struct Job {
    id: String,
    bot: String,
    /// The template that built the graph, when a template did.
    template: Option<String>,
    phase: Phase,
    error: Option<String>,
    files: Vec<PathBuf>,
    /// The images went to the bot's chat already.
    delivered: bool,
    progress: Option<Progress>,
    /// Node id to class, for naming the node `/ws` reports.
    classes: BTreeMap<String, String>,
    created: i64,
}

impl Job {
    fn queued(id: &str, bot: &str, template: Option<&str>, classes: BTreeMap<String, String>) -> Self {
        Self {
            id: id.to_owned(),
            bot: bot.to_owned(),
            template: template.map(str::to_owned),
            phase: Phase::Queued,
            error: None,
            files: vec![],
            delivered: false,
            progress: None,
            classes,
            created: chrono::Utc::now().timestamp_millis(),
        }
    }

    fn public(&self) -> Value {
        json!({
            "id": self.id,
            "template": self.template,
            "phase": self.phase.name(),
            "error": self.error,
            "files": self.files.iter().map(|f| f.to_string_lossy()).collect::<Vec<_>>(),
            "outputs": self.files.iter().map(|f| results::Output::describe(f).public()).collect::<Vec<_>>(),
            "delivered": self.delivered,
            "progress": self.progress.as_ref().map(Progress::public),
            "createdAt": self.created,
        })
    }
}

/// Cancels the job when the call waiting for it goes away unfinished: a bot's turn that is stopped
/// (from the phone, the desktop or the bot) stops the image it was waiting for.
struct CancelOnDrop {
    jobs: Arc<Jobs>,
    hub: Arc<Hub>,
    id: String,
    armed: bool,
}

impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        let (jobs, hub, id) = (self.jobs.clone(), self.hub.clone(), self.id.clone());
        tokio::spawn(async move {
            if jobs.phase_of(&id).is_some_and(Phase::active)
                && let Err(e) = jobs.cancel(&hub).await
            {
                tracing::warn!(error = format!("{e:#}"), "cancelling the ComfyUI job of a stopped turn");
            }
        });
    }
}

pub struct Jobs {
    /// Held while a job is validated and queued, so two bots asking at once can't both get the slot.
    gate: tokio::sync::Mutex<()>,
    current: Mutex<Option<Job>>,
    /// Finished jobs, newest first, so the phone and the card see what was made.
    history: Mutex<VecDeque<Job>>,
    events: broadcast::Sender<Value>,
}

impl Jobs {
    pub fn new(events: broadcast::Sender<Value>) -> Self {
        Self {
            gate: tokio::sync::Mutex::new(()),
            current: Mutex::new(None),
            history: Mutex::new(VecDeque::new()),
            events,
        }
    }

    /// The last job (finished or not), for the card and the `job_status` tool.
    pub fn status(&self) -> Value {
        self.current.locked().as_ref().map_or(Value::Null, Job::public)
    }

    /// The finished jobs, newest first.
    pub fn history(&self) -> Value {
        Value::Array(self.history.locked().iter().map(Job::public).collect())
    }

    /// When the job `id` was queued, for the fallback that takes the files a run wrote by itself.
    fn created_of(&self, id: &str) -> i64 {
        self.current.locked().as_ref().filter(|job| job.id == id).map_or(0, |job| job.created)
    }

    /// The first output of the last finished job, whatever its type.
    pub fn first_file(&self) -> Option<PathBuf> {
        self.current.locked().as_ref().filter(|j| j.phase == Phase::Done).and_then(|j| j.files.first().cloned())
    }

    fn phase_of(&self, id: &str) -> Option<Phase> {
        self.current.locked().as_ref().filter(|j| j.id == id).map(|j| j.phase)
    }

    /// Whether the `/ws` listener still has work to report.
    pub(crate) fn is_current(&self, id: &str) -> bool {
        self.phase_of(id).is_some_and(Phase::active)
    }

    fn announce(&self, job: &Job) {
        let _ = self.events.send(json!({ "type": "toolJob", "tool": "comfy", "job": job.public() }));
    }

    /// Changes the job while it is still active: a cancellation or a finish already recorded stays.
    /// A job that reaches a final phase is copied to the history.
    fn update(&self, id: &str, change: impl FnOnce(&mut Job)) {
        let mut guard = self.current.locked();
        let Some(job) = guard.as_mut().filter(|j| j.id == id && j.phase.active()) else { return };
        change(job);
        if !job.phase.active() {
            let mut history = self.history.locked();
            history.retain(|old| old.id != job.id);
            history.push_front(job.clone());
            history.truncate(KEEP_HISTORY);
        }
        self.announce(job);
    }

    /// `/ws`: the node that started and the sampler's step, while the job runs.
    pub(crate) fn progress(&self, id: &str, node: &str, step: u64, total: u64) {
        let mut guard = self.current.locked();
        let Some(job) = guard.as_mut().filter(|j| j.id == id && j.phase.active()) else { return };
        let class = job.classes.get(node).cloned().unwrap_or_default();
        let changed = job.progress.as_ref().is_none_or(|old| old.node != node || old.step != step);
        job.progress = Some(Progress { node: node.to_owned(), class, step, total });
        if changed {
            let public = job.public();
            drop(guard);
            let _ = self.events.send(json!({ "type": "toolJob", "tool": "comfy", "job": public }));
        }
    }

    /// Validates and queues `graph` for `bot`; the job is followed in the background.
    pub async fn submit(self: &Arc<Self>, hub: &Arc<Hub>, bot: &str, graph: &Value) -> Result<Value> {
        self.submit_for(hub, bot, None, graph).await
    }

    /// [`Self::submit`] for a graph a named template built.
    pub async fn submit_for(
        self: &Arc<Self>,
        hub: &Arc<Hub>,
        bot: &str,
        template: Option<&str>,
        graph: &Value,
    ) -> Result<Value> {
        let _slot = self.gate.lock().await;
        ensure!(
            !self.current.locked().as_ref().is_some_and(|j| j.phase.active()),
            "ComfyUI runs one job at a time: wait for the current one or cancel it"
        );
        let api = running_api(hub)?;
        let info = api.object_info().await?;
        validate(graph, &info)?;
        let id = api.submit(graph, "codync").await?;
        let job = Job::queued(&id, bot, template, classes(graph));
        *self.current.locked() = Some(job.clone());
        self.announce(&job);
        tokio::spawn(self.clone().follow(hub.clone(), id.clone()));
        tokio::spawn(comfy::progress::follow(hub.tools.comfy_port(), id, self.clone()));
        Ok(job.public())
    }

    /// Waits up to `limit` for the current job, from inside `bot`'s turn. When it is done and `bot` asked
    /// for it, its outputs go to `bot`'s chat (once). A wait that is abandoned (the turn stopped) cancels it.
    pub async fn wait(self: &Arc<Self>, hub: &Arc<Hub>, bot: &str, limit: Duration) -> Result<Value> {
        let Some((id, owner)) = self.current.locked().as_ref().map(|j| (j.id.clone(), j.bot.clone())) else {
            bail!("there is no ComfyUI job: start one with run_workflow or comfy_run_template");
        };
        // Only the bot that asked for the job stops it by stopping its turn.
        let mut guard = CancelOnDrop { jobs: self.clone(), hub: hub.clone(), id: id.clone(), armed: owner == bot };
        let deadline = tokio::time::Instant::now() + limit;
        while self.phase_of(&id).is_some_and(Phase::active) && tokio::time::Instant::now() < deadline {
            tokio::time::sleep(POLL_EVERY).await;
        }
        guard.armed = false;
        let mut status = self.status();
        if self.phase_of(&id).is_some_and(Phase::active) {
            status["note"] = json!("still running: call wait_for_job to keep waiting, or cancel_job to stop it");
        }
        let files = self.claim_delivery(&id, bot);
        if !files.is_empty() {
            match attach(hub, bot, &files).await {
                Ok(()) => status["delivered"] = json!(true),
                Err(e) => {
                    self.release_delivery(&id);
                    status["attachError"] = json!(format!("{e:#}"));
                }
            }
        }
        Ok(status)
    }

    /// The outputs of a finished job of `bot` that are not in its chat yet, marked as sent.
    fn claim_delivery(&self, id: &str, bot: &str) -> Vec<PathBuf> {
        let mut guard = self.current.locked();
        let Some(job) = guard.as_mut().filter(|j| j.id == id && j.bot == bot && j.phase == Phase::Done && !j.delivered)
        else {
            return vec![];
        };
        job.delivered = true;
        job.files.clone()
    }

    /// Sending failed: a later call may try again.
    fn release_delivery(&self, id: &str) {
        if let Some(job) = self.current.locked().as_mut().filter(|j| j.id == id) {
            job.delivered = false;
        }
    }

    /// Stops the current job: interrupts it when it runs, removes it when it waits.
    pub async fn cancel(&self, hub: &Arc<Hub>) -> Result<Value> {
        let Some(job) = self.current.locked().clone().filter(|j| j.phase.active()) else {
            bail!("there is no ComfyUI job to cancel");
        };
        let api = running_api(hub)?;
        if job.phase == Phase::Running {
            api.interrupt().await?;
        } else {
            api.dequeue(&job.id).await?;
        }
        self.update(&job.id, |j| j.phase = Phase::Cancelled);
        Ok(self.status())
    }

    /// Keeps the job's state current for the card while it waits and runs.
    async fn follow(self: Arc<Self>, hub: Arc<Hub>, id: String) {
        let started = tokio::time::Instant::now();
        loop {
            tokio::time::sleep(POLL_EVERY).await;
            if !self.phase_of(&id).is_some_and(Phase::active) {
                return;
            }
            let Ok(api) = running_api(&hub) else {
                self.update(&id, |j| {
                    j.phase = Phase::Error;
                    j.error = Some("ComfyUI stopped while the job was running".into());
                });
                return;
            };
            if started.elapsed() > MAX_RUN {
                let _ = api.interrupt().await;
                self.update(&id, |j| {
                    j.phase = Phase::Error;
                    j.error = Some("the job took longer than 30 minutes and was stopped".into());
                });
                return;
            }
            match self.poll(&api, &id).await {
                Ok(Some((phase, error, files))) => {
                    self.update(&id, |j| {
                        j.phase = phase;
                        j.error = error;
                        j.files = files;
                    });
                    return;
                }
                Ok(None) => {}
                Err(e) => tracing::debug!(error = format!("{e:#}"), "following the ComfyUI job"),
            }
        }
    }

    /// One look at the server: how the job ended, or `None` while it still waits or runs.
    async fn poll(&self, api: &Api, id: &str) -> Result<Option<(Phase, Option<String>, Vec<PathBuf>)>> {
        let record = api.history(id).await?;
        if !record.is_null() {
            return Ok(Some(if record["status"]["status_str"] == "error" {
                (Phase::Error, Some(failure(&record)), vec![])
            } else {
                let mut files = output_files(&record);
                // A graph that writes its own file and reports none (OneShot MV's director, Splat
                // Edit Engine's scene): take what the run touched.
                if files.is_empty() {
                    files = recent_outputs(&comfy::output_dir(), self.created_of(id));
                }
                (Phase::Done, None, files)
            }));
        }
        let running = api.queue().await?["queue_running"]
            .as_array()
            .is_some_and(|rows| rows.iter().any(|row| row.get(1).and_then(Value::as_str) == Some(id)));
        if running {
            self.update(id, |j| j.phase = Phase::Running);
        }
        Ok(None)
    }
}

#[cfg(test)]
impl Jobs {
    /// A test job that is already active, so `/ws` has something to report.
    pub(crate) fn activate_for_test(&self, id: &str) {
        *self.current.locked() = Some(Job::queued(id, "test", None, BTreeMap::new()));
    }

    /// Ends the test job so the `/ws` listener stops.
    pub(crate) fn finish_for_test(&self, id: &str) {
        self.update(id, |job| job.phase = Phase::Done);
    }
}

/// Sends `files` to `bot`'s chat; only works while the bot's turn runs.
async fn attach(hub: &Arc<Hub>, bot: &str, files: &[PathBuf]) -> Result<()> {
    for file in files {
        crate::chat::outbox::call(hub, bot, "send_file", &json!({ "path": file })).await?;
    }
    Ok(())
}

fn running_api(hub: &Hub) -> Result<Api> {
    ensure!(hub.tools.comfy_running(), "ComfyUI is not running: start it from Extensions › Apps");
    Ok(Api::new(hub.tools.comfy_port()))
}

/// The numbered nodes and their classes, so `/ws` can name the one that started.
fn classes(graph: &Value) -> BTreeMap<String, String> {
    graph
        .as_object()
        .into_iter()
        .flat_map(|nodes| nodes.iter())
        .filter_map(|(id, node)| Some((id.clone(), node["class_type"].as_str()?.to_owned())))
        .collect()
}

/// The server's error text from a failed run's `status.messages`.
fn failure(record: &Value) -> String {
    record["status"]["messages"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|m| m[0] == "execution_error")
        .and_then(|m| m[1]["exception_message"].as_str())
        .unwrap_or("the workflow failed")
        .trim()
        .to_owned()
}

/// Every saved output of a finished run, as paths inside ComfyUI's output folder. A node may save
/// images, videos, audio or meshes under any key, so the whole `outputs` record is walked for files
/// marked `output` instead of only reading `images`.
pub(super) fn output_files(record: &Value) -> Vec<PathBuf> {
    let root = comfy::output_dir();
    let mut files = Vec::new();
    collect_outputs(&record["outputs"], &root, &mut files);
    files.sort();
    files.dedup();
    files
}

/// Output files touched since `since` (milliseconds), newest last, at most five. A wrapper node can
/// write a film and its sidecars without reporting them, so a run's own files are the fallback.
fn recent_outputs(root: &std::path::Path, since: i64) -> Vec<PathBuf> {
    let mut files = Vec::new();
    walk_outputs(root, 0, since, &mut files);
    files.sort_by_key(|path| std::fs::metadata(path).and_then(|meta| meta.modified()).ok());
    if files.len() > 5 {
        files.drain(..files.len() - 5);
    }
    files
}

fn walk_outputs(dir: &std::path::Path, depth: usize, since: i64, files: &mut Vec<PathBuf>) {
    if depth > 3 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') || name.ends_with(".part") {
            continue;
        }
        match entry.file_type() {
            Ok(kind) if kind.is_dir() => walk_outputs(&path, depth + 1, since, files),
            Ok(kind) if kind.is_file() => {
                let touched = std::fs::metadata(&path)
                    .and_then(|meta| meta.modified())
                    .ok()
                    .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                    // Millis since 1970 fit an `i64` for the next 292 million years.
                    .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX));
                if touched >= since && results::ingestible(&path) {
                    files.push(path);
                }
            }
            _ => {}
        }
    }
}

fn collect_outputs(value: &Value, root: &std::path::Path, files: &mut Vec<PathBuf>) {
    match value {
        Value::Object(map) => {
            if let Some(name) = map.get("filename").and_then(Value::as_str) {
                let kind = map.get("type").and_then(Value::as_str).unwrap_or("output");
                let sub = map.get("subfolder").and_then(Value::as_str).unwrap_or_default();
                let unsafe_part = |s: &str| s.split(['/', '\\']).any(|p| p == "..") || s.contains(':');
                if kind == "output" && !unsafe_part(name) && !unsafe_part(sub) {
                    files.push(root.join(sub).join(name));
                }
            }
            map.values().for_each(|item| collect_outputs(item, root, files));
        }
        Value::Array(items) => items.iter().for_each(|item| collect_outputs(item, root, files)),
        _ => {}
    }
}

/// Checks a graph against the node classes the server knows, for a message better than a bare 400:
/// unknown nodes come with near names, and a missing checkpoint says where to put it.
pub fn validate(graph: &Value, info: &Value) -> Result<()> {
    let Some(nodes) = graph.as_object().filter(|n| !n.is_empty()) else {
        bail!("the workflow must be a ComfyUI API-format graph: an object of numbered nodes");
    };
    for (id, node) in nodes {
        let Some(class) = node["class_type"].as_str() else { bail!("node {id} has no class_type") };
        let Some(spec) = info.get(class) else {
            bail!("node {id}: ComfyUI has no node called `{class}`.{}", near(class, info));
        };
        let Some(wanted) = node["inputs"]["ckpt_name"].as_str() else { continue };
        let installed: Vec<&str> = spec["input"]["required"]["ckpt_name"][0]
            .as_array()
            .map(|all| all.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        if !installed.contains(&wanted) {
            bail!(
                "checkpoint `{wanted}` is not installed. Put the file in {} (the Models button opens it) or download one from {MODELS_URL}. Installed: {}",
                comfy::checkpoints_dir().display(),
                if installed.is_empty() { "none".into() } else { installed.join(", ") }
            );
        }
    }
    Ok(())
}

/// Up to five known node names that contain the start of `class`.
fn near(class: &str, info: &Value) -> String {
    let head: String = class.chars().take(5).collect::<String>().to_lowercase();
    let found: Vec<&str> = info
        .as_object()
        .into_iter()
        .flat_map(|all| all.keys())
        .filter(|k| k.to_lowercase().contains(&head))
        .take(5)
        .map(String::as_str)
        .collect();
    if found.is_empty() { String::new() } else { format!(" Similar: {}.", found.join(", ")) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info() -> Value {
        json!({
            "CheckpointLoaderSimple": {"input": {"required": {"ckpt_name": [["a.safetensors"]]}}},
            "KSampler": {"input": {"required": {}}},
        })
    }

    #[test]
    fn a_known_graph_passes() {
        let graph = json!({"1": {"class_type": "CheckpointLoaderSimple", "inputs": {"ckpt_name": "a.safetensors"}}});
        validate(&graph, &info()).expect("valid");
    }

    #[test]
    fn an_unknown_node_gets_near_names() {
        let graph = json!({"1": {"class_type": "KSamplerAdvanced", "inputs": {}}});
        let err = validate(&graph, &info()).expect_err("unknown").to_string();
        assert!(err.contains("no node called `KSamplerAdvanced`") && err.contains("Similar: KSampler"), "{err}");
    }

    #[test]
    fn a_missing_checkpoint_says_where_to_put_it() {
        let graph = json!({"1": {"class_type": "CheckpointLoaderSimple", "inputs": {"ckpt_name": "b.safetensors"}}});
        let err = validate(&graph, &info()).expect_err("missing").to_string();
        assert!(err.contains("`b.safetensors` is not installed") && err.contains("Installed: a.safetensors"), "{err}");
    }

    #[test]
    fn output_paths_stay_inside_the_output_folder_and_keep_every_type() {
        let record = json!({"outputs": {
            "9": {"images": [{"filename": "ok.png", "subfolder": "", "type": "output"}]},
            "10": {"gifs": [{"filename": "clip.mp4", "subfolder": "video", "type": "output"}]},
            "11": {"audio": [{"filename": "voice.wav", "subfolder": "", "type": "output"}]},
            "12": {"files": [{"filename": "..\\evil.png", "subfolder": "", "type": "output"}]},
            "13": {"images": [{"filename": "temp.png", "subfolder": "", "type": "temp"}]},
        }});
        let files = output_files(&record);
        assert_eq!(
            files,
            vec![
                comfy::output_dir().join("ok.png"),
                comfy::output_dir().join("video").join("clip.mp4"),
                comfy::output_dir().join("voice.wav")
            ]
        );
    }

    #[test]
    fn a_run_that_saves_by_itself_still_reports_its_file() {
        let root = std::env::temp_dir().join(format!("codync-outputs-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("run")).expect("dir");
        std::fs::write(root.join("old.png"), b"x").expect("old");
        std::thread::sleep(Duration::from_millis(20));
        std::fs::write(root.join("run").join("film.mp4"), b"x").expect("film");
        std::fs::write(root.join("notes.txt"), b"x").expect("notes");

        let files = recent_outputs(&root, 0);
        assert_eq!(files.len(), 2);
        assert_eq!(files.last().and_then(|p| p.file_name()).and_then(|n| n.to_str()), Some("film.mp4"));
        assert_eq!(recent_outputs(&root, chrono::Utc::now().timestamp_millis() + 60_000), Vec::<PathBuf>::new());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn a_finished_job_is_archived_once() {
        let jobs = Jobs::new(broadcast::channel(8).0);
        *jobs.current.locked() = Some(Job::queued("p1", "b", Some("text-to-image"), BTreeMap::new()));
        jobs.update("p1", |j| j.phase = Phase::Running);
        jobs.update("p1", |j| j.phase = Phase::Done);
        jobs.update("p1", |j| j.phase = Phase::Done);
        assert_eq!(jobs.history().as_array().map(Vec::len), Some(1));
        assert_eq!(jobs.status()["phase"], "done");
    }
}
