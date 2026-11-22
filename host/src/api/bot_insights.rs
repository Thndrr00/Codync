//! `botInsights`: a read-only screen of one bot's routines, model and daily usage. It reuses the
//! pure helpers ported from bdots (`bdots_cron`, `bdots_desk`, `bdots_text`) and the host's own
//! budget tally. No client computes these values: they only render what the host returns.

use crate::hub::Hub;
use crate::routines::Trigger;
use anyhow::{Context, Result};
use serde_json::{Value, json};
use std::sync::Arc;

/// One routine's schedule as the insights screen shows it.
fn routine_view(r: &crate::routines::Routine, now: i64) -> Value {
    let schedules: Vec<String> = r
        .triggers
        .iter()
        .map(|trigger| match trigger {
            Trigger::Cron { expression, time_zone } => {
                crate::bdots_cron::parse_cron(expression)
                    .and_then(|cron| crate::bdots_cron::describe_cron(&cron))
                    .unwrap_or_else(|| trigger.description())
                    + &format!(" · {time_zone}")
            }
            other => other.description(),
        })
        .collect();
    let next = r.next_runs.iter().flatten().min().copied();
    let until = next.map(|at| (at - now).max(0));
    json!({
        "id": r.id,
        "name": r.name,
        "enabled": r.enabled,
        "schedules": schedules,
        "nextIn": until.map(crate::bdots_text::format_span),
        "nextInMs": until,
    })
}

pub(super) fn read(hub: &Arc<Hub>, bot: &str) -> Result<Value> {
    let row = hub.store.bot(bot)?.filter(|r| !r.deleted).context("Unknown bot")?;
    let now = crate::store::now_ms();
    let description = crate::bdots_extra::plain_text(&row.config.description);

    let model = row.config.model.as_deref().unwrap_or_default();
    let provider = crate::bdots_desk::provider_of(model).map(str::to_owned);

    let routines: Vec<Value> =
        hub.routines.canvas_routines(&hub.store, bot)?.iter().map(|r| routine_view(r, now)).collect();

    let usage = hub.budget.bot_usage(bot, now);
    let used = usage.get("used").and_then(Value::as_u64).unwrap_or(0);
    let limit = usage.get("limit").and_then(Value::as_u64);

    let skills: Vec<String> = row.config.skills.clone();
    let mut resources = serde_json::Map::new();
    for key in ["script", "reference", "asset", "file"] {
        resources.insert(key.into(), json!(0));
    }
    for skill in &skills {
        let kind = crate::bdots_desk::resource_kind(skill);
        let entry = resources.entry(kind).or_insert(json!(0));
        let n = entry.as_u64().unwrap_or(0) + 1;
        *entry = json!(n);
    }

    // What the bot actually did lately, as one plain line per step (bdots' `describeCall`).
    let recent: Vec<String> = hub
        .store
        .history(bot, i64::MAX, 12)
        .unwrap_or_default()
        .into_iter()
        .filter(|e| e.kind == crate::store::EntryKind::Tool.as_str())
        .rev()
        .take(5)
        .map(|e| {
            let name = e.data["title"].as_str().unwrap_or("tool");
            crate::bdots_more::describe_call(name, &e.data["rawInput"])
        })
        .collect();

    Ok(json!({
        "botId": bot,
        "description": description,
        "model": model,
        "provider": provider,
        "recentWork": recent,
        "routines": routines,
        "usage": {
            "used": used,
            "usedText": crate::bdots_desk::compact_count(used),
            "limit": limit,
            "resetsAt": usage.get("resetsAt").cloned(),
        },
        "skills": {
            "total": skills.len(),
            "resources": Value::Object(resources),
        },
    }))
}
