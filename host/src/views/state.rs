//! The versioned state document of a view, and the JSON-patch-like deltas clients send.

use crate::hub::Hub;
use crate::store::{Store, now_ms};
use anyhow::{Result, bail};
use serde_json::{Map, Value, json};

/// kv prefix of every view's document.
pub const KEY_PREFIX: &str = "viewstate.v1.";
/// Largest state document, in bytes once serialized.
pub const MAX_STATE_BYTES: usize = 16 * 1024;
/// Deltas one patch may carry.
pub const MAX_OPS: usize = 32;
const MAX_PATH_SEGMENTS: usize = 4;
const MAX_SEGMENT: usize = 40;

/// What a view shows: a bot's chat, or its routine canvas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Chat,
    Routine,
}

/// One view, named as the wire names it (`chat:<botId>`, `routine:<botId>`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ViewId {
    pub kind: Kind,
    pub bot: String,
}

impl ViewId {
    pub fn parse(view: &str) -> Result<Self> {
        let (kind, bot) = view.split_once(':').ok_or_else(|| {
            anyhow::anyhow!("\"{view}\" isn't a view: they look like chat:<botId> or routine:<botId>")
        })?;
        let kind = match kind {
            "chat" => Kind::Chat,
            "routine" => Kind::Routine,
            other => bail!("\"{other}\" isn't a kind of view: they are chat and routine"),
        };
        if bot.is_empty() || bot.chars().count() > 200 || bot.chars().any(char::is_control) {
            bail!("\"{view}\" must name a bot id");
        }
        Ok(Self { kind, bot: bot.to_owned() })
    }

    pub fn as_str(&self) -> String {
        let kind = match self.kind {
            Kind::Chat => "chat",
            Kind::Routine => "routine",
        };
        format!("{kind}:{}", self.bot)
    }

    fn key(&self) -> String {
        format!("{KEY_PREFIX}{}", self.as_str())
    }
}

/// A view's state as saved: its version and the document the person's screen reflects.
#[derive(Clone, Debug, PartialEq)]
pub struct ViewState {
    pub rev: i64,
    pub state: Value,
}

/// What a patch did: saved at a new rev, or refused because the client's copy was stale.
pub enum Write {
    Saved(ViewState),
    Conflict(ViewState),
}

pub fn read(store: &Store, view: &ViewId) -> Result<ViewState> {
    let Some(raw) = store.kv_read(&view.key())? else {
        return Ok(ViewState { rev: 0, state: json!({}) });
    };
    let stored: Value = serde_json::from_str(&raw)?;
    Ok(ViewState { rev: stored["rev"].as_i64().unwrap_or(0), state: stored["state"].clone() })
}

/// Saves `state` as the view's next version and tells the apps. `base_rev` is the version the caller
/// last saw: when someone else wrote first, nothing is saved and the current version comes back.
pub fn write(hub: &Hub, view: &ViewId, state: Value, base_rev: Option<i64>, by: &'static str) -> Result<Write> {
    if !state.is_object() {
        bail!("a view's state must be an object");
    }
    let encoded = state.to_string();
    if encoded.len() > MAX_STATE_BYTES {
        bail!("a view's state may be at most {MAX_STATE_BYTES} bytes");
    }
    let current = read(&hub.store, view)?;
    if base_rev.is_some_and(|rev| rev != current.rev) {
        return Ok(Write::Conflict(current));
    }
    // The check and the save are not one transaction: two windows writing in the same instant resolve
    // last-wins, and the `viewState` event settles every client on the saved version.
    let saved = ViewState { rev: current.rev + 1, state };
    hub.emit_change(|| {
        let document = json!({"rev": saved.rev, "state": saved.state, "updatedAt": now_ms(), "by": by});
        hub.store.kv_set(&view.key(), &document.to_string())?;
        Ok(json!({
            "type": "viewState",
            "view": view.as_str(),
            "rev": saved.rev,
            "state": saved.state,
            "by": by,
        }))
    })?;
    Ok(Write::Saved(saved))
}

/// Tells the apps about a named action the agent invoked, after its effect and its state are saved.
pub fn announce(hub: &Hub, view: &ViewId, action: &str, arguments: &Value, rev: i64) -> Result<()> {
    hub.emit_change(|| {
        Ok(json!({
            "type": "viewAction",
            "view": view.as_str(),
            "action": action,
            "arguments": arguments,
            "rev": rev,
        }))
    })
}

/// Applies deltas (`set`, `remove`, `append`) to an object document, creating missing parents for
/// `set`. Paths are `/key` runs of at most four keys: no array indexes and no escapes to learn.
pub fn apply(state: &mut Value, ops: &Value) -> Result<()> {
    let Some(list) = ops.as_array() else { bail!("`ops` must be a list of changes") };
    if list.is_empty() || list.len() > MAX_OPS {
        bail!("`ops` must hold 1 to {MAX_OPS} changes");
    }
    if !state.is_object() {
        *state = json!({});
    }
    for (index, op) in list.iter().enumerate() {
        let at = format!("ops[{index}]");
        let Some(object) = op.as_object() else { bail!("`{at}` must be an object") };
        if let Some(key) = object.keys().find(|key| !["op", "path", "value"].contains(&key.as_str())) {
            bail!("`{at}` has an unknown prop \"{key}\"");
        }
        let kind = op["op"].as_str().ok_or_else(|| anyhow::anyhow!("`{at}.op` must be text"))?;
        let segments = path(&op["path"], &at)?;
        let (last, parents) = segments.split_last().ok_or_else(|| anyhow::anyhow!("`{at}.path` can't be empty"))?;
        match kind {
            "set" => {
                let Some(value) = object.get("value") else { bail!("`{at}.value` is required for set") };
                let parent =
                    descend(state, parents, true)?.ok_or_else(|| anyhow::anyhow!("`{at}.path` can't be set"))?;
                parent.insert(last.clone(), value.clone());
            }
            "remove" => {
                if let Some(parent) = descend(state, parents, false)? {
                    parent.remove(last);
                }
            }
            "append" => {
                let Some(value) = object.get("value") else { bail!("`{at}.value` is required for append") };
                let parent = descend(state, parents, false)?
                    .ok_or_else(|| anyhow::anyhow!("`{at}.path` must name a list to append to"))?;
                let Some(list) = parent.get_mut(last).and_then(Value::as_array_mut) else {
                    bail!("`{at}.path` must name a list to append to");
                };
                list.push(value.clone());
            }
            other => bail!("`{at}.op` must be set, remove or append (not \"{other}\")"),
        }
    }
    let encoded = state.to_string();
    if encoded.len() > MAX_STATE_BYTES {
        bail!("a view's state may be at most {MAX_STATE_BYTES} bytes");
    }
    Ok(())
}

/// The object at `segments`, creating missing ones when `create`; `None` when it isn't there and
/// `create` is false, or when a segment names something that isn't an object.
fn descend<'a>(root: &'a mut Value, segments: &[String], create: bool) -> Result<Option<&'a mut Map<String, Value>>> {
    let mut current = root;
    for segment in segments {
        if !current.is_object() {
            return Ok(None);
        }
        let object = current.as_object_mut().expect("checked above");
        if !object.contains_key(segment) {
            if !create {
                return Ok(None);
            }
            object.insert(segment.clone(), json!({}));
        }
        let Some(next) = object.get_mut(segment) else { return Ok(None) };
        if !next.is_object() {
            return Ok(None);
        }
        current = next;
    }
    Ok(Some(current.as_object_mut().expect("checked above")))
}

fn path(value: &Value, at: &str) -> Result<Vec<String>> {
    let Some(text) = value.as_str() else { bail!("`{at}.path` must be text") };
    let Some(rest) = text.strip_prefix('/') else {
        bail!("`{at}.path` must start with / (for example /selected)");
    };
    let segments: Vec<String> = rest.split('/').map(str::to_owned).collect();
    if segments.len() > MAX_PATH_SEGMENTS {
        bail!("`{at}.path` may name at most {MAX_PATH_SEGMENTS} keys");
    }
    for segment in &segments {
        if segment.is_empty()
            || segment.chars().count() > MAX_SEGMENT
            || segment.chars().any(char::is_control)
            || segment == "."
            || segment == ".."
        {
            bail!("`{at}.path` has an invalid key: use short keys of letters, digits, _ and -");
        }
    }
    Ok(segments)
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
