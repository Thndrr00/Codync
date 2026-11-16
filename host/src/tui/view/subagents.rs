//! A turn's subagents in the transcript: their card, and the header line the dock shows while
//! the card is out of view (`chat::chat`).

use ratatui::text::{Line, Span};

use super::super::app::Entry;
use super::super::l10n::tr;
use super::super::md::truncate;
use super::style::theme;
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
    let now = crate::store::now_ms();
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
