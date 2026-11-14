//! The voice providers' keys and the local voice server: what each provider does, whether this
//! computer holds its key, and where speech requests actually go.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use super::super::l10n::tr;
use super::super::manage::VoiceSheet;
use super::super::md::truncate;
use super::buffer::{centered, frame_box, put, restyle, rput};
use super::style::theme;

pub fn voice(buf: &mut Buffer, area: Rect, sheet: &VoiceSheet) {
    let t = theme();
    let frame = centered(area, 90, 25);
    let inner = frame_box(buf, frame, t.text, t.panel, Some((tr("Voice providers").as_ref(), t.text)));
    let mut y = inner.y;
    let bottom = inner.bottom().saturating_sub(2);
    let intro = tr("The key stays on this computer. The host checks it with the provider before keeping it.");
    if y < bottom {
        put(buf, inner.x, y, inner.width, &intro, t.secondary.patch(t.panel));
        y += 2;
    }
    // Where speech actually goes, before the providers whose keys it can replace.
    if y < bottom {
        let saved = sheet.server.as_ref().and_then(|v| v["baseUrl"].as_str()).unwrap_or_default();
        let effective = sheet.server_line();
        let state =
            if saved.is_empty() { tr("Off").into_owned() } else { truncate(&saved, usize::from(inner.width) / 2) };
        let name = format!("{}  {state}", tr("Local voice server"));
        put(buf, inner.x, y, inner.width.saturating_sub(28), &name, t.bold.patch(t.panel));
        if !effective.is_empty() {
            rput(buf, inner.right(), y, &truncate(&effective, 24), t.secondary.patch(t.panel));
        }
        y += 1;
        if let Some((ok, message)) = &sheet.tested {
            let style = if *ok { t.secondary } else { t.red };
            let line = format!("{}: {message}", if *ok { tr("Server works") } else { tr("The server didn't answer") });
            put(buf, inner.x, y, inner.width, &truncate(&line, usize::from(inner.width)), style.patch(t.panel));
            y += 1;
        }
        y += 1;
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
        "↵ add key  s local voice server  t test it  d remove key  r reload  esc close"
    } else {
        "↵ add key  s local voice server  t test it  r reload  esc close"
    };
    put(buf, inner.x, inner.bottom() - 1, inner.width, footer, t.dim.patch(t.panel));
}
