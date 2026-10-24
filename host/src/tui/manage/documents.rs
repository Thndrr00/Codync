//! Document library, revision-checked drafts, immutable reviews and activity.

use super::super::app::{App, Editor, Overlay};
use super::{Reply, ctrl};
use crossterm::event::{KeyCode, KeyEvent};
use serde_json::{Value, json};
mod blocks;
mod edit;
mod replies;

use std::time::{Duration, Instant};

fn s<'a>(value: &'a Value, key: &str) -> &'a str {
    value[key].as_str().unwrap_or_default()
}

#[derive(Clone)]
pub enum DocumentReply {
    List(String, u64, bool),
    Extras(String, u64, bool),
    Open(String, u64),
    Save(String, u64),
    Review(String, u64),
    Resolve(String, u64),
    Change(String, u64),
    Csv(String),
}

impl DocumentReply {
    fn scope(&self) -> &str {
        match self {
            Self::List(scope, ..)
            | Self::Extras(scope, ..)
            | Self::Open(scope, ..)
            | Self::Save(scope, ..)
            | Self::Review(scope, ..)
            | Self::Resolve(scope, ..)
            | Self::Change(scope, ..)
            | Self::Csv(scope) => scope,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DocumentMode {
    Browse,
    Search,
    Edit,
    Review,
    Move,
    Delete,
    Discard,
}

pub struct DocumentDraft {
    pub id: Option<String>,
    pub parent: Option<String>,
    pub revision: u64,
    pub title: Editor,
    pub body: Editor,
    pub nonce: String,
    pub dirty: bool,
}

impl DocumentDraft {
    fn from_document(document: &Value) -> Self {
        Self {
            id: Some(s(document, "id").into()),
            parent: document["parentId"].as_str().map(str::to_owned),
            revision: document["revision"].as_u64().unwrap_or(0),
            title: Editor::with(s(document, "title")),
            body: Editor::with(s(document, "body")),
            nonce: uuid::Uuid::new_v4().to_string(),
            dirty: false,
        }
    }
}

pub struct DocumentSheet {
    pub bot: String,
    pub scope: String,
    pub tab: usize,
    pub mode: DocumentMode,
    pub items: Vec<Value>,
    pub reviews: Vec<Value>,
    pub activity: Vec<Value>,
    pub query: Editor,
    pub cursor: usize,
    pub scroll: usize,
    pub draft: Option<DocumentDraft>,
    pub review: Option<Value>,
    pub conflict: Option<Value>,
    pub error: Option<String>,
    pub status: String,
    pub loading: bool,
    pub busy: bool,
    pub title_focus: bool,
    pub preview_latest: bool,
    pub next: Option<String>,
    generation: u64,
    list_request: u64,
    extras_request: u64,
    save_after: Option<Instant>,
    refresh_at: Instant,
}

impl DocumentSheet {
    pub fn rows(&self) -> &[Value] {
        match self.tab {
            1 => &self.reviews,
            2 => &self.activity,
            _ => &self.items,
        }
    }
    fn dirty(&self) -> bool {
        self.draft.as_ref().is_some_and(|draft| draft.dirty)
    }
    fn accepts(&self, reply: &DocumentReply) -> bool {
        if reply.scope() != self.scope {
            return false;
        }
        match reply {
            DocumentReply::List(_, request, _) => *request == self.list_request,
            DocumentReply::Extras(_, request, _) => *request == self.extras_request,
            DocumentReply::Open(_, generation)
            | DocumentReply::Save(_, generation)
            | DocumentReply::Review(_, generation)
            | DocumentReply::Resolve(_, generation)
            | DocumentReply::Change(_, generation) => *generation == self.generation,
            DocumentReply::Csv(_) => true,
        }
    }
}

impl App {
    pub fn open_documents(&mut self) {
        let Some(bot) = self.bot().map(|bot| bot.id.clone()) else {
            self.flash("Select a bot or group to open documents");
            return;
        };
        let mut sheet = Box::new(DocumentSheet {
            bot,
            scope: uuid::Uuid::new_v4().to_string(),
            tab: 0,
            mode: DocumentMode::Browse,
            items: Vec::new(),
            reviews: Vec::new(),
            activity: Vec::new(),
            query: Editor::default(),
            cursor: 0,
            scroll: 0,
            draft: None,
            review: None,
            conflict: None,
            error: None,
            status: String::new(),
            loading: false,
            busy: false,
            title_focus: true,
            preview_latest: false,
            next: None,
            generation: 0,
            list_request: 0,
            extras_request: 0,
            save_after: None,
            refresh_at: Instant::now(),
        });
        self.fetch_documents(&mut sheet, false);
        self.fetch_document_extras(&mut sheet);
        self.overlays.push(Overlay::Documents(sheet));
    }

    fn document_call(&self, method: &'static str, body: Value, reply: DocumentReply) {
        self.sheet(method, body, Reply::Documents(reply));
    }

    fn fetch_documents(&self, sheet: &mut DocumentSheet, more: bool) {
        sheet.list_request += 1;
        sheet.loading = true;
        self.document_call(
            "listDocuments",
            json!({"botId": sheet.bot, "query": sheet.query.text,
            "afterId": if more { sheet.next.clone() } else { None }, "limit": 100}),
            DocumentReply::List(sheet.scope.clone(), sheet.list_request, more),
        );
    }

    fn fetch_document_extras(&self, sheet: &mut DocumentSheet) {
        sheet.extras_request += 1;
        for (method, reviews) in [("listDocumentReviews", true), ("listComputerActivity", false)] {
            self.document_call(
                method,
                json!({"botId": sheet.bot}),
                DocumentReply::Extras(sheet.scope.clone(), sheet.extras_request, reviews),
            );
        }
        sheet.refresh_at = Instant::now() + Duration::from_secs(5);
    }

    fn save_document_draft(&self, sheet: &mut DocumentSheet, overwrite: bool) {
        let Some(draft) = &sheet.draft else {
            return;
        };
        if sheet.busy || !draft.dirty || draft.title.text.trim().is_empty() || (sheet.conflict.is_some() && !overwrite)
        {
            return;
        }
        let revision = if overwrite {
            sheet.conflict.as_ref().and_then(|v| v["revision"].as_u64()).unwrap_or(draft.revision)
        } else {
            draft.revision
        };
        let mut body =
            json!({"botId": sheet.bot, "title": draft.title.text, "body": draft.body.text, "ifRevision": revision});
        if let Some(id) = &draft.id {
            body["id"] = json!(id);
        } else {
            body["parentId"] = json!(draft.parent);
            body["clientNonce"] = json!(draft.nonce);
        }
        sheet.busy = true;
        sheet.error = None;
        sheet.save_after = None;
        sheet.status = "Saving…".into();
        self.document_call("saveDocument", body, DocumentReply::Save(sheet.scope.clone(), sheet.generation));
    }

    pub fn documents_key(&mut self, mut sheet: Box<DocumentSheet>, key: KeyEvent) -> Option<Overlay> {
        if key.code == KeyCode::Char('g') && ctrl(key) && sheet.tab == 0 {
            if sheet.busy || sheet.dirty() || self.selected.as_deref() != Some(sheet.bot.as_str()) {
                sheet.error = Some("Save or resolve your draft before discussing the document.".into());
                return Some(Overlay::Documents(sheet));
            }
            let id = if sheet.mode == DocumentMode::Edit {
                sheet.draft.as_ref().and_then(|draft| draft.id.clone())
            } else {
                sheet.items.get(sheet.cursor).and_then(|doc| doc["id"].as_str()).map(str::to_owned)
            };
            if let Some(id) = id {
                self.open_thread(format!("doc:{id}"));
                return None;
            }
        }
        if key.code == KeyCode::Char('s') && ctrl(key) {
            self.save_document_draft(&mut sheet, false);
            return Some(Overlay::Documents(sheet));
        }
        if sheet.mode == DocumentMode::Delete || sheet.mode == DocumentMode::Discard {
            if key.code == KeyCode::Char('y') && !sheet.busy {
                if sheet.mode == DocumentMode::Discard {
                    return None;
                }
                if let Some(draft) = &sheet.draft {
                    self.document_call(
                        "deleteDocument",
                        json!({"id": draft.id, "ifRevision": draft.revision}),
                        DocumentReply::Change(sheet.scope.clone(), sheet.generation),
                    );
                    sheet.busy = true;
                }
            }
            if matches!(key.code, KeyCode::Esc | KeyCode::Char('n')) {
                sheet.mode = DocumentMode::Edit;
            }
            return Some(Overlay::Documents(sheet));
        }
        if sheet.mode == DocumentMode::Edit {
            self.document_edit_key(&mut sheet, key);
            return Some(Overlay::Documents(sheet));
        }
        if sheet.mode == DocumentMode::Search {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => {
                    sheet.mode = DocumentMode::Browse;
                    self.fetch_documents(&mut sheet, false);
                }
                _ => {
                    sheet.query.key(key);
                }
            }
            return Some(Overlay::Documents(sheet));
        }
        if sheet.mode == DocumentMode::Review {
            match key.code {
                KeyCode::Esc => {
                    sheet.mode = DocumentMode::Browse;
                    sheet.review = None;
                }
                KeyCode::Char('a' | 'd') if !sheet.busy => {
                    if let Some(review) = &sheet.review
                        && s(review, "status") == "pending"
                    {
                        self.document_call("resolveAction", json!({"id": review["id"], "decision": if key.code == KeyCode::Char('a') { "approve" } else { "decline" }}),
                            DocumentReply::Resolve(sheet.scope.clone(), sheet.generation));
                        sheet.busy = true;
                    }
                }
                KeyCode::Up => sheet.scroll = sheet.scroll.saturating_sub(1),
                KeyCode::Down => sheet.scroll = sheet.scroll.saturating_add(1),
                _ => {}
            }
            return Some(Overlay::Documents(sheet));
        }
        let rows = if sheet.mode == DocumentMode::Move { sheet.items.len() + 1 } else { sheet.rows().len() };
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                if sheet.busy {
                    sheet.status = "Wait for the pending request before closing.".into();
                } else if sheet.dirty() {
                    self.save_document_draft(&mut sheet, false);
                    sheet.mode = DocumentMode::Discard;
                } else if sheet.mode == DocumentMode::Move {
                    sheet.mode = DocumentMode::Edit;
                } else {
                    return None;
                }
            }
            KeyCode::Left | KeyCode::Right if sheet.mode == DocumentMode::Browse => {
                sheet.tab =
                    if key.code == KeyCode::Left { sheet.tab.saturating_sub(1) } else { (sheet.tab + 1).min(2) };
                sheet.cursor = 0;
            }
            KeyCode::Down | KeyCode::Char('j') => sheet.cursor = (sheet.cursor + 1).min(rows.saturating_sub(1)),
            KeyCode::Up | KeyCode::Char('k') => sheet.cursor = sheet.cursor.saturating_sub(1),
            KeyCode::Char('/') if sheet.tab == 0 => sheet.mode = DocumentMode::Search,
            KeyCode::Char('n') if sheet.tab == 0 && !sheet.busy => self.new_document(&mut sheet, None),
            KeyCode::Char('r') => {
                self.fetch_documents(&mut sheet, false);
                self.fetch_document_extras(&mut sheet);
            }
            KeyCode::Char('l') if sheet.next.is_some() => self.fetch_documents(&mut sheet, true),
            KeyCode::Char('c') if sheet.tab == 2 => self.document_call(
                "exportComputerActivity",
                json!({"botId": sheet.bot}),
                DocumentReply::Csv(sheet.scope.clone()),
            ),
            KeyCode::Enter if !sheet.busy => self.document_enter(&mut sheet),
            _ => {}
        }
        Some(Overlay::Documents(sheet))
    }

    fn new_document(&self, sheet: &mut DocumentSheet, parent: Option<String>) {
        sheet.generation += 1;
        sheet.draft = Some(DocumentDraft {
            id: None,
            parent,
            revision: 0,
            title: Editor::default(),
            body: Editor::default(),
            nonce: uuid::Uuid::new_v4().to_string(),
            dirty: false,
        });
        sheet.mode = DocumentMode::Edit;
        sheet.title_focus = true;
        sheet.conflict = None;
        sheet.preview_latest = false;
        sheet.error = None;
        sheet.status = "New document · Tab changes field · Ctrl+S saves".into();
    }

    fn document_enter(&self, sheet: &mut DocumentSheet) {
        if sheet.mode == DocumentMode::Move {
            let parent = sheet
                .cursor
                .checked_sub(1)
                .and_then(|index| sheet.items.get(index))
                .map(|item| item["id"].clone())
                .unwrap_or(Value::Null);
            if let Some(draft) = &sheet.draft {
                self.document_call(
                    "moveDocument",
                    json!({"id": draft.id, "parentId": parent, "ifRevision": draft.revision}),
                    DocumentReply::Change(sheet.scope.clone(), sheet.generation),
                );
                sheet.busy = true;
            }
            return;
        }
        let Some(item) = sheet.rows().get(sheet.cursor).cloned() else {
            return;
        };
        sheet.generation += 1;
        sheet.busy = true;
        sheet.scroll = 0;
        if sheet.tab == 0 {
            self.document_call(
                "getDocument",
                json!({"id": item["id"]}),
                DocumentReply::Open(sheet.scope.clone(), sheet.generation),
            );
        } else if sheet.tab == 1 {
            sheet.mode = DocumentMode::Review;
            sheet.review = None;
            self.document_call(
                "getDocumentReview",
                json!({"id": item["id"]}),
                DocumentReply::Review(sheet.scope.clone(), sheet.generation),
            );
        } else {
            sheet.busy = false;
        }
    }

    pub fn documents_tick(&mut self) {
        let Some(index) = self.overlays.iter().position(|overlay| matches!(overlay, Overlay::Documents(_))) else {
            return;
        };
        let Overlay::Documents(mut sheet) = self.overlays.remove(index) else {
            return;
        };
        if sheet.save_after.is_some_and(|at| at <= Instant::now()) && !sheet.busy && sheet.conflict.is_none() {
            self.save_document_draft(&mut sheet, false);
        }
        if sheet.refresh_at <= Instant::now() {
            // Keep explicitly loaded later pages visible until the user refreshes.
            if sheet.items.len() <= 100 {
                self.fetch_documents(&mut sheet, false);
            }
            self.fetch_document_extras(&mut sheet);
        }
        self.overlays.insert(index, Overlay::Documents(sheet));
    }
}
