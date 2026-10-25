//! Replies preserve current drafts and reject stale sheet requests.
use super::{DocumentDraft, DocumentMode, DocumentReply, s};
use crate::tui::app::{App, Overlay, copy};
use serde_json::Value;
use std::time::{Duration, Instant};

impl App {
    pub fn on_document_reply(&mut self, reply: &DocumentReply, value: &Value) {
        let Some(index) = self
            .overlays
            .iter()
            .position(|overlay| matches!(overlay, Overlay::Documents(sheet) if sheet.accepts(reply)))
        else {
            return;
        };
        let Overlay::Documents(mut sheet) = self.overlays.remove(index) else {
            return;
        };
        match reply {
            &DocumentReply::List(_, _, more) => {
                let items = value["items"].as_array().cloned().unwrap_or_default();
                if more {
                    for item in items {
                        if !sheet.items.iter().any(|old| old["id"] == item["id"]) {
                            sheet.items.push(item);
                        }
                    }
                } else {
                    sheet.items = items;
                }
                sheet.next = value["next"].as_str().map(str::to_owned);
                sheet.loading = false;
            }
            &DocumentReply::Extras(_, _, reviews) => {
                let items = value["items"].as_array().cloned().unwrap_or_default();
                if reviews {
                    sheet.reviews = items;
                } else {
                    sheet.activity = items;
                }
            }
            DocumentReply::Open(..) => {
                sheet.draft = Some(DocumentDraft::from_document(&value["document"]));
                sheet.mode = DocumentMode::Edit;
                sheet.conflict = None;
                sheet.preview_latest = false;
                sheet.busy = false;
                sheet.status = "Saved document · Tab changes field".into();
            }
            DocumentReply::Save(..) => {
                sheet.busy = false;
                if s(value, "status") == "conflict" {
                    sheet.conflict = Some(value["document"].clone());
                    sheet.status = "Conflict: draft preserved. Ctrl+R reloads latest; Ctrl+O overwrites latest.".into();
                } else if let Some(draft) = &mut sheet.draft {
                    let saved = &value["document"];
                    draft.id = saved["id"].as_str().map(str::to_owned);
                    draft.revision = saved["revision"].as_u64().unwrap_or(draft.revision);
                    draft.dirty = draft.title.text.trim() != s(saved, "title") || draft.body.text != s(saved, "body");
                    if !draft.dirty {
                        draft.title = super::Editor::with(s(saved, "title"));
                    }
                    sheet.conflict = None;
                    sheet.preview_latest = false;
                    sheet.status =
                        if draft.dirty { "Newer edits retained; saving next…" } else { "All changes saved" }.into();
                    if draft.dirty {
                        sheet.save_after = Some(Instant::now() + Duration::from_millis(800));
                    }
                    if sheet.mode == DocumentMode::Discard && !draft.dirty {
                        sheet.mode = DocumentMode::Edit;
                    }
                    self.fetch_documents(&mut sheet, false);
                }
            }
            DocumentReply::Review(..) | DocumentReply::Resolve(..) => {
                sheet.review = Some(value["review"].clone());
                sheet.busy = false;
                self.fetch_document_extras(&mut sheet);
            }
            DocumentReply::Change(..) => {
                sheet.busy = false;
                if s(value, "status") == "conflict" {
                    sheet.conflict = Some(value["document"].clone());
                    sheet.mode = DocumentMode::Edit;
                } else if s(value, "status") == "deleted" {
                    sheet.draft = None;
                    sheet.mode = DocumentMode::Browse;
                } else {
                    sheet.draft = Some(DocumentDraft::from_document(&value["document"]));
                    sheet.mode = DocumentMode::Edit;
                }
                self.fetch_documents(&mut sheet, false);
            }
            DocumentReply::Csv(_) => {
                copy(s(value, "csv"));
                sheet.status = "Activity CSV copied".into();
            }
        }
        self.overlays.insert(index, Overlay::Documents(sheet));
    }

    pub fn document_error(&mut self, reply: &DocumentReply, error: &str) {
        for overlay in &mut self.overlays {
            if let Overlay::Documents(sheet) = overlay
                && sheet.accepts(reply)
            {
                sheet.error = Some(error.into());
                if matches!(reply, DocumentReply::List(..)) {
                    sheet.loading = false;
                }
                if !matches!(reply, DocumentReply::List(..) | DocumentReply::Extras(..) | DocumentReply::Csv(..)) {
                    sheet.busy = false;
                }
                if matches!(reply, DocumentReply::Save(..)) {
                    sheet.status = "Save unconfirmed; draft retained. Ctrl+S retries the same creation safely.".into();
                }
            }
        }
    }
}
