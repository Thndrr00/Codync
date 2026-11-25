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

/// The activity timeline: recent steps folded into labelled runs, with a summary and daily strip.
/// Uses only pure helpers, so every client renders the same host-computed values.
fn activity_view(hub: &Arc<Hub>, bot: &str, now: i64) -> Value {
    use crate::bdots_activity as a;

    let calls: Vec<a::Call> = hub
        .store
        .history(bot, i64::MAX, 48)
        .unwrap_or_default()
        .into_iter()
        .filter(|e| e.kind == crate::store::EntryKind::Tool.as_str())
        .filter(|e| now - e.created_at <= 86_400_000)
        .rev()
        .map(|e| {
            let name = e.data["title"].as_str().unwrap_or("tool").to_owned();
            let error = e.data["error"].as_str().map(a::strip_ansi);
            let exit = e.data["exitCode"].as_i64();
            a::Call {
                kind: crate::bdots_more::step_kind(&name),
                name,
                input: e.data["rawInput"].clone(),
                error,
                exit,
                at_ms: e.created_at,
            }
        })
        .collect();

    let groups: Vec<Value> = a::merge_adjacent(&a::group_calls(&calls))
        .into_iter()
        .map(|g| {
            json!({
                "kind": g.kind,
                "label": g.label,
                "icon": g.icon,
                "tone": g.tone,
                "count": g.count,
            })
        })
        .collect();

    let items: Vec<Value> = calls
        .iter()
        .take(12)
        .map(|c| {
            json!({
                "text": crate::bdots_more::describe_call(&c.name, &c.input),
                "detail": a::call_detail(&c.input),
                "status": a::call_status(c.error.as_deref(), c.exit),
                "icon": a::kind_icon(c.kind),
                "tone": a::kind_tone(c.kind),
            })
        })
        .collect();

    let days: Vec<Value> = a::daily_counts(&calls)
        .into_iter()
        .map(|(day, count)| {
            json!({
                "day": day,
                "label": a::day_label(&day, &a::bucket_day(now)),
                "count": count,
                "width": a::bar_width(count, calls.len(), 12),
            })
        })
        .collect();

    let counts = a::count_by_kind(&calls);
    json!({
        "headline": a::activity_headline(&calls),
        "steps": calls.len(),
        "errors": a::error_count(&calls),
        "dominant": a::dominant_kind(&calls),
        "span": a::window_span(&calls, now),
        "total": calls.len(),
        "counts": counts.iter().map(|(kind, n)| json!({"kind": kind, "count": n, "percent": a::percent(*n, calls.len())})).collect::<Vec<_>>(),
        "groups": groups,
        "items": items,
        "days": days,
    })
}

/// Configuration health: the same checks every client shows, computed here from host state.
fn health_view(row: &crate::store::BotRow, used: u64, limit: Option<u64>, routines: usize) -> Value {
    use crate::bdots_health as h;

    let config = &row.config;
    let secret = format!("bot_{}", bot_key(&row.id));
    let checks = h::build_checks(
        &config.name,
        &config.description,
        config.model.as_deref().unwrap_or(""),
        &config.cwd,
        &config.skills,
        routines,
        used,
        limit,
        &config.connectors,
        &secret,
        &bot_key(&row.id),
    );
    let score = h::health_score(&checks);
    json!({
        "score": score,
        "label": h::health_label(score),
        "tone": h::health_tone(score),
        "headline": h::health_headline(&checks),
        "blocking": h::has_blocking(&checks),
        "errors": h::error_count(&checks),
        "warnings": h::warn_count(&checks),
        "counts": h::count_by_severity(&checks).iter().map(|(s, n)| json!({"severity": s, "count": n})).collect::<Vec<_>>(),
        "checks": h::limit_checks(&checks, 10).iter().map(|c| json!({
            "id": c.id,
            "title": c.title,
            "detail": c.detail,
            "severity": c.severity,
            "tone": h::severity_tone(c.severity),
            "icon": h::severity_icon(c.severity),
        })).collect::<Vec<_>>(),
    })
}

/// Capability profile: what this bot can do with the configuration and tools it has.
fn profile_view(row: &crate::store::BotRow, routines: usize) -> Value {
    use crate::bdots_profile as p;

    let config = &row.config;
    let caps = p::build_caps(
        config.computer,
        &config.connectors,
        &config.skills,
        routines,
        &config.backend,
        config.model.as_deref(),
        config.fallbacks.len(),
        config.self_fallbacks,
        config.verify_command.as_deref(),
        config.background_model.as_deref(),
        config.saver,
        config.browser_profile.as_deref(),
        config.approval_rules.len(),
    );
    let coverage = p::coverage(&caps);
    json!({
        "headline": p::profile_headline(&caps),
        "enabled": p::enabled_count(&caps),
        "total": caps.len(),
        "missing": p::missing_count(&caps),
        "coverage": coverage,
        "bar": p::coverage_bar(&caps, 12),
        "nextUp": p::next_up(&p::sort_caps(&caps), 3).iter().map(|c| json!({"title": c.title, "icon": c.icon})).collect::<Vec<_>>(),
        "caps": p::sort_caps(&caps).iter().map(|c| json!({
            "id": c.id,
            "title": c.title,
            "detail": c.detail,
            "icon": c.icon,
            "tone": c.tone,
            "enabled": c.enabled,
            "stateIcon": p::state_icon(c.enabled),
            "stateTone": p::state_tone(c.enabled),
        })).collect::<Vec<_>>(),
    })
}

/// Weekly summary: seven day-buckets plus trends, built from the same history as `activity`.
fn week_view(hub: &Arc<Hub>, bot: &str, now: i64) -> Value {
    use crate::bdots_week as w;

    let entries = hub.store.history(bot, i64::MAX, 400).unwrap_or_default();
    let mut counts: Vec<(String, usize)> = Vec::new();
    for entry in entries {
        if entry.kind != crate::store::EntryKind::Tool.as_str() {
            continue;
        }
        if now - entry.created_at > 7 * 86_400_000 {
            continue;
        }
        let key = crate::bdots_activity::bucket_day(entry.created_at);
        if let Some(slot) = counts.iter_mut().find(|(k, _)| *k == key) {
            slot.1 += 1;
        } else {
            counts.push((key, 1));
        }
    }
    let refs: Vec<(&str, usize)> = counts.iter().map(|(k, n)| (k.as_str(), *n)).collect();
    let today = crate::bdots_activity::bucket_day(now);
    let days = w::build_days(&refs, &today, now);
    let trend = w::trend(&days);
    let verdict = w::week_verdict(&days);
    json!({
        "headline": w::week_headline(&days),
        "tone": w::week_tone(&days),
        "verdict": verdict,
        "total": w::total(&days),
        "totalLabel": w::total_label(&days),
        "average": w::daily_average(&days),
        "peak": w::peak_day(&days).map(|d| json!({"label": d.label, "count": d.count})).unwrap_or(Value::Null),
        "quietDays": w::quiet_days(&days),
        "activeStreak": w::active_streak(&days),
        "quietStreak": w::quiet_streak(&days),
        "aboveAverage": w::above_average_days(&days),
        "change": w::change_percent(&days),
        "perfectAttendance": w::perfect_attendance(&days),
        "grew": w::grew(&days),
        "trend": {
            "kind": format!("{trend:?}").to_ascii_lowercase(),
            "label": w::trend_label(trend),
            "icon": w::trend_icon(trend),
            "tone": w::trend_tone(trend),
        },
        "top": w::top_days(&days, 12).into_iter().map(|(label, count, bar)| json!({"label": label, "count": count, "bar": bar})).collect::<Vec<_>>(),
        "quiet": w::bottom_days(&days, 12).into_iter().map(|(label, count, bar)| json!({"label": label, "count": count, "bar": bar})).collect::<Vec<_>>(),
        "days": days.iter().map(|d| json!({"key": d.key, "label": d.label, "count": d.count})).collect::<Vec<_>>(),
    })
}

/// Routine summary: reuse the already-computed `routines` array, no second calculation.
fn routine_summary_view(routines: &[Value], now: i64) -> Value {
    use crate::bdots_routines as r;

    let lines: Vec<r::RoutineLine> = routines
        .iter()
        .map(|v| {
            let schedules: Vec<String> = v["schedules"]
                .as_array()
                .map(|a| a.iter().filter_map(Value::as_str).map(str::to_owned).collect())
                .unwrap_or_default();
            r::line(
                v["id"].as_str().unwrap_or(""),
                v["name"].as_str().unwrap_or(""),
                v["enabled"] == true,
                &schedules,
                v["nextInMs"].as_i64(),
                now,
            )
        })
        .collect();

    let soonest: Vec<Value> = r::soonest_first(&lines, 6)
        .into_iter()
        .map(|l| {
            json!({
                "id": l.id,
                "name": l.name,
                "enabled": l.enabled,
                "trigger": l.trigger,
                "triggerIcon": r::trigger_icon(&l.trigger),
                "nextIn": l.next_in,
                "stateIcon": r::state_icon(l.enabled),
                "stateTone": r::state_tone(l.enabled),
            })
        })
        .collect();
    let paused: Vec<Value> = r::paused(&lines).iter().map(|l| json!({"id": l.id, "name": l.name})).collect();

    json!({
        "count": r::count_label(&lines),
        "active": r::active_count(&lines),
        "paused": r::paused_count(&lines),
        "scheduled": r::scheduled_count(&lines),
        "activePercent": r::active_percent(&lines),
        "bar": r::active_bar(&lines, 12),
        "next": r::next_label(&lines),
        "mix": r::trigger_mix(&lines),
        "clock": r::has_clock(&lines),
        "event": r::has_event(&lines),
        "allScheduled": r::all_scheduled(&lines),
        "headline": r::schedule_headline(&lines),
        "tone": r::schedule_tone(&lines),
        "summary": r::summary(&lines).into_iter().map(|(k, v)| json!({"key": k, "value": v})).collect::<Vec<_>>(),
        "soonest": soonest,
        "pausedList": paused,
    })
}

/// Skill library summary: reuse the bot's configured skills, no second source of truth.
fn library_view(row: &crate::store::BotRow) -> Value {
    use crate::bdots_library as l;

    let skills = &row.config.skills;
    let lines: Vec<Value> = l::sorted(skills)
        .into_iter()
        .map(|s| {
            let line = l::skill_line(s);
            json!({
                "name": line.name,
                "kind": line.kind,
                "icon": line.icon,
                "tone": line.tone,
                "detail": line.detail,
            })
        })
        .collect();
    let kinds: Vec<Value> = l::count_kinds(skills)
        .into_iter()
        .map(|(kind, count)| {
            json!({
                "kind": kind,
                "count": count,
                "icon": l::kind_icon(kind),
                "tone": l::kind_tone(kind),
                "percent": l::kind_percent(count, skills.len()),
                "bar": l::kind_bar(count, skills.len(), 8),
            })
        })
        .collect();

    json!({
        "headline": l::library_headline(skills),
        "tone": l::library_tone(skills),
        "count": l::count_label(skills),
        "total": l::total_skills(skills),
        "distinct": l::distinct_skills(skills),
        "duplicates": l::duplicates(skills),
        "hasScript": l::has_script(skills),
        "hasReference": l::has_reference(skills),
        "hasAsset": l::has_asset(skills),
        "summary": l::summary(skills).into_iter().map(|(k, v)| json!({"key": k, "value": v})).collect::<Vec<_>>(),
        "kinds": kinds,
        "skills": lines,
    })
}

/// Trust posture: the bot's permission mode and approval rules, read-only.
fn trust_view(row: &crate::store::BotRow) -> Value {
    use crate::agent::approval_rules::Decision;
    use crate::bdots_trust as t;
    use crate::store::Permission;

    let mode = match row.config.permission {
        Permission::Ask => "ask",
        Permission::Auto => "auto",
    };
    let lines: Vec<t::RuleLine> = row
        .config
        .approval_rules
        .iter()
        .map(|r| {
            let decision = match r.decision {
                Decision::Allow => "allow",
                Decision::Ask => "ask",
                Decision::Deny => "deny",
            };
            t::rule_line(&r.pattern, r.kind.as_deref().unwrap_or(""), decision)
        })
        .collect();
    let score = t::trust_score(mode, &lines);
    let decisions = lines.iter().map(|r| r.decision).collect::<Vec<_>>();
    let patterns = lines.iter().map(|r| r.pattern.as_str()).collect::<Vec<_>>();

    json!({
        "mode": mode,
        "modeLabel": t::permission_label(mode),
        "modeIcon": t::permission_icon(mode),
        "modeTone": t::permission_tone(mode),
        "modeDetail": t::permission_detail(mode),
        "score": score,
        "tone": t::trust_tone(score),
        "label": t::trust_label(score),
        "headline": t::trust_headline(mode, &lines),
        "total": t::total_rules(&lines),
        "countLabel": t::rule_count_label(&lines),
        "broad": t::broad_count(&patterns),
        "command": t::command_count(&patterns),
        "hasDeny": t::has_deny(&lines),
        "hasBroadAllow": t::has_broad_allow(&lines),
        "asksForEverything": t::asks_for_everything(mode, &lines),
        "decisions": t::count_decisions(&decisions).into_iter().map(|(d, n)| json!({"decision": d, "count": n})).collect::<Vec<_>>(),
        "summary": t::summary(mode, &lines).into_iter().map(|(k, v)| json!({"key": k, "value": v})).collect::<Vec<_>>(),
        "rules": t::strictest(&lines, 8).into_iter().map(|r| json!({
            "pattern": r.pattern,
            "kind": r.kind,
            "decision": r.decision,
            "icon": r.icon,
            "tone": r.tone,
            "broad": r.broad,
        })).collect::<Vec<_>>(),
    })
}

/// Resilience: the fallback chain and the switches that keep the bot working.
fn resilience_view(row: &crate::store::BotRow) -> Value {
    use crate::bdots_fallbacks as f;

    let config = &row.config;
    let lines: Vec<f::FallbackLine> = config
        .fallbacks
        .iter()
        .enumerate()
        .map(|(i, fb)| f::fallback_line(i, &fb.backend, fb.model.as_deref(), fb.command.as_deref()))
        .collect();
    let score = f::resilience_score(
        &lines,
        config.self_fallbacks,
        config.fast_mode,
        config.saver,
        config.background_model.as_deref(),
    );

    json!({
        "headline": f::resilience_headline(&lines, config.self_fallbacks),
        "label": f::resilience_label(score),
        "tone": f::resilience_tone(score),
        "score": score,
        "bar": f::resilience_bar(score, 12),
        "chain": f::count_label(&lines),
        "total": f::total_options(&lines, config.self_fallbacks),
        "hasChain": f::has_chain(&lines),
        "hasOverride": f::has_override(&lines),
        "customCommands": f::custom_command_count(&lines),
        "selfFallbacks": config.self_fallbacks,
        "switches": f::switches(config.fast_mode, config.saver, config.background_model.as_deref())
            .into_iter().map(|(label, on)| json!({"label": label, "on": on})).collect::<Vec<_>>(),
        "summary": f::summary(&lines, config.self_fallbacks).into_iter().map(|(k, v)| json!({"key": k, "value": v})).collect::<Vec<_>>(),
        "fallbacks": f::ordered(&lines).iter().map(|l| json!({
            "backend": l.backend,
            "model": l.model,
            "command": l.command,
            "label": l.label,
            "icon": l.icon,
            "tone": l.tone,
        })).collect::<Vec<_>>(),
    })
}

/// Roster identity: how this bot appears to the person, read-only.
fn identity_view(row: &crate::store::BotRow) -> Value {
    use crate::bdots_identity as i;

    let c = &row.config;
    let traits = i::build_traits(
        &c.avatar_color,
        &c.avatar_shape,
        c.avatar_look.as_deref(),
        c.avatar_photo.as_deref(),
        c.pinned,
        c.lead,
        c.hidden,
        c.notify,
        c.section.as_deref(),
        c.mode.as_deref(),
        c.catalog.as_deref(),
    );
    let band_name = i::band(c.lead, c.pinned);
    let section = i::section_label(c.section.as_deref());
    let score_traits = i::enabled_count(&traits) as u32;
    let total_traits = traits.len() as u32;
    let score = crate::bdots_activity::percent(score_traits as usize, total_traits as usize);

    json!({
        "name": c.name,
        "headline": i::identity_headline(&c.name, band_name, &section),
        "tone": i::identity_tone(&c.name, c.hidden),
        "band": band_name,
        "bandIcon": i::band_icon(band_name),
        "bandTone": i::band_tone(band_name),
        "section": section,
        "mode": i::mode_label(c.mode.as_deref()),
        "rank": i::rank(c.position),
        "color": i::color_label(&c.avatar_color),
        "colorIcon": i::color_icon(&c.avatar_color),
        "shape": i::shape_label(&c.avatar_shape),
        "look": i::look_label(c.avatar_look.as_deref()),
        "visible": i::is_visible(c.hidden),
        "score": score,
        "count": i::count_label(&traits),
        "summary": i::summary(&traits).into_iter().map(|(k, v)| json!({"key": k, "value": v})).collect::<Vec<_>>(),
        "traits": traits.iter().map(|t| json!({
            "id": t.id,
            "label": t.label,
            "detail": t.detail,
            "icon": t.icon,
            "tone": t.tone,
            "on": t.on,
            "stateIcon": i::state_icon(t.on),
            "stateTone": i::state_tone(t.on),
        })).collect::<Vec<_>>(),
    })
}

/// Connected accounts: classify the bot's connectors into a readable mix.
fn connectors_view(row: &crate::store::BotRow) -> Value {
    use crate::bdots_connectors as c;

    let names = &row.config.connectors;
    let cats: Vec<Value> = c::count_categories(names)
        .into_iter()
        .map(|(cat, count)| {
            json!({
                "kind": cat,
                "count": count,
                "icon": c::category_icon(cat),
                "tone": c::category_tone(cat),
                "percent": c::category_percent(count, names.len()),
                "bar": c::category_bar(count, names.len(), 8),
            })
        })
        .collect();

    json!({
        "headline": c::connector_headline(names),
        "tone": c::connector_tone(names),
        "total": names.len(),
        "distinct": c::distinct(names),
        "duplicates": c::duplicates(names),
        "chat": c::can_chat(names),
        "mail": c::can_mail(names),
        "code": c::can_code(names),
        "money": c::can_money(names),
        "summary": c::summary(names).into_iter().map(|(k, v)| json!({"key": k, "value": v})).collect::<Vec<_>>(),
        "categories": cats,
        "connectors": c::sorted(names).iter().map(|x| json!({
            "name": x.name,
            "category": x.category,
            "icon": x.icon,
            "tone": x.tone,
            "detail": x.detail,
        })).collect::<Vec<_>>(),
    })
}

/// Model tuning: the ACP config options and the fast-mode switch.
fn tuning_view(row: &crate::store::BotRow) -> Value {
    use crate::bdots_tuning as t;

    let lines: Vec<t::OptionLine> = row.config.model_options.iter().map(|(k, v)| t::option_line(k, v)).collect();
    let fast = row.config.fast_mode;

    json!({
        "headline": t::tuning_headline(&lines, fast),
        "tone": t::tuning_tone(&lines, fast),
        "count": t::count_label(&lines),
        "total": lines.len(),
        "on": t::on_count(&lines),
        "off": t::off_count(&lines),
        "fast": fast,
        "effort": t::effort_label(&lines),
        "effortIcon": t::effort_icon(&lines),
        "reasoning": t::reasoning_on(&lines),
        "summary": t::summary(&lines, fast).into_iter().map(|(k, v)| json!({"key": k, "value": v})).collect::<Vec<_>>(),
        "options": t::sorted(&lines).iter().map(|l| json!({
            "key": l.key,
            "title": l.title,
            "value": l.value,
            "icon": l.icon,
            "tone": l.tone,
        })).collect::<Vec<_>>(),
        "enabled": t::enabled_first(&lines, 6).iter().map(|l| json!({"title": l.title, "value": l.value, "icon": l.icon})).collect::<Vec<_>>(),
    })
}

/// Memory posture: the shared/harness switches and any memory skills.
fn memory_view(row: &crate::store::BotRow) -> Value {
    use crate::bdots_memory as m;

    let c = &row.config;
    let shared = c.shared_memory;
    let harness = c.harness_memory;
    let skills = &c.skills;
    let lines = m::build_lines(shared, harness, skills);
    let score = m::memory_score(shared, harness, skills);

    json!({
        "headline": m::memory_headline(shared, harness, skills),
        "label": m::memory_label(score),
        "tone": m::memory_tone(score),
        "score": score,
        "shared": m::shared_label(shared),
        "sharedIcon": m::shared_icon(shared),
        "sharedTone": m::shared_tone(shared),
        "sharedDetail": m::shared_detail(shared),
        "harness": harness,
        "harnessIcon": m::harness_icon(harness),
        "harnessTone": m::harness_tone(harness),
        "skills": m::memory_skill_count(skills),
        "on": m::on_count(shared, harness, skills),
        "hasAny": m::has_any(shared, harness, skills),
        "summary": m::summary(shared, harness, skills).into_iter().map(|(k, v)| json!({"key": k, "value": v})).collect::<Vec<_>>(),
        "lines": lines.iter().map(|l| json!({
            "id": l.id,
            "title": l.title,
            "detail": l.detail,
            "icon": l.icon,
            "tone": l.tone,
            "on": l.on,
            "stateIcon": m::state_icon(l.on),
            "stateTone": m::state_tone(l.on),
        })).collect::<Vec<_>>(),
    })
}

fn bot_key(id: &str) -> String {
    id.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' }).collect()
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
        "activity": activity_view(hub, bot, now),
        "health": health_view(&row, used, limit, routines.len()),
        "profile": profile_view(&row, routines.len()),
        "week": week_view(hub, bot, now),
        "routineSummary": routine_summary_view(&routines, now),
        "library": library_view(&row),
        "trust": trust_view(&row),
        "resilience": resilience_view(&row),
        "identity": identity_view(&row),
        "connectors": connectors_view(&row),
        "tuning": tuning_view(&row),
        "memory": memory_view(&row),
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
