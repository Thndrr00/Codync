//! The band over the composer for a channel's request tree: its rows, the picked one, and the keys that act on it.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use super::super::app::{App, Bot, Task};
use super::super::l10n::tr;
use super::super::md::truncate;
use super::buffer::{put, rput, u};
use super::style::theme;

/// Draws the sub-task rows right over the composer at `box_top`, above the queue band: where the band starts.
pub(super) fn tasks_band(buf: &mut Buffer, r: Rect, app: &App, bot: &Bot, box_top: u16, typing: bool) -> u16 {
    let t = theme();
    let rows = if app.tasks_band.assign.is_some() { assign_rows(app, bot) } else { task_rows(app, &bot.tasks) };
    let strip_top = box_top.saturating_sub(u(rows.len()));
    for (i, (text, hint, picked)) in rows.iter().enumerate() {
        let y = strip_top + u(i);
        let style = if *picked { t.sel } else { t.secondary };
        if *picked {
            put(buf, r.x + 1, y, 2, "\u{25b8}", t.bold);
        }
        let hint = if typing { "esc".to_owned() } else { hint.clone() };
        let room = rput(buf, r.right() - 1, y, &hint, t.dim).saturating_sub(r.x + 4);
        put(buf, r.x + 4, y, room, &truncate(text, usize::from(room)), style);
    }
    strip_top
}

/// The rows the band shows: up to three of the tree, the picked one marked, with the keys beside the first and last.
fn task_rows(app: &App, tasks: &[Task]) -> Vec<(String, String, bool)> {
    let picked = app.tasks_band.cursor.min(tasks.len().saturating_sub(1));
    let start = picked.saturating_sub(super::super::app::tasks::BAND_ROWS - 1);
    let mut rows: Vec<(String, String, bool)> = tasks
        .iter()
        .enumerate()
        .skip(start)
        .take(super::super::app::tasks::BAND_ROWS)
        .map(|(i, task)| {
            let owner = task.owner_name.clone().unwrap_or_else(|| tr("The lead").into_owned());
            let state = tr(state_label(&task.state));
            let mut text = format!("{owner} \u{b7} {state}");
            if !task.instruction.is_empty() {
                text.push_str(" \u{b7} ");
                text.push_str(&task.instruction);
            }
            (text, String::new(), i == picked)
        })
        .collect();
    let waiting = tasks.iter().filter(|task| task.open()).count();
    let label = tr("sub-tasks").into_owned();
    let hidden = tasks.len().saturating_sub(rows.len() + start);
    match rows.as_mut_slice() {
        [] => {}
        [only] => only.1 = format!("{label} \u{b7} {}", keys(&tasks[picked])),
        [first, .., last] => {
            first.1 = format!("{label} \u{b7} {waiting}");
            let more = if hidden > 0 { format!(" +{hidden}") } else { String::new() };
            last.1 = format!("{}{more}", keys(&tasks[picked]));
        }
    }
    rows
}

/// The keys the picked row offers.
fn keys(task: &Task) -> String {
    if task.stopped() {
        tr("O goes on \u{b7} A reassigns \u{b7} B cancels").into_owned()
    } else {
        tr("W stops").into_owned()
    }
}

/// The member picker of Reassign: the channel's members, the picked one marked.
fn assign_rows(app: &App, bot: &Bot) -> Vec<(String, String, bool)> {
    let members: Vec<String> = bot.members.iter().map(|m| app.author_name(Some(m))).collect();
    let at = app.tasks_band.member.min(members.len().saturating_sub(1));
    members
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let hint = if i == at { tr("enter gives it \u{b7} esc").into_owned() } else { String::new() };
            (name.clone(), hint, i == at)
        })
        .collect()
}

/// A row's state as the band words it.
fn state_label(state: &str) -> &'static str {
    match state {
        "queued" => "waiting",
        "running" => "working",
        "completed" => "done",
        "failed" => "failed",
        "paused" => "stopped",
        _ => "cancelled",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::app::{Msg, test_app};
    use serde_json::json;

    #[test]
    fn the_band_words_each_state_and_offers_the_keys_of_the_picked_row() {
        let mut app = test_app();
        app.on_msg(Msg::Event(json!({"type": "bot", "bot": {
            "id": "g", "name": "Crew", "kind": "group", "members": ["a"],
            "tasks": [
                {"id": "t1", "instruction": "Ship it", "state": "running"},
                {"id": "t2", "instruction": "Assignment", "ownerName": "Bob", "state": "paused"},
            ],
        }})));
        app.selected = Some("g".into());
        let bot = app.bot().cloned().unwrap();
        let rows = task_rows(&app, &bot.tasks);
        assert_eq!(rows.len(), 2);
        assert!(rows[0].0.contains("The lead \u{b7} working \u{b7} Ship it"), "{}", rows[0].0);
        assert!(rows[0].1.contains("sub-tasks"), "{}", rows[0].1);
        app.tasks_band.cursor = 1;
        let rows = task_rows(&app, &bot.tasks);
        assert!(rows[1].0.contains("Bob \u{b7} stopped"), "{}", rows[1].0);
        assert!(rows[1].1.contains("reassigns"), "{}", rows[1].1);
    }
}
