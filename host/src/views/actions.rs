//! Named actions per view: what the agent may invoke, and what each one does through the host's own
//! API (never by writing storage itself).

use super::state::{self, Kind, ViewId};
use crate::hub::Hub;
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

/// A chat's actions: open one of its side threads.
pub const CHAT: [&str; 1] = ["open_thread"];
/// A routine canvas's actions: create a routine and show it, focus one already there, or focus the
/// lane of one agent working a shared card.
pub const ROUTINE: [&str; 3] = ["create_routine", "select_routine", "focus_lane"];

pub fn names(kind: Kind) -> &'static [&'static str] {
    match kind {
        Kind::Chat => &CHAT,
        Kind::Routine => &ROUTINE,
    }
}

pub fn invoke(hub: &Hub, view: &ViewId, action: &str, args: &Value) -> Result<Value> {
    if !names(view.kind).contains(&action) {
        bail!("view \"{}\" has no action \"{action}\"; it declares: {}", view.as_str(), names(view.kind).join(", "));
    }
    match (view.kind, action) {
        (Kind::Chat, "open_thread") => open_thread(hub, view, args),
        (Kind::Routine, "create_routine") => create_routine(hub, view, args),
        (Kind::Routine, "select_routine") => select_routine(hub, view, args),
        (Kind::Routine, "focus_lane") => focus_lane(hub, view, args),
        _ => bail!("view \"{}\" has no action \"{action}\"", view.as_str()),
    }
}

/// Opens a thread of the chat the person is reading, on their screen, by its root message.
fn open_thread(hub: &Hub, view: &ViewId, args: &Value) -> Result<Value> {
    let root = text(args, "rootId")?;
    let entry = hub.store.entry(&root).context("that message isn't in the chat")?;
    if entry.bot_id != view.bot {
        bail!("that message isn't in this chat");
    }
    if entry.thread_id.is_some() {
        bail!("that message is in a thread itself: open the thread's root instead");
    }
    answer(hub, view, "open_thread", args, json!({"thread": root}), json!({"rootId": root}))
}

/// Creates a routine through the same API the routine editor saves with, then shows it on the canvas.
fn create_routine(hub: &Hub, view: &ViewId, args: &Value) -> Result<Value> {
    let name = text(args, "name")?;
    let instruction = text(args, "instruction")?;
    let body = if args.get("schedule").is_some_and(Value::is_object) {
        let mut schedule = args["schedule"].clone();
        // A cron schedule with no zone runs on this computer's clock, like the editor's own default.
        if schedule["kind"] == "cron" && schedule["zone"].is_null() {
            schedule["zone"] = crate::routines::local_zone().into();
        }
        json!({"name": name, "instruction": instruction, "schedule": schedule})
    } else if args.get("triggers").is_some_and(Value::is_array) {
        json!({"name": name, "instruction": instruction, "triggers": args["triggers"]})
    } else {
        bail!(
            "give a `schedule` or `triggers`: for \"Monday at 8\" use \
             {{\"kind\":\"cron\",\"calendarStyle\":\"weekly\",\"weekday\":1,\"hour\":8,\"minute\":0,\"zone\":\"…\"}}"
        );
    };
    let saved = hub.routines.save(hub, &view.bot, &body)?;
    let routine = saved["routine"].clone();
    let id = routine["id"].as_str().context("the saved routine has an id")?.to_owned();
    let state = json!({"routineId": id, "routineName": routine["name"]});
    let announced = json!({"name": name, "routineId": id, "routineName": routine["name"]});
    answer(hub, view, "create_routine", &announced, state, json!({"routine": routine, "routineId": id}))
}

/// Focuses a routine that's already on the canvas (its own or one its flow passes through).
fn select_routine(hub: &Hub, view: &ViewId, args: &Value) -> Result<Value> {
    let id = text(args, "routineId")?;
    if crate::routines::flow::lane_bot_ids(hub, &view.bot, &id)?.is_none() {
        bail!("routine \"{id}\" isn't on this canvas: open the canvas or call list_routines");
    }
    let state = json!({"selected": {"kind": "routine", "id": id}});
    answer(hub, view, "select_routine", args, state, json!({"routineId": id}))
}

/// Focuses one agent's lane of a shared card: the person sees that agent's step, and both agents can
/// ask for their own lane without hiding the other.
fn focus_lane(hub: &Hub, view: &ViewId, args: &Value) -> Result<Value> {
    let routine_id = text(args, "routineId")?;
    let bot_id = text(args, "botId")?;
    let Some(lanes) = crate::routines::flow::lane_bot_ids(hub, &view.bot, &routine_id)? else {
        bail!("routine \"{routine_id}\" isn't on this canvas: open the canvas or call list_routines");
    };
    if !lanes.contains(&bot_id) {
        bail!("bot \"{bot_id}\" doesn't work routine \"{routine_id}\": its lanes are {}", lanes.join(", "));
    }
    let state = json!({"selected": {"kind": "lane", "routineId": routine_id, "botId": bot_id}});
    answer(hub, view, "focus_lane", args, state, json!({"routineId": routine_id, "botId": bot_id}))
}

/// Saves the view's new state (the agent wrote it, so it wins) and announces the action so every
/// client showing the view follows it.
fn answer(hub: &Hub, view: &ViewId, action: &str, args: &Value, patch: Value, result: Value) -> Result<Value> {
    let mut current = state::read(&hub.store, view)?.state;
    if let (Some(object), Some(additions)) = (current.as_object_mut(), patch.as_object()) {
        for (key, value) in additions {
            object.insert(key.clone(), value.clone());
        }
    }
    let saved = match state::write(hub, view, current, None, "agent")? {
        state::Write::Saved(saved) => saved,
        // Without a base rev a write is never in conflict.
        state::Write::Conflict(current) => current,
    };
    state::announce(hub, view, action, args, saved.rev)?;
    Ok(json!({
        "view": view.as_str(),
        "action": action,
        "result": result,
        "rev": saved.rev,
        "state": saved.state,
    }))
}

fn text(args: &Value, key: &str) -> Result<String> {
    let value = args[key].as_str().map(str::trim).unwrap_or_default();
    if value.is_empty() {
        bail!("`{key}` is required");
    }
    Ok(value.to_owned())
}
