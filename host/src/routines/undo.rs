//! The receipt a routine change leaves in the chat, and taking it back.
//!
//! A bot that creates, edits or deletes a routine through its tools does something the person can undo: the notice
//! carries the routine as it was before (`routineUndo`), and `undoRoutineChange` puts it back. It is the same shape
//! memory's `memoryUndo` uses (agentes.chat's ADR 0037, round 2), and the apps draw both with one button.

use super::Routine;
use crate::LockExt;
use crate::hub::Hub;
use anyhow::{Result, anyhow, bail};
use serde_json::{Value, json};

/// Takes back the change a notice told about: a routine that was new goes, one that was changed comes back as it was.
pub fn undo(hub: &Hub, b: &Value) -> Result<Value> {
    let id = b["entryId"].as_str().ok_or_else(|| anyhow!("entryId required"))?;
    let entry = hub.store.entry(id).ok_or_else(|| anyhow!("unknown message"))?;
    let undo = entry.data["routineUndo"].clone();
    if !undo.is_object() {
        bail!("That message has nothing to undo.");
    }
    if undo["undone"] == true {
        return Ok(json!({"entry": entry}));
    }
    let routine_id = undo["id"].as_str().ok_or_else(|| anyhow!("bad undo"))?.to_owned();
    let bot = entry.bot_id.clone();
    match undo.get("before").filter(|before| !before.is_null()) {
        // It was new: taking it back takes it away.
        None => {
            if hub.routines.remove(hub, &bot, &routine_id).is_err() {
                // Already gone (someone deleted it meanwhile): the receipt still closes.
            }
        }
        Some(before) => {
            let routine: Routine = serde_json::from_value(before.clone()).map_err(|_| anyhow!("bad undo"))?;
            hub.routines.change(&hub.store, |s| {
                match s.routines.iter_mut().find(|r| r.id == routine.id) {
                    Some(slot) => *slot = routine.clone(),
                    None => s.routines.push(routine.clone()),
                }
                Ok(())
            })?;
        }
    }
    let mut data = entry.data.clone();
    data["routineUndo"]["undone"] = true.into();
    let entry = hub.set_entry(id, &data).ok_or_else(|| anyhow!("could not update the message"))?;
    Ok(json!({"entry": entry}))
}

/// The routine as the receipt stores it: the shape `list` shows, which `Routine` reads back.
pub fn snapshot(hub: &Hub, bot: &str, id: &str) -> Option<Value> {
    let s = hub.routines.state.locked();
    let routine = s.routines.iter().find(|r| r.id == id && r.bot_id == bot && !r.deleted)?;
    serde_json::to_value(routine).ok()
}
