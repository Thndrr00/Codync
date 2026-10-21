//! Keyboard equivalent of the clients' roster search, attention filter and preview.
use super::super::app::{App, roster_tools::project};
use super::buffer::{centered, frame_box, put};
use super::overlay::field_line;
use super::style::theme;
use ratatui::{buffer::Buffer, layout::Rect};

pub(super) fn roster_tools(buf: &mut Buffer, area: Rect, app: &App) {
    let t = theme();
    let state = &app.roster_tools;
    let rect = centered(area, 86, area.height.saturating_sub(4).max(10));
    let inner = frame_box(
        buf,
        rect,
        t.text,
        t.panel,
        Some(("Roster · tab filter · ^h history · ^g projects · ^p preview", t.text)),
    );
    field_line(buf, inner, inner.y, &state.query, "name, project, agent · history: type, then Enter", true);
    put(
        buf,
        inner.x,
        inner.y + 1,
        inner.width,
        &format!("{} · {}", state.filter.title(), if state.by_project { "By project" } else { "Manual order" }),
        t.dim,
    );
    let mut y = inner.y + 3;
    if state.loading {
        put(buf, inner.x, y, inner.width, "Loading…", t.secondary);
        return;
    }
    if let Some(error) = &state.error {
        put(buf, inner.x, y, inner.width, &format!("Request failed: {error}"), t.red);
        return;
    }
    if let Some(entries) = &state.context {
        if entries.is_empty() {
            put(buf, inner.x, y, inner.width, "No messages yet", t.dim);
        }
        for entry in entries.iter().skip(state.context_offset) {
            if y + 2 >= inner.bottom() {
                break;
            }
            let lane = if entry["threadId"].is_string() { "Thread" } else { "Main chat" };
            put(
                buf,
                inner.x,
                y,
                inner.width,
                &format!(
                    "{}{lane} · {}",
                    if entry["id"].as_str() == state.context_anchor.as_deref() { "> " } else { "" },
                    entry["kind"].as_str().unwrap_or("message")
                ),
                t.secondary,
            );
            y += 1;
            let lines = super::super::md::wrap(
                &[ratatui::text::Span::styled(entry["data"]["text"].as_str().unwrap_or_default().to_owned(), t.text)],
                usize::from(inner.width),
                &[],
                &[],
            );
            for line in lines {
                if y + 1 >= inner.bottom() {
                    break;
                }
                super::buffer::put_line(buf, inner.x, y, inner.width, &line);
                y += 1;
            }
            y += 1;
        }
        put(
            buf,
            inner.x,
            inner.bottom().saturating_sub(1),
            inner.width,
            "Preview does not mark as read · Enter returns",
            t.dim,
        );
    } else if state.history {
        if state.hits.is_empty() {
            put(
                buf,
                inner.x,
                y,
                inner.width,
                "Enter searches full history · no matches after a completed search",
                t.dim,
            );
        }
        for (index, hit) in state.hits.iter().enumerate().skip(state.cursor.saturating_sub(3)) {
            if y + 1 >= inner.bottom() {
                break;
            }
            let name = hit["botId"].as_str().and_then(|id| app.bots.get(id)).map_or("Bot", |bot| bot.name.as_str());
            let lane = if hit["threadId"].is_string() { "Thread" } else { "Main chat" };
            put(
                buf,
                inner.x,
                y,
                inner.width,
                &format!("{name} · {lane} · {}", hit["text"].as_str().unwrap_or_default()),
                if index == state.cursor { t.bold } else { t.secondary },
            );
            y += 2;
        }
        if state.next.is_some() {
            put(buf, inner.x, inner.bottom().saturating_sub(1), inner.width, "^n more messages", t.dim);
        }
    } else {
        let bots = app.roster();
        if bots.is_empty() {
            put(buf, inner.x, y, inner.width, "No bots match these filters", t.dim);
        }
        for (index, bot) in bots.iter().enumerate().skip(state.cursor.saturating_sub(3)) {
            if y + 1 >= inner.bottom() {
                break;
            }
            put(
                buf,
                inner.x,
                y,
                inner.width,
                &format!("{} · {} · {}", bot.name, bot.backend, project(bot)),
                if index == state.cursor { t.bold } else { t.secondary },
            );
            y += 2;
        }
    }
}
