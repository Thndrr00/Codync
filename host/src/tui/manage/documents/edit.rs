use super::{DocumentDraft, DocumentMode, DocumentSheet, ctrl};
use crate::tui::app::App;
use crossterm::event::{KeyCode, KeyEvent};
use std::time::{Duration, Instant};

impl App {
    pub(super) fn document_edit_key(&self, sheet: &mut DocumentSheet, key: KeyEvent) {
        if key.code == KeyCode::Esc {
            if sheet.busy {
                sheet.status = "Wait for the pending save.".into();
            } else if sheet.dirty() {
                self.save_document_draft(sheet, false);
                sheet.mode = DocumentMode::Discard;
            } else {
                sheet.mode = DocumentMode::Browse;
            }
            return;
        }
        if ctrl(key) {
            match key.code {
                KeyCode::Char('p') if sheet.conflict.is_some() => {
                    sheet.preview_latest = !sheet.preview_latest;
                    sheet.scroll = 0;
                }
                KeyCode::Char('r') if sheet.conflict.is_some() && !sheet.busy => {
                    if let Some(document) = sheet.conflict.take() {
                        sheet.draft = Some(DocumentDraft::from_document(&document));
                    }
                    sheet.error = None;
                    sheet.preview_latest = false;
                }
                KeyCode::Char('o') => self.save_document_draft(sheet, true),
                KeyCode::Char('x') if !sheet.busy && sheet.draft.as_ref().is_some_and(|draft| draft.id.is_some()) => {
                    sheet.mode = DocumentMode::Delete
                }
                KeyCode::Char('m') if !sheet.busy && !sheet.dirty() => {
                    sheet.mode = DocumentMode::Move;
                    sheet.cursor = 0;
                }
                KeyCode::Char('n') if !sheet.busy && !sheet.dirty() => {
                    let parent = sheet.draft.as_ref().and_then(|draft| draft.id.clone());
                    if parent.is_some() {
                        self.new_document(sheet, parent);
                    }
                }
                _ => {}
            }
            return;
        }
        if sheet.preview_latest {
            if key.code == KeyCode::Down {
                sheet.scroll = sheet.scroll.saturating_add(1);
            }
            if key.code == KeyCode::Up {
                sheet.scroll = sheet.scroll.saturating_sub(1);
            }
            return;
        }
        if key.code == KeyCode::Tab {
            if !sheet.title_focus
                && let Some(draft) = &mut sheet.draft
                && super::blocks::complete(&mut draft.body)
            {
                draft.dirty = true;
                sheet.save_after = Some(Instant::now() + Duration::from_millis(800));
                sheet.status = "Inserted Markdown only; no action was executed".into();
                return;
            }
            sheet.title_focus = !sheet.title_focus;
            return;
        }
        if let Some(draft) = &mut sheet.draft {
            let editor = if sheet.title_focus { &mut draft.title } else { &mut draft.body };
            let before = editor.text.clone();
            if key.code == KeyCode::Enter && !sheet.title_focus {
                editor.insert("\n");
            } else if key.code != KeyCode::Enter {
                editor.key(key);
            }
            if before != editor.text {
                draft.dirty = true;
                sheet.save_after = Some(Instant::now() + Duration::from_millis(800));
                sheet.status = "Unsaved draft".into();
            }
        }
    }
}
