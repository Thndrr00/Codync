//! A bot's Missions tab: what it is trying to get done, the steps taken and what is left, so a person
//! can push it on, pause it, finish it or drop it with a reason.

use super::{
    App, Buffer, Rect, centered, footer, frame_box, input_line, name_of, page_tabs, put, row, theme, truncate, window,
};
use crate::store::MissionStatus;
use crate::tui::l10n::tr;
use crate::tui::manage::Page;
use crate::tui::manage::missions::{MissionEdit, MissionSheet, mark, status_label};

const KEYS: &str = "↑↓ pick �� a on �� p pause �� d finish �� x abandon �� c checkpoint �� s step �� tab steps �� n new �� f finished �� esc";

pub fn draw(buf: &mut Buffer, area: Rect, app: &App, m: &MissionSheet) {
    let t = theme();
    let r = centered(area, 96, 32);
    let title = name_of(app, &m.bot);
    let inner = frame_box(buf, r, t.text, t.panel, Some((&title, t.text)));
    let inner = page_tabs(buf, inner, Page::Missions);
    let Some(items) = m.items.as_deref() else {
        put(buf, inner.x, inner.y, inner.width, &tr("Loading�?�"), t.dim.patch(t.panel));
        footer(buf, inner, m.error.as_deref(), KEYS);
        return;
    };
    if items.is_empty() {
        put(buf, inner.x, inner.y, inner.width, &tr("No missions"), t.bold.patch(t.panel));
        put(
            buf,
            inner.x,
            inner.y + 1,
            inner.width,
            &tr("A mission is a goal that outlives one turn; the bot keeps what is done and what is left."),
            t.secondary.patch(t.panel),
        );
        put(buf, inner.x, inner.y + 2, inner.width, &tr("n starts one."), t.dim.patch(t.panel));
        footer(buf, inner, m.error.as_deref(), KEYS);
        return;
    }

    let h = usize::from(inner.height.saturating_sub(7));
    for (y, i) in (inner.y..).zip(window(m.cursor, h)..items.len()).take(h) {
        let mission = &items[i];
        let base = row(buf, inner, y, i == m.cursor && !m.focus_steps);
        let head = format!("{:<10} {}", status_label(mission.status), mission.title);
        put(buf, inner.x, y, inner.width, &truncate(&head, usize::from(inner.width)), t.text.patch(base));
    }

    let mission = &items[m.cursor.min(items.len() - 1)];
    let mut y = (inner.y..).zip(0..h).last().map_or(inner.y, |(y, _)| y + 1);
    if mission.goal != mission.title && !mission.goal.is_empty() {
        put(
            buf,
            inner.x,
            y,
            inner.width,
            &truncate(&mission.goal, usize::from(inner.width)),
            t.secondary.patch(t.panel),
        );
        y += 1;
    }
    if let Some(reason) = &mission.stop_reason {
        let line = format!("{} {reason}", if mission.status == MissionStatus::Abandoned { "dropped:" } else { "why:" });
        put(buf, inner.x, y, inner.width, &truncate(&line, usize::from(inner.width)), t.dim.patch(t.panel));
        y += 1;
    }
    for (i, step) in mission.steps.iter().enumerate() {
        if y >= inner.bottom().saturating_sub(2) {
            break;
        }
        let base = row(buf, inner, y, m.focus_steps && i == m.step);
        let line = format!("{} {}", mark(step.status), step.text);
        put(
            buf,
            inner.x,
            y,
            inner.width,
            &truncate(&line, usize::from(inner.width)),
            step_style(step.status).patch(base),
        );
        y += 1;
    }

    let by = inner.bottom().saturating_sub(2);
    if let Some((what, ed)) = &m.edit {
        let label = match what {
            MissionEdit::New => "New mission",
            MissionEdit::Step => "New step",
            MissionEdit::Checkpoint => "What is done and what is left",
            MissionEdit::Abandon => "Why it stopped",
        };
        let label_at = by.saturating_sub(1);
        put(buf, inner.x, label_at, inner.width, &tr(label), t.dim.patch(t.panel));
        input_line(buf, inner, by, ed, label, true, false);
    } else {
        let checkpoint = if mission.checkpoint.is_empty() {
            tr("No checkpoint yet �� c writes one")
        } else {
            tr(&mission.checkpoint)
        };
        let line = format!("checkpoint: {checkpoint}");
        put(buf, inner.x, by, inner.width, &truncate(&line, usize::from(inner.width)), t.dim.patch(t.panel));
    }
    footer(buf, inner, m.error.as_deref(), KEYS);
}

/// The text of a line built from a label and a value.
fn truth(line: &str) -> String {
    line.to_owned()
}

fn step_style(status: crate::store::MissionStepStatus) -> ratatui::style::Style {
    let t = theme();
    match status {
        crate::store::MissionStepStatus::Done | crate::store::MissionStepStatus::Skipped => t.dim,
        _ => t.text,
    }
}
