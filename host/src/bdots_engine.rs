#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from agentes-chat's `brain.ts`: engine kinds, states and "what thinks"
//! lines. Composes helpers already ported into this host. Only `botInsights` calls this.

/// Engine rendered by the brain portrait (EngineKind).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineKind {
    ClaudeCode,
    Codex,
    Agent,
}

/// State of a brain engine (EngineState).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineState {
    Idle,
    Working,
    Off,
}

/// Brain kinds that draw Claude Code.
pub const CLAUDE_BRAINS: [&str; 1] = ["claude_code"];

/// Brain kinds that draw Codex's terminal until they have a portrait.
pub const CODEX_BRAINS: [&str; 5] = ["codex", "cursor", "opencode", "grok", "antigravity"];

/// Plans with their own named line in `thinksWith`.
pub const NAMED_PLANS: [&str; 6] = ["claude", "chatgpt", "cursor", "zen", "grok", "gemini"];

/// Engine of a brain kind (engineOfBrain).
pub fn engine_of_brain(brain: Option<&str>) -> EngineKind {
    match brain.unwrap_or("").to_ascii_lowercase().as_str() {
        "claude_code" => EngineKind::ClaudeCode,
        kind if CODEX_BRAINS.contains(&kind) => EngineKind::Codex,
        _ => EngineKind::Agent,
    }
}

/// Engine of a mission executor (engineOfExecutor).
pub fn engine_of_executor(executor: Option<&str>) -> EngineKind {
    match executor.unwrap_or("").to_ascii_lowercase().as_str() {
        "claude" => EngineKind::ClaudeCode,
        "codex" => EngineKind::Codex,
        _ => EngineKind::Agent,
    }
}

/// State of a brain option (engineStateOfOption).
pub fn engine_state(kind: Option<&str>, available: bool, computer_online: Option<bool>) -> EngineState {
    if kind == Some("model") {
        return EngineState::Idle;
    }
    if !available || computer_online == Some(false) { EngineState::Off } else { EngineState::Idle }
}

/// Whether an option's engine is off.
pub fn is_off(state: EngineState) -> bool {
    state == EngineState::Off
}

/// Kind name of an engine, as a stable string.
pub fn engine_name(kind: EngineKind) -> &'static str {
    match kind {
        EngineKind::ClaudeCode => "claude_code",
        EngineKind::Codex => "codex",
        EngineKind::Agent => "agent",
    }
}

/// Human label of an engine, reusing the text formatter.
pub fn engine_label(kind: EngineKind) -> String {
    crate::bdots_textfmt::upper_first(engine_name(kind).replace('_', " ").as_str())
}

/// SF Symbol for an engine.
pub fn engine_icon(kind: EngineKind) -> &'static str {
    match kind {
        EngineKind::ClaudeCode => "terminal.fill",
        EngineKind::Codex => "chevron.left.forwardslash.chevron.right",
        EngineKind::Agent => "sparkles",
    }
}

/// Semantic tone for an engine state.
pub fn state_tone(state: EngineState) -> &'static str {
    match state {
        EngineState::Idle => "good",
        EngineState::Working => "accent",
        EngineState::Off => "secondary",
    }
}

/// Human label of an engine state.
pub fn state_label(state: EngineState) -> &'static str {
    match state {
        EngineState::Idle => "Idle",
        EngineState::Working => "Working",
        EngineState::Off => "Off",
    }
}

/// Plan family used in a `thinksWith` line.
pub fn plan_family(plan: &str) -> &'static str {
    let lower = plan.to_ascii_lowercase();
    NAMED_PLANS.iter().copied().find(|named| *named == lower).unwrap_or("coding")
}

/// Whether a model is the automatic choice (isAuto).
pub fn is_auto(model: Option<&str>) -> bool {
    model.is_some_and(|value| value.eq_ignore_ascii_case("auto"))
}

/// Display model for a line: none when automatic.
pub fn display_model(model: Option<&str>) -> Option<&str> {
    model.filter(|value| !is_auto(Some(value)) && !value.trim().is_empty())
}

/// "What thinks" line for a plan and model (thinksWith).
pub fn thinks_with(plan: &str, model: Option<&str>, owner: Option<&str>) -> String {
    let family = plan_family(plan);
    let chosen = display_model(model);
    let plan_label = crate::bdots_textfmt::upper_first(plan);
    let line = match chosen {
        Some(model) => format!("{model} on {plan_label}"),
        None => format!("{family} automatic on {plan_label}"),
    };
    owner.map_or(line.clone(), |owner| format!("{line} · {owner}"))
}

/// Brain line for an API model (brainLine, model branch).
pub fn api_brain_line(label: &str, maker: Option<&str>) -> String {
    maker.map_or_else(|| label.to_owned(), |maker| format!("{label} · {maker}"))
}

/// Whether a brain kind is a model (API) brain.
pub fn is_model_brain(kind: Option<&str>) -> bool {
    kind == Some("model") || kind.is_none()
}

/// Whether a computer name should be shown: several agents use different computers.
pub fn show_computer(computer: Option<&str>, computers: &[String]) -> bool {
    computer.is_some_and(|value| computers.iter().any(|other| other != value))
}

/// Distinct computer names in first-seen order.
pub fn distinct_computers(values: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for value in values {
        if !value.is_empty() && !out.contains(value) {
            out.push(value.clone());
        }
    }
    out
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(brain: &str, state: EngineState, computers: usize, auto: bool) -> Vec<(&'static str, String)> {
    vec![
        ("engine", engine_label(engine_of_brain(Some(brain)))),
        ("state", state_label(state).to_owned()),
        ("computers", count_label(computers)),
        ("auto", if auto { "yes" } else { "no" }.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engines() {
        assert_eq!(engine_of_brain(Some("claude_code")), EngineKind::ClaudeCode);
        assert_eq!(engine_of_brain(Some("cursor")), EngineKind::Codex);
        assert_eq!(engine_of_brain(None), EngineKind::Agent);
        assert_eq!(engine_of_executor(Some("claude")), EngineKind::ClaudeCode);
        assert_eq!(engine_of_executor(Some("x")), EngineKind::Agent);
        assert_eq!(engine_label(EngineKind::Codex), "Codex");
        assert_eq!(engine_icon(EngineKind::Agent), "sparkles");
    }

    #[test]
    fn states() {
        assert_eq!(engine_state(Some("model"), false, None), EngineState::Idle);
        assert_eq!(engine_state(Some("codex"), false, None), EngineState::Off);
        assert_eq!(engine_state(Some("codex"), true, Some(false)), EngineState::Off);
        assert_eq!(engine_state(Some("codex"), true, None), EngineState::Idle);
        assert!(is_off(EngineState::Off));
        assert_eq!(state_tone(EngineState::Working), "accent");
        assert_eq!(state_label(EngineState::Idle), "Idle");
    }

    #[test]
    fn lines_and_models() {
        assert_eq!(plan_family("claude"), "claude");
        assert_eq!(plan_family("other"), "coding");
        assert!(is_auto(Some("AUTO")));
        assert_eq!(display_model(Some("auto")), None);
        assert_eq!(display_model(Some("sonnet")), Some("sonnet"));
        assert!(thinks_with("claude", Some("sonnet"), None).contains("sonnet"));
        assert!(thinks_with("claude", None, Some("u")).contains("u"));
        assert_eq!(api_brain_line("GPT", Some("openai")), "GPT · openai");
    }

    #[test]
    fn computers_and_summary() {
        assert!(is_model_brain(None));
        assert!(!is_model_brain(Some("codex")));
        assert!(show_computer(Some("mac"), &["mac".to_owned(), "pc".to_owned()]));
        assert_eq!(distinct_computers(&["a".to_owned(), "a".to_owned(), "b".to_owned()]), vec!["a", "b"]);
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary("codex", EngineState::Idle, 2, true).len(), 4);
    }
}
