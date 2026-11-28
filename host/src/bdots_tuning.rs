#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers for a bot's model tuning: the ACP config options (effort, reasoning, fast) plus
//! the related switches. Reuses `bdots_desk`, `bdots_text` and `bdots_more` instead of copying
//! their validators. Only `botInsights` calls this.

/// One tuning option reduced to what the card shows.
#[derive(Clone, Debug, PartialEq)]
pub struct OptionLine {
    pub key: String,
    pub title: String,
    pub value: String,
    pub icon: &'static str,
    pub tone: &'static str,
}

/// Human title for an ACP option key.
pub fn option_title(key: &str) -> String {
    let cleaned = key.trim().replace(['-', '_', '.'], " ");
    cleaned
        .split_whitespace()
        .map(|w| {
            let mut chars = w.chars();
            chars.next().map_or(String::new(), |c| c.to_uppercase().collect::<String>() + chars.as_str())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// SF Symbol for an ACP option key, shared by the four clients.
pub fn option_icon(key: &str) -> &'static str {
    let lower = key.to_ascii_lowercase();
    if lower.contains("effort") || lower.contains("reason") || lower.contains("think") {
        "brain"
    } else if lower.contains("fast") || lower.contains("speed") {
        "bolt"
    } else if lower.contains("verbos") || lower.contains("length") {
        "text.alignleft"
    } else {
        "gearshape"
    }
}

/// Semantic tone for an option's value.
pub fn value_tone(value: &str) -> &'static str {
    match value.to_ascii_lowercase().as_str() {
        "high" | "max" | "ultra" => "warn",
        "medium" | "normal" => "accent",
        "low" | "minimal" | "none" => "good",
        _ => "secondary",
    }
}

/// Whether an option value is on.
pub fn is_on(value: &str) -> bool {
    matches!(value.to_ascii_lowercase().as_str(), "true" | "yes" | "on" | "1" | "high")
}

/// Short display value, reusing the shared shortener.
pub fn short_value(value: &str) -> String {
    crate::bdots_more::shorten(value.trim(), 48)
}

/// Build a display line from an ACP option pair.
pub fn option_line(key: &str, value: &str) -> OptionLine {
    OptionLine {
        key: key.to_owned(),
        title: option_title(key),
        value: short_value(value),
        icon: option_icon(key),
        tone: value_tone(value),
    }
}

/// Count options whose value is on.
pub fn on_count(lines: &[OptionLine]) -> usize {
    lines.iter().filter(|l| is_on(&l.value)).count()
}

/// Count options whose value is off.
pub fn off_count(lines: &[OptionLine]) -> usize {
    lines.len() - on_count(lines)
}

/// Whether a tuning option is present.
pub fn has_option(lines: &[OptionLine], key: &str) -> bool {
    lines.iter().any(|l| l.key.eq_ignore_ascii_case(key))
}

/// Value of an option, or a default.
pub fn option_value(lines: &[OptionLine], key: &str) -> String {
    lines.iter().find(|l| l.key.eq_ignore_ascii_case(key)).map_or_else(|| "default".to_owned(), |l| l.value.clone())
}

/// Whether the bot runs with high effort.
pub fn high_effort(lines: &[OptionLine]) -> bool {
    lines.iter().any(|l| {
        l.key.to_ascii_lowercase().contains("effort")
            && matches!(l.value.to_ascii_lowercase().as_str(), "high" | "max" | "ultra")
    })
}

/// Whether reasoning/thinking is enabled.
pub fn reasoning_on(lines: &[OptionLine]) -> bool {
    lines.iter().any(|l| {
        let k = l.key.to_ascii_lowercase();
        (k.contains("reason") || k.contains("think")) && is_on(&l.value)
    })
}

/// Human label for the effort level.
pub fn effort_label(lines: &[OptionLine]) -> &'static str {
    if high_effort(lines) { "Thorough" } else { "Default" }
}

/// SF Symbol for the effort level.
pub fn effort_icon(lines: &[OptionLine]) -> &'static str {
    if high_effort(lines) { "tortoise.fill" } else { "gauge" }
}

/// One plain sentence describing the tuning.
pub fn tuning_headline(lines: &[OptionLine], fast: bool) -> String {
    if lines.is_empty() && !fast {
        return "Using the model defaults.".to_owned();
    }
    format!(
        "{} · {} option{} set{}",
        effort_label(lines),
        lines.len(),
        if lines.len() == 1 { "" } else { "s" },
        if fast { " · fast mode" } else { "" }
    )
}

/// Semantic tone for the tuning card.
pub fn tuning_tone(lines: &[OptionLine], fast: bool) -> &'static str {
    if lines.is_empty() && !fast { "secondary" } else { "good" }
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(lines: &[OptionLine]) -> String {
    crate::bdots_desk::compact_count(lines.len() as u64)
}

/// Options sorted by title, for a stable list across clients.
pub fn sorted(lines: &[OptionLine]) -> Vec<&OptionLine> {
    let mut out: Vec<&OptionLine> = lines.iter().collect();
    out.sort_by(|a, b| a.title.cmp(&b.title));
    out
}

/// Up to `max` options that are on, for a short strip.
pub fn enabled_first(lines: &[OptionLine], max: usize) -> Vec<&OptionLine> {
    let mut out: Vec<&OptionLine> = lines.iter().collect();
    out.sort_by(|a, b| is_on(&b.value).cmp(&is_on(&a.value)).then_with(|| a.title.cmp(&b.title)));
    out.into_iter().take(max).collect()
}

/// Summary rows for the card.
pub fn summary(lines: &[OptionLine], fast: bool) -> Vec<(&'static str, String)> {
    vec![
        ("options", count_label(lines)),
        ("effort", effort_label(lines).to_owned()),
        ("headline", tuning_headline(lines, fast)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines() -> Vec<OptionLine> {
        vec![option_line("effort", "high"), option_line("reasoning", "true"), option_line("verbosity", "low")]
    }

    #[test]
    fn titles_icons_and_tones() {
        assert_eq!(option_title("thinking_level"), "Thinking Level");
        assert_eq!(option_title("effort"), "Effort");
        assert_eq!(option_icon("effort"), "brain");
        assert_eq!(option_icon("fast"), "bolt");
        assert_eq!(option_icon("other"), "gearshape");
        assert_eq!(value_tone("high"), "warn");
        assert_eq!(value_tone("low"), "good");
        assert_eq!(value_tone("weird"), "secondary");
    }

    #[test]
    fn on_off_and_values() {
        assert!(is_on("true"));
        assert!(is_on("HIGH"));
        assert!(!is_on("false"));
        assert_eq!(short_value(" abcdef "), "abcdef");
        let l = lines();
        assert_eq!(on_count(&l), 2);
        assert_eq!(off_count(&l), 1);
        assert!(has_option(&l, "EFFORT"));
        assert_eq!(option_value(&l, "effort"), "high");
        assert_eq!(option_value(&l, "missing"), "default");
    }

    #[test]
    fn effort_and_reasoning() {
        let l = lines();
        assert!(high_effort(&l));
        assert!(reasoning_on(&l));
        assert_eq!(effort_label(&l), "Thorough");
        assert_eq!(effort_icon(&l), "tortoise.fill");
        let empty: Vec<OptionLine> = Vec::new();
        assert!(!high_effort(&empty));
        assert_eq!(effort_label(&empty), "Default");
        assert_eq!(effort_icon(&empty), "gauge");
    }

    #[test]
    fn headlines_tones_and_order() {
        let l = lines();
        assert!(tuning_headline(&l, true).contains("fast"));
        assert!(tuning_headline(&[], false).contains("defaults"));
        assert_eq!(tuning_tone(&[], false), "secondary");
        assert_eq!(tuning_tone(&l, false), "good");
        assert_eq!(count_label(&l), "3");
        assert_eq!(sorted(&l)[0].title, "Effort");
        assert_eq!(enabled_first(&l, 1)[0].title, "Effort");
        assert_eq!(summary(&l, false).len(), 3);
    }
}
