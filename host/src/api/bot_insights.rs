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

/// Workspace: the folder, agent command, verification command and browser profile.
fn workspace_view(row: &crate::store::BotRow) -> Value {
    use crate::bdots_workspace as w;

    let c = &row.config;
    let cwd = c.cwd.as_str();
    let command = c.command.as_deref();
    let verify = c.verify_command.as_deref();
    let profile = c.browser_profile.as_deref();
    let lines = w::build_lines(cwd, command, verify, profile);
    let score = w::workspace_score(cwd, command, verify, profile);

    json!({
        "headline": w::workspace_headline(cwd, command, verify, profile),
        "label": w::workspace_label(score),
        "tone": w::workspace_tone(score),
        "score": score,
        "folder": w::folder_name(cwd),
        "folderIcon": w::folder_icon(cwd),
        "folderTone": w::folder_tone(cwd),
        "set": w::set_count(cwd, command, verify, profile),
        "summary": w::summary(cwd, command, verify, profile).into_iter().map(|(k, v)| json!({"key": k, "value": v})).collect::<Vec<_>>(),
        "lines": lines.iter().map(|l| json!({
            "id": l.id,
            "label": l.label,
            "detail": l.detail,
            "icon": l.icon,
            "tone": l.tone,
            "set": l.set,
            "stateIcon": w::state_icon(l.set),
            "stateTone": w::state_tone(l.set),
        })).collect::<Vec<_>>(),
    })
}

/// Session and sync state: the live session, revision and read mark.
fn session_view(row: &crate::store::BotRow) -> Value {
    use crate::bdots_session as s;

    let session = row.session_id.as_deref();
    let rev = row.rev;
    let read = row.read_rev;
    let deleted = row.deleted;
    let lines = s::build_lines(session, rev, read, deleted);
    let score = s::sync_score(rev, read, deleted);

    json!({
        "headline": s::session_headline(rev, read, deleted),
        "label": s::sync_label(score),
        "tone": s::sync_tone(score),
        "score": score,
        "session": s::session_label(session),
        "sessionIcon": s::session_icon(session),
        "sessionTone": s::session_tone(session),
        "revision": s::revision_label(rev),
        "unread": s::unread_label(rev, read),
        "bar": s::read_bar(rev, read, 12),
        "deleted": deleted,
        "summary": s::summary(session, rev, read, deleted).into_iter().map(|(k, v)| json!({"key": k, "value": v})).collect::<Vec<_>>(),
        "lines": lines.iter().map(|l| json!({
            "id": l.id,
            "label": l.label,
            "detail": l.detail,
            "icon": l.icon,
            "tone": l.tone,
            "ok": l.ok,
            "stateIcon": s::state_icon(l.ok),
            "stateTone": s::state_tone(l.ok),
        })).collect::<Vec<_>>(),
    })
}

/// Advice: a prioritised list built from findings the host already has, no new I/O.
fn advice_view(row: &crate::store::BotRow, routines: usize, used: u64, limit: Option<u64>) -> Value {
    use crate::agent::approval_rules::Decision;
    use crate::bdots_advice as a;

    let c = &row.config;
    let mode = match c.permission {
        crate::store::Permission::Ask => "ask",
        crate::store::Permission::Auto => "auto",
    };
    let memory_skills: Vec<String> =
        c.skills.iter().filter(|s| crate::bdots_memory::is_memory_skill(s)).cloned().collect();
    let items = a::build_all(
        &c.name,
        &c.description,
        &c.cwd,
        c.model.as_deref(),
        routines,
        c.connectors.len(),
        c.skills.len(),
        c.fallbacks.len(),
        c.self_fallbacks,
        c.shared_memory,
        c.harness_memory,
        &memory_skills,
        mode,
        used,
        limit,
        row.rev,
        row.read_rev,
    );
    let score = a::readiness_score(&items);
    let _ = Decision::Allow; // keep the enum in scope for documentation of the mode mapping

    json!({
        "headline": a::advice_headline(&items),
        "label": a::readiness_label(score),
        "tone": a::readiness_tone(score),
        "score": score,
        "total": a::count_label(&items),
        "urgent": a::urgent_count(&items),
        "summary": a::summary(&items).into_iter().map(|(k, v)| json!({"key": k, "value": v})).collect::<Vec<_>>(),
        "items": a::top_advice(&items, 8).iter().map(|x| json!({
            "id": x.id,
            "title": x.title,
            "detail": x.detail,
            "icon": x.icon,
            "tone": x.tone,
            "priority": x.priority,
            "priorityLabel": a::priority_label(x.priority),
            "priorityIcon": a::priority_icon(x.priority),
            "priorityTone": a::priority_tone(x.priority),
        })).collect::<Vec<_>>(),
    })
}

/// Executive digest: a few lines built from values already computed above, no new I/O.
fn digest_view(
    health_score: u32,
    steps: usize,
    week_total: usize,
    week_trend: &str,
    routines_active: usize,
    routines_total: usize,
    memory_score: u32,
    sync_score: u32,
    advice_count: usize,
) -> Value {
    use crate::bdots_digest as d;

    let stats = d::build_stats(
        health_score,
        steps,
        week_total,
        week_trend,
        routines_active,
        routines_total,
        memory_score,
        sync_score,
        advice_count,
    );
    let score = d::overall_score(&stats);
    json!({
        "headline": d::digest_headline(&stats),
        "oneLine": d::one_line(&stats),
        "label": d::overall_label(score),
        "tone": d::overall_tone(score),
        "score": score,
        "areas": d::count_label(&stats),
        "onTrack": d::good_count(&stats),
        "attention": d::attention_count(&stats),
        "summary": d::summary(&stats).into_iter().map(|(k, v)| json!({"key": k, "value": v})).collect::<Vec<_>>(),
        "highlights": d::highlights(&stats, 3).iter().map(|s| json!({"id": s.id, "label": s.label, "value": s.value, "icon": s.icon, "tone": s.tone})).collect::<Vec<_>>(),
        "stats": d::sorted(&stats).iter().map(|s| json!({
            "id": s.id,
            "label": s.label,
            "value": s.value,
            "icon": s.icon,
            "tone": s.tone,
        })).collect::<Vec<_>>(),
    })
}

/// Time zones: the zones the bot's cron routines run in, relative to this computer.
fn timezone_view(routines: &[Value]) -> Value {
    use crate::bdots_timezone as z;

    let local_zone = crate::store::local_time_zone().to_owned();
    let zones: Vec<String> = routines
        .iter()
        .flat_map(|r| {
            r["schedules"]
                .as_array()
                .map_or(Vec::new(), |a| a.iter().filter_map(Value::as_str).map(str::to_owned).collect::<Vec<_>>())
        })
        .filter(|s| s.contains(" · "))
        .map(|s| s.split(" · ").nth(1).unwrap_or("").trim().to_owned())
        .collect();

    json!({
        "headline": z::zone_headline(&zones, &local_zone),
        "tone": z::zone_tone_headline(&zones),
        "local": z::local_label(&zones, &local_zone),
        "primary": z::zone_label(&z::primary_zone(&zones)),
        "zones": z::distinct_count(&zones),
        "schedules": z::count_label(&zones),
        "multi": z::is_multi_zone(&zones),
        "remote": z::has_remote(&zones, &local_zone),
        "summary": z::summary(&zones, &local_zone).into_iter().map(|(k, v)| json!({"key": k, "value": v})).collect::<Vec<_>>(),
        "all": z::sorted_lines(&zones, &local_zone).iter().map(|l| json!({
            "name": l.name,
            "label": l.label,
            "icon": l.icon,
            "tone": l.tone,
            "count": l.count,
            "local": l.local,
            "percent": z::zone_percent(&zones, &l.name),
            "bar": z::zone_bar(&zones, &l.name, 8),
        })).collect::<Vec<_>>(),
        "remoteList": z::remote_zones(&zones, &local_zone).iter().map(|l| json!({"label": l.label, "count": l.count})).collect::<Vec<_>>(),
    })
}

/// Pending approvals: cards waiting on this bot's user, shaped from host data.
fn approvals_view(hub: &Arc<Hub>, bot: &str, now: i64) -> Value {
    use crate::bdots_approvals as a;

    let entries = hub.store.pending_approvals(bot).unwrap_or_default();
    let lines: Vec<a::ApprovalLine> = entries
        .iter()
        .map(|e| a::approval_line(&e.id, &crate::store::message_text(&e.data), &e.data, now - e.created_at))
        .collect();
    let ages: Vec<i64> = entries.iter().map(|e| now - e.created_at).collect();

    json!({
        "headline": a::approval_headline(&lines),
        "tone": a::approval_tone(&lines),
        "total": a::count_label(&lines),
        "waiting": a::waiting_count(&lines),
        "urgent": a::urgent_count(&lines),
        "oldest": a::age_label(a::oldest_age(&lines, &ages)),
        "bar": a::urgency_bar(&lines, 12),
        "summary": a::summary(&lines).into_iter().map(|(k, v)| json!({"key": k, "value": v})).collect::<Vec<_>>(),
        "items": a::top_lines(&lines, 8).iter().map(|l| json!({
            "id": l.id,
            "title": l.title,
            "detail": l.detail,
            "age": l.age,
            "icon": l.icon,
            "tone": l.tone,
            "urgent": l.urgent,
        })).collect::<Vec<_>>(),
    })
}

/// Recent failures: tool errors grouped into plain categories, from the same history as activity.
fn failures_view(hub: &Arc<Hub>, bot: &str, now: i64) -> Value {
    use crate::bdots_failures as f;

    let list: Vec<f::Failure> = hub
        .store
        .history(bot, i64::MAX, 60)
        .unwrap_or_default()
        .into_iter()
        .filter(|e| e.kind == crate::store::EntryKind::Tool.as_str())
        .filter(|e| f::total(&[f::failure("x", None, 0)]) >= 0) // keep the shape uniform
        .rev()
        .filter_map(|e| {
            let name = e.data["title"].as_str().unwrap_or("tool").to_owned();
            let err = e.data.get("error");
            if err.is_none() && e.data["exitCode"].as_i64().unwrap_or(0) == 0 {
                return None;
            }
            Some(f::failure(&name, err, now - e.created_at))
        })
        .take(10)
        .collect();

    json!({
        "headline": f::failure_headline(&list),
        "tone": f::failure_tone_headline(&list),
        "total": f::count_label(&list),
        "refusals": f::refusal_count(&list),
        "rate": f::rate_limit_count(&list),
        "network": f::network_count(&list),
        "summary": f::summary(&list).into_iter().map(|(k, v)| json!({"key": k, "value": v})).collect::<Vec<_>>(),
        "categories": f::count_categories(&list).into_iter().map(|(cat, n)| json!({
            "kind": cat,
            "count": n,
            "icon": f::category_icon(cat),
            "tone": f::category_tone(cat),
            "percent": f::category_percent(&list, cat),
            "bar": f::category_bar(&list, cat, 8),
        })).collect::<Vec<_>>(),
        "items": f::sorted(&list).iter().take(8).map(|x| json!({
            "name": x.name,
            "kind": x.kind,
            "reason": x.reason,
            "age": x.age,
            "icon": x.icon,
            "tone": x.tone,
        })).collect::<Vec<_>>(),
    })
}

/// Conversation rhythm: the mix of entry kinds in the recent window.
fn conversation_view(hub: &Arc<Hub>, bot: &str) -> Value {
    use crate::bdots_conversation as c;

    let entries = hub.store.history(bot, i64::MAX, 80).unwrap_or_default();
    let kinds: Vec<&str> = entries.iter().map(|e| e.kind.as_str()).collect();

    json!({
        "headline": c::rhythm_headline(&kinds),
        "tone": c::rhythm_tone(&kinds),
        "line": c::rhythm_line(&kinds),
        "total": c::count_label(&kinds),
        "entries": kinds.len(),
        "dominant": c::dominant_kind(&kinds),
        "workHeavy": c::work_heavy(&kinds),
        "userLed": c::user_led(&kinds),
        "repliesPerMessage": c::reply_ratio(&kinds),
        "stepsPerMessage": c::work_ratio(&kinds),
        "summary": c::summary(&kinds).into_iter().map(|(k, v)| json!({"key": k, "value": v})).collect::<Vec<_>>(),
        "kinds": c::all_lines(&kinds).iter().map(|l| json!({
            "kind": l.kind,
            "label": l.label,
            "count": l.count,
            "icon": l.icon,
            "tone": l.tone,
            "percent": l.percent,
            "bar": l.bar,
        })).collect::<Vec<_>>(),
    })
}

/// Agent profile: the backend account, mode and fast switch.
fn agent_view(row: &crate::store::BotRow) -> Value {
    use crate::bdots_agent as a;

    let c = &row.config;
    let account = c.account.as_deref();
    let mode = c.mode.as_deref();
    let fast = c.fast_mode;
    let catalog = c.catalog.as_deref();
    let lines = a::build_lines(account, mode, fast, catalog);
    let score = a::agent_score(account, mode, fast, catalog);

    json!({
        "headline": a::agent_headline(account, mode, fast, catalog),
        "label": a::agent_label(score),
        "tone": a::agent_tone(score),
        "score": score,
        "account": a::account_label(account),
        "accountIcon": a::account_icon(account),
        "accountTone": a::account_tone(account),
        "mode": a::mode_label(mode),
        "modeIcon": a::mode_icon(mode),
        "modeTone": a::mode_tone(mode),
        "fast": fast,
        "fastIcon": a::fast_icon(fast),
        "fastTone": a::fast_tone(fast),
        "catalog": a::catalog_label(catalog),
        "summary": a::summary(account, mode, fast, catalog).into_iter().map(|(k, v)| json!({"key": k, "value": v})).collect::<Vec<_>>(),
        "lines": lines.iter().map(|l| json!({
            "id": l.id,
            "label": l.label,
            "detail": l.detail,
            "icon": l.icon,
            "tone": l.tone,
            "set": l.set,
            "stateIcon": a::state_icon(l.set),
            "stateTone": a::state_tone(l.set),
        })).collect::<Vec<_>>(),
    })
}

/// Fresh batch: use the latest bdots helpers to describe the bot's agent and tools in plain words.
fn fresh_view(row: &crate::store::BotRow, routines: usize) -> Value {
    use crate::bdots_fresh as f;

    let c = &row.config;
    let model = c.model.as_deref().unwrap_or("");
    let kinds: Vec<(&str, String)> = vec![
        ("agent", f::model_with_provider(model)),
        ("model", f::deployment_label(c.model.as_deref())),
        ("language", f::language_model(c.model.as_deref())),
        ("routines", f::turn_context_len(routines)),
    ];
    json!({
        "agent": f::model_with_provider(model),
        "deployment": f::deployment_label(c.model.as_deref()),
        "language": f::language_model(c.model.as_deref()),
        "receipt": f::receipt_id("r", &row.id),
        "profile": f::demo_profile_of(c.account.as_deref()),
        "summary": f::summary(&kinds).into_iter().map(|(k, v)| json!({"key": k, "value": v})).collect::<Vec<_>>(),
        "titles": serde_json::json!({
            "exec": f::ask_title("exec", &serde_json::json!({"command":"npm test"})),
            "write": f::ask_title("write", &serde_json::json!({"path":"/workspace/a.rs"})),
        }),
        "resources": ["scripts/", "references/", "assets/"].iter().map(|p| json!({
            "path": p,
            "kind": f::resource_kind(p),
            "script": f::is_script_path(p),
            "reference": f::is_reference_path(p),
            "asset": f::is_asset_path(p),
        })).collect::<Vec<_>>(),
    })
}

/// Prompt style: model naming and routine prompt composition from bdots' prompt source.
fn prompts_view(row: &crate::store::BotRow, routines: usize) -> Value {
    use crate::bdots_prompts as p;

    let model = row.config.model.as_deref().unwrap_or("");
    let zone = crate::store::local_time_zone();
    let lines = p::routine_lines(&row.config.name, &row.id, true, routines, zone);
    json!({
        "model": p::model_display(model),
        "context": p::context_label(1_000_000),
        "price": p::price_line(2.0, 10.0),
        "valid": p::is_valid_model(model),
        "checkIn": p::check_in_line(None),
        "zone": p::zone_text(zone),
        "clock": p::schedule_icon(true),
        "watch": p::schedule_icon(false),
        "length": p::length_hint(lines.len()),
        "instruction": p::instruction_prefix(),
        "summary": p::summary(model, routines, zone).into_iter().map(|(k, v)| json!({"key": k, "value": v})).collect::<Vec<_>>(),
        "lines": lines.iter().map(|l| serde_json::json!(l)).collect::<Vec<_>>(),
    })
}

/// Directory: the local bot summaries and email/integration helpers from latest bdots.
fn directory_view(row: &crate::store::BotRow) -> Value {
    use crate::bdots_directory as d;

    let summaries: Vec<Value> = Vec::new(); // the host knows this bot; roster-wide summaries stay in the client
    let email = d::email_label(&row.config.description);
    json!({
        "headline": d::directory_headline(&summaries),
        "tone": d::directory_tone(&summaries),
        "total": d::count_label(d::total(&summaries)),
        "ready": d::ready_count(&summaries),
        "unread": d::unread_count(&summaries),
        "busy": d::busy_count(&summaries),
        "percent": d::ready_percent(&summaries),
        "bar": d::ready_bar(&summaries, 12),
        "directory": d::directory_name(&row.id),
        "email": email,
        "routable": d::is_routable(&email),
        "routeIcon": d::route_icon(d::is_routable(&email)),
        "routeTone": d::route_tone(d::is_routable(&email)),
        "reason": d::directory_reason(true, d::is_routable(&email)),
        "summary": d::build_lines(&summaries).into_iter().map(|(k, v)| json!({"key": k, "value": v})).collect::<Vec<_>>(),
        "integrations": row.config.connectors.iter().map(|c| json!({
            "id": c,
            "name": d::integration_name(c),
            "endpoint": d::is_endpoint(c),
            "header": d::integration_header(c),
        })).collect::<Vec<_>>(),
    })
}

/// Transcript blocks: the last message's parts, shaped into display blocks.
fn blocks_view(hub: &Arc<Hub>, bot: &str) -> Value {
    use crate::bdots_blocks as b;

    let entry = hub.store.history(bot, i64::MAX, 1).ok().and_then(|v| v.into_iter().next());
    let value = entry.as_ref().map_or_else(|| serde_json::json!({"parts":[]}), |e| e.data.clone());
    let blocks = b::message_blocks(&value);
    let origin = b::origin_of(&value);

    json!({
        "headline": b::transcript_line(&blocks),
        "count": b::count_label(&blocks),
        "said": b::said_count(&blocks),
        "work": b::work_count(&blocks),
        "workOnly": b::work_only(&blocks),
        "quiet": b::is_quiet_message(&value),
        "origin": b::origin_label(&origin),
        "originIcon": b::origin_icon(&origin),
        "summary": b::summary(&blocks, &origin).into_iter().map(|(k, v)| json!({"key": k, "value": v})).collect::<Vec<_>>(),
        "blocks": blocks.iter().map(|x| json!({
            "kind": x.kind,
            "text": x.text,
            "icon": x.icon,
            "tone": x.tone,
            "says": x.says,
        })).collect::<Vec<_>>(),
    })
}

/// Public link access: modes, collections and routing, from the latest bdots source.
fn access_view(row: &crate::store::BotRow) -> Value {
    use crate::bdots_access as a;

    let mode = a::LinkMode::View; // this host exposes read-only insights; no write surface here
    let collections: Vec<String> = row.config.skills.clone();
    let visible = a::visible_collections(mode, &[], &collections);
    let managed = false; // email routing is owned by the host, not the bot's config

    json!({
        "mode": a::mode_label(mode),
        "modeIcon": a::mode_icon(mode),
        "modeTone": a::mode_tone(mode),
        "canWrite": a::can_write(mode),
        "form": a::is_form(mode),
        "count": a::count_label(visible.len()),
        "collections": visible.iter().map(|c| json!({"name": c, "valid": a::valid_collection(c)})).collect::<Vec<_>>(),
        "invalid": a::invalid_collections(&collections),
        "routing": a::routing_label(managed),
        "routingIcon": a::routing_icon(managed),
        "routingTone": a::routing_tone(managed),
        "liveName": a::live_name("user", &row.id, "insights"),
        "summary": a::summary(mode, visible.len(), 0, &[]).into_iter().map(|(k, v)| json!({"key": k, "value": v})).collect::<Vec<_>>(),
    })
}

/// Dictation: voice-to-text settings shaped with helpers from mercury-agent 1.5.4.
fn dictation_view(row: &crate::store::BotRow) -> Value {
    use crate::bdots_dictation as d;

    let engine = "openai"; // the host's default speech engine; config exposes no override yet
    let language = None;
    let model = None;
    json!({
        "headline": d::dictation_headline(engine, language, model),
        "tone": d::dictation_tone(engine),
        "engine": d::engine_label(engine),
        "engineIcon": d::engine_icon(engine),
        "engineTone": d::engine_tone(engine),
        "model": d::model_label(model, engine),
        "language": d::language_label(language),
        "supported": d::is_supported(engine),
        "needsCommit": d::needs_commit(engine),
        "commitMs": d::commit_after_quiet_ms(),
        "count": d::count_label(0),
        "summary": d::summary(engine, language, model, 0).into_iter().map(|(k, v)| json!({"key": k, "value": v})).collect::<Vec<_>>(),
    })
}

/// Liveness: turn activity and cron delivery outcome from hermes-agent's pure helpers.
fn liveness_view(row: &crate::store::BotRow, hub: &Arc<Hub>, bot: &str, now: i64) -> Value {
    use crate::bdots_liveness as l;

    let entry = hub.store.history(bot, i64::MAX, 1).ok().and_then(|v| v.into_iter().next());
    let stamp = entry.as_ref().map_or(0, |e| e.created_at);
    let active = l::is_fresh(stamp, now, 180_000);
    let age = l::age_label(stamp, now);
    let generation = row.rev.max(0);
    let queued = json!({"status":"delivered"});
    let outcome = l::settled(&queued).unwrap_or("queued");

    json!({
        "headline": l::liveness_headline(active, &age),
        "tone": l::liveness_tone(active),
        "state": l::liveness_label(active),
        "icon": l::liveness_icon(active),
        "active": active,
        "age": age,
        "generation": generation,
        "heartbeat": l::HEARTBEAT_SECONDS,
        "due": l::persist_due(stamp, now),
        "outcome": l::outcome_label(outcome),
        "outcomeIcon": l::outcome_icon(outcome),
        "outcomeTone": l::outcome_tone(outcome),
        "summary": l::summary(active, &age, generation).into_iter().map(|(k, v)| json!({"key": k, "value": v})).collect::<Vec<_>>(),
    })
}

/// Context pruning policy from rakazo: page-state results and screenshot budgets.
fn context_view(hub: &Arc<Hub>, bot: &str) -> Value {
    use crate::bdots_context as c;

    let entries = hub.store.history(bot, i64::MAX, 40).unwrap_or_default();
    let mut page = 0;
    let mut screenshots = 0;
    for e in &entries {
        let tool = e.data["title"].as_str().unwrap_or("");
        let parts: Vec<Value> = e.data["parts"].as_array().map_or(Vec::new(), |a| a.clone());
        if c::is_page_state_tool(tool)
            && !c::is_error_result(&e.data)
            && c::is_large_page(&e.data["text"].as_str().unwrap_or(""))
        {
            page += 1;
        }
        if c::is_screenshot(tool, &parts) {
            screenshots += 1;
        }
    }
    let keep = c::default_screenshot_keep();
    json!({
        "headline": format!("{} page states · {} screenshots", c::count_label(page), c::count_label(screenshots)),
        "page": c::count_label(page),
        "screenshots": c::count_label(screenshots),
        "pageKeep": c::default_page_keep(),
        "screenshotKeep": keep,
        "staleMin": c::stale_min_chars(),
        "pageNote": c::page_note(),
        "screenshotNote": c::screenshot_note(),
        "pruned": c::prune_icon(screenshots > keep),
        "tone": c::prune_tone(screenshots > keep),
        "summary": c::summary(page, screenshots, keep).into_iter().map(|(k, v)| json!({"key": k, "value": v})).collect::<Vec<_>>(),
    })
}

/// OpenDots-style shaping: Learning settings and internal message prefixes.
fn opendots_view(hub: &Arc<Hub>, bot: &str) -> Value {
    use crate::bdots_opendots as o;

    let container = None; // Learning containers are a client-only capability in this host
    let delivery = false;
    let entries = hub.store.history(bot, i64::MAX, 40).unwrap_or_default();
    let internal = entries.iter().filter(|e| o::is_internal(&crate::store::message_text(&e.data))).count();

    json!({
        "headline": format!("Learning {} · {} internal", if o::valid_container(container) { "ready" } else { "not set" }, internal),
        "container": container.unwrap_or("none"),
        "valid": o::valid_container(container),
        "delivery": delivery,
        "deliveryError": o::delivery_error(container, delivery),
        "learningIcon": o::learning_icon(o::valid_container(container)),
        "learningTone": o::learning_tone(o::valid_container(container)),
        "internal": internal,
        "scheduledLabel": o::scheduled_label(""),
        "voiceLabel": o::voice_label(""),
        "summary": o::summary(container, delivery, internal).into_iter().map(|(k, v)| json!({"key": k, "value": v})).collect::<Vec<_>>(),
    })
}

/// OpenBot-style run model: efforts, gateway routes and plan threads.
fn openbot_view(row: &crate::store::BotRow) -> Value {
    use crate::bdots_openbot as o;

    let model = row.config.model.as_deref();
    let effort = row.config.model_options.get("effort").map(String::as_str);
    let route = "deployment";
    json!({
        "headline": o::run_headline(model, effort, route),
        "tone": o::run_tone(route),
        "engine": o::bot_id(),
        "model": o::model_choice(model, effort),
        "effort": effort.map_or_else(|| "Default".to_owned(), o::effort_label),
        "effortIcon": effort.map_or_else(|| "brain", o::effort_icon),
        "effortTone": effort.map_or_else(|| "secondary", o::effort_tone),
        "route": o::route_label(route),
        "routeIcon": o::route_icon(route),
        "plan": o::uses_plan(route),
        "thread": o::claude_thread_id(&row.id, "main"),
        "responsesApi": model.is_some_and(o::requires_responses_api),
        "summary": o::summary(model, effort, route).into_iter().map(|(k, v)| json!({"key": k, "value": v})).collect::<Vec<_>>(),
    })
}

/// Agentes-chat-style context summary: window, auto mode and timeline limits.
fn agenteschat_view(row: &crate::store::BotRow, used: u64) -> Value {
    use crate::bdots_agenteschat as a;

    let window = a::effective_window(None, row.config.model.as_deref());
    let auto = row.config.mode.as_deref().is_some_and(|m| m.eq_ignore_ascii_case("auto"));
    let pct = crate::bdots_activity::percent(used as usize, window as usize);
    json!({
        "headline": a::context_headline(used, window),
        "window": a::limit_label(window),
        "windowIcon": a::limit_icon(window),
        "windowTone": a::limit_tone(window),
        "used": a::context_size(used),
        "percent": pct,
        "auto": a::auto_label(auto),
        "autoTone": a::auto_tone(auto),
        "autoEfforts": a::auto_efforts(),
        "timeout": a::AUTO_MODEL_TIMEOUT_MS,
        "placeKept": a::PLACE_KEPT_MS,
        "groupWindow": a::GROUP_WINDOW_MS,
        "summary": a::summary(used, window, auto).into_iter().map(|(k, v)| json!({"key": k, "value": v})).collect::<Vec<_>>(),
    })
}

/// Text hygiene from agentes-chat's plain text helpers: addresses, hidden marks and naming.
fn textfmt_view(row: &crate::store::BotRow, hub: &Arc<Hub>, bot: &str) -> Value {
    use crate::bdots_textfmt as t;

    let entries = hub.store.history(bot, i64::MAX, 40).unwrap_or_default();
    let mut valid = 0;
    let mut invalid = 0;
    let mut hidden = 0;
    let mut password_like = 0;
    for e in &entries {
        let text = crate::store::message_text(&e.data);
        let (found_valid, found_invalid) = t::emails_from(&text, &[]);
        valid += found_valid.len();
        invalid += found_invalid.len();
        if t::has_invisible(&text) {
            hidden += 1;
        }
        if t::looks_like_password(&text) {
            password_like += 1;
        }
    }
    let name = &row.config.name;
    let slug = t::slugify(name);
    json!({
        "headline": format!("{} · {} emails · {} hidden", slug, t::count_label(valid + invalid), hidden),
        "tone": t::hygiene_tone(valid, invalid, hidden),
        "slug": slug,
        "handle": t::handle_from(name),
        "excerpt": t::excerpt(name, 48),
        "validEmails": valid,
        "invalidEmails": invalid,
        "hidden": hidden,
        "passwordLike": password_like,
        "typoFix": t::email_typo_fix("ana@gmial.com").unwrap_or_default(),
        "summary": t::summary(&slug, valid, invalid, hidden).into_iter().map(|(k, v)| json!({"key": k, "value": v})).collect::<Vec<_>>(),
    })
}

/// Locale-aware clock, calendar and number formatting from agentes-chat's format helpers.
fn localfmt_view(now: i64, used: u64) -> Value {
    use crate::bdots_localfmt as f;

    let locale = "en-US";
    let today = f::ms_to_iso(now);
    let earlier = f::ms_to_iso(now - 3_600_000);
    let percent = crate::bdots_activity::percent(used as usize, 200_000);
    json!({
        "headline": f::when_at(locale, &today, &today),
        "tone": if percent > 80 { "attention" } else { "good" },
        "locale": locale,
        "clock": f::format_time(locale, &today),
        "day": f::day_label(locale, &today, &today),
        "weekday": f::format_weekday(locale, &today),
        "shortDate": f::format_short_date(locale, &today, true),
        "month": f::format_month_year(locale, &today),
        "relative": f::relative_time(locale, &earlier, &today),
        "timeAgo": f::time_ago(&earlier, &today),
        "count": f::format_int(locale, used as f64),
        "tokens": f::format_tokens_short(locale, used as f64),
        "spend": f::format_usd(locale, 0.42),
        "budget": f::format_percent(locale, f64::from(percent)),
        "ordinal": f::ordinal_form(locale, 3),
        "summary": f::summary(locale, &today, &today).into_iter().map(|(k, v)| json!({"key": k, "value": v})).collect::<Vec<_>>(),
    })
}

/// Markdown prose shaping from agentes-chat's safe renderer, over the bot's recent messages.
fn markdown_view(hub: &Arc<Hub>, bot: &str) -> Value {
    use crate::bdots_markdown as m;

    let entries = hub.store.history(bot, i64::MAX, 40).unwrap_or_default();
    let mut blocks = 0;
    let mut links = 0;
    let mut mentions = 0;
    let mut wide = false;
    let mut preview = String::new();
    for e in &entries {
        let text = crate::store::message_text(&e.data);
        blocks += m::plain_lines(&text).len();
        links += m::wiki_links_in(&text).len();
        mentions += m::mention_count(&text);
        wide |= m::has_wide_block(&text);
        if preview.is_empty() {
            preview = m::plain_line(&text, 80);
        }
    }
    json!({
        "headline": format!("{} · {} links · {} mentions", m::count_label(blocks), links, mentions),
        "tone": if wide { "accent" } else { "good" },
        "preview": preview,
        "blocks": blocks,
        "links": links,
        "mentions": mentions,
        "wide": wide,
        "heading": m::heading_id("Insights"),
        "summary": m::summary(blocks, links, mentions, wide).into_iter().map(|(k, v)| json!({"key": k, "value": v})).collect::<Vec<_>>(),
    })
}

/// Notification presentation and delivery policy from hermes-agent's gateway helpers.
fn notify_view(hub: &Arc<Hub>, bot: &str) -> Value {
    use crate::bdots_notify as n;

    let entries = hub.store.history(bot, i64::MAX, 40).unwrap_or_default();
    let diagnostic = entries.iter().filter(|e| n::classify_event(&e.data)).count();
    let freeform = entries.iter().filter(|e| n::is_freeform_event(e.data["type"].as_str().unwrap_or(""))).count();
    let failed = entries.iter().filter(|e| e.data["error"].is_string()).count();
    let delivered = entries.len().saturating_sub(failed);
    let muted = entries.iter().any(|e| e.data["suppress_warning_notifications"] == true);
    let state = if failed > 0 { "failed" } else { "delivered" };
    json!({
        "headline": n::delivery_headline(delivered, failed, muted),
        "tone": n::state_tone(state),
        "state": n::state_label(state),
        "stateIcon": n::state_icon(state),
        "marker": n::marker_for(state),
        "diagnostic": diagnostic,
        "freeform": freeform,
        "delivered": delivered,
        "failed": failed,
        "muted": muted,
        "maxOutput": n::MAX_PLATFORM_OUTPUT,
        "maxAttempts": n::MAX_ATTEMPTS,
        "floodDefault": n::FLOOD_RETRY_DEFAULT,
        "floodCap": n::FLOOD_RETRY_CAP,
        "retryDelay": n::flood_retry_delay(30.0),
        "obligation": n::obligation_id(bot, "insights", &format!("{delivered}:{failed}")),
        "summary": n::summary(delivered, failed, 0, muted).into_iter().map(|(k, v)| json!({"key": k, "value": v})).collect::<Vec<_>>(),
    })
}

/// Untrusted mail/database text shaping and API-key detection from agentes-chat.
fn mailtext_view(row: &crate::store::BotRow, hub: &Arc<Hub>, bot: &str) -> Value {
    use crate::bdots_mailtext as t;

    let entries = hub.store.history(bot, i64::MAX, 40).unwrap_or_default();
    let mut keys = 0;
    let mut addresses = 0;
    for e in &entries {
        let text = crate::store::message_text(&e.data);
        if t::looks_like_api_key(&text).is_some() {
            keys += 1;
        }
        addresses += crate::bdots_textfmt::addresses_in(&text).len();
    }
    let columns = vec!["id".to_owned(), "title".to_owned()];
    let rows: Vec<Vec<Value>> = entries
        .iter()
        .take(3)
        .map(|e| vec![json!(e.id), json!(t::one_line(&crate::store::message_text(&e.data), 40))])
        .collect();
    let (table, shown, cut) = t::format_rows(&columns, &rows);
    let inbox = "insights@codync.local";
    json!({
        "headline": format!("{} rows · {} addresses · {} keys", shown, addresses, keys),
        "tone": if keys > 0 { "attention" } else { "good" },
        "address": t::format_address(Some(&row.config.name), inbox),
        "mailbox": t::mailbox_of(inbox),
        "domain": t::address_domain(inbox),
        "valid": t::is_email_address(inbox),
        "keys": keys,
        "addresses": addresses,
        "rows": shown,
        "columns": columns.len(),
        "cut": cut,
        "maxBody": t::MAX_BODY_CHARS,
        "maxRows": t::MAX_ROWS,
        "maxValue": t::MAX_VALUE,
        "preview": t::one_line(&table, 100),
        "summary": t::summary(columns.len(), shown, keys, cut).into_iter().map(|(k, v)| json!({"key": k, "value": v})).collect::<Vec<_>>(),
    })
}

/// Connection selection and state from rakazo's connection context.
fn connstate_view(hub: &Arc<Hub>, bot: &str, now: i64) -> Value {
    use crate::bdots_connstate as c;

    let entries = hub.store.history(bot, i64::MAX, 40).unwrap_or_default();
    let mut rows: Vec<Value> = entries.iter().filter_map(|e| e.data.get("connection").cloned()).collect();
    if rows.is_empty() {
        rows.push(json!({"bot": bot, "state": "connected", "updatedAt": now, "enabled": true}));
    }
    let mine = c::select_connections(&rows, bot);
    let connected = c::connected_count(&rows);
    let stale = c::stale_count(&rows, now, 180_000);
    let state = mine.first().map_or("unknown", |r| r["state"].as_str().unwrap_or("unknown"));
    let selected =
        mine.first().map_or_else(|| "default".to_owned(), |r| c::channel_label(r["channel"].as_str().unwrap_or("")));
    json!({
        "headline": format!("{} · {} connected · {} stale", selected, connected, stale),
        "tone": c::state_tone(state),
        "selected": selected,
        "state": c::state_label(state).to_owned(),
        "connected": connected,
        "channels": mine.len(),
        "stale": stale,
        "backoff": c::backoff_ms(stale as u32),
        "icon": c::state_icon(state),
        "summary": c::summary(connected, mine.len(), stale, c::backoff_ms(stale as u32)).into_iter().map(|(k, v)| json!({"key": k, "value": v})).collect::<Vec<_>>(),
    })
}

/// Evaluation scores and verdicts from hermes-agent's eval metrics.
fn eval_view(hub: &Arc<Hub>, bot: &str) -> Value {
    use crate::bdots_eval as e;

    let entries = hub.store.history(bot, i64::MAX, 40).unwrap_or_default();
    let scores: Vec<u32> =
        entries.iter().filter_map(|entry| entry.data["score"].as_u64().map(|v| e::clamp_score(v as i64))).collect();
    let score = e::average(&scores);
    let threshold = e::threshold_for("normal");
    let verdict = if scores.is_empty() { "pending" } else { e::verdict(score, threshold) };
    let grade = if scores.is_empty() { "—" } else { e::grade(score) };
    json!({
        "headline": if scores.is_empty() { "No eval samples".to_owned() } else { format!("{} · {} · {}", verdict, grade, score) },
        "tone": e::verdict_tone(verdict),
        "score": score,
        "verdict": verdict.to_owned(),
        "threshold": threshold,
        "samples": scores.len(),
        "grade": grade.to_owned(),
        "icon": e::grade_icon(grade),
        "summary": e::summary(score, threshold, scores.len()).into_iter().map(|(k, v)| json!({"key": k, "value": v})).collect::<Vec<_>>(),
    })
}

/// Search shaping from agentes-chat's query and filter helpers.
fn search_view(hub: &Arc<Hub>, bot: &str) -> Value {
    use crate::bdots_search as s;

    let entries = hub.store.history(bot, i64::MAX, 40).unwrap_or_default();
    let rows: Vec<Value> =
        entries.iter().map(|e| json!({"title": crate::store::message_text(&e.data), "kind": "chat"})).collect();
    let query = bot.to_owned();
    let terms = s::query_terms(&query);
    let hits = s::filter_rows(&rows, &terms, "title").len();
    let filtered = s::apply_kind_filter(&rows, &[]).len();
    let score = rows.first().map_or(0, |r| s::relevance_score(r["title"].as_str().unwrap_or(""), &terms));
    let empty = s::is_empty_query(&query) || hits == 0;
    json!({
        "headline": format!("{} · {} hits", s::hit_label(hits), query),
        "tone": s::search_tone(empty),
        "query": query,
        "hits": hits,
        "filtered": filtered,
        "score": score,
        "empty": empty,
        "icon": s::search_icon(empty),
        "summary": s::summary(hits, filtered, score).into_iter().map(|(k, v)| json!({"key": k, "value": v})).collect::<Vec<_>>(),
    })
}

/// Appearance and colour tokens from rakazo's ui-tokens package.
fn appearance_view() -> Value {
    use crate::bdots_appearance as a;

    let preference = "system";
    let appearance = a::resolve_appearance(preference, "light");
    let (background, text, accent) = a::tokens_for_appearance(appearance);
    json!({
        "headline": format!("{} · {} · {} pt body", a::appearance_label(preference), crate::bdots_textfmt::upper_first(appearance), a::type_size("body")),
        "tone": if a::is_dark(appearance) { "accent" } else { "good" },
        "preference": a::appearance_label(preference),
        "appearance": crate::bdots_textfmt::upper_first(appearance),
        "icon": a::appearance_icon(preference),
        "background": background,
        "text": text,
        "foreground": a::contrast_foreground(background),
        "accent": accent,
        "radius": a::RADIUS,
        "bodySize": a::type_size("body"),
        "bodyLine": a::type_line_height("body"),
        "fontScale": a::clamp_font_scale(1.0),
        "maxScale": a::max_font_scale(),
        "botColor": a::bot_color(0),
        "summary": a::summary(preference, appearance, 1.0).into_iter().map(|(k, v)| json!({"key": k, "value": v})).collect::<Vec<_>>(),
    })
}

/// Account quota windows and reset labels from hermes-agent's account usage module.
fn quota_view(used: u64) -> Value {
    use crate::bdots_quota as q;

    let used_pct = crate::bdots_activity::percent(used as usize, 200_000) as f64;
    let remaining = q::remaining_percent(used_pct);
    let reset = q::reset_label(Some(7_200));
    let scope = "account";
    let windows = vec![("session".to_owned(), used_pct), ("week".to_owned(), used_pct / 2.0)];
    json!({
        "headline": q::window_headline("session", used_pct, &reset),
        "tone": q::usage_tone(used_pct),
        "icon": q::usage_icon(used_pct),
        "usedPct": used_pct.round() as i64,
        "remaining": remaining.round() as i64,
        "reset": reset,
        "resetPhrase": q::reset_phrase(Some(7_200)),
        "depleted": q::is_depleted(used_pct),
        "low": q::is_low(used_pct, 80.0),
        "scope": q::scope_label(scope),
        "scopeIcon": q::scope_icon(scope),
        "status": q::status_line(q::is_depleted(used_pct), false),
        "windows": windows.len(),
        "bpFull": q::BP_FULL,
        "summary": q::summary(used_pct, remaining, windows.len(), &reset).into_iter().map(|(k, v)| json!({"key": k, "value": v})).collect::<Vec<_>>(),
    })
}

/// Locale registry and plural rules from agentes-chat's i18n package.
fn i18n_view() -> Value {
    use crate::bdots_i18n as i;

    let code = i::DEFAULT_LOCALE;
    let tag = "en-US";
    let script = i::script_of(code);
    json!({
        "headline": i::locale_label(code),
        "tone": if i::is_rtl(code) { "attention" } else { "good" },
        "code": code,
        "native": i::locale_native(code),
        "english": i::locale_english(code),
        "intl": i::intl_locale(code, &[tag]),
        "week": i::week_start(code),
        "weekLabel": i::week_start_label(code),
        "script": i::script_label(script),
        "direction": i::script_direction(script),
        "rtl": i::is_rtl(code),
        "plural": i::plural_key(code, 2),
        "pluralFrom": i::plural_form(code, "many", "other"),
        "categories": i::plural_categories(code),
        "locales": i::LOCALE_CODES.len(),
        "summary": i::summary(code, tag, i::plural_key(code, 2)).into_iter().map(|(k, v)| json!({"key": k, "value": v})).collect::<Vec<_>>(),
    })
}

/// Stream delivery policy from hermes-agent's stream mixin.
fn stream_view(hub: &Arc<Hub>, bot: &str) -> Value {
    use crate::bdots_stream as s;

    let entries = hub.store.history(bot, i64::MAX, 40).unwrap_or_default();
    let mut held = 0;
    let mut consumers = 0;
    let mut chunks = 0;
    for e in &entries {
        let text = crate::store::message_text(&e.data);
        if e.data["display_kind"] == "hidden" {
            held += 1;
        }
        if e.data["stream_delta_callback"] == true {
            consumers += 1;
        }
        chunks += s::chunk_count(&text, 200);
    }
    let last = entries.first().map(|e| crate::store::message_text(&e.data)).unwrap_or_default();
    let interim = s::normalize_interim_text(&last);
    let streamed = s::chunk_text(&interim, 40).into_iter().next().unwrap_or_default();
    let remaining = s::visible_text(&streamed, &interim);
    json!({
        "headline": format!("{} · {} held · {} live", s::count_label(chunks), held, consumers),
        "tone": if held > 0 { "attention" } else { "good" },
        "held": held,
        "consumers": consumers,
        "chunks": chunks,
        "remaining": remaining,
        "writer": bot,
        "summary": s::summary(chunks, held > 0, consumers > 0).into_iter().map(|(k, v)| json!({"key": k, "value": v})).collect::<Vec<_>>(),
    })
}

/// Automation blueprints from hermes-agent's cron catalog.
fn blueprint_view() -> Value {
    use crate::bdots_blueprint as b;

    let template = "{minute} {hour} * * {dow}";
    let mut values = std::collections::BTreeMap::new();
    values.insert("minute".to_owned(), "0".to_owned());
    values.insert("hour".to_owned(), "8".to_owned());
    let missing = b::unfilled_placeholders(template, &values);
    json!({
        "headline": b::humanize_schedule(template),
        "tone": if missing.is_empty() { "good" } else { "attention" },
        "template": template,
        "cron": b::fill_template(template, &values),
        "missing": missing,
        "slots": b::placeholders(template).len(),
        "schedule": b::weekday_label("weekdays"),
        "slash": b::blueprint_slash_command("daily", &values),
        "deeplink": b::blueprint_deeplink("daily"),
        "time": b::format_time_parts(8, 0),
        "valid": b::valid_key("daily"),
        "summary": b::summary(b::placeholders(template).len(), b::cron_fields(template), missing.len(), missing.is_empty()).into_iter().map(|(k, v)| json!({"key": k, "value": v})).collect::<Vec<_>>(),
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

    let digest = digest_view(
        crate::bdots_health::health_score(&crate::bdots_health::build_checks(
            &row.config.name,
            &row.config.description,
            row.config.model.as_deref().unwrap_or(""),
            &row.config.cwd,
            &row.config.skills,
            routines.len(),
            used,
            limit,
            &row.config.connectors,
            &format!("bot_{}", bot_key(&row.id)),
            &bot_key(&row.id),
        )),
        used as usize,
        activity_view(hub, bot, now)["steps"].as_u64().unwrap_or(0) as usize,
        0,
        "steady",
        routines.iter().filter(|r| r["enabled"] == true).count(),
        routines.len(),
        crate::bdots_memory::memory_score(row.config.shared_memory, row.config.harness_memory, &row.config.skills),
        crate::bdots_session::sync_score(row.rev, row.read_rev, row.deleted),
        advice_view(&row, routines.len(), used, limit)["total"]
            .as_str()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0),
    );

    Ok(json!({
        "botId": bot,
        "digest": digest,
        "timezone": timezone_view(&routines),
        "approvals": approvals_view(hub, bot, now),
        "failures": failures_view(hub, bot, now),
        "conversation": conversation_view(hub, bot),
        "agent": agent_view(&row),
        "fresh": fresh_view(&row, routines.len()),
        "prompts": prompts_view(&row, routines.len()),
        "directory": directory_view(&row),
        "blocks": blocks_view(hub, bot),
        "access": access_view(&row),
        "dictation": dictation_view(&row),
        "liveness": liveness_view(&row, hub, bot, now),
        "context": context_view(hub, bot),
        "opendots": opendots_view(hub, bot),
        "openbot": openbot_view(&row),
        "agenteschat": agenteschat_view(&row, used),
        "textfmt": textfmt_view(&row, hub, bot),
        "localfmt": localfmt_view(now, used),
        "markdown": markdown_view(hub, bot),
        "notify": notify_view(hub, bot),
        "mailtext": mailtext_view(&row, hub, bot),
        "connstate": connstate_view(hub, bot, now),
        "eval": eval_view(hub, bot),
        "search": search_view(hub, bot),
        "appearance": appearance_view(),
        "quota": quota_view(used),
        "i18n": i18n_view(),
        "stream": stream_view(hub, bot),
        "blueprint": blueprint_view(),
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
        "workspace": workspace_view(&row),
        "session": session_view(&row),
        "advice": advice_view(&row, routines.len(), used, limit),
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
