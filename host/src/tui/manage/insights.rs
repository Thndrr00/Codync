//! A bot's Insights page: the host-computed summary of its routines, model and daily use.

use crossterm::event::KeyEvent;
use serde_json::{Value, json};

use super::super::app::{App, Overlay};
use super::{Insights, Page, Reply};

impl App {
    /// Opens the Insights page for the bot in view.
    pub(in crate::tui) fn open_insights(&mut self) {
        let Some(b) = self.bot().filter(|b| !b.group) else {
            self.flash("Insights belong to a bot, not a group");
            return;
        };
        let bot = b.id.clone();
        self.sheet("botInsights", json!({"botId": bot}), Reply::Insights(bot.clone()));
        self.overlays.push(Overlay::Insights(Insights { bot, data: None }));
    }

    /// Shift+←→ moves to the neighbouring page; a normal key closes the sheet.
    pub(in crate::tui) fn insights_key(&mut self, k: KeyEvent) -> Option<Overlay> {
        if let Some(page) = Page::Insights.step(k) {
            self.open_page(page);
        }
        None
    }

    /// The fetched insights, shown in the open sheet.
    pub(in crate::tui) fn insights_loaded(&mut self, bot: &str, data: &Value) {
        for o in &mut self.overlays {
            if let Overlay::Insights(i) = o
                && i.bot == bot
            {
                i.data = Some(data.clone());
            }
        }
    }
}
