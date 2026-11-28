#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from OpenBot-CopilotKit v1.1.1 (agent-langgraph-ts): reasoning efforts,
//! model routing and plan threads. Composes validators already ported into this host. Only the
//! `botInsights` endpoint calls this.

/// Server spelling of reasoning efforts (RUN_EFFORTS).
pub fn is_run_effort(effort: &str) -> bool {
    matches!(effort.to_ascii_lowercase().as_str(), "none" | "minimal" | "low" | "medium" | "high" | "xhigh" | "max")
}

/// Effort accepted by Anthropic (ANTHROPIC_EFFORTS).
pub fn is_anthropic_effort(effort: &str) -> bool {
    matches!(effort.to_ascii_lowercase().as_str(), "low" | "medium" | "high" | "xhigh" | "max")
}

/// Effort accepted by Google (GOOGLE_EFFORTS).
pub fn is_google_effort(effort: &str) -> bool {
    matches!(effort.to_ascii_lowercase().as_str(), "minimal" | "low" | "medium" | "high")
}

/// Rank of an effort, for comparing levels (none = 0).
pub fn effort_rank(effort: &str) -> u8 {
    match effort.to_ascii_lowercase().as_str() {
        "none" => 0,
        "minimal" => 1,
        "low" => 2,
        "medium" => 3,
        "high" => 4,
        "xhigh" => 5,
        "max" => 6,
        _ => 0,
    }
}

/// Human label for an effort.
pub fn effort_label(effort: &str) -> &'static str {
    match effort.to_ascii_lowercase().as_str() {
        "none" => "No reasoning",
        "minimal" => "Minimal",
        "low" => "Low",
        "medium" => "Medium",
        "high" => "High",
        "xhigh" => "Extra high",
        "max" => "Maximum",
        _ => "Default",
    }
}

/// SF Symbol for an effort.
pub fn effort_icon(effort: &str) -> &'static str {
    match effort_rank(effort) {
        0 | 1 => "brain",
        2 | 3 => "brain.circle",
        _ => "brain.head.profile",
    }
}

/// Semantic tone for an effort.
pub fn effort_tone(effort: &str) -> &'static str {
    match effort_rank(effort) {
        0 | 1 => "secondary",
        2 | 3 => "accent",
        _ => "good",
    }
}

/// Supported route kinds from the model gateway (openbotModelRoute).
pub fn is_route(route: &str) -> bool {
    matches!(route.to_ascii_lowercase().as_str(), "openai" | "anthropic" | "chatgpt-plan" | "claude-plan")
}

/// Human label for a model route.
pub fn route_label(route: &str) -> &'static str {
    match route.to_ascii_lowercase().as_str() {
        "openai" => "OpenAI gateway",
        "anthropic" => "Anthropic gateway",
        "chatgpt-plan" => "ChatGPT plan",
        "claude-plan" => "Claude plan",
        _ => "Deployment default",
    }
}

/// SF Symbol for a model route.
pub fn route_icon(route: &str) -> &'static str {
    if route.to_ascii_lowercase().contains("plan") { "person.crop.circle.badge.checkmark" } else { "network" }
}

/// Whether a route uses a person's plan rather than a deployment key.
pub fn uses_plan(route: &str) -> bool {
    route.to_ascii_lowercase().contains("plan")
}

/// Plan thread id from a bot id and thread id (claudeThreadId).
pub fn claude_thread_id(bot: &str, thread: &str) -> String {
    format!("{}:{}", crate::bdots_more::shorten(bot, 64), crate::bdots_more::shorten(thread, 64))
}

/// Whether a run forwards an explicit model (forwardedProps.openbotModel).
pub fn has_forwarded_model(model: Option<&str>) -> bool {
    model.is_some_and(|m| !m.trim().is_empty())
}

/// Display label for a run model selection.
pub fn model_choice(model: Option<&str>, effort: Option<&str>) -> String {
    let m = model.filter(|m| !m.trim().is_empty()).map_or_else(|| "Deployment default".to_owned(), str::to_owned);
    match effort.filter(|e| !e.trim().is_empty()) {
        Some(e) => format!("{} · {}", m, effort_label(e)),
        None => m,
    }
}

/// Whether a model needs the OpenAI Responses API (requiresResponsesApi), approximated by family.
pub fn requires_responses_api(model: &str) -> bool {
    let lower = model.to_ascii_lowercase();
    lower.starts_with("gpt-5") || lower.contains("codex")
}

/// Whether a run is send-once (openbotSendOnce): no host-side retry.
pub fn is_send_once(value: bool) -> bool {
    value
}

/// Gate label for send-once runs.
pub fn send_once_label(value: bool) -> &'static str {
    if value { "Ask once" } else { "Host retries" }
}

/// Agent id for this engine (BOT_ID).
pub fn bot_id() -> &'static str {
    "agent-langgraph-ts"
}

/// One plain sentence describing a run's model choice.
pub fn run_headline(model: Option<&str>, effort: Option<&str>, route: &str) -> String {
    format!("{} · {}", model_choice(model, effort), route_label(route))
}

/// Semantic tone for the run card.
pub fn run_tone(route: &str) -> &'static str {
    if uses_plan(route) { "good" } else { "accent" }
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(model: Option<&str>, effort: Option<&str>, route: &str) -> Vec<(&'static str, String)> {
    vec![
        ("engine", bot_id().to_owned()),
        ("model", model_choice(model, effort)),
        ("route", route_label(route).to_owned()),
        ("headline", run_headline(model, effort, route)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn efforts_and_levels() {
        assert!(is_run_effort("high"));
        assert!(!is_run_effort("ultra"));
        assert!(is_anthropic_effort("xhigh"));
        assert!(!is_anthropic_effort("minimal"));
        assert!(is_google_effort("minimal"));
        assert!(!is_google_effort("max"));
        assert_eq!(effort_rank("none"), 0);
        assert_eq!(effort_rank("max"), 6);
        assert_eq!(effort_label("xhigh"), "Extra high");
        assert_eq!(effort_icon("high"), "brain.head.profile");
        assert_eq!(effort_tone("low"), "accent");
    }

    #[test]
    fn routes_and_plans() {
        assert!(is_route("openai"));
        assert!(is_route("chatgpt-plan"));
        assert!(!is_route("local"));
        assert_eq!(route_label("claude-plan"), "Claude plan");
        assert_eq!(route_icon("openai"), "network");
        assert!(uses_plan("chatgpt-plan"));
        assert!(!uses_plan("openai"));
    }

    #[test]
    fn threads_and_models() {
        assert_eq!(claude_thread_id("b", "t"), "b:t");
        assert!(has_forwarded_model(Some("gpt-5")));
        assert!(!has_forwarded_model(None));
        assert_eq!(model_choice(None, Some("high")), "Deployment default · High");
        assert!(requires_responses_api("gpt-5"));
        assert!(requires_responses_api("codex-mini"));
        assert!(!requires_responses_api("claude-sonnet"));
    }

    #[test]
    fn send_once_and_summary() {
        assert!(is_send_once(true));
        assert_eq!(send_once_label(true), "Ask once");
        assert_eq!(send_once_label(false), "Host retries");
        assert_eq!(bot_id(), "agent-langgraph-ts");
        assert!(run_headline(None, None, "openai").contains("OpenAI"));
        assert_eq!(run_tone("chatgpt-plan"), "good");
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(None, None, "openai").len(), 4);
    }
}
