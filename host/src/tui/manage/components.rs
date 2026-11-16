//! The component gallery in the terminal: the cards Codync can draw, their props, the data functions
//! each may name, and the bots a card is withheld from.
//!
//! The list is read-only: authoring a card and withholding it per bot are the desktop's playground and
//! the phones' gallery, where the schema can be drawn as a form. The terminal shows what a bot may use.

use super::super::app::{App, Overlay};
use super::super::view::{centered, frame_box, hline, put, theme};
use super::Reply;
use crate::tui::l10n::tr;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use serde_json::{Value, json};

/// The sheet: the host's `listComponents` reply once it arrives.
#[derive(Default)]
pub struct ComponentsSheet {
    pub gallery: Option<Value>,
    pub cursor: usize,
    pub error: Option<String>,
}

impl ComponentsSheet {
    /// The catalogue entries, in the host's order.
    pub fn components(&self) -> Vec<Value> {
        self.gallery.as_ref().and_then(|g| g["components"].as_array()).cloned().unwrap_or_default()
    }

    fn catalog(&self) -> Vec<Value> {
        self.components()
    }

    /// The bots that may not use `name`, as their names.
    fn withheld(&self, name: &str) -> Vec<String> {
        self.gallery
            .as_ref()
            .and_then(|g| g["bots"].as_array())
            .into_iter()
            .flatten()
            .filter(|bot| bot["withheld"].as_array().is_some_and(|list| list.iter().any(|held| held == name)))
            .map(|bot| bot["name"].as_str().unwrap_or_default().to_owned())
            .collect()
    }
}

impl App {
    pub fn open_components(&mut self) {
        self.overlays.push(Overlay::Components(Box::<ComponentsSheet>::default()));
        self.sheet("listComponents", json!({}), Reply::Components);
    }

    pub(in crate::tui) fn components_key(&mut self, mut s: ComponentsSheet, k: KeyEvent) -> Option<Overlay> {
        let n = s.catalog().len();
        match k.code {
            KeyCode::Esc | KeyCode::Char('q') => return None,
            KeyCode::Down | KeyCode::Char('j') => s.cursor = (s.cursor + 1).min(n.saturating_sub(1)),
            KeyCode::Up | KeyCode::Char('k') => s.cursor = s.cursor.saturating_sub(1),
            KeyCode::Char('r') => {
                self.overlays.push(Overlay::Components(Box::new(s)));
                self.sheet("listComponents", json!({}), Reply::Components);
                return None;
            }
            _ => {}
        }
        Some(Overlay::Components(Box::new(s)))
    }

    pub(in crate::tui) fn components_loaded(&mut self, gallery: Value) {
        if let Some(Overlay::Components(s)) = self.overlays.iter_mut().find(|o| matches!(o, Overlay::Components(_))) {
            let count = gallery["components"].as_array().map_or(0, Vec::len);
            s.cursor = s.cursor.min(count.saturating_sub(1));
            s.gallery = Some(gallery);
            s.error = None;
        }
    }
}

/// One component's row: its name, what it is, and the props it takes.
pub fn draw(buf: &mut Buffer, area: Rect, app: &App, s: &ComponentsSheet) {
    let t = theme();
    let r = centered(area, 84, 26);
    let inner = frame_box(buf, r, t.text, t.panel, Some((&tr("Components"), t.text)));
    let mut y = inner.y;
    put(buf, inner.x, y, inner.width, &tr("The cards Codync can draw in a chat."), t.dim.patch(t.panel));
    y += 2;
    if s.gallery.is_none() && s.error.is_none() {
        put(buf, inner.x, y, inner.width, &tr("Loading…"), t.dim.patch(t.panel));
        return;
    }
    if let Some(error) = &s.error {
        put(
            buf,
            inner.x,
            y,
            inner.width,
            &crate::tui::md::truncate(&tr(error), usize::from(inner.width)),
            t.red.patch(t.panel),
        );
        y += 1;
    }
    let components = s.components();
    let width = usize::from(inner.width);
    for (index, component) in components.iter().enumerate() {
        if y >= inner.bottom().saturating_sub(3) {
            break;
        }
        let selected = index == s.cursor;
        let style = if selected { t.sel } else { t.text.patch(t.panel) };
        let name = component["name"].as_str().unwrap_or_default();
        let summary = tr(component["summary"].as_str().unwrap_or_default());
        let withheld = s.withheld(name);
        let mark = if withheld.is_empty() {
            String::new()
        } else {
            format!(" · {}", tr("withheld from {bots}").replace("{bots}", &withheld.join(", ")))
        };
        let line = format!("{name} · v{}{mark}", component["version"]);
        put(buf, inner.x, y, inner.width, &crate::tui::md::truncate(&line, width), style);
        y += 1;
        put(
            buf,
            inner.x + 2,
            y,
            inner.width.saturating_sub(2),
            &crate::tui::md::truncate(&summary, width.saturating_sub(2)),
            t.dim.patch(t.panel),
        );
        y += 1;
    }
    y += 1;
    if let Some(component) = components.get(s.cursor) {
        hline(buf, inner.x, y, inner.width, t.panel);
        y += 1;
        let props: Vec<&str> = component["schema"]["properties"]
            .as_object()
            .map(|props| props.keys().map(String::as_str).collect())
            .unwrap_or_default();
        let functions: Vec<&str> =
            component["data"].as_array().into_iter().flatten().filter_map(Value::as_str).collect();
        let props = format!("{}: {}", tr("Props"), props.join(", "));
        let functions = format!(
            "{}: {}",
            tr("Data functions"),
            if functions.is_empty() { tr("None") } else { functions.join(", ") }
        );
        put(buf, inner.x, y, inner.width, &crate::tui::md::truncate(&props, width), t.secondary.patch(t.panel));
        y += 1;
        put(buf, inner.x, y, inner.width, &crate::tui::md::truncate(&functions, width), t.secondary.patch(t.panel));
    }
    let _ = app;
    let keys = tr("↑↓ move · r reload · q close");
    put(buf, inner.x, inner.bottom().saturating_sub(1), inner.width, &keys, t.dim.patch(t.panel));
}
