#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers for summarising a bot's routines: how many run, their trigger mix and their
//! upcoming schedule. Composes `bdots_cron`, `bdots_text` and `bdots_activity` instead of adding
//! a second copy. Only the `botInsights` endpoint calls this, so every helper is live.

/// A routine reduced to what the summary card needs.
#[derive(Clone, Debug, PartialEq)]
pub struct RoutineLine {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub trigger: String,
    pub next_in_ms: Option<i64>,
    pub next_in: String,
}

/// Count routines that are enabled.
pub fn active_count(lines: &[RoutineLine]) -> usize {
    lines.iter().filter(|r| r.enabled).count()
}

/// Count routines that are paused.
pub fn paused_count(lines: &[RoutineLine]) -> usize {
    lines.len() - active_count(lines)
}

/// Count routines with a next run scheduled.
pub fn scheduled_count(lines: &[RoutineLine]) -> usize {
    lines.iter().filter(|r| r.next_in_ms.is_some()).count()
}

/// The soonest next run across all routines.
pub fn next_run(lines: &[RoutineLine]) -> Option<i64> {
    lines.iter().filter_map(|r| r.next_in_ms).min()
}

/// The soonest routine, or `None`.
pub fn next_routine(lines: &[RoutineLine]) -> Option<&RoutineLine> {
    lines.iter().filter(|r| r.next_in_ms.is_some()).min_by_key(|r| r.next_in_ms)
}

/// Human label for the soonest run, using the shared span formatter.
pub fn next_label(lines: &[RoutineLine]) -> String {
    match next_run(lines) {
        Some(ms) => crate::bdots_text::format_span(ms.max(0)),
        None => "Not scheduled".to_owned(),
    }
}

/// Whole percent of routines that are enabled.
pub fn active_percent(lines: &[RoutineLine]) -> u32 {
    crate::bdots_activity::percent(active_count(lines), lines.len())
}

/// A bar of filled cells for enabled routines.
pub fn active_bar(lines: &[RoutineLine], width: usize) -> String {
    "▮".repeat(crate::bdots_activity::bar_width(active_count(lines), lines.len(), width))
}

/// Count routines by trigger word (first token of the description).
pub fn count_by_trigger(lines: &[RoutineLine]) -> Vec<(String, usize)> {
    let mut out: Vec<(String, usize)> = Vec::new();
    for line in lines {
        let word = line.trigger.split_whitespace().next().unwrap_or("other").to_owned();
        if let Some(entry) = out.iter_mut().find(|(w, _)| *w == word) {
            entry.1 += 1;
        } else {
            out.push((word, 1));
        }
    }
    out
}

/// Whether any routine runs on a clock (cron) schedule.
pub fn has_clock(lines: &[RoutineLine]) -> bool {
    lines.iter().any(|r| r.trigger.to_ascii_lowercase().contains("every") || r.trigger.contains(':'))
}

/// Whether any routine is event-driven rather than clock-driven.
pub fn has_event(lines: &[RoutineLine]) -> bool {
    lines.iter().any(|r| {
        let lower = r.trigger.to_ascii_lowercase();
        lower.contains("when") || lower.contains("watch") || lower.contains("file") || lower.contains("webhook")
    })
}

/// Human mix of trigger types: "3 clock · 2 event".
pub fn trigger_mix(lines: &[RoutineLine]) -> String {
    let clock = lines.iter().filter(|r| has_clock(std::slice::from_ref(r))).count();
    let event = lines.len() - clock;
    format!("{clock} clock · {event} event")
}

/// Whether every enabled routine has a next run.
pub fn all_scheduled(lines: &[RoutineLine]) -> bool {
    let active: Vec<&RoutineLine> = lines.iter().filter(|r| r.enabled).collect();
    !active.is_empty() && active.iter().all(|r| r.next_in_ms.is_some())
}

/// Up to `max` routines, soonest first, then paused last.
pub fn soonest_first(lines: &[RoutineLine], max: usize) -> Vec<&RoutineLine> {
    let mut ranked: Vec<&RoutineLine> = lines.iter().collect();
    ranked.sort_by(|a, b| {
        b.enabled.cmp(&a.enabled).then_with(|| a.next_in_ms.cmp(&b.next_in_ms)).then_with(|| a.name.cmp(&b.name))
    });
    ranked.into_iter().take(max).collect()
}

/// Paused routines only, for a "review these" strip.
pub fn paused(lines: &[RoutineLine]) -> Vec<&RoutineLine> {
    lines.iter().filter(|r| !r.enabled).collect()
}

/// One plain sentence describing the schedule.
pub fn schedule_headline(lines: &[RoutineLine]) -> String {
    if lines.is_empty() {
        return "No routines yet.".to_owned();
    }
    let active = active_count(lines);
    let paused = paused_count(lines);
    if active == 0 {
        return format!("{} paused routines", paused);
    }
    format!("{active} active · next {}", next_label(lines))
}

/// Semantic tone for the schedule card.
pub fn schedule_tone(lines: &[RoutineLine]) -> &'static str {
    if lines.is_empty() || active_count(lines) == 0 { "warn" } else { "good" }
}

/// SF Symbol for a trigger description.
pub fn trigger_icon(trigger: &str) -> &'static str {
    let lower = trigger.to_ascii_lowercase();
    if lower.contains("every") || lower.contains(':') {
        "clock"
    } else if lower.contains("when") || lower.contains("watch") || lower.contains("file") {
        "doc.on.doc"
    } else if lower.contains("webhook") || lower.contains("http") {
        "antenna.radiowaves.left.and.right"
    } else {
        "calendar.badge.clock"
    }
}

/// SF Symbol for a routine's enabled state.
pub fn state_icon(enabled: bool) -> &'static str {
    if enabled { "bolt.circle.fill" } else { "pause.circle" }
}

/// Semantic tone for a routine's enabled state.
pub fn state_tone(enabled: bool) -> &'static str {
    if enabled { "good" } else { "secondary" }
}

/// Compact count label for the card, reusing the shared formatter.
pub fn count_label(lines: &[RoutineLine]) -> String {
    crate::bdots_desk::compact_count(lines.len() as u64)
}

/// Build a `RoutineLine` from the raw schedule fields already computed by the host.
pub fn line(id: &str, name: &str, enabled: bool, schedules: &[String], next_ms: Option<i64>, now: i64) -> RoutineLine {
    let trigger = schedules.first().cloned().unwrap_or_default();
    let next_in = next_ms.map(|at| crate::bdots_text::format_span((at - now).max(0))).unwrap_or_default();
    RoutineLine { id: id.to_owned(), name: name.to_owned(), enabled, trigger, next_in_ms: next_ms, next_in }
}

/// Assemble the whole summary from already-computed host lines.
pub fn summary(lines: &[RoutineLine]) -> Vec<(&'static str, String)> {
    vec![
        ("active", active_count(lines).to_string()),
        ("paused", paused_count(lines).to_string()),
        ("scheduled", scheduled_count(lines).to_string()),
        ("next", next_label(lines)),
        ("mix", trigger_mix(lines)),
        ("headline", schedule_headline(lines)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(id: &str, enabled: bool, trigger: &str, next: Option<i64>) -> RoutineLine {
        RoutineLine {
            id: id.to_owned(),
            name: format!("R{id}"),
            enabled,
            trigger: trigger.to_owned(),
            next_in_ms: next,
            next_in: next.map_or_default(|n| crate::bdots_text::format_span(n)),
        }
    }

    #[test]
    fn counts_and_schedule_state() {
        let lines = vec![line("a", true, "every hour", Some(60_000)), line("b", false, "when a file changes", None)];
        assert_eq!(active_count(&lines), 1);
        assert_eq!(paused_count(&lines), 1);
        assert_eq!(scheduled_count(&lines), 1);
        assert_eq!(next_run(&lines), Some(60_000));
        assert_eq!(next_routine(&lines).map(|r| r.id.as_str()), Some("a"));
        assert_eq!(active_percent(&lines), 50);
        assert!(!active_bar(&lines, 8).is_empty());
        assert!(!all_scheduled(&lines));
    }

    #[test]
    fn triggers_and_mix() {
        let lines = vec![line("a", true, "every day at 9am", Some(1)), line("b", true, "when email arrives", Some(2))];
        assert!(has_clock(&lines));
        assert!(has_event(&lines));
        assert_eq!(count_by_trigger(&lines).len(), 2);
        assert!(trigger_mix(&lines).contains("clock"));
        assert_eq!(trigger_icon("every hour"), "clock");
        assert_eq!(trigger_icon("when a file changes"), "doc.on.doc");
        assert_eq!(state_icon(true), "bolt.circle.fill");
        assert_eq!(state_tone(false), "secondary");
    }

    #[test]
    fn ordering_and_strips() {
        let lines = vec![
            line("a", false, "every hour", None),
            line("b", true, "every day", Some(200)),
            line("c", true, "every day", Some(100)),
        ];
        assert_eq!(soonest_first(&lines, 3)[0].id, "c");
        assert_eq!(paused(&lines).len(), 1);
        assert_eq!(count_label(&lines), "3");
        assert_eq!(summary(&lines).len(), 6);
    }

    #[test]
    fn headlines_and_tones() {
        assert!(schedule_headline(&[]).contains("No routines"));
        assert_eq!(schedule_tone(&[]), "warn");
        let lines = vec![line("a", true, "every hour", Some(60_000))];
        assert!(schedule_headline(&lines).contains("active"));
        assert_eq!(schedule_tone(&lines), "good");
        assert!(all_scheduled(&lines));
    }

    #[test]
    fn line_shapes_the_host_fields() {
        let line = line("x", "Build", true, &["every weekday".into()], Some(120_000), 0);
        assert_eq!(line.name, "Build");
        assert_eq!(line.next_in_ms, Some(120_000));
        assert_eq!(next_label(std::slice::from_ref(&line)), "2 min");
    }
}
