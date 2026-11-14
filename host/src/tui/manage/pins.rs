//! A chat's pinned messages: a decision, a path or a detail that stays at hand. `p` takes a pin back.

use crossterm::event::{KeyCode, KeyEvent};

use super::super::app::{App, Overlay};
use super::PinsSheet;

impl App {
    pub(in crate::tui) fn pins_key(&mut self, mut p: PinsSheet, k: KeyEvent) -> Option<Overlay> {
        let n = p.pins.as_ref().map_or(0, Vec::len);
        let current = p.current().cloned();
        let bot = p.bot.clone();
        match (k.code, current) {
            (KeyCode::Esc | KeyCode::Char('q' | 'P'), _) => return None,
            (KeyCode::Down | KeyCode::Char('j'), _) => p.cursor = (p.cursor + 1).min(n.saturating_sub(1)),
            (KeyCode::Up | KeyCode::Char('k'), _) => p.cursor = p.cursor.saturating_sub(1),
            (KeyCode::Char('r'), _) => {
                self.overlays.push(Overlay::Pins(Box::new(p)));
                self.fetch_pins(&bot);
                return None;
            }
            (KeyCode::Enter | KeyCode::Char('o'), Some(pin)) => {
                let Some(root) = pin["threadId"].as_str().map(str::to_owned) else {
                    self.flash("That message is in the chat itself");
                    return Some(Overlay::Pins(Box::new(p)));
                };
                self.overlays.push(Overlay::Pins(Box::new(p)));
                self.open_thread(&bot, &root);
                return None;
            }
            (KeyCode::Char('p'), Some(pin)) => {
                let Some(message) = pin["messageId"].as_str().map(str::to_owned) else {
                    return Some(Overlay::Pins(Box::new(p)));
                };
                // Taken back here at once; the message's new rev confirms it.
                if let Some(pins) = &mut p.pins {
                    pins.retain(|one| one["messageId"] != message.as_str());
                }
                p.cursor = p.cursor.min(n.saturating_sub(2));
                self.pin_message(&bot, &message, false);
            }
            _ => {}
        }
        Some(Overlay::Pins(Box::new(p)))
    }
}
