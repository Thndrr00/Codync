//! List input rates for estimating what text an agent loads into its context costs.
//!
//! A skill's `SKILL.md` enters a turn as uncached input, so one input rate is enough. These are
//! API-equivalent list prices per million tokens, not subscription charges, and they are only an
//! estimate: an agent on a plan spends plan allowance, not dollars. A model with no entry stays
//! unpriced, never $0.00.
//!
//! Sources, read 2026-10-21 (the same rates OpenBot 0.65 uses in `usage-pricing.ts`):
//! OpenAI <https://developers.openai.com/api/docs/pricing>,
//! Anthropic <https://platform.claude.com/docs/en/about-claude/pricing>,
//! xAI <https://docs.x.ai/developers/pricing>,
//! Google <https://ai.google.dev/gemini-api/docs/pricing>.

/// USD per million input tokens. The key is `(backend, model)`.
const PRICES: &[(&str, &str, f64)] = &[
    // Anthropic. Claude Code names a 1M context model with a `[1m]` suffix; it bills at these rates.
    ("claude", "claude-fable-5-1", 10.0),
    ("claude", "claude-fable-5", 10.0),
    ("claude", "claude-opus-5-5", 4.0),
    ("claude", "claude-opus-5", 5.0),
    ("claude", "claude-opus-4-8", 5.0),
    ("claude", "claude-opus-4-7", 5.0),
    ("claude", "claude-opus-4-6", 5.0),
    ("claude", "claude-sonnet-5-5", 2.0),
    ("claude", "claude-sonnet-5", 2.0),
    ("claude", "claude-sonnet-4-6", 3.0),
    ("claude", "claude-haiku-5-5", 0.1),
    ("claude", "claude-haiku-4-5", 1.0),
    // OpenAI, through Codex.
    ("codex", "gpt-6-astra", 10.0),
    ("codex", "gpt-6.1-sol", 2.0),
    ("codex", "gpt-6.1-terra", 1.0),
    ("codex", "gpt-6-sol", 2.0),
    ("codex", "gpt-6-luna", 0.1),
    ("codex", "gpt-5.6-sol", 4.0),
    ("codex", "gpt-5.6-terra", 2.0),
    ("codex", "gpt-5.6-luna", 0.2),
    ("codex", "gpt-5.5", 5.0),
    // xAI.
    ("grok", "grok-4.6", 2.0),
    ("grok", "grok-build-0.1", 1.0),
    // Google, also through Antigravity (charged at standard Gemini list rates).
    ("gemini", "gemini-3.8-flash", 0.75),
    ("gemini", "gemini-4-argon", 2.0),
    ("antigravity", "gemini-3.8-flash", 0.75),
    ("antigravity", "gemini-4-argon", 2.0),
    ("antigravity", "claude-opus-5-5", 4.0),
    ("antigravity", "claude-sonnet-5-5", 2.0),
];

/// The standard input rate of `model` on `backend`, in USD per million tokens.
///
/// A model id may carry a context suffix (`[1m]`) or an effort suffix (`-high`); neither changes
/// the rate. OpenCode can run any model through any provider, so it has no list price of its own.
pub fn input_price(backend: &str, model: &str) -> Option<f64> {
    // An isolated account is `<agent>:<account>`; the agent's rates still apply.
    let backend = backend.split(':').next().unwrap_or(backend);
    if backend == "opencode" {
        return None;
    }
    let model = model.trim();
    let bare = model.strip_suffix("[1m]").unwrap_or(model);
    let bare = bare
        .strip_suffix("-minimal")
        .or_else(|| bare.strip_suffix("-low"))
        .or_else(|| bare.strip_suffix("-medium"))
        .or_else(|| bare.strip_suffix("-high"))
        .unwrap_or(bare);
    PRICES.iter().find(|(agent, id, _)| *agent == backend && *id == bare).map(|(_, _, rate)| *rate)
}

/// What `tokens` of `model`'s text costs at list input rates, or nothing when the model is unpriced.
#[allow(clippy::cast_precision_loss, reason = "a token count is far below f64's exact integer range")]
pub fn estimate(tokens: u64, backend: &str, model: Option<&str>) -> Option<f64> {
    let model = model?;
    let rate = input_price(backend, model)?;
    Some(tokens as f64 / 1_000_000.0 * rate)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_known_model_is_priced_by_its_input_rate() {
        let cost = estimate(1_000_000, "claude", Some("claude-sonnet-5-5")).expect("priced");
        assert!((cost - 2.0).abs() < 1e-9);
        let quarter = estimate(250_000, "codex", Some("gpt-5.5")).expect("priced");
        assert!((quarter - 1.25).abs() < 1e-9);
    }

    #[test]
    fn a_context_or_effort_suffix_keeps_the_rate() {
        assert_eq!(input_price("claude", "claude-opus-5-5[1m]"), Some(4.0));
        assert_eq!(input_price("antigravity", "gemini-3.8-flash-high"), Some(0.75));
        assert_eq!(input_price("claude:acc_1", "claude-sonnet-5"), Some(2.0));
    }

    #[test]
    fn an_unknown_or_unpriced_model_is_never_free() {
        assert_eq!(estimate(1_000, "claude", Some("a-model-from-the-future")), None);
        assert_eq!(estimate(1_000, "opencode", Some("claude-sonnet-5")), None);
        assert_eq!(estimate(1_000, "claude", None), None);
    }
}
