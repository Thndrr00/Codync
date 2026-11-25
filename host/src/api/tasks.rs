//! Agent Tasks dispatch. Reads are available to paired devices; the intents that drive the
//! computer's one agent (start, stop, send, branch and the writes that touch its transcript)
//! stay local. Names, text and message bodies never reach analytics: `observe_call` has no event
//! for these methods, so nothing about their content is recorded.

use super::devices::{Caller, Forbidden};
use super::str_arg;
use crate::chat::tasks;
use crate::hub::Hub;
use crate::store::TaskState;
use anyhow::{Context, Result, bail};
use serde_json::Value;
use std::sync::Arc;

pub(super) const METHODS: &[&str] = &[
    "taskCreate",
    "taskList",
    "taskRename",
    "taskStart",
    "taskStop",
    "taskClose",
    "taskReopen",
    "taskDelete",
    "taskMarkRead",
    "taskReadPage",
    "taskReadAround",
    "taskBranch",
    "taskDeleteMessage",
    "taskSend",
    "taskSplit",
    "taskFollowUp",
    "taskCollectResults",
    "taskAwaitEvent",
];

/// Stopping, sending and branching drive the shared agent; deleting removes its transcript lines.
const LOCAL: &[&str] = &[
    "taskStart",
    "taskStop",
    "taskSend",
    "taskBranch",
    "taskDeleteMessage",
    "taskDelete",
    "taskSplit",
    "taskFollowUp",
    "taskCollectResults",
    "taskAwaitEvent",
];

fn seq(b: &Value, key: &str) -> Result<Option<i64>> {
    b.get(key).map(|v| v.as_i64().with_context(|| format!("`{key}` must be an integer"))).transpose()
}

pub(super) async fn call(hub: &Arc<Hub>, caller: &Caller, method: &str, b: Value) -> Result<Value> {
    if LOCAL.contains(&method) && !matches!(caller, Caller::Local) {
        return Err(Forbidden.into());
    }
    Ok(match method {
        "taskCreate" => tasks::task_create(hub, str_arg(&b, "botId")?, str_arg(&b, "name")?)?,
        "taskList" => {
            let status = b["status"].as_str().map(TaskState::parse).transpose()?;
            tasks::task_list(hub, str_arg(&b, "botId")?, status)?
        }
        "taskRename" => tasks::task_rename(hub, str_arg(&b, "taskId")?, str_arg(&b, "name")?)?,
        "taskStart" => tasks::task_start(hub, str_arg(&b, "taskId")?)?,
        "taskStop" => tasks::task_stop(hub, str_arg(&b, "taskId")?)?,
        "taskClose" => {
            let archived = match b.get("archived") {
                None | Some(Value::Null) => false,
                Some(Value::Bool(value)) => *value,
                Some(_) => bail!("`archived` must be a boolean"),
            };
            tasks::task_close(hub, str_arg(&b, "taskId")?, archived)?
        }
        "taskReopen" => tasks::task_reopen(hub, str_arg(&b, "taskId")?)?,
        "taskDelete" => tasks::task_delete(hub, str_arg(&b, "taskId")?)?,
        "taskMarkRead" => tasks::task_mark_read(hub, str_arg(&b, "taskId")?, b["throughMessageId"].as_str())?,
        "taskReadPage" => {
            let limit = seq(&b, "limit")?;
            let before = seq(&b, "before")?;
            let after = seq(&b, "after")?;
            tasks::task_read_page(hub, str_arg(&b, "taskId")?, limit, before, after)?
        }
        "taskReadAround" => {
            let limit = seq(&b, "limit")?;
            tasks::task_read_around(hub, str_arg(&b, "taskId")?, str_arg(&b, "messageId")?, limit)?
        }
        "taskBranch" => {
            let source = b["taskId"].as_str().or_else(|| b["sourceTaskId"].as_str());
            let source = source.context("`taskId` is required")?;
            let request_id = b["requestId"].as_str();
            tasks::task_branch(
                hub,
                str_arg(&b, "botId")?,
                source,
                str_arg(&b, "name")?,
                seq(&b, "before")?,
                request_id,
            )?
        }
        "taskDeleteMessage" => tasks::task_delete_message(hub, str_arg(&b, "taskId")?, str_arg(&b, "messageId")?)?,
        "taskSend" => {
            let text = str_arg(&b, "text")?;
            let nonce = b["clientNonce"].as_str();
            let reply = b["replyToMessageId"].as_str();
            tasks::task_send(hub, str_arg(&b, "botId")?, str_arg(&b, "taskId")?, text, nonce, reply)?
        }
        "taskSplit" => {
            let specs = b["tasks"].as_array().context("`tasks` must be an array")?;
            tasks::task_split(hub, str_arg(&b, "botId")?, specs, b["parentChat"].as_str(), b["parentThread"].as_str())?
        }
        "taskFollowUp" => {
            tasks::task_follow_up(hub, str_arg(&b, "botId")?, str_arg(&b, "taskId")?, str_arg(&b, "text")?)?
        }
        "taskCollectResults" => tasks::task_collect_results(hub, str_arg(&b, "botId")?)?.into(),
        "taskAwaitEvent" => {
            let wait = tasks::parse_wait(b["runId"].as_str(), b["source"].as_str(), b["event"].as_str())?;
            let timeout = tasks::clamp_wait(b["timeoutSeconds"].as_u64());
            tasks::task_await_event(hub, str_arg(&b, "taskId")?, wait, timeout).await?
        }
        _ => bail!("unknown task method"),
    })
}
