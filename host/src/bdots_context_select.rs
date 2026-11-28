#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from rakazo's `context-selection.ts`: provider-neutral token estimation,
//! framing overhead and budget arithmetic. Composes helpers already ported into this host. Only
//! `botInsights` calls this.

use serde_json::Value;

/// Framing overhead added to every estimated context value.
pub const FRAMING_TOKENS: u32 = 16;

/// Cache warmth states for a context selection.
pub const CACHE_STATES: [&str; 3] = ["likely-warm", "likely-cold", "unknown"];

/// Deliberately low count, about five characters a token (lowTextTokens).
pub fn low_text_tokens(text: &str) -> u32 {
    (text.chars().count() / 5) as u32
}

/// Character kind used by the estimator: lowercase, uppercase, digit, other.
fn char_kind(c: char) -> u8 {
    if c.is_ascii_lowercase() {
        1
    } else if c.is_ascii_uppercase() {
        2
    } else if c.is_ascii_digit() {
        3
    } else {
        0
    }
}

/// Provider-neutral token estimate for text, calibrated above five tokenizers (estimateTextTokens).
pub fn estimate_text_tokens(text: &str) -> u32 {
    let mut total = 0.0_f64;
    let mut previous = 0_u8;
    for c in text.chars() {
        let kind = char_kind(c);
        let code = c as u32;
        if code >= 128 {
            total += if code < 0x0590 { 0.6 } else { 1.25 };
        } else if kind == 3 {
            total += 1.0;
        } else if kind == 0 && code > 32 && code < 127 {
            total += 0.4;
        } else {
            total += 0.3;
        }
        if (previous == 1 && kind == 2)
            || (previous == 3 && (kind == 1 || kind == 2))
            || ((previous == 1 || previous == 2) && kind == 3)
        {
            total += 1.0;
        }
        previous = kind;
    }
    let ceiling = total.ceil() as u32;
    ceiling.min(text.len() as u32)
}

/// Estimate for a value with framing overhead (estimateContextTokens).
pub fn estimate_context_tokens(value: &Value, image_tokens: u32) -> u32 {
    let serialized = serde_json::to_string(value).unwrap_or_default();
    estimate_text_tokens(&serialized).saturating_add(image_tokens).saturating_add(FRAMING_TOKENS)
}

/// Fixed cost of system prompt, tools and extra context (fixedTokens).
pub fn fixed_tokens(system_prompt: &str, tools: &Value, extra: Option<&str>) -> u32 {
    let extra_tokens = extra.map_or(0, estimate_text_tokens);
    estimate_text_tokens(system_prompt).saturating_add(estimate_context_tokens(tools, 0)).saturating_add(extra_tokens)
}

/// Available tokens for the conversation after fixed costs.
pub fn available_tokens(window: u32, output_reserve: u32, fixed: u32) -> Option<u32> {
    window.checked_sub(output_reserve)?.checked_sub(fixed)
}

/// Whether a context fits an available budget.
pub fn fits_budget(estimate: u32, available: u32) -> bool {
    estimate <= available
}

/// Whether the fixed cost alone fills the window (model too small).
pub fn model_too_small(window: u32, output_reserve: u32, fixed: u32) -> bool {
    available_tokens(window, output_reserve, fixed).is_none_or(|available| available == 0)
}

/// Number of recent exchanges to keep for a strategy.
pub fn recent_exchanges(strategy: &str) -> usize {
    match strategy.to_ascii_lowercase().as_str() {
        "aggressive" => 2,
        "balanced" => 4,
        "conservative" => 8,
        _ => 4,
    }
}

/// Whether a message index is protected from pruning.
pub fn is_protected(index: usize, protected: &[usize]) -> bool {
    protected.contains(&index)
}

/// Slice of history allowed by an earlier budget, keeping the most recent that fit.
pub fn fit_history(scores: &[u32], tokens: u32) -> usize {
    let mut used = 0;
    let mut kept = 0;
    for score in scores.iter().rev() {
        let Some(next) = used.checked_add(*score) else {
            break;
        };
        if next > tokens {
            break;
        }
        used = next;
        kept += 1;
    }
    kept
}

/// Cache label for a warmth state.
pub fn cache_label(state: &str) -> &'static str {
    match state {
        "likely-warm" => "Warm",
        "likely-cold" => "Cold",
        _ => "Unknown",
    }
}

/// SF Symbol for a cache state.
pub fn cache_icon(state: &str) -> &'static str {
    if state == "likely-warm" {
        "flame.fill"
    } else if state == "likely-cold" {
        "snowflake"
    } else {
        "questionmark.circle"
    }
}

/// Semantic tone for a cache state.
pub fn cache_tone(state: &str) -> &'static str {
    match state {
        "likely-warm" => "good",
        "likely-cold" => "attention",
        _ => "secondary",
    }
}

/// Human label of a strategy.
pub fn strategy_label(strategy: &str) -> &'static str {
    match strategy.to_ascii_lowercase().as_str() {
        "aggressive" => "Aggressive",
        "conservative" => "Conservative",
        _ => "Balanced",
    }
}

/// Marker written when history messages were omitted (OMITTED_BEFORE_USE).
pub fn omitted_marker(dropped: usize) -> String {
    if dropped == 0 { String::new() } else { format!("[{} earlier messages omitted]", dropped) }
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(estimate: u32, available: u32, fixed: u32, kept: usize) -> Vec<(&'static str, String)> {
    vec![
        ("estimate", count_label(estimate as usize)),
        ("available", count_label(available as usize)),
        ("fixed", count_label(fixed as usize)),
        ("kept", count_label(kept)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn estimates() {
        assert_eq!(low_text_tokens("abcdefghij"), 2);
        assert!(estimate_text_tokens("hello world") > 0);
        assert!(estimate_text_tokens("camelCase123") > estimate_text_tokens("aaaaaaaaaaaa"));
        assert_eq!(estimate_text_tokens(""), 0);
        let value = json!({"a":1});
        assert!(estimate_context_tokens(&value, 50) > estimate_context_tokens(&value, 0));
    }

    #[test]
    fn budgets() {
        assert_eq!(
            fixed_tokens("system", &json!({}), Some("extra")),
            estimate_text_tokens("system") + estimate_context_tokens(&json!({}), 0) + estimate_text_tokens("extra")
        );
        assert_eq!(available_tokens(200_000, 4_000, 6_000), Some(190_000));
        assert_eq!(available_tokens(5_000, 4_000, 2_000), None);
        assert!(fits_budget(100, 100));
        assert!(!fits_budget(101, 100));
        assert!(model_too_small(5_000, 4_000, 2_000));
    }

    #[test]
    fn strategies_and_history() {
        assert_eq!(recent_exchanges("aggressive"), 2);
        assert_eq!(recent_exchanges("balanced"), 4);
        assert_eq!(recent_exchanges("other"), 4);
        assert!(is_protected(2, &[1, 2]));
        assert!(!is_protected(3, &[1, 2]));
        assert_eq!(fit_history(&[10, 20, 30], 50), 2);
        assert_eq!(fit_history(&[10, 20, 30], 5), 0);
    }

    #[test]
    fn presentation() {
        assert_eq!(cache_label("likely-warm"), "Warm");
        assert_eq!(cache_icon("likely-cold"), "snowflake");
        assert_eq!(cache_tone("unknown"), "secondary");
        assert_eq!(strategy_label("conservative"), "Conservative");
        assert_eq!(strategy_label("x"), "Balanced");
        assert_eq!(omitted_marker(0), "");
        assert!(omitted_marker(3).contains("omitted"));
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(100, 200, 50, 3).len(), 4);
    }
}
