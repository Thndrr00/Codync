//! This computer's policies sheet: the list, the field being typed, and the what-if's answer.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use super::super::app::{PoliciesSheet, PolicyMode};
use super::super::l10n::tr;
use super::super::md::truncate;
use super::buffer::{centered, frame_box, put};
use super::style::theme;

pub(super) fn policies(buf: &mut Buffer, area: Rect, s: &PoliciesSheet) {
    let t = theme();
    let rows = s.rows.len().max(1) + 10;
    let r = centered(area, 92, u(rows.clamp(16, usize::from(area.height.saturating_sub(2)).max(16))));
    let title = format!("{} · {}", tr("Policies"), tr("this computer"));
    let inner = frame_box(buf, r, t.text, t.panel, Some((title.as_str(), t.text)));
    if inner.height < 9 {
        return;
    }
    let width = usize::from(inner.width);
    let mut y = inner.y + 1;
    let about = if s.rows.iter().any(|row| row.enabled && row.effect == "allow") {
        "A deny rule refuses first; an allow rule lets through only what it names, and nothing else."
    } else if s.rows.iter().any(|row| row.enabled) {
        "Deny rules refuse first; what they don't refuse goes on to the bot's approval rules and Approvals setting."
    } else {
        "No policy is on: the bots' approval rules and Approvals setting decide everything, as before."
    };
    put(buf, inner.x, y, inner.width, &truncate(&tr(about), width), t.secondary.patch(t.panel));
    y += 2;
    if s.rows.is_empty() {
        put(buf, inner.x, y, inner.width, &tr("No policies yet: a adds one of the presets."), t.dim.patch(t.panel));
        y += 1;
    }
    let room = usize::from(inner.bottom().saturating_sub(y)).saturating_sub(6).max(1);
    let off = s.sel.saturating_sub(room - 1);
    for (i, row) in s.rows.iter().enumerate().skip(off).take(room) {
        let selected = i == s.sel;
        let state = if row.enabled { "on " } else { "off" };
        let line = format!("{state} {:<5} {}  {}", row.effect, row.name, row.expression);
        let style = if selected { t.bold.patch(t.panel) } else { t.text.patch(t.panel) };
        put(buf, inner.x, y, inner.width, &truncate(&line, width), style);
        y += 1;
        if selected && !row.message.is_empty() {
            put(buf, inner.x, y, inner.width, &truncate(&format!("     {}", row.message), width), t.dim.patch(t.panel));
            y += 1;
        }
    }
    let prompt = match s.mode {
        PolicyMode::Name | PolicyMode::Expression | PolicyMode::Message | PolicyMode::Try => {
            let label = match s.mode {
                PolicyMode::Name => tr("Name"),
                PolicyMode::Expression => tr("Expression"),
                PolicyMode::Message => tr("Message"),
                _ => tr("Try a command"),
            };
            format!("{label}: {}", s.editor.text)
        }
        PolicyMode::List => tr("Select a rule with j and k.").into_owned(),
    };
    let prompt_y = inner.bottom().saturating_sub(4);
    put(
        buf,
        inner.x,
        prompt_y,
        inner.width,
        &truncate(&prompt, width),
        if s.mode == PolicyMode::List { t.dim } else { t.bold }.patch(t.panel),
    );
    if let Some(answer) = &s.answer {
        put(buf, inner.x, prompt_y + 1, inner.width, &truncate(answer, width), t.secondary.patch(t.panel));
    }
    if let Some(error) = &s.error {
        put(buf, inner.x, prompt_y + 2, inner.width, &truncate(error, width), t.amber.patch(t.panel));
    }
    put(buf, inner.x, inner.bottom() - 1, inner.width, &truncate(&tr(&s.footer()), width), t.dim.patch(t.panel));
}
