//! Roster presentation and read-only history windows, separate from the continuous mirror.

use super::{After, App, Bot, Editor, Focus, Overlay, Status};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::{Value, json};
use std::collections::HashSet;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Filter {
    #[default]
    All,
    Unread,
    Attention,
    Working,
    Pinned,
}

impl Filter {
    pub fn title(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Unread => "Unread",
            Self::Attention => "Needs attention",
            Self::Working => "Working",
            Self::Pinned => "Pinned",
        }
    }

    pub fn matches(self, bot: &Bot) -> bool {
        match self {
            Self::All => true,
            Self::Unread => bot.unread > 0,
            Self::Attention => matches!(bot.status, Status::NeedsInput | Status::Error),
            Self::Working => bot.status == Status::Working,
            Self::Pinned => bot.pinned,
        }
    }

    fn next(self) -> Self {
        match self {
            Self::All => Self::Unread,
            Self::Unread => Self::Attention,
            Self::Attention => Self::Working,
            Self::Working => Self::Pinned,
            Self::Pinned => Self::All,
        }
    }
}

pub fn project(bot: &Bot) -> &str {
    if bot.group {
        "Group chats"
    } else if bot.cwd.is_empty() {
        "No project"
    } else {
        &bot.cwd
    }
}

#[derive(Default)]
pub struct RosterTools {
    pub query: Editor,
    pub filter: Filter,
    pub by_project: bool,
    pub collapsed: HashSet<String>,
    pub history: bool,
    pub hits: Vec<Value>,
    pub context: Option<Vec<Value>>,
    pub context_offset: usize,
    pub context_anchor: Option<String>,
    pub cursor: usize,
    pub next: Option<i64>,
    pub loading: bool,
    pub error: Option<String>,
    pub sequence: u64,
    pub stopping: std::collections::HashMap<String, (Option<i64>, std::time::Instant)>,
    pub preferences_context: Option<String>,
}

impl RosterTools {
    fn reset_results(&mut self) {
        self.sequence += 1;
        self.loading = false;
        self.hits.clear();
        self.context = None;
        self.context_anchor = None;
        self.context_offset = 0;
        self.next = None;
        self.cursor = 0;
        self.error = None;
    }
}

#[derive(Clone, Copy)]
pub enum RosterRead {
    Search,
    Preview,
    Context,
}

impl App {
    pub fn navigable_roster(&self) -> Vec<&Bot> {
        self.roster()
            .into_iter()
            .filter(|bot| !self.roster_tools.by_project || !self.roster_tools.collapsed.contains(project(bot)))
            .collect()
    }

    pub(super) fn stop_from_roster(&mut self, id: &str) {
        let Some(bot) = self.bots.get(id).filter(|b| matches!(b.status, Status::Working | Status::NeedsInput)) else {
            return;
        };
        if !self.online {
            self.flash("Reconnect before stopping this bot");
            return;
        }
        if self.roster_tools.stopping.get(id).is_some_and(|(_, at)| at.elapsed().as_secs() < 20) {
            return;
        }
        self.roster_tools.stopping.insert(id.to_owned(), (bot.started_at, std::time::Instant::now()));
        self.call("stop", json!({"botId":id}), After::RosterStopped(id.to_owned()));
    }

    pub fn roster_matches(&self, bot: &Bot) -> bool {
        self.roster_tools.filter.matches(bot)
            && (self.roster_tools.history
                || self.roster_tools.query.text.split_whitespace().all(|term| {
                    let term = term.to_lowercase();
                    [&bot.name, &bot.description, &bot.cwd, &bot.backend]
                        .iter()
                        .any(|value| value.to_lowercase().contains(&term))
                }))
    }

    pub(super) fn roster_tools_key(&mut self, key: KeyEvent) -> Option<Overlay> {
        let control = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Char('l') if control => {
                self.roster_tools.query.clear();
                self.roster_tools.filter = Filter::All;
                self.roster_tools.reset_results();
            }
            KeyCode::Esc => {
                self.roster_tools.sequence += 1;
                self.roster_tools.loading = false;
                return None;
            }
            KeyCode::Tab => {
                self.roster_tools.filter = self.roster_tools.filter.next();
                self.roster_tools.reset_results();
            }
            KeyCode::Char('g') if control => {
                self.roster_tools.by_project = !self.roster_tools.by_project;
                self.save_roster_preferences();
            }
            KeyCode::Char('h') if control => {
                self.roster_tools.history = !self.roster_tools.history;
                self.roster_tools.reset_results();
            }
            KeyCode::Char('p') if control => {
                let bots = self.roster();
                let id = bots.get(self.roster_tools.cursor.min(bots.len().saturating_sub(1))).map(|b| b.id.clone());
                if let Some(id) = id {
                    self.roster_read("rosterPreview", json!({"botId":id}), RosterRead::Preview);
                }
            }
            KeyCode::Char('b') if control => {
                let bots = self.roster();
                let highlighted = bots.get(self.roster_tools.cursor.min(bots.len().saturating_sub(1)));
                if let Some(project) = highlighted.map(|bot| project(bot).to_owned()) {
                    if !self.roster_tools.collapsed.remove(&project) {
                        self.roster_tools.collapsed.insert(project);
                    }
                    self.save_roster_preferences();
                }
            }
            KeyCode::Char('n') if control && self.roster_tools.next.is_some() => self.search_roster(true),
            KeyCode::Down if self.roster_tools.context.is_some() => {
                self.roster_tools.context_offset = (self.roster_tools.context_offset + 1)
                    .min(self.roster_tools.context.as_ref().map_or(0, |entries| entries.len().saturating_sub(1)));
            }
            KeyCode::Up if self.roster_tools.context.is_some() => {
                self.roster_tools.context_offset = self.roster_tools.context_offset.saturating_sub(1);
            }
            KeyCode::Down => {
                let count = if self.roster_tools.history { self.roster_tools.hits.len() } else { self.roster().len() };
                self.roster_tools.cursor = self.roster_tools.cursor.saturating_add(1).min(count.saturating_sub(1));
            }
            KeyCode::Up => self.roster_tools.cursor = self.roster_tools.cursor.saturating_sub(1),
            KeyCode::Enter if self.roster_tools.context.is_some() => self.roster_tools.context = None,
            KeyCode::Enter if self.roster_tools.history && !self.roster_tools.hits.is_empty() => {
                if let Some(hit) =
                    self.roster_tools.hits.get(self.roster_tools.cursor.min(self.roster_tools.hits.len() - 1)).cloned()
                {
                    self.roster_tools.context_anchor = hit["id"].as_str().map(str::to_owned);
                    self.selected = hit["botId"].as_str().map(str::to_owned);
                    self.roster_read(
                        "messageContext",
                        json!({"botId":hit["botId"],"entryId":hit["id"]}),
                        RosterRead::Context,
                    );
                }
            }
            KeyCode::Enter if self.roster_tools.history => self.search_roster(false),
            KeyCode::Enter => {
                let bots = self.roster();
                let id = bots.get(self.roster_tools.cursor.min(bots.len().saturating_sub(1))).map(|b| b.id.clone());
                if let Some(id) = id {
                    self.select(&id);
                    self.focus = Focus::Chat;
                    self.chat_page = true;
                    return None;
                }
            }
            _ => {
                self.roster_tools.query.key(key);
                self.roster_tools.reset_results();
            }
        }
        Some(Overlay::RosterTools)
    }

    fn search_roster(&mut self, more: bool) {
        if self.roster_tools.query.text.trim().is_empty() {
            return;
        }
        let ids: Vec<&str> = self
            .bots
            .values()
            .filter(|b| !b.hidden && self.roster_tools.filter.matches(b))
            .map(|b| b.id.as_str())
            .collect();
        if ids.is_empty() {
            self.roster_tools.hits.clear();
            self.roster_tools.loading = false;
            return;
        }
        let before = if more { self.roster_tools.next } else { None };
        let body = json!({"query":self.roster_tools.query.text,"botIds":ids,"beforeSeq":before});
        if !more {
            self.roster_tools.hits.clear();
            self.roster_tools.cursor = 0;
        }
        self.roster_read("searchMessages", body, RosterRead::Search);
    }

    fn roster_read(&mut self, method: &'static str, body: Value, kind: RosterRead) {
        self.roster_tools.sequence += 1;
        self.roster_tools.loading = true;
        self.roster_tools.error = None;
        self.call(method, body, After::Roster(self.roster_tools.sequence, kind));
    }

    pub(super) fn roster_reply(&mut self, sequence: u64, kind: RosterRead, result: Result<Value, String>) {
        if sequence != self.roster_tools.sequence {
            return;
        }
        self.roster_tools.loading = false;
        match result {
            Err(error) => self.roster_tools.error = Some(error),
            Ok(value) => match kind {
                RosterRead::Search => {
                    self.roster_tools.hits.extend(value["results"].as_array().cloned().unwrap_or_default());
                    self.roster_tools.next = value["next"].as_i64();
                }
                RosterRead::Preview | RosterRead::Context => {
                    let entries: Vec<Value> = value["entries"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter(|entry| {
                            entry["kind"] == "user" || entry["kind"] == "agent" && entry["data"]["final"] == true
                        })
                        .cloned()
                        .collect();
                    self.roster_tools.context_offset = if matches!(kind, RosterRead::Context) {
                        entries
                            .iter()
                            .position(|entry| entry["id"].as_str() == self.roster_tools.context_anchor.as_deref())
                            .unwrap_or(0)
                    } else {
                        self.roster_tools.context_anchor = None;
                        0
                    };
                    self.roster_tools.context = Some(entries);
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_preserve_host_order_and_folded_projects_are_not_keyboard_targets() {
        let mut app = super::super::test_app();
        for value in [
            json!({"id":"a","name":"A","cwd":"/a/project","position":1,"unread":2}),
            json!({"id":"b","name":"B","cwd":"/b/project","position":2,"status":"error"}),
        ] {
            let bot = Bot::parse(&value).unwrap();
            app.bots.insert(bot.id.clone(), bot);
        }
        app.roster_tools.filter = Filter::Attention;
        assert_eq!(app.roster().iter().map(|b| b.id.as_str()).collect::<Vec<_>>(), ["b"]);
        app.roster_tools.filter = Filter::All;
        app.roster_tools.by_project = true;
        app.roster_tools.collapsed.insert("/a/project".into());
        assert_eq!(app.navigable_roster().iter().map(|b| b.id.as_str()).collect::<Vec<_>>(), ["b"]);
        assert_eq!(app.roster().len(), 2);
    }

    #[test]
    fn stale_reads_are_ignored_and_preview_never_enters_the_mirror() {
        let mut app = super::super::test_app();
        app.roster_tools.sequence = 2;
        app.roster_reply(1, RosterRead::Search, Ok(json!({"results":[{"id":"late"}]})));
        assert_eq!(app.roster_tools.hits.len(), 0);
        app.roster_reply(
            2,
            RosterRead::Preview,
            Ok(json!({"entries":[{"id":"preview","kind":"user","data":{"text":"hi"}}]})),
        );
        assert_eq!(app.roster_tools.context.as_ref().unwrap().len(), 1);
        assert!(app.entries.is_empty());
    }

    #[test]
    fn folding_targets_the_highlighted_bot_instead_of_the_open_chat() {
        let mut app = super::super::test_app();
        for value in [
            json!({"id":"a","name":"A","cwd":"/a/project","position":1}),
            json!({"id":"b","name":"B","cwd":"/b/project","position":2}),
        ] {
            let bot = Bot::parse(&value).unwrap();
            app.bots.insert(bot.id.clone(), bot);
        }
        app.selected = Some("a".into());
        app.roster_tools.cursor = 1;
        app.roster_tools_key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL));
        assert!(app.roster_tools.collapsed.contains("/b/project"));
        assert!(!app.roster_tools.collapsed.contains("/a/project"));
        app.roster_tools_key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL));
        assert!(app.roster_tools.collapsed.is_empty());
    }

    #[test]
    fn changing_search_or_filter_retires_errors_and_pending_results() {
        for key in [
            KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE),
            KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE),
            KeyEvent::new(KeyCode::Char('h'), KeyModifiers::CONTROL),
            KeyEvent::new(KeyCode::Char('l'), KeyModifiers::CONTROL),
        ] {
            let mut app = super::super::test_app();
            app.roster_tools.error = Some("host unavailable".into());
            app.roster_tools.loading = true;
            app.roster_tools.next = Some(10);
            app.roster_tools.cursor = 3;
            app.roster_tools_key(key);
            assert!(app.roster_tools.error.is_none());
            assert!(!app.roster_tools.loading);
            assert!(app.roster_tools.next.is_none());
            assert_eq!(app.roster_tools.cursor, 0);
            app.roster_reply(0, RosterRead::Search, Ok(json!({"results":[{"id":"late"}]})));
            assert_eq!(app.roster_tools.hits.len(), 0);
        }
    }
}
