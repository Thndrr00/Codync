//! Agent Tasks: named, long-lived turns of an agent, separate from its chat.
//! A task is a row in the store plus a root notice entry; its messages are entries in that root's
//! thread, posted through the same `chat::messages::post_in_lane` path as any side conversation, so
//! the bot's one actor, queue and steering decide when they run. Stop reuses `hub.stop_bot`.
//! No second agent loop or message store is started here.
//! Origin: OpenBot nightly's `agent-tasks` subsystem, adapted to Codync's one-agent bot queue.

mod await_event;
mod parallel;

use crate::hub::Hub;
use crate::store::tasks::{MAX_PAGE, MAX_PER_BOT, TaskRun, clamp_limit, validate_name};
use crate::store::{Entry, EntryKind, Lane, TaskInfo, TaskState};
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub use await_event::{EventWait, clamp_wait, parse_wait};

/// How many messages a branch copies from its source. The seed is a snapshot, not a live link.
const MAX_SEED: usize = 50;

/// The lifecycle in one place, so start, close, reopen and archive agree. A same-state call is a
/// harmless no-op; an archived task must be reopened before it can be closed again.
fn can_transition(from: TaskState, to: TaskState) -> bool {
    if from == to {
        return true;
    }
    match (from, to) {
        (TaskState::Open, TaskState::Closed | TaskState::Archived)
        | (TaskState::Closed, TaskState::Open | TaskState::Archived)
        | (TaskState::Archived, TaskState::Open) => true,
        _ => false,
    }
}

/// A closed or archived task is read-only until it is reopened.
fn accepts_messages(state: TaskState) -> bool {
    state == TaskState::Open
}

/// Which entry kinds a branch copies: the user's words and the agent's final answers, never the
/// thinking, tool trace, notices or drafts that led to them.
fn message_kind(kind: &str, data: &Value) -> Option<EntryKind> {
    match kind {
        "user" => Some(EntryKind::User),
        "agent" if data["final"].as_bool() == Some(true) => Some(EntryKind::Agent),
        _ => None,
    }
}

/// Pure seed selection: the newest `MAX_SEED` messages before `before`, capped and in time order.
fn seed_entries(entries: &[Entry], before: Option<i64>) -> Vec<&Entry> {
    let mut kept: Vec<&Entry> = entries
        .iter()
        .filter(|e| before.is_none_or(|seq| e.seq < seq))
        .filter(|e| message_kind(&e.kind, &e.data).is_some())
        .collect();
    let excess = kept.len().saturating_sub(MAX_SEED);
    if excess > 0 {
        kept.drain(..excess);
    }
    kept
}

fn require(hub: &Hub, id: &str) -> Result<TaskInfo> {
    hub.store.task(id)?.with_context(|| format!("unknown task `{id}`"))
}

/// Saves a task change under the hub's emission lock, so the `task` event leaves after the rev it
/// names and no other writer slips between the two.
fn emit(hub: &Hub, save: impl FnOnce() -> Result<TaskInfo>) -> Result<Value> {
    let mut answer = None;
    hub.emit_change(|| {
        let task = save()?;
        let payload = serde_json::to_value(&task)?;
        answer = Some(payload.clone());
        Ok(json!({"type": "task", "rev": task.rev, "task": payload}))
    })?;
    answer.context("task change was not saved")
}

fn emit_deleted(hub: &Hub, id: &str, rev: i64) -> Result<Value> {
    hub.emit_change(|| Ok(json!({"type": "task", "rev": rev, "taskId": id, "deleted": true})))?;
    Ok(json!({"deleted": true, "rev": rev}))
}

/// A lifecycle-free event, after the runtime (and any scheduling mark) has already been saved.
fn emit_notice(hub: &Hub, event: Value) -> Result<()> {
    hub.emit_change(|| Ok(event))
}

pub(crate) fn task_create(hub: &Arc<Hub>, bot: &str, name: &str) -> Result<Value> {
    let row = hub.store.bot(bot)?.filter(|r| !r.deleted).context("unknown bot")?;
    ensure!(!row.config.is_group(), "tasks belong to an agent bot");
    let name = validate_name(name)?;
    ensure!(hub.store.task_count(bot)? < MAX_PER_BOT, "this bot already has {MAX_PER_BOT} tasks; delete one first");
    // The root is a chat notice: it anchors the thread, gives clients a card to attach the task
    // to and uses the existing `entry` event path. Messages then live in `Lane::in_thread`.
    let root = format!("task:{}", uuid::Uuid::new_v4());
    hub.add_entry_once(
        &root,
        &Lane::main(bot),
        EntryKind::Notice,
        0,
        &json!({"text": name, "task": root, "system": true}),
    )?;
    emit(hub, || hub.store.task_insert(bot, &name, &root))
}

/// Posts one admitted task's prompt into its own lane, so the bot's existing actor and queue run
/// it in the task's session. No second agent loop is started here.
fn schedule(hub: &Arc<Hub>, bot: &str) -> Result<usize> {
    let waiting = hub.store.task_queued_for(bot)?;
    let running = hub.store.task_running_count(bot)?;
    let cfg = hub.store.bot(bot)?.filter(|r| !r.deleted).context("unknown bot")?.config;
    let admitted: Vec<TaskInfo> = parallel::admit(&waiting, running).into_iter().cloned().collect();
    let mut started = 0;
    let mut failure = None;
    for task in admitted {
        if !hub.store.task_claim(&task.id)? {
            continue;
        }
        let mut data = json!({"text": task.prompt.clone(), "taskId": task.id, "source": "task"});
        if let Some((chat, thread)) = parallel::result_parent(&task) {
            data["parent"] = json!({"chat": chat, "thread": thread});
        }
        match crate::chat::messages::post_in_lane(hub, &cfg, Lane::in_thread(bot, &task.root), data) {
            Ok(_) => started += 1,
            Err(error) => {
                // A claim that could not be handed to the actor goes back to the queue instead of
                // becoming a task that looks done but never ran.
                hub.store.task_set_run(&task.id, TaskRun::Queued)?;
                tracing::warn!(task = %task.id, error = format!("{error:#}"), "could not start task");
                failure.get_or_insert(error);
            }
        }
    }
    if let Some(error) = failure {
        return Err(error);
    }
    Ok(started)
}

/// An agent splitting unrelated work: create all tasks first, then fill at most `MAX_CONCURRENT`
/// slots. The rest keep waiting in the same actor and start as earlier tasks finish.
pub(crate) fn task_split(
    hub: &Arc<Hub>,
    bot: &str,
    values: &[Value],
    parent_chat: Option<&str>,
    parent_thread: Option<&str>,
) -> Result<Value> {
    let row = hub.store.bot(bot)?.filter(|r| !r.deleted).context("unknown bot")?;
    ensure!(!row.config.is_group(), "tasks belong to an agent bot");
    let specs = parallel::validate_split(values)?;
    ensure!(
        hub.store.task_count(bot)? + specs.len() <= MAX_PER_BOT,
        "this bot already has {MAX_PER_BOT} tasks; delete one first"
    );
    let parent = parent_chat.unwrap_or(bot);
    // A thread only belongs to this bot; never route a result into another agent's side thread.
    let thread = parent_thread.filter(|_| parent == bot);
    ensure!(hub.store.bot(parent)?.filter(|r| !r.deleted).is_some(), "unknown parent chat");
    let mut ids = Vec::new();
    for spec in specs {
        let created = task_create(hub, bot, &spec.name)?;
        let id = created["task"]["id"].as_str().context("task was not created")?.to_owned();
        hub.store.task_queue(&id, Some(parent), thread, &spec.text)?;
        ids.push(id);
    }
    let scheduled = schedule(hub, bot)?;
    Ok(json!({"tasks": ids, "scheduled": scheduled, "maxConcurrent": parallel::MAX_CONCURRENT}))
}

/// A follow-up is the deliberate way to run a task again, including one stopped from its tab.
pub(crate) fn task_follow_up(hub: &Arc<Hub>, bot: &str, id: &str, text: &str) -> Result<Value> {
    let task = require(hub, id)?;
    ensure!(task.bot_id == bot, "the task belongs to another bot");
    ensure!(parallel::follow_up_allowed(&task), "reopen the task before sending to it");
    let text = text.trim();
    ensure!(!text.is_empty(), "a follow-up can't be empty");
    let (chat, thread) = parallel::result_parent(&task).unwrap_or((bot, None));
    let task = emit(hub, || hub.store.task_queue(id, Some(chat), thread, text))?;
    schedule(hub, bot)?;
    Ok(task)
}

/// Delivers the task's teammate-style result into the parent chat (or its side thread).
fn route(hub: &Arc<Hub>, chat: &str, thread: Option<&str>, task: &TaskInfo, text: &str) -> Result<()> {
    let cfg = hub.store.bot(chat)?.filter(|r| !r.deleted).context("unknown parent chat")?.config;
    let lane = match thread {
        Some(root) => Lane::in_thread(chat, root),
        None => Lane::main(chat),
    };
    let data = json!({
        "text": text,
        "source": "task",
        "teamReply": true,
        "taskId": task.id,
        "taskName": task.name,
        "fromBotId": task.bot_id,
        "delegationId": task.id,
    });
    crate::chat::messages::post_in_lane(hub, &cfg, lane, data)?;
    Ok(())
}

/// Routes newly finished task turns back to their parents, once each. Called by `taskList` and by
/// the explicit `taskCollectResults` method.
pub(crate) fn task_collect_results(hub: &Arc<Hub>, bot: &str) -> Result<usize> {
    let mut routed = 0;
    let mut freed: Vec<String> = Vec::new();
    for task in hub.store.task_with_parent()? {
        if task.bot_id != bot {
            continue;
        }
        let Some((chat, thread)) = parallel::result_parent(&task) else { continue };
        let messages = hub.store.task_messages(&task.root, MAX_PAGE, None, None)?;
        let Some(final_entry) = parallel::final_after(&messages, task.routed_seq) else { continue };
        let text = parallel::result_text(&task.name, final_entry.data["text"].as_str(), "");
        route(hub, chat, thread, &task, &text)?;
        let seq = final_entry.seq;
        let free = parallel::run_after_result(&task);
        emit(hub, || {
            hub.store.task_mark_routed(&task.id, seq)?;
            hub.store.task_set_run(&task.id, free)
        })?;
        freed.push(task.bot_id);
        routed += 1;
    }
    // A returned result is what frees a slot; fill it in the same pass instead of waiting for the
    // next tab refresh, so a split of ten tasks keeps four running until the queue drains.
    freed.sort();
    freed.dedup();
    for id in freed {
        schedule(hub, &id)?;
    }
    Ok(routed)
}

/// Waits in the task's own side thread for a routine run or a script event from the stream the host
/// already keeps. The run that wakes the wait continues the request in that same thread.
pub(crate) async fn task_await_event(hub: &Arc<Hub>, id: &str, wait: EventWait, timeout: Duration) -> Result<Value> {
    let task = require(hub, id)?;
    ensure!(task.state == TaskState::Open, "reopen the task before waiting in it");
    let deadline = Instant::now() + timeout;
    loop {
        let task = require(hub, id)?;
        let runs = hub.routines.runs_for(&task.bot_id);
        let probes: Vec<await_event::EventProbe<'_>> = runs
            .iter()
            .map(|run| await_event::EventProbe {
                run_id: &run.id,
                status: status_str(run.status),
                source: await_event::field(&run.event, "source"),
                name: {
                    let name = await_event::field(&run.event, "event");
                    if name.is_empty() { await_event::field(&run.event, "name") } else { name }
                },
            })
            .collect();
        let found = probes.iter().find(|probe| await_event::matches(&wait, *probe)).copied();
        let remaining = deadline.saturating_duration_since(Instant::now());
        let expired = remaining.is_zero();
        match await_event::outcome(&wait, found.as_ref(), &task, expired) {
            await_event::WaitOutcome::Waiting => {
                tokio::time::sleep(await_event::POLL.min(remaining)).await;
            }
            await_event::WaitOutcome::Cancelled => return Ok(json!({"cancelled": true})),
            await_event::WaitOutcome::TimedOut => return Ok(json!({"timedOut": true})),
            await_event::WaitOutcome::Arrived => {
                let probe = found.context("a matching event was found")?;
                let cfg = hub.store.bot(&task.bot_id)?.filter(|r| !r.deleted).context("unknown bot")?.config;
                let text = await_event::wake_text(&probe);
                crate::chat::messages::post_in_lane(
                    hub,
                    &cfg,
                    Lane::in_thread(&task.bot_id, &task.root),
                    json!({"text": text, "taskId": task.id, "source": "task-event"}),
                )?;
                return Ok(json!({
                    "event": {"runId": probe.run_id, "source": probe.source, "name": probe.name},
                    "status": probe.status,
                    "continuedInThread": true,
                }));
            }
        }
    }
}

/// The wire form of a run status, without serializing a whole run for one string.
fn status_str(status: crate::routines::Status) -> &'static str {
    use crate::routines::Status;
    match status {
        Status::Pending => "pending",
        Status::Starting => "starting",
        Status::Running => "running",
        Status::Recovering => "recovering",
        Status::Succeeded => "succeeded",
        Status::Failed => "failed",
        Status::Interrupted => "interrupted",
        Status::Cancelled => "cancelled",
    }
}

pub(crate) fn task_list(hub: &Arc<Hub>, bot: &str, status: Option<TaskState>) -> Result<Value> {
    hub.store.bot(bot)?.filter(|r| !r.deleted).context("unknown bot")?;
    // A tab that lists tasks is the natural place to deliver results that finished while it was
    // closed; the explicit `taskCollectResults` method does the same for other clients.
    task_collect_results(hub, bot)?;
    Ok(json!({"tasks": hub.store.tasks(bot, status)?}))
}

pub(crate) fn task_rename(hub: &Hub, id: &str, name: &str) -> Result<Value> {
    emit(hub, || hub.store.task_rename(id, name))
}

pub(crate) fn task_start(hub: &Hub, id: &str) -> Result<Value> {
    let task = require(hub, id)?;
    ensure!(can_transition(task.state, TaskState::Open), "a task in `{}` can't be started", task.state.as_str());
    emit(hub, || hub.store.task_set_state(id, TaskState::Open, true))
}

pub(crate) fn task_stop(hub: &Arc<Hub>, id: &str) -> Result<Value> {
    let task = require(hub, id)?;
    // A stop from the tab is final for this scheduling attempt: stamp it before touching the
    // runtime, so nothing (a later split, a result, a retry) queues the task again on its own.
    let stopped = emit(hub, || hub.store.task_set_run(id, TaskRun::Stopped))?;
    // One actor serves the bot's chat and all of its tasks. Stopping the task interrupts its
    // current turn there; it does not spawn or kill a separate agent.
    hub.stop_bot(&task.bot_id)?;
    // Telling the agent is a turn in the task's own lane, not just a card: the same actor that was
    // stopped reads it next, so the task is not quietly started again.
    let cfg = hub.store.bot(&task.bot_id)?.filter(|r| !r.deleted).context("unknown bot")?.config;
    crate::chat::messages::post_in_lane(
        hub,
        &cfg,
        Lane::in_thread(&task.bot_id, &task.root),
        json!({
            "text": parallel::stop_notice(&task.name),
            "taskId": task.id,
            "source": "task",
            "system": true,
        }),
    )?;
    // The parent agent is told through the same teammate-style path as a result, so it combines a
    // stop with the other outcomes instead of waiting for a task that will not restart.
    if let Some((chat, thread)) = parallel::result_parent(&task) {
        let messages = hub.store.task_messages(&task.root, MAX_PAGE, None, None)?;
        let cursor = messages.last().map_or(task.routed_seq, |entry| entry.seq);
        let text = parallel::result_text(&task.name, None, "stopped from its tab");
        route(hub, chat, thread, &task, &text)?;
        emit(hub, || hub.store.task_mark_routed(id, cursor))?;
    }
    emit_notice(
        hub,
        json!({"type": "task", "rev": stopped["rev"], "taskId": task.id, "stopped": true, "agentTold": true}),
    )?;
    Ok(json!({"stopped": true, "agentTold": true, "rev": stopped["rev"]}))
}

pub(crate) fn task_close(hub: &Hub, id: &str, archived: bool) -> Result<Value> {
    let task = require(hub, id)?;
    let target = if archived { TaskState::Archived } else { TaskState::Closed };
    ensure!(can_transition(task.state, target), "a task in `{}` can't be closed", task.state.as_str());
    emit(hub, || hub.store.task_set_state(id, target, false))
}

pub(crate) fn task_reopen(hub: &Hub, id: &str) -> Result<Value> {
    let task = require(hub, id)?;
    ensure!(can_transition(task.state, TaskState::Open), "a task in `{}` can't be reopened", task.state.as_str());
    emit(hub, || hub.store.task_set_state(id, TaskState::Open, false))
}

pub(crate) fn task_delete(hub: &Arc<Hub>, id: &str) -> Result<Value> {
    let task = require(hub, id)?;
    // Delete the thread's lines one by one so each reaches clients as an `entryDeleted`, then
    // remove the row and stamp a final rev. A failed line read is not a reason to leak the task.
    for entry_id in hub.store.task_entry_ids(&task.root)? {
        hub.delete_entry(&entry_id)?;
    }
    let rev = hub.store.task_delete(id)?.context("unknown task")?;
    emit_deleted(hub, id, rev)
}

pub(crate) fn task_mark_read(hub: &Hub, id: &str, through: Option<&str>) -> Result<Value> {
    let task = require(hub, id)?;
    let mark = match through {
        Some(entry_id) => {
            let entry = hub
                .store
                .entry(entry_id)
                .filter(|e| e.id == task.root || e.thread_id.as_deref() == Some(task.root.as_str()))
                .context("unknown message in this task")?;
            entry.rev
        }
        None => hub.store.current_rev(),
    };
    emit(hub, || {
        // The root entry is a thread root too: keep its chip's read mark in step with the task's.
        hub.store.mark_thread_read(&task.root)?;
        hub.store.task_mark_read(id, mark)
    })
}

pub(crate) fn task_read_page(
    hub: &Hub,
    id: &str,
    limit: Option<i64>,
    before: Option<i64>,
    after: Option<i64>,
) -> Result<Value> {
    let task = require(hub, id)?;
    let limit = clamp_limit(limit);
    let mut rows = hub.store.task_messages(&task.root, limit + 1, before, after)?;
    let has_more = rows.len() as i64 > limit;
    rows.truncate(limit as usize);
    Ok(json!({"entries": rows, "hasMore": has_more}))
}

pub(crate) fn task_read_around(hub: &Hub, id: &str, message_id: &str, limit: Option<i64>) -> Result<Value> {
    let task = require(hub, id)?;
    let anchor = hub
        .store
        .entry(message_id)
        .filter(|e| e.thread_id.as_deref() == Some(task.root.as_str()))
        .context("unknown message in this task")?;
    let rows = hub.store.task_messages_around(&task.root, anchor.seq, clamp_limit(limit))?;
    Ok(json!({"entries": rows}))
}

pub(crate) fn task_branch(
    hub: &Arc<Hub>,
    bot: &str,
    source_id: &str,
    name: &str,
    before: Option<i64>,
) -> Result<Value> {
    let source = require(hub, source_id)?;
    ensure!(source.bot_id == bot, "the task belongs to another bot");
    let row = hub.store.bot(bot)?.filter(|r| !r.deleted).context("unknown bot")?;
    ensure!(!row.config.is_group(), "tasks belong to an agent bot");
    let name = validate_name(name)?;
    ensure!(hub.store.task_count(bot)? < MAX_PER_BOT, "this bot already has {MAX_PER_BOT} tasks; delete one first");
    let prior = hub.store.task_messages(&source.root, MAX_SEED as i64, None, before)?;
    let seed = seed_entries(&prior, before);

    let created = task_create(hub, bot, &name)?;
    let new_id = created["task"]["id"].as_str().context("task was not created")?.to_owned();
    let new_task = require(hub, &new_id)?;
    let lane = Lane::in_thread(bot, &new_task.root);
    let mut copied = 0;
    for entry in seed {
        let Some(kind) = message_kind(&entry.kind, &entry.data) else { continue };
        let mut data = entry.data.clone();
        if let Some(fields) = data.as_object_mut() {
            fields.remove("task");
            fields.remove("clientNonce");
        }
        data["branchedFrom"] = json!({"taskId": source.id, "entryId": entry.id});
        hub.add_entry(&lane, kind, entry.turn, &data);
        copied += 1;
    }
    emit_notice(hub, json!({"type": "task", "rev": new_task.rev, "taskId": new_task.id.clone(), "seeded": copied}))?;
    Ok(json!({"task": new_task, "seeded": copied}))
}

pub(crate) fn task_delete_message(hub: &Arc<Hub>, id: &str, message_id: &str) -> Result<Value> {
    let task = require(hub, id)?;
    let entry = hub
        .store
        .entry(message_id)
        .filter(|e| e.thread_id.as_deref() == Some(task.root.as_str()))
        .context("unknown message in this task")?;
    ensure!(hub.delete_entry(&entry.id)?, "the message was already gone");
    Ok(json!({"deleted": true}))
}

pub(crate) fn task_send(
    hub: &Arc<Hub>,
    bot: &str,
    id: &str,
    text: &str,
    nonce: Option<&str>,
    reply_to: Option<&str>,
) -> Result<Value> {
    let task = require(hub, id)?;
    ensure!(task.bot_id == bot, "the task belongs to another bot");
    ensure!(accepts_messages(task.state), "reopen the task before sending to it");
    let text = text.trim();
    ensure!(!text.is_empty(), "a message can't be empty");
    let row = hub.store.bot(bot)?.filter(|r| !r.deleted).context("unknown bot")?;
    ensure!(!row.config.is_group(), "tasks belong to an agent bot");
    if let Some(reply) = reply_to {
        hub.store
            .entry(reply)
            .filter(|e| e.thread_id.as_deref() == Some(task.root.as_str()))
            .context("unknown message in this task")?;
    }
    let mut data = json!({"text": text, "taskId": id});
    if let Some(nonce) = nonce {
        data["clientNonce"] = json!(nonce);
    }
    if let Some(reply) = reply_to {
        data["replyTo"] = json!({"id": reply});
    }
    // The shared lane keeps this task's messages in its thread and its own agent session.
    let entry = crate::chat::messages::post_in_lane(hub, &row.config, Lane::in_thread(bot, &task.root), data)?;
    Ok(json!({"entry": entry}))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(seq: i64, kind: &str, final_answer: bool) -> Entry {
        Entry {
            id: format!("e{seq}"),
            seq,
            bot_id: "b".into(),
            rev: seq,
            kind: kind.into(),
            turn: 1,
            data: json!({"text": format!("m{seq}"), "final": final_answer}),
            created_at: 0,
            updated_at: 0,
            thread_id: Some("task:r1".into()),
        }
    }

    #[test]
    fn the_lifecycle_lets_a_closed_task_reopen_but_not_an_archive_be_closed() {
        assert!(can_transition(TaskState::Open, TaskState::Closed));
        assert!(can_transition(TaskState::Open, TaskState::Archived));
        assert!(can_transition(TaskState::Closed, TaskState::Open));
        assert!(can_transition(TaskState::Archived, TaskState::Open));
        assert!(!can_transition(TaskState::Archived, TaskState::Closed));
        assert!(can_transition(TaskState::Open, TaskState::Open), "a repeated action is harmless");
    }

    #[test]
    fn only_an_open_task_accepts_messages() {
        assert!(accepts_messages(TaskState::Open));
        assert!(!accepts_messages(TaskState::Closed));
        assert!(!accepts_messages(TaskState::Archived));
    }

    #[test]
    fn a_branch_seeds_only_user_and_final_messages_before_the_anchor() {
        let entries = vec![
            entry(1, "user", false),
            entry(2, "agent", false),
            entry(3, "tool", false),
            entry(4, "agent", true),
            entry(5, "user", false),
            entry(6, "user", false),
        ];
        let seed = seed_entries(&entries, Some(5));
        assert_eq!(seed.iter().map(|e| e.seq).collect::<Vec<_>>(), [1, 4]);
        assert_eq!(seed_entries(&entries, None).len(), 4, "no anchor takes all messages");
        assert!(seed_entries(&entries, Some(1)).is_empty());
    }

    #[test]
    fn a_branch_never_copies_more_than_the_seed_cap() {
        let entries: Vec<Entry> = (1..=MAX_SEED as i64 + 12).map(|i| entry(i, "user", false)).collect();
        let seed = seed_entries(&entries, None);
        assert_eq!(seed.len(), MAX_SEED);
        assert_eq!(seed.first().map(|e| e.seq), Some(13), "the oldest of the newest slice is kept");
        assert_eq!(seed.last().map(|e| e.seq), Some(MAX_SEED as i64 + 12));
    }

    #[test]
    fn only_user_and_final_agent_kinds_survive_a_branch() {
        assert_eq!(message_kind("user", &json!({})), Some(EntryKind::User));
        assert_eq!(message_kind("agent", &json!({"final": true})), Some(EntryKind::Agent));
        assert_eq!(message_kind("agent", &json!({"final": false})), None);
        assert_eq!(message_kind("tool", &json!({"final": true})), None);
        assert_eq!(message_kind("notice", &json!({})), None);
    }

    #[test]
    fn rename_validation_rejects_blank_and_overlong_names_before_any_write() {
        assert!(validate_name("   ").is_err());
        assert!(validate_name(&"x".repeat(121)).is_err());
        assert_eq!(validate_name("  Ship it  ").unwrap(), "Ship it");
    }
}
