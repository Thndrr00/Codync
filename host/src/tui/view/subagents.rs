//! A turn's subagents in the transcript: their card, and the header line the dock shows while
//! the card is out of view (`chat::chat`).

use ratatui::text::{Line, Span};

use super::super::app::Entry;
use super::super::l10n::tr;
use super::super::md::truncate;
use super::style::theme;
use super::time::now_ms;
use super::transcript::{Built, gap};

/// The card: their progress and time, then a row for each with its task and newest step.
pub(super) fn card(out: &mut Built, e: &Entry, width: usize) {
    let t = theme();
    let list = e.data["subagents"].as_array().cloned().unwrap_or_default();
    if list.is_empty() {
        return;
    }
    gap(out);
    let from = out.lines.len();
    let running = list.iter().any(|s| s["state"].as_str().unwrap_or("running") == "running");
    let finished = list.len() - list.iter().filter(|s| s["state"].as_str().unwrap_or("running") == "running").count();
    let now = now_ms();
    let start = list.iter().filter_map(|s| s["startedAt"].as_i64()).min().unwrap_or(now);
    let end = if running { now } else { list.iter().filter_map(|s| s["endedAt"].as_i64()).max().unwrap_or(now) };
    let header = format!("{} {} of {}  {}", tr("Subagents"), finished, list.len(), duration(end - start));
    out.lines.push(Line::from(vec![Span::styled(" ▦ ", t.secondary), Span::styled(header.clone(), t.bold)]));
    let current = list.iter().find(|s| s["state"].as_str().unwrap_or("running") == "running");
    let dock = Line::from(vec![
        Span::styled(" ▦ ", t.secondary),
        Span::styled(format!("{header} · "), t.secondary),
        Span::styled(
            current
                .and_then(|s| s["step"].as_str().or(s["task"].as_str()))
                .filter(|step| !step.is_empty())
                .map_or_else(|| tr("Working…").into_owned(), str::to_owned),
            t.dim,
        ),
        Span::styled("  ›", t.dim),
    ]);
    for subagent in &list {
        let state = subagent["state"].as_str().unwrap_or("running");
        let (mark, style) = match state {
            "done" => ("✓", t.secondary),
            "failed" => ("×", t.red),
            "stopped" => ("−", t.dim),
            _ => ("◌", t.secondary),
        };
        let task = subagent["task"].as_str().unwrap_or_default();
        let at = subagent["startedAt"].as_i64().unwrap_or(now);
        let ended = subagent["endedAt"].as_i64().unwrap_or(if running { now } else { at });
        out.lines.push(Line::from(vec![
            Span::styled(format!("   {mark} "), style),
            Span::styled(truncate(task, width.saturating_sub(24)), t.text),
            Span::styled(format!("  {}", duration(ended - at)), t.dim),
        ]));
        let step = subagent["step"].as_str().unwrap_or_default();
        if !step.is_empty() {
            out.lines
                .push(Line::from(Span::styled(format!("     {}", truncate(step, width.saturating_sub(7))), t.dim)));
        }
    }
    out.messages.push((e.id.clone(), from, out.lines.len()));
    out.subagents.push((e.id.clone(), running, from, out.lines.len(), dock));
}

/// "12s" or "1m 05s": how long a subagent, or all of them, took.
fn duration(ms: i64) -> String {
    let total = (ms / 1_000).max(0);
    if total < 60 { format!("{total}s") } else { format!("{}m {:02}s", total / 60, total % 60) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::app::Kind;
    use serde_json::json;

    fn entry(data: serde_json::Value) -> Entry {
        Entry { id: "card".into(), seq: 1, turn: 1, kind: Kind::Subagents, data, created_at: 0 }
    }

    fn drawn(e: &Entry) -> Built {
        let mut out = Built { lines: vec![], buttons: vec![], messages: vec![], subagents: vec![] };
        card(&mut out, e, 80);
        out
    }

    #[test]
    fn a_finished_card_counts_its_subagents_and_shows_each_time() {
        let out = drawn(&entry(json!({"subagents": [
            {"id": "t1", "task": "Read the tests", "state": "done", "step": "Read src/lib.rs",
             "startedAt": 1_000, "endedAt": 4_000},
            {"id": "t2", "task": "Fix the retry", "state": "failed", "startedAt": 1_000, "endedAt": 62_000},
        ]})));
        let lines: Vec<String> = out.lines.iter().map(ToString::to_string).collect();
        assert_eq!(lines[0], format!(" ▦ {} 2 of 2  1m 01s", tr("Subagents")), "{lines:?}");
        assert_eq!(lines[1], "   ✓ Read the tests  3s");
        assert_eq!(lines[2], "     Read src/lib.rs");
        assert_eq!(lines[3], "   × Fix the retry  1m 01s");
        assert!(out.messages.iter().any(|(id, _, _)| id == "card"), "the card can be picked");
        let docked = matches!(out.subagents.as_slice(), [(id, false, _, _, _)] if id == "card");
        assert!(docked, "a finished card docks nothing");
    }

    #[test]
    fn a_running_card_docks_its_newest_step() {
        let now = now_ms();
        let out = drawn(&entry(json!({"subagents": [
            {"id": "t1", "task": "Review the upload API", "state": "done",
             "startedAt": now - 5_000, "endedAt": now - 2_000},
            {"id": "t2", "task": "Fix the retry", "state": "running", "step": "Read src/chat/uploads.rs",
             "startedAt": now - 1_000},
        ]})));
        let lines: Vec<String> = out.lines.iter().map(ToString::to_string).collect();
        assert!(lines[0].starts_with(&format!(" ▦ {} 1 of 2  ", tr("Subagents"))), "{lines:?}");
        assert!(lines.iter().any(|l| l == "   ◌ Fix the retry  1s"), "{lines:?}");
        assert!(lines.iter().any(|l| l == "     Read src/chat/uploads.rs"));
        let dock = match out.subagents.as_slice() {
            [(id, running, _, _, dock)] => {
                assert_eq!(id, "card");
                assert!(*running);
                dock.clone()
            }
            other => panic!("one running card: {}", other.len()),
        };
        let text = dock.to_string();
        assert!(text.starts_with(&format!(" ▦ {} 1 of 2  ", tr("Subagents"))), "{text}");
        assert!(text.ends_with("· Read src/chat/uploads.rs  ›"), "{text}");
    }

    #[test]
    fn a_card_with_no_subagents_draws_nothing() {
        let out = drawn(&entry(json!({"subagents": []})));
        assert!(out.lines.is_empty() && out.messages.is_empty() && out.subagents.is_empty());
    }
}
