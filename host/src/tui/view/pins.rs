//! The chat's pinned messages, newest first, with the pinner and the author.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};

use super::super::app::App;
use super::super::l10n::tr;
use super::super::manage::PinsSheet;
use super::super::md::truncate;
use super::buffer::{centered, frame_box, put, put_line};
use super::style::theme;
use super::time::when;

/// Who pinned a message, as the list says it.
fn pinner(pin: &serde_json::Value, agent: &str) -> String {
    if pin["pinnedBy"]["kind"] == "agent" {
        return tr("{name} (agent)").replace("{name}", agent);
    }
    pin["pinnedBy"]["device"].as_str().map_or_else(|| tr("You").into_owned(), str::to_owned)
}

/// A pinned message's author: a person, or this chat's agent.
fn author(app: &App, pin: &serde_json::Value, agent: &str) -> String {
    if pin["author"] == "user" {
        return tr("You").into_owned();
    }
    pin["authorBotId"].as_str().and_then(|id| app.bots.get(id)).map_or_else(|| agent.to_owned(), |b| b.name.clone())
}

pub(super) fn pins(buf: &mut Buffer, area: Rect, app: &App, s: &PinsSheet) {
    let t = theme();
    let r = centered(area, 90, area.height.saturating_sub(4));
    let inner = frame_box(buf, r, t.text, t.panel, Some((tr("Pinned messages").as_ref(), t.text)));
    if inner.height < 6 {
        return;
    }
    let width = usize::from(inner.width);
    put(
        buf,
        inner.x,
        inner.y + 1,
        inner.width,
        &truncate(&tr("Kept at hand in this chat. A message can be unpinned by whoever reads it here."), width),
        t.secondary.patch(t.panel),
    );
    let agent = app.bots.get(&s.bot).map_or_else(|| tr("the agent").into_owned(), |b| b.name.clone());
    let mut y = inner.y + 3;
    match &s.pins {
        None => {
            put(buf, inner.x, y, inner.width, &tr("Loading…"), t.dim.patch(t.panel));
        }
        Some(pins) if pins.is_empty() => {
            put(
                buf,
                inner.x,
                y,
                inner.width,
                &truncate(&tr("Nothing is pinned here yet. Open a message's menu and pick Pin."), width),
                t.dim.patch(t.panel),
            );
        }
        Some(pins) => {
            let room = usize::from(inner.bottom().saturating_sub(y)).saturating_sub(3) / 3;
            let room = room.max(1);
            let off = s.cursor.saturating_sub(room - 1);
            for (i, pin) in pins.iter().enumerate().skip(off).take(room) {
                if y + 2 >= inner.bottom() {
                    break;
                }
                let selected = i == s.cursor;
                let base = if selected { t.sel } else { t.panel };
                if selected {
                    for row in 0..3 {
                        super::buffer::restyle(
                            buf,
                            Rect::new(inner.x.saturating_sub(1), y + row, inner.width + 2, 1),
                            t.sel,
                        );
                    }
                }
                let title = pin["threadTitle"].as_str().filter(|s| !s.is_empty());
                let mut head = vec![Span::raw(" ")];
                if let Some(title) = title {
                    head.push(Span::styled(format!("{title} · "), t.dim.patch(base)));
                }
                let text = pin["text"].as_str().filter(|s| !s.is_empty()).unwrap_or("A message without text");
                head.push(Span::styled(tr(text).into_owned(), t.text.patch(base)));
                put_line(buf, inner.x, y, inner.width, &Line::from(head));
                let foot = format!("   {} · {}", pinner(pin, &agent), author(app, pin, &agent));
                put(buf, inner.x, y + 1, inner.width, &truncate(&foot, width), t.dim.patch(base));
                let at = pin["pinnedAt"].as_i64().unwrap_or_default();
                let right = inner.right().saturating_sub(10);
                put(buf, right, y, 10, &when(at), t.dim.patch(base));
                y += 3;
            }
        }
    }
    if let Some(e) = &s.error {
        let err = format!(" {}: {}", tr("Couldn't load the pins."), e);
        put(buf, inner.x, inner.bottom().saturating_sub(1), inner.width, &truncate(&err, width), t.red.patch(t.panel));
    }
    let keys = format!(" ↑↓ {} · enter {} · p {} · r {} · esc", tr("pick"), tr("open"), tr("Unpin"), tr("reload"));
    put(buf, inner.x, inner.bottom().saturating_sub(2), inner.width, &truncate(&keys, width), t.dim.patch(t.panel));
}
