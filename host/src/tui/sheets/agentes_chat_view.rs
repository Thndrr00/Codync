//! The agentes.chat behaviors in one sheet: setup, presence, a stretch card and an approval card.
//! It renders the answers the host already gave; it never decides any of them.

use super::{App, Buffer, Rect, centered, footer, frame_box, put, truncate};
use crate::tui::l10n::tr;

const KEYS: &str = "esc close";

/// The state the app keeps for this sheet, filled by async host calls in `app`.
#[derive(Default)]
pub struct AgentesChatSheet {
    pub bot: String,
    pub setup: Option<serde_json::Value>,
    pub presence: Option<serde_json::Value>,
    pub stretch: Option<serde_json::Value>,
    pub approval: Option<serde_json::Value>,
    pub error: Option<String>,
}

pub fn draw(buf: &mut Buffer, area: Rect, app: &App, sheet: &AgentesChatSheet) {
    let t = super::theme();
    let r = centered(area, 80, 22);
    let title = format!("{} · Agentes.chat", super::name_of(app, &sheet.bot));
    let inner = frame_box(buf, r, t.text, t.panel, Some((&title, t.text)));

    let mut y = inner.y;
    let mut line = |text: String, dim: bool| {
        put(
            buf,
            inner.x,
            y,
            inner.width,
            &truncate(&text, usize::from(inner.width)),
            if dim { t.dim.patch(t.panel) } else { t.text.patch(t.panel) },
        );
        y += 1;
    };

    if let Some(error) = &sheet.error {
        line(format!("! {error}"), false);
    }

    line(tr("Setup").to_string(), false);
    match &sheet.setup {
        Some(setup) => {
            if let Some(steps) = setup["steps"].as_array() {
                for step in steps {
                    let mark = if step["done"] == true { "✓" } else { "○" };
                    line(format!("  {mark} {}", step["label"].as_str().unwrap_or("")), false);
                }
            }
        }
        None => line(tr("Loading…").to_string(), true),
    }

    line(String::new(), true);
    line(tr("Presence").to_string(), false);
    line(format!("  {}", sheet.presence.as_ref().and_then(|p| p["text"].as_str()).unwrap_or("—")), true);

    line(String::new(), true);
    line(tr("Cards").to_string(), false);
    if let Some(card) = &sheet.stretch {
        line(format!("  {} · {}", card["type"].as_str().unwrap_or(""), card["bodyCode"].as_str().unwrap_or("")), true);
    }
    if let Some(approval) = &sheet.approval {
        line(format!("  {}", approval["reason"].as_str().unwrap_or("")), false);
        line(format!("  status: {}", approval["status"].as_str().unwrap_or("")), true);
    }

    footer(buf, inner, sheet.error.as_deref(), KEYS);
}
