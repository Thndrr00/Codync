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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::identity::Identity;
    use crate::store::{BotConfig, Store};
    use std::sync::Arc;

    fn fixture() -> (Arc<Hub>, String, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("codync-routine-undo-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("dir");
        let store = Store::open(&dir.join("test.db")).expect("store");
        let hub = Hub::new(store, "undo-test".into(), Identity::load_or_create(&dir).expect("identity"), "t".into(), 0);
        let id = uuid::Uuid::new_v4().to_string();
        let cfg: BotConfig =
            serde_json::from_value(json!({"id": id, "name": "Bot", "backend": "fixture", "command": "x", "cwd": dir}))
                .expect("config");
        hub.store.save_bot(&cfg).expect("bot");
        (hub, id, dir)
    }

    fn body(name: &str) -> Value {
        json!({"name": name, "instruction": "Check the status", "triggers": [{"type": "webhook"}]})
    }

    /// The receipt the chat now holds, as an entry.
    fn receipt(hub: &Hub, bot: &str, changed: bool) -> crate::store::Entry {
        hub.store
            .history(bot, i64::MAX, 50)
            .expect("history")
            .into_iter()
            .find(|e| e.data["routineUndo"].is_object() && e.data["routineUndo"]["before"].is_object() == changed)
            .expect("the chat says what happened")
    }

    fn live(hub: &Hub, bot: &str, id: &str) -> bool {
        hub.routines.list(bot, |_| false)["routines"]
            .as_array()
            .is_some_and(|routines| routines.iter().any(|r| r["id"] == id && r["deleted"] != true))
    }

    #[test]
    fn undoing_a_new_routine_takes_it_away() {
        let (hub, bot, dir) = fixture();
        let saved = hub.routines.save(&hub, &bot, &body("Check")).expect("saved");
        let id = saved["routine"]["id"].as_str().expect("id").to_owned();
        assert!(live(&hub, &bot, &id));
        let receipt = receipt(&hub, &bot, false);
        assert_eq!(receipt.data["routineUndo"]["before"], Value::Null, "a new routine has nothing before it");
        undo(&hub, &json!({"entryId": receipt.id})).expect("undone");
        assert!(!live(&hub, &bot, &id), "the routine is gone");
        assert_eq!(hub.store.entry(&receipt.id).expect("entry").data["routineUndo"]["undone"], true);
        // Undoing twice changes nothing and does not bring it back.
        undo(&hub, &json!({"entryId": receipt.id})).expect("twice is fine");
        assert!(!live(&hub, &bot, &id));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn undoing_a_change_puts_the_routine_back_as_it_was() {
        let (hub, bot, dir) = fixture();
        let saved = hub.routines.save(&hub, &bot, &body("Check")).expect("saved");
        let id = saved["routine"]["id"].as_str().expect("id").to_owned();
        let mut change = body("Renamed");
        change["id"] = id.clone().into();
        hub.routines.save(&hub, &bot, &change).expect("changed");
        let receipt = receipt(&hub, &bot, true);
        assert_eq!(receipt.data["routineUndo"]["before"]["name"], "Check", "the receipt keeps what was there");
        undo(&hub, &json!({"entryId": receipt.id})).expect("undone");
        let list = hub.routines.list(&bot, |_| false);
        let back = list["routines"]
            .as_array()
            .expect("list")
            .iter()
            .find(|r| r["id"] == id.as_str())
            .expect("the routine is back");
        assert_eq!(back["name"], "Check");
        assert_eq!(back["instruction"], "Check the status");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn undoing_a_delete_brings_the_routine_back() {
        let (hub, bot, dir) = fixture();
        let saved = hub.routines.save(&hub, &bot, &body("Check")).expect("saved");
        let id = saved["routine"]["id"].as_str().expect("id").to_owned();
        hub.routines.remove(&hub, &bot, &id).expect("removed");
        assert!(!live(&hub, &bot, &id));
        let receipt = receipt(&hub, &bot, true);
        undo(&hub, &json!({"entryId": receipt.id})).expect("undone");
        assert!(live(&hub, &bot, &id), "the routine is back");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
