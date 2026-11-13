//! The voice providers' keys: what each provider does, and whether this computer holds its key.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use super::super::l10n::tr;
use super::super::manage::VoiceSheet;
use super::super::md::truncate;
use super::buffer::{centered, frame_box, put, restyle, rput};
use super::style::theme;

pub fn voice(buf: &mut Buffer, area: Rect, sheet: &VoiceSheet) {
    let t = theme();
    let frame = centered(area, 90, 22);
    let inner = frame_box(buf, frame, t.text, t.panel, Some((tr("Voice providers").as_ref(), t.text)));
    let mut y = inner.y;
    let bottom = inner.bottom().saturating_sub(2);
    let intro = tr("The key stays on this computer. The host checks it with the provider before keeping it.");
    if y < bottom {
        put(buf, inner.x, y, inner.width, &intro, t.secondary.patch(t.panel));
        y += 2;
    }
    match &sheet.providers {
        None => {
            put(buf, inner.x, y, inner.width, tr("Loading…").as_ref(), t.dim.patch(t.panel));
        }
        Some(providers) if providers.is_empty() => {}
        Some(providers) => {
            for (index, provider) in providers.iter().enumerate() {
                if y >= bottom {
                    break;
                }
                let selected = index == sheet.cursor;
                let base = if selected { t.sel } else { t.panel };
                if selected {
                    restyle(buf, Rect::new(inner.x - 1, y, inner.width + 2, 1), t.sel);
                }
                let name = format!("{}  {}", provider.name, tr(provider.role));
                put(
                    buf,
                    inner.x,
                    y,
                    inner.width.saturating_sub(24),
                    &truncate(&name, usize::from(inner.width) - 24),
                    t.bold.patch(base),
                );
                let state = if provider.configured { tr("Key saved") } else { tr("No key") };
                let style = if provider.configured { t.secondary } else { t.dim };
                rput(buf, inner.right(), y, state.as_ref(), style.patch(base));
                y += 1;
            }
        }
    }
    if let Some(error) = &sheet.error {
        put(buf, inner.x, bottom, inner.width, &truncate(error, usize::from(inner.width)), t.red.patch(t.panel));
    }
    let footer = if sheet.current().is_some_and(|p| p.configured) {
        "↵ add key  d remove key  r reload  esc close"
    } else {
        "↵ add key  r reload  esc close"
    };
    put(buf, inner.x, inner.bottom() - 1, inner.width, footer, t.dim.patch(t.panel));
}
