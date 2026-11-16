//! A channel request's task tree: the request itself, the assignments the room gives its members, and the
//! assignments those members bring in by naming someone else.
//!
//! The tree is what makes a channel's work controllable. Every row has an owner and a state
//! ([`TaskState`]). Stop pauses a row and everything below it, so the room asks nobody there until
//! Continue; Reassign gives a stopped row another member and resumes what waited with it; Cancel ends the
//! request from its root down, and Drop (the lead's, [`drop_run`]) cancels a whole run because a person asked
//! for it. The room turn checks the tree before every speaker ([`next`]), so a paused or cancelled branch
//! never speaks, and a request's messages stay whatever happens to its tree.

use crate::agent::bot::Cmd;
use crate::hub::Hub;
use crate::store::{BotConfig, Entry, GroupTask, Lane, TaskKind, TaskState, new_task};
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;

/// One request's tree in a lane.
pub struct Run {
    pub id: String,
    pub group: String,
    pub lane: Lane,
    /// The user's message it answers.
    pub request: String,
}

/// What the room may do with a member before its turn.
pub enum Gate {
    /// The row is ready: its member takes the turn now, and reports to this row.
    Start(String),
    /// The row or one above it is stopped: no turn starts.
    Paused,
    /// This member already took its turn for this round (a resumed turn only asks what waits).
    Done,
    /// The request was cancelled or dropped: the room turn ends.
    Ended,
}

/// How a member's turn came out.
pub enum Outcome<'a> {
    Spoke,
    Passed,
    Failed(&'a str),
}

/// The run `request` starts, or — for a late answer or a resumed turn — the run it belongs to.
pub fn open(hub: &Hub, group: &BotConfig, lane: &Lane, request: &Entry, supersede: bool, resumed: bool) -> Result<Run> {
    let key = lane.key();
    let rows = hub.store.group_tasks(&group.id, &key)?;
    if resumed {
        let root = rows.first().context("nothing to continue in this channel")?;
        return Ok(Run {
            id: root.root_id.clone(),
            group: group.id.clone(),
            lane: lane.clone(),
            request: root.request_id.clone(),
        });
    }
    if let Some(root) = rows.first().filter(|root| root.kind == TaskKind::Request && root.request_id == request.id) {
        return Ok(Run {
            id: root.id.clone(),
            group: group.id.clone(),
            lane: lane.clone(),
            request: request.id.clone(),
        });
    }
    if supersede {
        // A newer request replaces the lane's earlier one: nothing of it can still speak.
        for row in rows.iter().filter(|row| row.state.is_open()) {
            set_state(hub, row, TaskState::Cancelled, Some("A newer request replaced it"));
        }
    }
    let text = request.data["text"].as_str().unwrap_or_default();
    let mut root = new_task(
        &group.id,
        &key,
        &request.id,
        "",
        None,
        group.chief_bot_id.as_deref(),
        TaskKind::Request,
        TaskState::Running,
        0,
        text,
    );
    root.root_id.clone_from(&root.id);
    hub.store.save_group_task(&root)?;
    hub.store.delete_older_group_tasks(&group.id, &key, &root.id)?;
    emit(hub, &group.id);
    Ok(Run { id: root.id, group: group.id.clone(), lane: lane.clone(), request: request.id.clone() })
}

/// The members a resumed turn asks, in the order their work was given out.
pub fn waiting(hub: &Hub, run: &Run) -> Result<Vec<String>> {
    let rows = hub.store.group_tasks(&run.group, &run.lane.key())?;
    let mut out: Vec<String> = vec![];
    for owner in rows
        .iter()
        .filter(|row| row.kind == TaskKind::Assignment && row.state == TaskState::Queued)
        .filter_map(|row| row.owner_id.clone())
    {
        if !out.contains(&owner) {
            out.push(owner);
        }
    }
    Ok(out)
}

/// Plans a round's speakers in the tree: a queued row each, so Stop and the people in the channel see the whole
/// branch before any member speaks.
pub fn plan(hub: &Hub, run: &Run, speakers: &[(String, Option<String>)], round: usize) -> Result<()> {
    let rows = hub.store.group_tasks(&run.group, &run.lane.key())?;
    let round = i32::try_from(round).unwrap_or_default();
    for (member, parent) in speakers {
        let already = rows
            .iter()
            .any(|row| row.owner_id.as_deref() == Some(member.as_str()) && (row.state.is_open() || row.round == round));
        if already {
            continue;
        }
        let row = new_task(
            &run.group,
            &run.lane.key(),
            &run.request,
            &run.id,
            parent.as_deref(),
            Some(member),
            TaskKind::Assignment,
            TaskState::Queued,
            round,
            "",
        );
        hub.store.save_group_task(&row)?;
    }
    emit(hub, &run.group);
    Ok(())
}

/// Whether `member` may take its turn now, and the row that records it.
pub fn next(hub: &Hub, run: &Run, member: &str, parent: Option<&str>, round: usize) -> Result<Gate> {
    let rows = hub.store.group_tasks(&run.group, &run.lane.key())?;
    let Some(root) = rows.iter().find(|row| row.id == run.id) else { return Ok(Gate::Ended) };
    if root.state == TaskState::Cancelled {
        return Ok(Gate::Ended);
    }
    let live = rows.iter().find(|row| row.owner_id.as_deref() == Some(member) && row.state.is_open());
    if let Some(row) = live {
        return match row.state {
            TaskState::Queued | TaskState::Running => {
                if row.state != TaskState::Running {
                    set_state(hub, row, TaskState::Running, None);
                }
                // A row this member had queued elsewhere in the run (it was reassigned to it) gives way.
                for other in rows.iter().filter(|other| other.owner_id.as_deref() == Some(member) && other.id != row.id)
                {
                    if other.state == TaskState::Queued {
                        set_state(hub, other, TaskState::Completed, Some("Given to another row"));
                    }
                }
                Ok(Gate::Start(row.id.clone()))
            }
            // A failed row waits for the person to Continue or Reassign it.
            _ => Ok(Gate::Paused),
        };
    }
    if rows.iter().any(|row| {
        row.owner_id.as_deref() == Some(member)
            && row.state == TaskState::Completed
            && row.round == i32::try_from(round).unwrap_or_default()
    }) {
        return Ok(Gate::Done);
    }
    let paused = branch_paused(&rows, &run.id, parent);
    let row = new_task(
        &run.group,
        &run.lane.key(),
        &run.request,
        &run.id,
        parent,
        Some(member),
        TaskKind::Assignment,
        if paused { TaskState::Paused } else { TaskState::Running },
        i32::try_from(round).unwrap_or_default(),
        "",
    );
    hub.store.save_group_task(&row)?;
    emit(hub, &run.group);
    Ok(if paused { Gate::Paused } else { Gate::Start(row.id) })
}

/// The row that brought `member` in: the member whose message named it, this round. None at the request.
pub fn parent_for(
    hub: &Hub,
    run: &Run,
    members: &[BotConfig],
    since: &[Entry],
    member: &str,
) -> Result<Option<String>> {
    let rows = hub.store.group_tasks(&run.group, &run.lane.key())?;
    let Some(target) = members.iter().find(|m| m.id == member) else { return Ok(None) };
    for entry in since.iter().rev().filter(|e| e.kind == crate::store::EntryKind::Agent.as_str()) {
        let Some(author) = entry.data["author"].as_str().filter(|author| *author != member) else { continue };
        let text = entry.data["text"].as_str().unwrap_or_default();
        if super::names(text, target) || super::names_everyone(text) {
            let parent = rows.iter().rev().find(|row| row.owner_id.as_deref() == Some(author));
            return Ok(parent.map(|row| row.id.clone()));
        }
    }
    Ok(None)
}

/// Records how a member's turn came out; a row the person paused or cancelled meanwhile keeps its state.
pub fn finish(hub: &Hub, group: &str, task: &str, outcome: Outcome<'_>) -> Result<()> {
    let Some(row) = hub.store.group_task(task)? else { return Ok(()) };
    if row.state != TaskState::Running {
        return Ok(());
    }
    match outcome {
        Outcome::Spoke | Outcome::Passed => set_state(hub, &row, TaskState::Completed, None),
        Outcome::Failed(note) => set_state(hub, &row, TaskState::Failed, Some(note)),
    }
    Ok(())
}

/// The room turn of `run` ended: the request is done when nothing below it waits.
pub fn settled(hub: &Hub, run: &Run) -> Result<()> {
    let rows = hub.store.group_tasks(&run.group, &run.lane.key())?;
    let Some(root) = rows.iter().find(|row| row.id == run.id) else { return Ok(()) };
    if matches!(root.state, TaskState::Running | TaskState::Queued)
        && !rows.iter().any(|row| row.id != root.id && row.state.is_open())
    {
        set_state(hub, root, TaskState::Completed, None);
    }
    Ok(())
}

/// Stop in a channel: every open row of its trees pauses, so no member speaks until Continue.
pub fn pause_all(hub: &Hub, group: &str) {
    let Ok(rows) = hub.store.open_group_tasks(group) else { return };
    for row in rows.iter().filter(|row| row.state.is_open()) {
        set_state(hub, row, TaskState::Paused, Some("Stopped"));
    }
}

/// A stopped row of `group`, its branch paused, resumed, reassigned or ended (`TaskState`'s controls).
pub fn act(hub: &Arc<Hub>, group: &str, task: &str, action: &str, owner: Option<&str>) -> Result<Value> {
    let row = hub.store.group_task(task)?.filter(|row| row.group_id == group).context("unknown sub-task")?;
    let rows = hub.store.group_tasks(group, &row.lane)?;
    let branch: Vec<GroupTask> = by_id(&rows, &crate::store::task_branch(&rows, &row.id));
    match action {
        "stop" => {
            if row.state.is_open() {
                for row in &branch {
                    set_state(hub, row, TaskState::Paused, Some("Stopped"));
                }
                interrupt(hub, &branch, group);
            }
            if row.kind == TaskKind::Request {
                hub.groups.stop(group);
            }
        }
        "continue" => {
            resume_branch(hub, group, &row, &rows, owner)?;
        }
        "reassign" => {
            let owner = owner.filter(|owner| !owner.trim().is_empty()).context("ownerId is required")?;
            if row.owner_id.as_deref() == Some(owner) {
                bail!("that member already has this sub-task");
            }
            member(hub, group, owner)?;
            resume_branch(hub, group, &row, &rows, Some(owner))?;
        }
        "cancel" => {
            cancel(hub, group, &row, &rows)?;
        }
        _ => bail!("unknown sub-task action: {action}"),
    }
    hub.emit_bot(group);
    Ok(json!({"tasks": value(hub, group, &row.lane)}))
}

/// Drop: the lead cancels the channel's newest run for `by`, messages and all.
pub fn drop_run(hub: &Arc<Hub>, group: &BotConfig, by: &str) -> Result<usize> {
    let rows = hub.store.open_group_tasks(&group.id)?;
    let Some(root) = rows.iter().filter(|row| row.kind == TaskKind::Request).max_by_key(|row| row.created_at) else {
        bail!("this channel has no request to drop");
    };
    let lane = lane_of(&root.lane, &group.id);
    let before = rows.iter().filter(|row| row.state.is_open()).count();
    cancel(hub, &group.id, root, &rows)?;
    let text = format!("{by} asked to drop this request: {}", root.instruction);
    let data = json!({"text": text, "style": crate::agent::bot::NoticeStyle::Info});
    hub.add_entry(&lane, crate::store::EntryKind::Notice, hub.store.max_turn(&group.id), &data);
    hub.emit_bot(&group.id);
    Ok(before)
}

/// How many of the channel's sub-tasks still wait.
pub fn open_count(hub: &Hub, group: &str) -> usize {
    hub.store.open_group_tasks(group).map_or(0, |rows| rows.iter().filter(|row| row.state.is_open()).count())
}

/// A member left the channel: its open rows, and anything that waited on them, end.
pub fn member_left(hub: &Hub, group: &str, member: &str) {
    let Ok(rows) = hub.store.open_group_tasks(group) else { return };
    let ids: Vec<String> = rows
        .iter()
        .filter(|row| row.owner_id.as_deref() == Some(member) && row.state.is_open())
        .flat_map(|row| crate::store::task_branch(&rows, &row.id))
        .collect();
    for row in rows.iter().filter(|row| ids.contains(&row.id) && row.state.is_open()) {
        set_state(hub, row, TaskState::Cancelled, Some("Its member left the channel"));
    }
}

/// The tree a client shows for one lane.
pub fn value(hub: &Hub, group: &str, lane: &str) -> Vec<Value> {
    hub.store.group_tasks(group, lane).unwrap_or_default().iter().map(|row| row_value(hub, row)).collect()
}

/// A row as clients show it.
pub fn row_value(hub: &Hub, row: &GroupTask) -> Value {
    json!({
        "id": row.id, "groupId": row.group_id, "lane": row.lane, "requestId": row.request_id,
        "rootId": row.root_id, "parentId": row.parent_id, "ownerId": row.owner_id,
        "ownerName": row.owner_id.as_deref().map(|id| hub.bot_name(id)),
        "kind": row.kind, "state": row.state, "instruction": row.instruction, "note": row.note,
        "createdAt": row.created_at, "updatedAt": row.updated_at,
    })
}

/// Continue and Reassign: the row (and what waited below it) is queued again, and the room asks it.
fn resume_branch(hub: &Arc<Hub>, group: &str, row: &GroupTask, rows: &[GroupTask], owner: Option<&str>) -> Result<()> {
    if row.state != TaskState::Paused && row.state != TaskState::Failed {
        bail!("only a stopped sub-task can be continued or reassigned");
    }
    if super::group(hub, group)?.archived {
        bail!("this channel is archived: restore it before its sub-tasks go on");
    }
    let mut resumed = row.clone();
    if let Some(owner) = owner {
        resumed.owner_id = Some(owner.to_owned());
    }
    set_state(hub, &resumed, TaskState::Queued, None);
    let ids = crate::store::task_branch(rows, &row.id);
    for below in by_id(rows, &ids).into_iter().filter(|below| below.id != row.id && below.state == TaskState::Paused) {
        set_state(hub, &below, TaskState::Queued, None);
    }
    hub.emit_bot(group);
    super::resume(hub, group, &lane_of(&row.lane, group));
    Ok(())
}

/// Cancel and Drop: the request from its root down ends, and its member turns stop where they are.
fn cancel(hub: &Arc<Hub>, group: &str, row: &GroupTask, rows: &[GroupTask]) -> Result<()> {
    let ids = crate::store::task_branch(rows, &row.root_id);
    let branch = by_id(rows, &ids);
    for below in &branch {
        if below.state != TaskState::Cancelled {
            set_state(hub, below, TaskState::Cancelled, Some("Cancelled"));
        }
    }
    interrupt(hub, &branch, group);
    hub.groups.stop(group);
    Ok(())
}

/// Ends the turns running in `rows`' branches; the room hears a cancelled answer as a pass.
fn interrupt(hub: &Hub, rows: &[GroupTask], group: &str) {
    let owners: Vec<&str> =
        rows.iter().filter(|row| row.state == TaskState::Running).filter_map(|row| row.owner_id.as_deref()).collect();
    for owner in owners.into_iter().collect::<std::collections::HashSet<_>>() {
        let _ = hub.send_cmd(owner, Cmd::CancelGroup { chat: group.to_owned() });
    }
}

/// `row` and its ancestors up to the request: a stopped one stops everything below it.
fn branch_paused(rows: &[GroupTask], root: &str, parent: Option<&str>) -> bool {
    let by: HashMap<&str, &GroupTask> = rows.iter().map(|row| (row.id.as_str(), row)).collect();
    let mut at = parent;
    let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
    while let Some(id) = at {
        if !seen.insert(id) {
            break;
        }
        match by.get(id) {
            Some(row) => {
                if row.state == TaskState::Paused || row.state == TaskState::Failed {
                    return true;
                }
                if row.id == root {
                    break;
                }
                at = row.parent_id.as_deref();
            }
            None => break,
        }
    }
    by.get(root).is_some_and(|root| root.state == TaskState::Paused || root.state == TaskState::Failed)
}

fn by_id(rows: &[GroupTask], ids: &[String]) -> Vec<GroupTask> {
    ids.iter().filter_map(|id| rows.iter().find(|row| &row.id == id)).cloned().collect()
}

fn set_state(hub: &Hub, row: &GroupTask, state: TaskState, note: Option<&str>) {
    let mut row = row.clone();
    row.state = state;
    row.note = note.map(str::to_owned);
    row.updated_at = crate::store::now_ms();
    if let Err(error) = hub.store.save_group_task(&row) {
        tracing::warn!(group = row.group_id, error = format!("{error:#}"), "sub-task state not saved");
    }
    emit(hub, &row.group_id);
}

fn emit(hub: &Hub, group: &str) {
    hub.emit_bot(group);
}

/// A channel member by id.
fn member(hub: &Hub, group: &str, id: &str) -> Result<()> {
    let row = hub.store.bot(group)?.filter(|row| !row.deleted).context("unknown channel")?;
    if !row.config.members.iter().any(|member| member == id) {
        bail!("{id} isn't in this channel: take a member's id from its settings");
    }
    Ok(())
}

/// A row's lane key back to its lane.
fn lane_of(key: &str, group: &str) -> Lane {
    match key.strip_prefix(group).and_then(|rest| rest.strip_prefix('/')) {
        Some(thread) => Lane::in_thread(group, thread),
        None => Lane::main(group),
    }
}
