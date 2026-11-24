//! A bot's Insights page: the host-computed summary of its routines, model and daily use.

use serde_json::Value;

use super::{App, Buffer, Rect, centered, footer, frame_box, name_of, page_tabs, put, theme};
use crate::tui::l10n::tr;
use crate::tui::manage::{Insights, Page};

pub fn draw(buf: &mut Buffer, area: Rect, app: &App, i: &Insights) {
    let t = theme();
    let r = centered(area, 76, 24);
    let title = name_of(app, &i.bot);
    let inner = frame_box(buf, r, t.text, t.panel, Some((&title, t.text)));
    let inner = page_tabs(buf, inner, Page::Insights);
    let Some(data) = i.data.as_ref().filter(|v| !v.is_null()) else {
        put(buf, inner.x, inner.y, inner.width, &tr("Loading…"), t.dim.patch(t.panel));
        footer(buf, inner, None, "esc · shift+←→ pages");
        return;
    };

    let mut y = inner.y;
    let line = |buf: &mut Buffer, y: &mut u16, text: String, style| {
        put(buf, inner.x, *y, inner.width, &text, style);
        *y += 1;
    };

    let provider = data["provider"].as_str().unwrap_or("");
    let description = data["description"].as_str().unwrap_or("");
    if !description.is_empty() {
        put(
            buf,
            inner.x,
            y,
            inner.width,
            &crate::tui::md::truncate(description, inner.width as usize),
            t.secondary.patch(t.panel),
        );
        y += 2;
    }
    let model = data["model"].as_str().unwrap_or("");
    if !provider.is_empty() || !model.is_empty() {
        line(
            buf,
            &mut y,
            format!("{}: {}", tr("Agent"), if provider.is_empty() { model } else { provider }),
            t.bold.patch(t.panel),
        );
        if !model.is_empty() && !provider.is_empty() {
            line(buf, &mut y, format!("{}: {}", tr("Model"), model), t.secondary.patch(t.panel));
        }
        y += 1;
    }

    let used = data["usage"]["usedText"].as_str().unwrap_or("0");
    let limit = data["usage"]["limit"].as_u64();
    let usage = match limit {
        Some(limit) => format!("{}: {} / {limit}", tr("Tokens"), used),
        None => format!("{}: {}", tr("Tokens"), used),
    };
    line(buf, &mut y, usage, t.text.patch(t.panel));
    line(
        buf,
        &mut y,
        format!("{}: {}", tr("Skills"), data["skills"]["total"].as_u64().unwrap_or(0)),
        t.secondary.patch(t.panel),
    );
    y += 1;

    let schedule = &data["routineSummary"];
    if schedule.is_object() {
        line(buf, &mut y, tr("Schedule").into_owned(), t.bold.patch(t.panel));
        let headline = schedule["headline"].as_str().unwrap_or("");
        let bar = schedule["bar"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{bar} {headline}"), t.text.patch(t.panel));
        let active = schedule["active"].as_u64().unwrap_or(0);
        let paused = schedule["paused"].as_u64().unwrap_or(0);
        let mix = schedule["mix"].as_str().unwrap_or("");
        line(
            buf,
            &mut y,
            format!("  {}: {active} · {}: {paused} · {mix}", tr("Active"), tr("Paused")),
            t.secondary.patch(t.panel),
        );
        let soonest: Vec<String> = schedule["soonest"]
            .as_array()
            .map(|a| {
                a.iter()
                    .take(6)
                    .map(|r| {
                        format!(
                            "  {} {} · {}{}",
                            if r["enabled"] == true { "●" } else { "○" },
                            r["name"].as_str().unwrap_or(""),
                            r["trigger"].as_str().unwrap_or(""),
                            {
                                let n = r["nextIn"].as_str().unwrap_or("");
                                if n.is_empty() { String::new() } else { format!(" · {n}") }
                            }
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        for item in soonest {
            line(buf, &mut y, item, t.tertiary.patch(t.panel));
        }
        y += 1;
    }

    let week = &data["week"];
    if week.is_object() {
        line(buf, &mut y, tr("This week").into_owned(), t.bold.patch(t.panel));
        let verdict = week["verdict"].as_str().unwrap_or("");
        let headline = week["headline"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{verdict} · {headline}"), t.text.patch(t.panel));
        let steps = week["totalLabel"].as_str().unwrap_or("0");
        let trend = week["trend"]["label"].as_str().unwrap_or("");
        line(buf, &mut y, format!("  {}: {steps} · {}: {trend}", tr("Steps"), tr("Trend")), t.secondary.patch(t.panel));
        let top: Vec<String> = week["top"]
            .as_array()
            .map(|a| {
                a.iter()
                    .take(5)
                    .map(|d| {
                        format!(
                            "  {} {} {}",
                            d["label"].as_str().unwrap_or(""),
                            d["bar"].as_str().unwrap_or(""),
                            d["count"].as_u64().unwrap_or(0)
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        for day in top {
            line(buf, &mut y, day, t.tertiary.patch(t.panel));
        }
        y += 1;
    }

    let profile = &data["profile"];
    if profile.is_object() {
        line(buf, &mut y, tr("Capabilities").into_owned(), t.bold.patch(t.panel));
        let headline = profile["headline"].as_str().unwrap_or("");
        let bar = profile["bar"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{bar} {headline}"), t.accent.patch(t.panel));
        let caps: Vec<String> = profile["caps"]
            .as_array()
            .map(|a| {
                a.iter()
                    .take(15)
                    .map(|c| {
                        format!(
                            "  {} {}: {}",
                            if c["enabled"] == true { "●" } else { "○" },
                            c["title"].as_str().unwrap_or(""),
                            c["detail"].as_str().unwrap_or("")
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        for cap in caps {
            line(buf, &mut y, cap, t.secondary.patch(t.panel));
        }
        y += 1;
    }

    let health = &data["health"];
    if health.is_object() {
        line(buf, &mut y, tr("Health").into_owned(), t.bold.patch(t.panel));
        let label = health["label"].as_str().unwrap_or("");
        let headline = health["headline"].as_str().unwrap_or("");
        line(buf, &mut y, format!("{label} · {headline}"), t.text.patch(t.panel));
        let checks: Vec<String> = health["checks"]
            .as_array()
            .map(|a| {
                a.iter()
                    .take(10)
                    .map(|c| {
                        format!(
                            "  {} {}: {}",
                            if c["severity"] == "error" { "!" } else { "·" },
                            c["title"].as_str().unwrap_or(""),
                            c["detail"].as_str().unwrap_or("")
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        for check in checks {
            line(buf, &mut y, check, t.secondary.patch(t.panel));
        }
        y += 1;
    }

    let activity = &data["activity"];
    if activity.is_object() {
        line(buf, &mut y, tr("Activity").into_owned(), t.bold.patch(t.panel));
        let headline = activity["headline"].as_str().unwrap_or("");
        let steps = activity["steps"].as_u64().unwrap_or(0);
        let errors = activity["errors"].as_u64().unwrap_or(0);
        line(buf, &mut y, format!("{headline} · {steps} {}", tr("steps")), t.secondary.patch(t.panel));
        if errors > 0 {
            line(buf, &mut y, format!("{errors} {}", tr("failed")), t.red.patch(t.panel));
        }
        let groups: Vec<String> = activity["groups"]
            .as_array()
            .map(|a| {
                a.iter()
                    .take(6)
                    .map(|g| format!("{} {}", g["count"].as_u64().unwrap_or(0), g["label"].as_str().unwrap_or("")))
                    .collect()
            })
            .unwrap_or_default();
        for group in groups {
            line(buf, &mut y, format!("  • {group}"), t.secondary.patch(t.panel));
        }
        let days: Vec<String> = activity["days"]
            .as_array()
            .map(|a| {
                a.iter()
                    .rev()
                    .take(5)
                    .map(|d| {
                        format!(
                            "  {} {} {}",
                            d["label"].as_str().unwrap_or(""),
                            "▮".repeat(d["width"].as_u64().unwrap_or(0).max(1) as usize),
                            d["count"].as_u64().unwrap_or(0)
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        for day in days {
            line(buf, &mut y, day, t.dim.patch(t.panel));
        }
        let items: Vec<String> = activity["items"]
            .as_array()
            .map(|a| {
                a.iter()
                    .take(6)
                    .map(|i| {
                        let text = i["text"].as_str().unwrap_or("");
                        let detail = i["detail"].as_str().unwrap_or("");
                        if detail.is_empty() { format!("  • {text}") } else { format!("  • {text} — {detail}") }
                    })
                    .collect()
            })
            .unwrap_or_default();
        for item in items {
            line(buf, &mut y, item, t.secondary.patch(t.panel));
        }
        y += 1;
    }

    let recent: Vec<&str> =
        data["recentWork"].as_array().map(|a| a.iter().filter_map(Value::as_str).collect()).unwrap_or_default();
    if !recent.is_empty() {
        line(buf, &mut y, tr("Recent work").into_owned(), t.bold.patch(t.panel));
        for item in recent {
            line(buf, &mut y, format!("• {item}"), t.secondary.patch(t.panel));
        }
        y += 1;
    }

    line(
        buf,
        &mut y,
        format!("{} ({})", tr("Routines"), data["routines"].as_array().map_or(0, Vec::len)),
        t.bold.patch(t.panel),
    );
    let routines: &[Value] = data["routines"].as_array().map_or(&[], Vec::as_slice);
    if routines.is_empty() {
        line(buf, &mut y, tr("No routines yet.").into_owned(), t.dim.patch(t.panel));
    }
    for routine in routines {
        let name = routine["name"].as_str().unwrap_or("?");
        let flag = if routine["enabled"] == true { "" } else { " (paused)" };
        line(buf, &mut y, format!("• {name}{flag}"), t.text.patch(t.panel));
        let schedules = routine["schedules"]
            .as_array()
            .map(|a| a.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(" · "))
            .unwrap_or_default();
        if !schedules.is_empty() {
            line(buf, &mut y, format!("  {schedules}"), t.secondary.patch(t.panel));
        }
        if let Some(next) = routine["nextIn"].as_str() {
            line(buf, &mut y, format!("  {}: {next}", tr("Next in")), t.dim.patch(t.panel));
        }
    }
    footer(buf, inner, None, "esc · shift+←→ pages");
}
