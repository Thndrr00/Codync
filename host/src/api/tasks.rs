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
];

/// Stopping, sending and branching drive the shared agent; deleting removes its transcript lines.
const LOCAL: &[&str] = &["taskStart", "taskStop", "taskSend", "taskBranch", "taskDeleteMessage", "taskDelete"];

fn seq(b: &Value, key: &str) -> Result<Option<i64>> {
    b.get(key).map(|v| v.as_i64().with_context(|| format!("`{key}` must be an integer"))).transpose()
}

pub(super) fn call(hub: &Arc<Hub>, caller: &Caller, method: &str, b: Value) -> Result<Value> {
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
            tasks::task_branch(hub, str_arg(&b, "botId")?, source, str_arg(&b, "name")?, seq(&b, "before")?)?
        }
        "taskDeleteMessage" => tasks::task_delete_message(hub, str_arg(&b, "taskId")?, str_arg(&b, "messageId")?)?,
        "taskSend" => {
            let text = str_arg(&b, "text")?;
            let nonce = b["clientNonce"].as_str();
            let reply = b["replyToMessageId"].as_str();
            tasks::task_send(hub, str_arg(&b, "botId")?, str_arg(&b, "taskId")?, text, nonce, reply)?
        }
        _ => bail!("unknown task method"),
    })
}
