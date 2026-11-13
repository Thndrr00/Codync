//! The server's Library sheet: every bot's artifacts and files, newest first, with the bot's name. A file's text opens
//! in the same sheet; an image is marked, since a terminal shows no pictures.

use super::{App, Buffer, Rect, centered, footer, frame_box, put, row, rput, theme, window};
use crate::tui::l10n::tr;
use crate::tui::manage::{ServerLibrary, ServerLibraryItem};
use crate::tui::md::truncate;
use crate::tui::view::when;
use ratatui::widgets::{Paragraph, Widget, Wrap};

pub fn draw(buf: &mut Buffer, area: Rect, _app: &App, s: &ServerLibrary) {
    let t = theme();
    let r = centered(area, 90, 28);
    let title = format!("{} · {}", tr("Server Library"), tr(s.filter.label()));
    let inner = frame_box(buf, r, t.text, t.panel, Some((&title, t.text)));
    if let Some(preview) = &s.preview {
        put(buf, inner.x, inner.y, inner.width, &tr(&preview.name), t.bold.patch(t.panel));
        let body = Rect { x: inner.x, y: inner.y + 1, width: inner.width, height: inner.height.saturating_sub(3) };
        Paragraph::new(preview.text.as_str())
            .wrap(Wrap { trim: false })
            .scroll((s.scroll, 0))
            .style(t.panel)
            .render(body, buf);
        footer(buf, inner, s.error.as_deref(), "↑↓ scroll · esc back");
        return;
    }
    match s.items.as_deref() {
        None => {
            put(buf, inner.x, inner.y, inner.width, &tr("Loading…"), t.dim.patch(t.panel));
        }
        Some(_) if s.visible().is_empty() => {
            put(buf, inner.x, inner.y, inner.width, &tr("Library is empty"), t.bold.patch(t.panel));
            let note = "Artifacts and files from every bot on this computer show here.";
            put(buf, inner.x, inner.y + 1, inner.width, &tr(note), t.secondary.patch(t.panel));
        }
        Some(_) => {
            let rows = s.visible();
            let h = usize::from(inner.height.saturating_sub(3));
            let room = usize::from(inner.width.saturating_sub(10));
            for (line, (index, item)) in rows.iter().enumerate().skip(window(s.cursor, h)).take(h).enumerate() {
                let y = inner.y + u16::try_from(line).unwrap_or(0);
                let base = row(buf, inner, y, index == s.cursor);
                put(
                    buf,
                    inner.x,
                    y,
                    u16::try_from(room + 2).unwrap_or(inner.width),
                    &truncate(&label(item), room + 2),
                    t.text.patch(base),
                );
                rput(buf, inner.right(), y, &when(item.at()), t.dim.patch(base));
            }
        }
    }
    footer(buf, inner, s.error.as_deref(), "↵ preview or open · f save to Downloads · r rename · tab filter · esc");
}

/// A row: its bot's name, what it is, and `▣` when the host says it is an image.
fn label(item: &ServerLibraryItem) -> String {
    item.label()
}
