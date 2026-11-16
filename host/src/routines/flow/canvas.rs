//! One bot's canvas as clients load it: its routines and those whose flow passes through it, the
//! links, where the nodes sit and the bots placed before any link reaches them.
//!
//! A card is one routine, and every bot its flow reaches works it in its own lane (`lanes`): two
//! agents on the same card stay distinguishable on every client, and the lanes come from the saved
//! links and steps, so a reload shows the same ones.

use super::graph;
use crate::LockExt;
use crate::hub::{Hub, flows::flow_event};
use crate::routines::{Routine, Routines, Run, public};
use crate::store::{FlowLink, Store, now_ms};
use anyhow::Result;
use serde_json::{Value, json};
use std::collections::HashSet;

/// Runs shown per routine, newest first.
const RECENT_RUNS: usize = 10;
const UPCOMING_WINDOW_MS: i64 = 7 * 24 * 60 * 60 * 1000;
/// A routine every few seconds fires far more often than a canvas can show.
const UPCOMING_LIMIT: usize = 400;

impl Routines {
    fn flow_routine(&self, store: &Store, r: &Routine, now: i64, links: &[FlowLink]) -> Result<Value> {
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
        let lanes = lanes(store, r, links, &steps)?;
        Ok(json!({
            "routine": public(r),
            "recentRuns": runs,
            "upcomingRuns": if r.enabled { upcoming(r, now) } else { vec![] },
            "steps": steps,
            "lanes": lanes,
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
    let links = store.flow_links_for(&ids)?;
    let cards = routines
        .iter()
        .map(|r| {
            let own: Vec<FlowLink> = links.iter().filter(|link| link.routine_id == r.id).cloned().collect();
            hub.routines.flow_routine(store, r, now, &own)
        })
        .collect::<Result<Vec<_>>>()?;
    let rev = store.flow_revs_since(-1).into_iter().find(|(b, _)| b == bot).map_or(0, |(_, rev)| rev);
    Ok(json!({
        "botId": bot,
        "rev": rev,
        "routines": cards,
        "links": links,
        "positions": store.flow_positions(bot)?,
        "placedBotIds": store.flow_placed_bots(bot)?,
    }))
}

/// One lane per bot the routine's flow reaches, the routine's own bot first: who works this card,
/// how deep in the flow it is and how its step of the latest run went.
fn lanes(store: &Store, r: &Routine, links: &[FlowLink], steps: &[crate::store::FlowStep]) -> Result<Vec<Value>> {
    let depths = match graph::depths(&r.bot_id, links) {
        Ok(depths) => depths,
        // Stored links are validated when they are made; a bad row still leaves the owner's lane.
        Err(_) => std::collections::BTreeMap::from([(r.bot_id.clone(), 0)]),
    };
    let mut ordered: Vec<(String, usize)> = depths.into_iter().collect();
    ordered.sort_by(|(a, da), (b, db)| da.cmp(db).then_with(|| a.cmp(b)));
    ordered
        .into_iter()
        .map(|(bot_id, depth)| {
            let name = store.bot(&bot_id)?.map(|row| row.config.name).unwrap_or_else(|| bot_id.clone());
            let step = steps.iter().find(|step| step.bot_id == bot_id);
            Ok(json!({
                "botId": bot_id,
                "name": name,
                "depth": depth,
                "status": step.map(|step| step.status),
            }))
        })
        .collect()
}

/// The bots that work one card, in lane order, when the card is on `canvas_bot`'s canvas.
pub fn lane_bot_ids(hub: &Hub, canvas_bot: &str, routine_id: &str) -> Result<Option<Vec<String>>> {
    let routines = hub.routines.canvas_routines(&hub.store, canvas_bot)?;
    let Some(routine) = routines.iter().find(|r| r.id == routine_id) else { return Ok(None) };
    let links = hub.store.flow_links(routine_id)?;
    let depths = graph::depths(&routine.bot_id, &links)
        .unwrap_or_else(|_| std::collections::BTreeMap::from([(routine.bot_id.clone(), 0)]));
    let mut ordered: Vec<(String, usize)> = depths.into_iter().collect();
    ordered.sort_by(|(a, da), (b, db)| da.cmp(db).then_with(|| a.cmp(b)));
    Ok(Some(ordered.into_iter().map(|(bot_id, _)| bot_id).collect()))
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
