//! A bot's pages of details, as in the desktop inspector and the iPhone's details: Details (whether it
//! may use the computer, and its routines), Library (the artifacts it made and the files shared in its
//! chat), Missions, Memory and Settings. Shift+←→ moves between them.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::super::app::App;
use super::Reply;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Page {
    Details,
    Library,
    Missions,
    Memory,
    Settings,
    /// The agentes.chat behaviors (setup, presence, cards, vault) for this bot.
    Agentes,
}

impl Page {
    pub const ALL: [Self; 6] =
        [Self::Details, Self::Library, Self::Missions, Self::Memory, Self::Settings, Self::Agentes];

    pub fn label(self) -> &'static str {
        match self {
            Self::Details => "Details",
            Self::Library => "Library",
            Self::Missions => "Missions",
            Self::Memory => "Memory",
            Self::Settings => "Settings",
            Self::Agentes => "Agentes.chat",
        }
    }

    /// The page shift+← or shift+→ moves to from this one.
    pub fn step(self, k: KeyEvent) -> Option<Self> {
        if !k.modifiers.contains(KeyModifiers::SHIFT) {
            return None;
        }
        let i = Self::ALL.iter().position(|p| *p == self)?;
        let next = match k.code {
            KeyCode::Left => i.checked_sub(1)?,
            KeyCode::Right => i + 1,
            _ => return None,
        };
        Self::ALL.get(next).copied()
    }
}

impl App {
    /// Opens `page` of the bot in view, in place of the page it came from.
    pub(in crate::tui) fn open_page(&mut self, page: Page) {
        match page {
            Page::Details => self.open_routines(),
            Page::Library => self.open_library(),
            Page::Missions => self.open_missions(),
            Page::Memory => self.open_memory(),
            Page::Settings => self.edit_bot(),
            Page::Agentes => self.open_agentes_chat(),
        }
    }

    /// Opens the agentes.chat sheet for the bot in view: setup, presence and the card preview.
    pub(in crate::tui) fn open_agentes_chat(&mut self) {
        let Some(b) = self.bot().filter(|b| !b.group) else {
            self.flash("Agentes.chat belongs to a bot, not a group");
            return;
        };
        let bot = b.id.clone();
        self.call(
            "botSetup",
            serde_json::json!({"botId": bot}),
            super::super::app::After::Sheet(Reply::AgentesSetup(bot.clone())),
        );
        self.overlays.push(super::super::app::Overlay::Agentes(Box::new(
            super::super::sheets::agentes_chat_view::AgentesChatSheet { bot, ..Default::default() },
        )));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shift_arrows_move_between_pages_and_stop_at_the_ends() {
        let key = |code| KeyEvent::new(code, KeyModifiers::SHIFT);
        assert_eq!(Page::Details.step(key(KeyCode::Right)), Some(Page::Library));
        assert_eq!(Page::Memory.step(key(KeyCode::Left)), Some(Page::Missions));
        assert_eq!(Page::Library.step(key(KeyCode::Left)), Some(Page::Details));
        assert_eq!(Page::Settings.step(key(KeyCode::Right)), None);
        assert_eq!(Page::Details.step(key(KeyCode::Left)), None);
        assert_eq!(
            Page::Memory.step(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)),
            None,
            "plain → is the page's own"
        );
    }
}
