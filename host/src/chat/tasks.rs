//! Agent Tasks: named, long-lived turns of an agent, separate from its chat.
//! A task is a row in the store plus a root notice entry; its messages are entries in that root's
//! thread, posted through the same `chat::messages::post_in_lane` path as any side conversation, so
//! the bot's one actor, queue and steering decide when they run. Stop reuses `hub.stop_bot`.
//! No second agent loop or message store is started here.
//! Origin: OpenBot nightly's `agent-tasks` subsystem, adapted to Codync's one-agent bot queue.

use crate::hub::Hub;
use crate::store::tasks::{MAX_PER_BOT, clamp_limit, validate_name};
use crate::store::{Entry, EntryKind, Lane, TaskInfo, TaskState};
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};
use std::sync::Arc;

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

/// An event without a store write, for a stop: the runtime changed, the row did not.
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

pub(crate) fn task_list(hub: &Hub, bot: &str, status: Option<TaskState>) -> Result<Value> {
    hub.store.bot(bot)?.filter(|r| !r.deleted).context("unknown bot")?;
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
    // One actor serves the bot's chat and all of its tasks. Stopping the task interrupts its
    // current turn there; it does not spawn or kill a separate agent.
    hub.stop_bot(&task.bot_id)?;
    emit_notice(hub, json!({"type": "task", "rev": task.rev, "taskId": task.id, "stopped": true}))?;
    Ok(json!({"stopped": true, "rev": task.rev}))
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
