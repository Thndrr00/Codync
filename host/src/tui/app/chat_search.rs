//! Search inside one chat: what it said, what its pages hold and what its memory keeps, best match first, each
//! with its snippet and date. Typing narrows on Enter; a person can keep one side of the chat, one channel of it,
//! add the tool traces, open a match in its context, and read a page or a note. The host does the searching; this
//! sheet only keeps the keys.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::{Value, json};

use super::{After, App, Editor, Overlay};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Side {
    #[default]
    Everyone,
    Mine,
    Bot,
}

impl Side {
    pub fn title(self) -> &'static str {
        match self {
            Self::Everyone => "Everyone",
            Self::Mine => "Me",
            Self::Bot => "Bot",
        }
    }

    fn next(self) -> Self {
        match self {
            Self::Everyone => Self::Mine,
            Self::Mine => Self::Bot,
            Self::Bot => Self::Everyone,
        }
    }

    /// The `author` the host understands; everyone leaves it off.
    fn wire(self) -> Option<&'static str> {
        match self {
            Self::Everyone => None,
            Self::Mine => Some("user"),
            Self::Bot => Some("agent"),
        }
    }
}

/// Where in the chat the words are looked for: everywhere, the chat itself, or its side conversations.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Scope {
    #[default]
    Everything,
    Main,
    Threads,
}

impl Scope {
    pub fn title(self) -> &'static str {
        match self {
            Self::Everything => "Everything",
            Self::Main => "This chat",
            Self::Threads => "Threads",
        }
    }

    fn next(self) -> Self {
        match self {
            Self::Everything => Self::Main,
            Self::Main => Self::Threads,
            Self::Threads => Self::Everything,
        }
    }

    /// The `channel` the host understands; everywhere leaves it off.
    fn wire(self) -> Option<&'static str> {
        match self {
            Self::Everything => None,
            Self::Main => Some("main"),
            Self::Threads => Some("threads"),
        }
    }
}

#[derive(Clone, Copy)]
pub enum ChatSearchRead {
    Search,
    Context,
    Page,
    Note,
}

pub struct ChatSearchSheet {
    pub bot_id: String,
    pub bot_name: String,
    pub query: Editor,
    pub side: Side,
    pub scope: Scope,
    pub tools: bool,
    pub hits: Vec<Value>,
    pub sel: usize,
    /// The messages around the hit that was opened, and where its own line is.
    pub context: Option<Vec<Value>>,
    pub context_offset: usize,
    /// The hit whose context is open or being fetched.
    pub anchor: Option<String>,
    /// A page's or note's title and body, shown over the results.
    pub page: Option<(String, String)>,
    /// Whether that page is a memory note rather than a document.
    pub note: bool,
    pub loading: bool,
    pub error: Option<String>,
    pub sequence: u64,
}

impl ChatSearchSheet {
    fn reset(&mut self) {
        self.sequence += 1;
        self.loading = false;
        self.hits.clear();
        self.sel = 0;
        self.context = None;
        self.context_offset = 0;
        self.anchor = None;
        self.page = None;
        self.note = false;
        self.error = None;
    }

    fn picked(&self) -> Option<&Value> {
        self.hits.get(self.sel.min(self.hits.len().saturating_sub(1)))
    }
}

impl App {
    pub(super) fn open_chat_search(&mut self) {
        let Some(bot) = self.bot() else {
            self.flash("Pick a chat first");
            return;
        };
        let sheet = ChatSearchSheet {
            bot_id: bot.id.clone(),
            bot_name: bot.name.clone(),
            query: Editor::default(),
            side: Side::Everyone,
            scope: Scope::Everything,
            tools: false,
            hits: Vec::new(),
            sel: 0,
            context: None,
            context_offset: 0,
            anchor: None,
            page: None,
            note: false,
            loading: false,
            error: None,
            sequence: 0,
        };
        self.overlays.push(Overlay::ChatSearch(Box::new(sheet)));
    }

    pub(super) fn chat_search_key(&mut self, mut s: Box<ChatSearchSheet>, k: KeyEvent) -> Option<Overlay> {
        let control = k.modifiers.contains(KeyModifiers::CONTROL);
        let count = s.hits.len();
        match k.code {
            KeyCode::Esc | KeyCode::Enter if s.context.is_some() || s.page.is_some() => {
                s.context = None;
                s.page = None;
                s.note = false;
            }
            KeyCode::Esc => return None,
            KeyCode::Down if s.context.is_some() => {
                s.context_offset =
                    (s.context_offset + 1).min(s.context.as_ref().map_or(0, |entries| entries.len().saturating_sub(1)));
            }
            KeyCode::Up if s.context.is_some() => {
                s.context_offset = s.context_offset.saturating_sub(1);
            }
            KeyCode::Down => s.sel = (s.sel + 1).min(count.saturating_sub(1)),
            KeyCode::Up => s.sel = s.sel.saturating_sub(1),
            KeyCode::Tab => {
                s.side = s.side.next();
                s.reset();
                self.search_chat(&mut s, false);
            }
            KeyCode::Char('s') if control => {
                s.scope = s.scope.next();
                s.reset();
                self.search_chat(&mut s, false);
            }
            KeyCode::Char('t') if control => {
                s.tools = !s.tools;
                s.reset();
                self.search_chat(&mut s, false);
            }
            KeyCode::Enter if s.hits.is_empty() => self.search_chat(&mut s, false),
            KeyCode::Enter => {
                if let Some(hit) = s.picked().cloned() {
                    self.open_chat_hit(&mut s, &hit);
                }
            }
            _ => {
                s.query.key(k);
                s.reset();
            }
        }
        Some(Overlay::ChatSearch(s))
    }

    fn search_chat(&mut self, s: &mut ChatSearchSheet, more: bool) {
        let query = s.query.text.trim().to_owned();
        if query.is_empty() {
            return;
        }
        s.sequence += 1;
        s.loading = true;
        s.error = None;
        if !more {
            s.hits.clear();
            s.sel = 0;
        }
        let mut body = json!({"botId": s.bot_id, "query": query, "limit": 30, "tools": s.tools});
        if let Some(author) = s.side.wire() {
            body["author"] = json!(author);
        }
        if let Some(channel) = s.scope.wire() {
            body["channel"] = json!(channel);
        }
        self.call("search", body, After::ChatSearch(s.sequence, ChatSearchRead::Search));
    }

    fn open_chat_hit(&mut self, s: &mut ChatSearchSheet, hit: &Value) {
        let Some(id) = hit["id"].as_str() else { return };
        s.sequence += 1;
        s.loading = true;
        s.error = None;
        s.anchor = Some(id.to_owned());
        let sequence = s.sequence;
        if hit["kind"] == "memory" {
            s.note = true;
            self.call(
                "memoryFile",
                json!({
                    "botId": s.bot_id,
                    "file": hit["title"].as_str().unwrap_or_default(),
                    "shared": hit["shared"] == true,
                }),
                After::ChatSearch(sequence, ChatSearchRead::Note),
            );
        } else if hit["kind"] == "document" {
            self.call("getDocument", json!({"id": id}), After::ChatSearch(sequence, ChatSearchRead::Page));
        } else {
            self.call(
                "messageContext",
                json!({"botId": s.bot_id, "entryId": id}),
                After::ChatSearch(sequence, ChatSearchRead::Context),
            );
        }
    }

    pub(super) fn chat_search_reply(&mut self, sequence: u64, kind: ChatSearchRead, result: Result<Value, String>) {
        let Some(Overlay::ChatSearch(s)) = self.overlays.last_mut() else { return };
        if sequence != s.sequence {
            return;
        }
        s.loading = false;
        match result {
            Err(error) => s.error = Some(error),
            Ok(value) => match kind {
                ChatSearchRead::Search => {
                    s.hits.extend(value["hits"].as_array().cloned().unwrap_or_default());
                    s.sel = s.sel.min(s.hits.len().saturating_sub(1));
                }
                ChatSearchRead::Context => {
                    let id = s.anchor.clone().unwrap_or_default();
                    let entries: Vec<Value> = value["entries"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter(|entry| {
                            entry["kind"] == "user" || entry["kind"] == "agent" && entry["data"]["final"] == true
                        })
                        .cloned()
                        .collect();
                    s.context_offset = entries.iter().position(|entry| entry["id"] == id.as_str()).unwrap_or(0);
                    s.context = Some(entries);
                }
                ChatSearchRead::Page => {
                    let document = &value["document"];
                    s.page = Some((
                        document["title"].as_str().unwrap_or_default().to_owned(),
                        document["body"].as_str().unwrap_or_default().to_owned(),
                    ));
                }
                ChatSearchRead::Note => {
                    s.page = Some((
                        value["file"].as_str().unwrap_or_default().to_owned(),
                        value["content"].as_str().unwrap_or_default().to_owned(),
                    ));
                }
            },
        }
    }
}
