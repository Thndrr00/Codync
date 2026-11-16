//! The learned-preferences sheet: what the bot proposes, and the two settings that change how much it learns.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use super::super::app::LearningSheet;
use super::super::app::learning::{inbox_title, kind_label};
use super::super::l10n::tr;
use super::super::md::truncate;
use super::buffer::{centered, frame_box, put, restyle, u};
use super::style::theme;

pub(super) fn learning(buf: &mut Buffer, area: Rect, s: &LearningSheet) {
    let t = theme();
    let count = s.inbox.as_ref().and_then(|v| v["proposals"].as_array()).map_or(0, Vec::len);
    let r = centered(area, 92, u((count * 5 + 10).clamp(12, usize::from(area.height.saturating_sub(2)).max(12))));
    let title = format!("{} · {}", tr(&inbox_title(count)), s.bot_name);
    let inner = frame_box(buf, r, t.text, t.panel, Some((title.as_str(), t.text)));
    if inner.height < 7 {
        return;
    }
    let width = usize::from(inner.width);
    let mut y = inner.y + 1;
    let enabled = s.inbox.as_ref().and_then(|v| v["enabled"].as_bool()).unwrap_or(true);
    let auto = s.inbox.as_ref().and_then(|v| v["autoApproveAfter"].as_u64()).unwrap_or(0);
    put(
        buf,
        inner.x,
        y,
        inner.width,
        &truncate(
            &format!(
                "Learning from my corrections and instructions: {} · save without review after: {}",
                if enabled { "on" } else { "off" },
                if auto == 0 { "never".to_owned() } else { format!("{auto} times") }
            ),
            width,
        ),
        t.secondary.patch(t.panel),
    );
    y += 2;
    match s.inbox.as_ref().and_then(|v| v["proposals"].as_array()) {
        None => put(buf, inner.x, y, inner.width, &tr("Loading…"), t.dim.patch(t.panel)),
        Some(items) if items.is_empty() => {
            put(buf, inner.x, y, inner.width, &tr("Nothing to review."), t.dim.patch(t.panel))
        }
        Some(items) => {
            let room = (usize::from(inner.bottom().saturating_sub(y)).saturating_sub(2) / 5).max(1);
            let off = s.sel.saturating_sub(room - 1);
            for (i, p) in items.iter().enumerate().skip(off).take(room) {
                let selected = i == s.sel;
                if selected {
                    restyle(buf, Rect::new(inner.x.saturating_sub(1), y, inner.width + 2, 4), t.sel);
                }
                let base = if selected { t.sel } else { t.panel };
                put(
                    buf,
                    inner.x,
                    y,
                    inner.width,
                    &truncate(p["text"].as_str().unwrap_or_default(), width),
                    t.text.patch(base),
                );
                put(buf, inner.x, y + 1, inner.width, &truncate(&kind_label(p), width), t.dim.patch(base));
                put(
                    buf,
                    inner.x,
                    y + 2,
                    inner.width,
                    &truncate(&format!("{}: {}", tr("From"), p["source"].as_str().unwrap_or_default()), width),
                    t.secondary.patch(base),
                );
                y += 5;
            }
        }
    }
    put(
        buf,
        inner.x,
        inner.bottom() - 1,
        inner.width,
        &tr("↑↓ pick  a remember  x dismiss  t learning on/off  , . repeats  r refresh  esc close"),
        t.dim.patch(t.panel),
    );
}
