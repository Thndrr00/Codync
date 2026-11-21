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

    let recent: Vec<&str> =
        data["recentWork"].as_array().map(|a| a.iter().filter_map(Value::as_str).collect()).unwrap_or_default();
    if !recent.is_empty() {
        line(buf, &mut y, tr("Recent work"), t.bold.patch(t.panel));
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
        line(buf, &mut y, tr("No routines yet."), t.dim.patch(t.panel));
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
