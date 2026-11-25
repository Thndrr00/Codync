//! One bot's canvas as clients load it: its routines and those whose flow passes through it, the
//! links, where the nodes sit and the bots placed before any link reaches them.

use super::{document, folders, format, groups, output_template, pause, trigger_fields};
use crate::LockExt;
use crate::hub::{Hub, flows::flow_event};
use crate::routines::{Routine, Routines, Run, public};
use crate::store::{Store, flow_steps, now_ms};
use anyhow::Result;
use serde_json::{Value, json};
use std::collections::HashSet;

/// Runs shown per routine, newest first.
const RECENT_RUNS: usize = 10;
const UPCOMING_WINDOW_MS: i64 = 7 * 24 * 60 * 60 * 1000;
/// A routine every few seconds fires far more often than a canvas can show.
const UPCOMING_LIMIT: usize = 400;

impl Routines {
    fn flow_routine(&self, hub: &Hub, store: &Store, r: &Routine, now: i64) -> Result<Value> {
        let runs: Vec<Run> = self
            .state
            .locked()
            .runs
            .iter()
            .rev()
            .filter(|run| run.routine_id == r.id)
            .take(RECENT_RUNS)
            .cloned()
            .collect();
        let steps = store.flow_last_steps(&r.id)?;
        let flow_runs = store.flow_runs()?.into_iter().filter(|run| run.routine_id == r.id).collect::<Vec<_>>();
        // What each step actually ran with, keyed by step id. Older runs have no settings, so a
        // client can show "no saved settings" rather than treating the field as missing.
        let mut step_settings = serde_json::Map::new();
        for step in &steps {
            if let Some(settings) = store.flow_step_settings(&step.id)? {
                step_settings.insert(step.id.clone(), settings);
            }
        }
        Ok(json!({
            "routine": public(r),
            "paused": pause::is_paused(store, &r.id),
            "upcomingRuns": if r.enabled && !pause::is_paused(store, &r.id) { upcoming(r, now) } else { vec![] },
            "recentRuns": runs,
            "runNumbers": flow_steps::numbered_runs(&flow_runs),
            "steps": steps,
            "stepSettings": step_settings,
            "output": output_template::load(store, &r.id),
            // Fixed fields this routine's triggers give to later steps, for the client's `{{` picker.
            "triggerFields": trigger_fields::fixed_field_names(&r.triggers)
                .into_iter()
                .map(|(name, ty)| json!({"name": name, "step": "", "icon": "bolt", "type": ty, "values": []}))
                .collect::<Vec<_>>(),
            // Bounded histories of the event triggers; no event bodies or message text are sent.
            "githubDeliveries": self.github_history(&r.bot_id, &r.id)["deliveries"],
            "chatDeliveries": self.chat_history(&r.bot_id, &r.id)["deliveries"],
            // The webhook trigger node's URL. Its secret is never projected: rotation returns it once.
            "webhook": self.webhook_status(hub, &r.bot_id, &r.id),
        }))
    }

    /// The routines a bot's canvas shows: its own, then those whose links name it.
    pub(super) fn canvas_routines(&self, store: &Store, bot: &str) -> Result<Vec<Routine>> {
        let touching = store.flow_routines_touching(bot)?;
        let state = self.state.locked();
        let own = state.routines.iter().filter(|r| r.bot_id == bot && !r.deleted);
        let own_ids: HashSet<&str> = own.clone().map(|r| r.id.as_str()).collect();
        let others = touching
            .iter()
            .filter(|id| !own_ids.contains(id.as_str()))
            .filter_map(|id| state.routines.iter().find(|r| r.id == *id && !r.deleted));
        Ok(own.chain(others).cloned().collect())
    }
}

pub(super) fn canvas(hub: &Hub, bot: &str) -> Result<Value> {
    let (store, now) = (&hub.store, now_ms());
    let routines = hub.routines.canvas_routines(store, bot)?;
    let ids: Vec<String> = routines.iter().map(|r| r.id.clone()).collect();
    let cards = routines.iter().map(|r| hub.routines.flow_routine(hub, store, r, now)).collect::<Result<Vec<_>>>()?;
    let rev = store.flow_revs_since(-1).into_iter().find(|(b, _)| b == bot).map_or(0, |(_, rev)| rev);
    let mut canvas = json!({
        "botId": bot,
        "rev": rev,
        "routines": cards,
        "links": store.flow_links_for(&ids)?,
        "positions": store.flow_positions(bot)?,
        "placedBotIds": store.flow_placed_bots(bot)?,
        "mediaNodes": store.flow_media_for(&ids)?,
        "groups": groups::load(store, bot),
        "formatVersion": format::version(store, bot),
    });
    // The saved document carries drawing state the runnable tables do not: node sizes and locks,
    // freehand strokes and kinds this host may not know. An unknown kind is shown as a locked note
    // and flagged, so the rest of the canvas stays usable instead of the whole workflow failing.
    let stored = document::history(store, bot).last().map(|snapshot| snapshot.document.clone()).unwrap_or_default();
    let (nodes, needs_newer) = format::project_unknown(&stored.nodes);
    if let Some(object) = canvas.as_object_mut() {
        object.insert("nodeSizes".to_owned(), serde_json::to_value(document::node_sizes(&stored))?);
        object.insert("lockedNodes".to_owned(), serde_json::to_value(document::locked_nodes(&stored))?);
        object.insert("strokes".to_owned(), serde_json::to_value(document::strokes(&stored))?);
        object.insert("placeholderNodes".to_owned(), serde_json::to_value(nodes)?);
        if needs_newer {
            object.insert("needsNewerApp".to_owned(), json!(true));
        }
    }
    // An older canvas has no folder blob at all, so the field is omitted rather than sent empty.
    if let Some(snapshot) = folders::load(store) {
        canvas["folders"] = serde_json::to_value(folders::pruned(&snapshot, &super::api::known_workflows(hub)))?;
    }
    Ok(canvas)
}

/// The times a routine fires in the next week, soonest first. Watches and webhooks have none.
fn upcoming(r: &Routine, from: i64) -> Vec<i64> {
    let end = from.saturating_add(UPCOMING_WINDOW_MS);
    let mut times = vec![];
    for trigger in &r.triggers {
        let mut at = from;
        while times.len() < UPCOMING_LIMIT {
            match trigger.next(at) {
                Ok(Some(next))
                    if next < end && next > at && !matches!(trigger, crate::routines::Trigger::Watch { .. }) =>
                {
                    times.push(next);
                    at = next;
                }
                _ => break,
            }
        }
    }
    times.sort_unstable();
    times.dedup();
    times.truncate(UPCOMING_LIMIT);
    times
}

/// The `routineFlow` events a client's catch-up holds: every canvas changed after `since`.
pub fn catch_up(store: &Store, since: i64) -> Vec<Value> {
    store.flow_revs_since(since).into_iter().map(|(bot, rev)| flow_event(rev, &[bot])).collect()
}
