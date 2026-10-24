//! Keyboard-oriented document editor and immutable proposal inspection.

use super::{
    App, Buffer, Rect, centered, composer_lines, footer, frame_box, input_line, name_of, put, row, s, theme, window,
};
use crate::tui::manage::documents::{DocumentMode, DocumentSheet};
use ratatui::widgets::{Paragraph, Widget, Wrap};
use std::fmt::Write as _;

pub fn draw(buf: &mut Buffer, area: Rect, app: &App, sheet: &DocumentSheet) {
    let palette = theme();
    let title = format!("{} · Documents", name_of(app, &sheet.bot));
    let inner = frame_box(buf, centered(area, 110, 36), palette.text, palette.panel, Some((&title, palette.text)));
    if inner.height < 6 {
        return;
    }
    let labels = ["Documents", "Review", "Activity"];
    let mut x = inner.x;
    for (index, label) in labels.iter().enumerate() {
        let style =
            if index == sheet.tab { palette.text.patch(palette.sel) } else { palette.secondary.patch(palette.panel) };
        x = put(buf, x, inner.y, inner.right().saturating_sub(x), &format!(" {label} "), style).saturating_add(1);
    }
    let body = Rect::new(inner.x, inner.y + 2, inner.width, inner.height.saturating_sub(6));
    let keys = match sheet.mode {
        DocumentMode::Delete => {
            put(
                buf,
                body.x,
                body.y,
                body.width,
                "Delete this document AND all its children?",
                palette.red.patch(palette.panel),
            );
            "y confirm deletion · n/Esc keep document"
        }
        DocumentMode::Discard => {
            put(
                buf,
                body.x,
                body.y,
                body.width,
                "Your draft has unsaved changes. Waiting for save, or discard explicitly?",
                palette.red.patch(palette.panel),
            );
            "Ctrl+S retry save · y discard draft and close · n/Esc keep editing"
        }
        DocumentMode::Search => {
            input_line(buf, body, body.y, &sheet.query, "Search document title", true, true);
            "Enter search · Esc search and return"
        }
        DocumentMode::Edit => {
            editor(buf, body, sheet);
            "Tab field / complete slash · Ctrl+S save · Ctrl+G discuss · Ctrl+N child · Ctrl+M move · Ctrl+X delete · Esc back"
        }
        DocumentMode::Review => {
            review(buf, body, sheet);
            "↑↓ scroll · a approve exact proposal · d decline · Esc back"
        }
        DocumentMode::Move => {
            list(buf, body, sheet, true);
            "↑↓ destination · Enter move · Esc cancel (host validates hierarchy)"
        }
        DocumentMode::Browse => {
            list(buf, body, sheet, false);
            "←→ tab · ↑↓ select · Enter inspect · n create · / search · l more · r refresh · c CSV · Esc close"
        }
    };
    put(
        buf,
        inner.x,
        inner.bottom().saturating_sub(3),
        inner.width,
        &sheet.status,
        palette.secondary.patch(palette.panel),
    );
    footer(buf, inner, sheet.error.as_deref(), keys);
}

fn list(buf: &mut Buffer, area: Rect, sheet: &DocumentSheet, moving: bool) {
    let palette = theme();
    let rows = sheet.rows();
    if !moving && rows.is_empty() {
        let text = if sheet.loading {
            "Loading…"
        } else if sheet.tab == 0 {
            "No documents. Press n to create one."
        } else if sheet.tab == 1 {
            "No document proposals to review."
        } else {
            "No computer activity recorded."
        };
        put(buf, area.x, area.y, area.width, text, palette.secondary.patch(palette.panel));
        return;
    }
    let count = if moving { sheet.items.len() + 1 } else { rows.len() };
    let offset = window(sheet.cursor, usize::from(area.height) / 2);
    for (line, index) in (offset..count).take(usize::from(area.height) / 2).enumerate() {
        let y = area.y + u16::try_from(line * 2).unwrap_or(0);
        let style = row(buf, area, y, index == sheet.cursor);
        if moving && index == 0 {
            put(buf, area.x, y, area.width, "Library root", palette.text.patch(style));
            continue;
        }
        let value = if moving { &sheet.items[index - 1] } else { &rows[index] };
        let title = if !moving && sheet.tab == 2 { s(value, "action") } else { s(value, "title") };
        put(buf, area.x, y, area.width, title, palette.text.patch(style));
        let detail = if !moving && sheet.tab == 2 {
            format!(
                "{} · {} ({}) · {}",
                s(value, "result"),
                s(value, "actorKind"),
                s(value, "actorId"),
                value["createdAt"]
            )
        } else if !moving && sheet.tab == 1 {
            format!("{} · expires {}", s(value, "status"), value["expiresAt"])
        } else {
            format!("Revision {} · parent {}", value["revision"], value["parentId"])
        };
        put(buf, area.x, y + 1, area.width, &detail, palette.secondary.patch(palette.panel));
    }
}

fn editor(buf: &mut Buffer, area: Rect, sheet: &DocumentSheet) {
    let Some(draft) = &sheet.draft else {
        return;
    };
    let palette = theme();
    input_line(buf, area, area.y, &draft.title, "Document title", sheet.title_focus, false);
    let hint = if sheet.conflict.is_some() {
        "CONFLICT · Ctrl+P inspect latest/draft · Ctrl+R use latest (discard draft) · Ctrl+O overwrite with draft"
    } else {
        "Markdown · /doc /routine /skill /task + Tab insert content · autosave 800 ms"
    };
    put(buf, area.x, area.y + 1, area.width, hint, palette.secondary.patch(palette.panel));
    let body = Rect::new(area.x, area.y + 3, area.width, area.height.saturating_sub(3));
    if sheet.preview_latest {
        if let Some(latest) = &sheet.conflict {
            let text = format!(
                "LATEST SAVED REVISION {}\n{}\n\n{}",
                latest["revision"],
                s(latest, "title"),
                s(latest, "body")
            );
            Paragraph::new(text)
                .style(palette.text.patch(palette.panel))
                .wrap(Wrap { trim: false })
                .scroll((u16::try_from(sheet.scroll).unwrap_or(u16::MAX), 0))
                .render(body, buf);
        }
        return;
    }
    let (lines, (_, cursor_y)) = composer_lines(&draft.body, usize::from(body.width.max(1)));
    let offset =
        if sheet.title_focus { 0 } else { cursor_y.saturating_sub(usize::from(body.height.saturating_sub(1))) };
    for (index, line) in lines.iter().skip(offset).take(usize::from(body.height)).enumerate() {
        put(
            buf,
            body.x,
            body.y + u16::try_from(index).unwrap_or(0),
            body.width,
            line,
            palette.text.patch(palette.panel),
        );
    }
    if !sheet.title_focus && body.height > 0 {
        let (_, (cursor_x, cursor_y)) = composer_lines(&draft.body, usize::from(body.width.max(1)));
        super::set_cursor(ratatui::layout::Position::new(
            body.x + u16::try_from(cursor_x).unwrap_or(0),
            body.y + u16::try_from(cursor_y.saturating_sub(offset)).unwrap_or(0),
        ));
    }
}

fn review(buf: &mut Buffer, area: Rect, sheet: &DocumentSheet) {
    let palette = theme();
    let Some(review) = &sheet.review else {
        put(buf, area.x, area.y, area.width, "Loading exact proposal…", palette.secondary.patch(palette.panel));
        return;
    };
    let draft = &review["draft"];
    let mut text = format!(
        "{}\nStatus: {} · base revision {}\nTarget: {} · parent: {}\n\nEXACT PROPOSED MARKDOWN\n{}",
        s(draft, "title"),
        s(review, "status"),
        draft["ifRevision"],
        draft["id"],
        draft["parentId"],
        s(draft, "body")
    );
    if !review["document"].is_null() {
        let _ = write!(
            text,
            "\n\nCURRENT DOCUMENT\n{}\n{}",
            s(&review["document"], "title"),
            s(&review["document"], "body")
        );
    }
    if s(review, "status") == "conflict" {
        text.push_str("\n\nThe document changed. This proposal was not applied.");
    }
    Paragraph::new(text)
        .style(palette.text.patch(palette.panel))
        .wrap(Wrap { trim: false })
        .scroll((u16::try_from(sheet.scroll).unwrap_or(u16::MAX), 0))
        .render(area, buf);
}
