#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers from bdots' latest prompt and model sources. Composes the validators already
//! ported (`bdots_desk`, `bdots_text`, `bdots_cron`, `bdots_more`) instead of duplicating them.
//! Only `botInsights` calls this.

/// Short display name for a model id (models.ts modelName).
pub fn model_name(id: &str) -> String {
    let clean = id.trim_start_matches("@cf/");
    let clean = clean.rsplit('/').next().unwrap_or(clean);
    if clean.is_empty() { "unknown".to_owned() } else { clean.to_owned() }
}

/// Short display name with provider, reusing the provider helper.
pub fn model_display(id: &str) -> String {
    let provider = crate::bdots_desk::provider_of(id).unwrap_or("model");
    format!("{} · {}", provider, model_name(id))
}

/// Human context-window label, reusing the shared formatter.
pub fn context_label(tokens: u64) -> String {
    let value = crate::bdots_text::compact_count(tokens);
    format!("{value} context")
}

/// Price line from input/output per million, reusing the shared formatter.
pub fn price_line(input: f64, output: f64) -> String {
    format!(
        "{} in · {} out per 1M",
        crate::bdots_extra::format_money(crate::bdots_extra::to_minor_units(input, "USD"), "USD"),
        crate::bdots_extra::format_money(crate::bdots_extra::to_minor_units(output, "USD"), "USD")
    )
}

/// Whether a model id is valid, reusing the shared validator.
pub fn is_valid_model(id: &str) -> bool {
    crate::bdots_desk::is_model_id(id)
}

/// Human line for a check-in's last time.
pub fn check_in_line(last_ms: Option<i64>) -> String {
    match last_ms {
        Some(ms) => format!("Last check-in {}", crate::bdots_text::format_span(ms.max(0))),
        None => "First check-in".to_owned(),
    }
}

/// Header for a routine prompt, from its name and id.
pub fn routine_header(name: &str, id: &str) -> String {
    if name.trim().is_empty() { format!("Routine {id}") } else { name.to_owned() }
}

/// Whether a routine reports only new entries.
pub fn only_new_line(only_new: bool) -> &'static str {
    if only_new { "Report only new work since last run." } else { "Report everything in the window." }
}

/// A bullet for a reported entry, in plain words.
pub fn reported_line(title: &str, when_ms: i64) -> String {
    format!("• {} · {}", crate::bdots_more::shorten(title, 64), crate::bdots_text::format_span(when_ms.max(0)))
}

/// Number of reported entries as a compact label.
pub fn reported_count(entries: usize) -> String {
    crate::bdots_desk::compact_count(entries as u64)
}

/// Human label for a time zone, reusing the shared formatter.
pub fn zone_text(zone: &str) -> String {
    if zone.trim().is_empty() { "computer time".to_owned() } else { zone.to_owned() }
}

/// Whether a prompt kind is scheduled by the clock.
pub fn is_clock_kind(kind: &str) -> bool {
    matches!(kind, "check-in" | "routine")
}

/// Whether a prompt kind watches for changes.
pub fn is_watch_kind(kind: &str) -> bool {
    matches!(kind, "crawl" | "webhook" | "app-event")
}

/// SF Symbol for a prompt kind, reusing the fresh module.
pub fn prompt_icon(kind: &str) -> &'static str {
    crate::bdots_fresh::prompt_icon(kind)
}

/// SF Symbol for a prompt's schedule.
pub fn schedule_icon(is_clock: bool) -> &'static str {
    if is_clock { "clock" } else { "bolt" }
}

/// Semantic tone for a prompt.
pub fn prompt_tone(kind: &str) -> &'static str {
    crate::bdots_fresh::prompt_tone(kind)
}

/// Prompt length hint: short, medium or long.
pub fn length_hint(lines: usize) -> &'static str {
    if lines <= 3 {
        "short"
    } else if lines <= 10 {
        "medium"
    } else {
        "long"
    }
}

/// A plain instruction prefix for a prompt.
pub fn instruction_prefix() -> &'static str {
    "Work only within your own chat. Never reveal keys or private data."
}

/// Whether a prompt should be sent with its own sent times.
pub fn with_sent_times(count: usize) -> bool {
    count > 0
}

/// A line naming who sent a message, for the transcript.
pub fn sender_line(name: &str, sent: bool) -> String {
    format!("{}{}", crate::bdots_more::shorten(name, 48), if sent { " (sent)" } else { "" })
}

/// A note appended to a message, trimmed.
pub fn note_line(note: Option<&str>) -> String {
    note.filter(|n| !n.trim().is_empty())
        .map_or_else(String::new, |n| format!(" [{}]", crate::bdots_more::shorten(n, 64)))
}

/// Build the parts of a routine prompt as plain lines.
pub fn routine_lines(name: &str, id: &str, only_new: bool, reported: usize, zone: &str) -> Vec<String> {
    vec![
        routine_header(name, id),
        only_new_line(only_new).to_owned(),
        format!("{} reported entries", reported_count(reported)),
        format!("Time zone: {}", zone_text(zone)),
    ]
}

/// Summary rows for the card.
pub fn summary(id: &str, reported: usize, zone: &str) -> Vec<(&'static str, String)> {
    vec![
        ("model", model_display(id)),
        ("context", context_label(1_000_000)),
        ("reported", reported_count(reported)),
        ("zone", zone_text(zone)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_helpers() {
        assert_eq!(model_name("@cf/meta/llama-3"), "llama-3");
        assert_eq!(model_name("claude/sonnet"), "sonnet");
        assert!(model_display("claude-sonnet-4").contains("claude"));
        assert!(context_label(1_000_000).contains("context"));
        assert!(price_line(2.0, 10.0).contains("in"));
        assert!(is_valid_model("claude-sonnet-4"));
    }

    #[test]
    fn prompt_lines() {
        assert!(check_in_line(None).contains("First"));
        assert!(check_in_line(Some(60_000)).contains("Last"));
        assert_eq!(routine_header("", "r1"), "Routine r1");
        assert_eq!(routine_header("Build", "r1"), "Build");
        assert!(only_new_line(true).contains("only new"));
        assert!(reported_line("Done", 60_000).contains("Done"));
        assert_eq!(reported_count(1500), "1.5K");
        assert_eq!(zone_text(""), "computer time");
    }

    #[test]
    fn kinds_and_icons() {
        assert!(is_clock_kind("routine"));
        assert!(!is_clock_kind("crawl"));
        assert!(is_watch_kind("webhook"));
        assert!(!is_watch_kind("routine"));
        assert_eq!(prompt_icon("crawl"), "globe");
        assert_eq!(schedule_icon(true), "clock");
        assert_eq!(schedule_icon(false), "bolt");
        assert_eq!(prompt_tone("email"), "secondary");
    }

    #[test]
    fn lengths_senders_and_notes() {
        assert_eq!(length_hint(2), "short");
        assert_eq!(length_hint(6), "medium");
        assert_eq!(length_hint(20), "long");
        assert!(instruction_prefix().contains("keys"));
        assert!(with_sent_times(1));
        assert!(!with_sent_times(0));
        assert!(sender_line("Ada", true).contains("sent"));
        assert!(note_line(Some(" hi ")).contains("hi"));
        assert!(note_line(None).is_empty());
    }

    #[test]
    fn routine_prompt_and_summary() {
        let lines = routine_lines("Build", "r1", true, 3, "UTC");
        assert_eq!(lines.len(), 4);
        assert!(lines[1].contains("only new"));
        let rows = summary("claude-sonnet-4", 3, "UTC");
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[2].1, "3");
    }
}
