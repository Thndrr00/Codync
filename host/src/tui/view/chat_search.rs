//! Searching one chat from the terminal: the field, whose words were said, where in the chat, the matches, and a
//! hit's context, page or note in place of the list.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};

use super::super::app::ChatSearchSheet;
use super::super::l10n::tr;
use super::super::md::truncate;
use super::buffer::{centered, frame_box, put, put_line};
use super::overlay::field_line;
use super::style::{Theme, theme};

pub(super) fn chat_search(buf: &mut Buffer, area: Rect, s: &ChatSearchSheet) {
    let t = theme();
    let title = format!("{} · {}", tr("Search this chat"), s.bot_name);
    let r = centered(area, 86, area.height.saturating_sub(4).max(10));
    let inner = frame_box(buf, r, t.text, t.panel, Some((title.as_str(), t.text)));
    if inner.height < 6 {
        return;
    }
    let width = usize::from(inner.width);
    field_line(buf, inner, inner.y, &s.query, "words to look for", true);
    put(
        buf,
        inner.x,
        inner.y + 1,
        inner.width,
        &format!(
            "{} · {} · {} tools · tab side · ^s where · ^t traces",
            tr(s.side.title()),
            tr(s.scope.title()),
            tr(if s.tools { "with" } else { "no" })
        ),
        t.dim,
    );
    let top = inner.y + 3;
    if let Some((page, body)) = &s.page {
        put(buf, inner.x, top, inner.width, &truncate(page, width), t.bold);
        for (count, line) in body.lines().enumerate() {
            let y = top + 1 + u16::try_from(count).unwrap_or(u16::MAX);
            if y + 2 >= inner.bottom() {
                break;
            }
            put(buf, inner.x, y, inner.width, &truncate(line, width), t.secondary);
        }
        footer(
            buf,
            inner,
            if s.note { "A note from its memory · esc returns" } else { "A page of this bot · esc returns" },
        );
        return;
    }
    if let Some(entries) = &s.context {
        let mut y = top;
        if entries.is_empty() {
            put(buf, inner.x, y, inner.width, &tr("No messages around it"), t.dim);
        }
        for entry in entries.iter().skip(s.context_offset) {
            if y + 2 >= inner.bottom() {
                break;
            }
            let lane = if entry["threadId"].is_string() { "Thread" } else { "Main chat" };
            let anchor = entry["id"].as_str().is_some_and(|id| Some(id) == s.anchor.as_deref());
            put(
                buf,
                inner.x,
                y,
                inner.width,
                &format!(
                    "{}{lane} · {}",
                    if anchor { "> " } else { "" },
                    tr(if entry["kind"] == "user" { "Me" } else { "Bot" })
                ),
                if anchor { t.bold } else { t.secondary },
            );
            y += 1;
            let lines = super::super::md::wrap(
                &[Span::styled(entry["data"]["text"].as_str().unwrap_or_default().to_owned(), t.text)],
                width,
                &[],
                &[],
            );
            for line in lines {
                if y + 1 >= inner.bottom() {
                    break;
                }
                put_line(buf, inner.x, y, inner.width, &line);
                y += 1;
            }
            y += 1;
        }
        footer(buf, inner, "↑↓ scroll · esc returns");
        return;
    }
    if s.loading {
        put(buf, inner.x, top, inner.width, &tr("Searching…"), t.secondary);
        return;
    }
    if let Some(error) = &s.error {
        put(buf, inner.x, top, inner.width, &format!("{}: {error}", tr("Request failed")), t.red);
        return;
    }
    if s.hits.is_empty() {
        put(
            buf,
            inner.x,
            top,
            inner.width,
            &tr("Type a few words, then Enter: every one of them has to appear"),
            t.dim,
        );
        footer(buf, inner, "^t traces · tab side · ^s where · esc close");
        return;
    }
    let mut y = top;
    for (index, hit) in s.hits.iter().enumerate().skip(s.sel.saturating_sub(3)) {
        if y + 1 >= inner.bottom() {
            break;
        }
        let selected = index == s.sel;
        let who = if hit["kind"] == "document" {
            format!("{} · {}", tr("Page"), hit["title"].as_str().unwrap_or_default())
        } else if hit["kind"] == "memory" {
            format!("{} · {}", tr("Note"), hit["title"].as_str().unwrap_or_default())
        } else {
            format!(
                "{} · {}",
                tr(if hit["entryKind"] == "user" { "Me" } else { "Bot" }),
                super::when(hit["at"].as_i64().unwrap_or_default())
            )
        };
        put(buf, inner.x, y, inner.width, &truncate(&who, width), if selected { t.bold } else { t.secondary });
        y += 1;
        put_line(buf, inner.x, y, inner.width, &snippet_line(hit["snippet"].as_str().unwrap_or_default(), t));
        y += 2;
    }
    footer(buf, inner, "↑↓ pick · enter opens · esc close");
}

fn footer(buf: &mut Buffer, inner: Rect, keys: &str) {
    let t = theme();
    put(buf, inner.x, inner.bottom() - 1, inner.width, &tr(keys), t.dim.patch(t.panel));
}

/// The snippet with its matches bold, clipped to the row: the host brackets them.
fn snippet_line(snippet: &str, t: &Theme) -> Line<'static> {
    let mut spans = Vec::new();
    for (index, part) in snippet.split('[').enumerate() {
        if index == 0 {
            if !part.is_empty() {
                spans.push(Span::styled(part.to_owned(), t.text));
            }
            continue;
        }
        match part.split_once(']') {
            Some((matched, rest)) => {
                spans.push(Span::styled(matched.to_owned(), t.bold));
                if !rest.is_empty() {
                    spans.push(Span::styled(rest.to_owned(), t.text));
                }
            }
            None => spans.push(Span::styled(format!("[{part}"), t.text)),
        }
    }
    Line::from(spans)
}
