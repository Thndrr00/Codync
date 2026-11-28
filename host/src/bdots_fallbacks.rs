#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers for a bot's resilience: its agent fallback chain and the switches that let it
//! keep working (fast mode, background model, saver). Composes `bdots_desk`, `bdots_profile`
//! and `bdots_activity` instead of duplicating them. Only `botInsights` calls this.

/// One fallback agent reduced to what the card shows.
#[derive(Clone, Debug, PartialEq)]
pub struct FallbackLine {
    pub backend: String,
    pub model: String,
    pub command: String,
    pub label: String,
    pub icon: &'static str,
    pub tone: &'static str,
}

/// Symbol for a fallback's position in the chain.
pub fn position_icon(index: usize) -> &'static str {
    match index {
        0 => "1.circle.fill",
        1 => "2.circle.fill",
        2 => "3.circle.fill",
        _ => "number.circle.fill",
    }
}

/// Semantic tone for a fallback position: first is immediate, later are last resorts.
pub fn position_tone(index: usize) -> &'static str {
    if index == 0 {
        "good"
    } else if index < 3 {
        "accent"
    } else {
        "secondary"
    }
}

/// Human label for a fallback's backend, reusing the provider helper.
pub fn backend_label(backend: &str) -> String {
    if backend.is_empty() {
        return "Unknown agent".to_owned();
    }
    crate::bdots_desk::provider_of(backend).map_or_else(|| backend.to_owned(), str::to_owned)
}

/// Whether a fallback has a model override.
pub fn has_model(model: Option<&str>) -> bool {
    model.is_some_and(|m| crate::bdots_desk::is_model_id(m))
}

/// Whether a fallback runs a custom command.
pub fn has_command(command: Option<&str>) -> bool {
    command.is_some_and(|c| !c.trim().is_empty())
}

/// Short display detail for a fallback.
pub fn fallback_detail(backend: &str, model: Option<&str>, command: Option<&str>) -> String {
    let label = backend_label(backend);
    let extra = if has_model(model) {
        model.unwrap_or_default().to_owned()
    } else if has_command(command) {
        "custom command".to_owned()
    } else {
        "default settings".to_owned()
    };
    crate::bdots_more::shorten(&format!("{label} · {extra}"), 64)
}

/// Build a display line for one fallback.
pub fn fallback_line(index: usize, backend: &str, model: Option<&str>, command: Option<&str>) -> FallbackLine {
    FallbackLine {
        backend: backend.to_owned(),
        model: model.unwrap_or("").to_owned(),
        command: command.unwrap_or("").to_owned(),
        label: format!("{} {}", position_icon(index), backend_label(backend)),
        icon: position_icon(index),
        tone: position_tone(index),
    }
}

/// Number of explicit fallback agents.
pub fn chain_length(lines: &[FallbackLine]) -> usize {
    lines.len()
}

/// Total resilience options: explicit fallbacks plus the self-fallback switch.
pub fn total_options(lines: &[FallbackLine], self_fallbacks: bool) -> usize {
    chain_length(lines) + usize::from(self_fallbacks)
}

/// Whether the bot has any resilience at all.
pub fn has_any(lines: &[FallbackLine], self_fallbacks: bool) -> bool {
    total_options(lines, self_fallbacks) > 0
}

/// Whether a fallback chain exists (not just the self switch).
pub fn has_chain(lines: &[FallbackLine]) -> bool {
    !lines.is_empty()
}

/// Whether any fallback overrides the model.
pub fn has_override(lines: &[FallbackLine]) -> bool {
    lines.iter().any(|l| !l.model.is_empty())
}

/// Count fallbacks with a custom command.
pub fn custom_command_count(lines: &[FallbackLine]) -> usize {
    lines.iter().filter(|l| !l.command.is_empty()).count()
}

/// A 0–100 resilience score.
pub fn resilience_score(
    lines: &[FallbackLine],
    self_fallbacks: bool,
    fast: bool,
    saver: bool,
    background: Option<&str>,
) -> u32 {
    let mut score = 20;
    score += (has_chain(lines) as u32) * 20;
    score += (self_fallbacks as u32) * 10;
    score += (fast as u32) * 10;
    score += (saver as u32) * 10;
    score += (background.is_some() as u32) * 15;
    score += (has_override(lines) as u32) * 10;
    score.min(100)
}

/// Semantic tone for a resilience score.
pub fn resilience_tone(score: u32) -> &'static str {
    if score >= 70 {
        "good"
    } else if score >= 40 {
        "attention"
    } else {
        "danger"
    }
}

/// Short label for a resilience score.
pub fn resilience_label(score: u32) -> &'static str {
    if score >= 70 {
        "Resilient"
    } else if score >= 40 {
        "Some cover"
    } else {
        "Single point"
    }
}

/// Bar of filled cells for the resilience score.
pub fn resilience_bar(score: u32, width: usize) -> String {
    let filled = ((score as usize * width) / 100).clamp(0, width);
    "▮".repeat(filled)
}

/// One plain sentence about resilience.
pub fn resilience_headline(lines: &[FallbackLine], self_fallbacks: bool) -> String {
    let options = total_options(lines, self_fallbacks);
    if options == 0 {
        return "No fallback if the agent stops.".to_owned();
    }
    format!("{options} way{} to keep working", if options == 1 { "" } else { "s" })
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(lines: &[FallbackLine]) -> String {
    crate::bdots_desk::compact_count(lines.len() as u64)
}

/// Fallback lines ordered by position, with the chain first.
pub fn ordered(lines: &[FallbackLine]) -> Vec<&FallbackLine> {
    lines.iter().collect()
}

/// Switch labels for the resilience card.
pub fn switches(fast: bool, saver: bool, background: Option<&str>) -> Vec<(&'static str, bool)> {
    vec![("Fast mode", fast), ("Budget saver", saver), ("Background model", background.is_some())]
}

/// Assemble the whole resilience summary.
pub fn summary(lines: &[FallbackLine], self_fallbacks: bool) -> Vec<(&'static str, String)> {
    vec![
        ("chain", count_label(lines)),
        ("options", total_options(lines, self_fallbacks).to_string()),
        ("headline", resilience_headline(lines, self_fallbacks)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines() -> Vec<FallbackLine> {
        vec![fallback_line(0, "codex", Some("gpt-5"), None), fallback_line(1, "gemini", None, Some("gemini-cli"))]
    }

    #[test]
    fn icons_and_tones_by_position() {
        assert_eq!(position_icon(0), "1.circle.fill");
        assert_eq!(position_icon(5), "number.circle.fill");
        assert_eq!(position_tone(0), "good");
        assert_eq!(position_tone(4), "secondary");
    }

    #[test]
    fn backends_and_detectors() {
        assert_eq!(backend_label(""), "Unknown agent");
        assert_eq!(backend_label("claude"), "Anthropic");
        assert!(has_model(Some("claude-sonnet-4")));
        assert!(!has_model(None));
        assert!(has_command(Some(" gemini-cli ")));
        assert!(!has_command(None));
        assert!(fallback_detail("codex", Some("gpt-5"), None).contains("codex"));
    }

    #[test]
    fn chains_and_counts() {
        let l = lines();
        assert_eq!(chain_length(&l), 2);
        assert_eq!(total_options(&l, true), 3);
        assert!(has_any(&l, false));
        assert!(has_chain(&l));
        assert!(has_override(&l));
        assert_eq!(custom_command_count(&l), 1);
        assert_eq!(ordered(&l).len(), 2);
        assert_eq!(count_label(&l), "2");
        assert_eq!(summary(&l, true).len(), 3);
    }

    #[test]
    fn empty_chain_has_no_cover() {
        let l: Vec<FallbackLine> = Vec::new();
        assert!(!has_any(&l, false));
        assert!(!has_chain(&l));
        assert!(resilience_headline(&l, false).contains("No fallback"));
    }

    #[test]
    fn score_labels_and_bar() {
        let l = lines();
        let low = resilience_score(&[], false, false, false, None);
        let high = resilience_score(&l, true, true, true, Some("haiku"));
        assert!(high > low);
        assert_eq!(resilience_tone(80), "good");
        assert_eq!(resilience_label(50), "Some cover");
        assert!(!resilience_bar(high, 10).is_empty());
        assert_eq!(switches(true, false, None).len(), 3);
        assert!(switches(true, false, None)[0].1);
    }
}
