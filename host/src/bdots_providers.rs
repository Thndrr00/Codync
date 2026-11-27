#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from agentes-chat's `providers.ts`: provider catalog, key shapes, guesses
//! and model choices. Composes helpers already ported into this host. Only `botInsights` calls this.

use serde_json::Value;

/// Automatic model value (AUTO).
pub const AUTO: &str = "auto";

/// Provider ids in catalog order.
pub const PROVIDER_IDS: [&str; 8] =
    ["anthropic", "openai", "gemini", "openrouter", "deepseek", "moonshot", "zai", "minimax"];

/// Provider labels in catalog order.
pub const PROVIDER_LABELS: [&str; 8] =
    ["Claude", "OpenAI", "Gemini", "OpenRouter", "DeepSeek", "Kimi", "Z.ai", "MiniMax"];

/// Whether an id is a known provider (isProviderId).
pub fn is_provider_id(id: &str) -> bool {
    PROVIDER_IDS.contains(&id)
}

/// Label of a provider, falling back to the given id (providerLabel).
pub fn provider_label(id: &str) -> String {
    PROVIDER_IDS
        .iter()
        .position(|value| *value == id)
        .map_or_else(|| id.to_owned(), |index| PROVIDER_LABELS[index].to_owned())
}

/// Whether a provider id is recognized by label (run errors name providers by label).
pub fn provider_by_label(label: &str) -> Option<&'static str> {
    PROVIDER_LABELS.iter().position(|value| *value == label).map(|index| PROVIDER_IDS[index])
}

/// Whether a key is the automatic choice (isAuto).
pub fn is_auto(value: Option<&str>) -> bool {
    value == Some(AUTO)
}

/// Prefix of an Anthropic key.
pub fn is_anthropic_key(key: &str) -> bool {
    key.starts_with("sk-ant-")
        && key[7..].chars().count() >= 16
        && key[7..].chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
}

/// Prefix of an OpenAI key (excludes Anthropic and OpenRouter).
pub fn is_openai_key(key: &str) -> bool {
    key.starts_with("sk-")
        && !key.starts_with("sk-ant-")
        && !key.starts_with("sk-or-")
        && key[3..].chars().count() >= 16
        && key[3..].chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
}

/// Whether a key looks like a Gemini key (AQ. since 2026, AIza before).
pub fn is_gemini_key(key: &str) -> bool {
    (key.starts_with("AQ.") && key[3..].chars().count() >= 20)
        || (key.starts_with("AIza") && key[4..].chars().count() >= 30)
}

/// Whether a key looks like an OpenRouter key.
pub fn is_openrouter_key(key: &str) -> bool {
    let Some(rest) = key.strip_prefix("sk-or-v") else {
        return false;
    };
    let Some((version, digits)) = rest.split_once('-') else {
        return false;
    };
    version.chars().all(|c| c.is_ascii_digit())
        && digits.chars().count() >= 32
        && digits.chars().all(|c| c.is_ascii_alphanumeric())
}

/// Whether a key is valid for a provider (keyLooksValid).
pub fn key_looks_valid(provider: &str, key: &str) -> bool {
    match provider {
        "anthropic" => is_anthropic_key(key),
        "openai" => is_openai_key(key),
        "gemini" => is_gemini_key(key),
        "openrouter" => is_openrouter_key(key),
        "deepseek" | "moonshot" => key.starts_with("sk-") && key[3..].chars().count() >= 16,
        "zai" | "minimax" => {
            !key.starts_with("sk-") && !key.starts_with("AQ.") && !key.starts_with("AIza") && key.chars().count() >= 16
        }
        _ => false,
    }
}

/// Providers whose key pattern matches (providersForKey).
pub fn providers_for_key(key: &str) -> Vec<&'static str> {
    PROVIDER_IDS.iter().copied().filter(|id| key_looks_valid(id, key)).collect()
}

/// Guess a provider from a key and an optional hint (providerGuess).
pub fn provider_guess(key: Option<&str>, asked: Option<&str>) -> &'static str {
    if let Some(hint) = asked {
        if is_provider_id(hint) {
            return PROVIDER_IDS[PROVIDER_IDS.iter().position(|id| *id == hint).unwrap()];
        }
    }
    key.and_then(|value| providers_for_key(value).first().copied()).unwrap_or("openai")
}

/// Placeholder shown in a key field, reusing the shared formatter.
pub fn key_placeholder(id: &str) -> String {
    match id {
        "anthropic" => "sk-ant-…".to_owned(),
        "openai" | "deepseek" | "moonshot" => "sk-…".to_owned(),
        "gemini" => "AQ.…".to_owned(),
        "openrouter" => "sk-or-v1-…".to_owned(),
        _ => crate::bdots_textfmt::upper_first("key"),
    }
}

/// Site where a provider hands out keys.
pub fn provider_site(id: &str) -> &'static str {
    match id {
        "anthropic" => "platform.claude.com/settings/keys",
        "openai" => "platform.openai.com/api-keys",
        "gemini" => "aistudio.google.com/api-keys",
        "openrouter" => "openrouter.ai/keys",
        "deepseek" => "platform.deepseek.com/api_keys",
        "moonshot" => "platform.kimi.ai/console/api-keys",
        "zai" => "z.ai/manage-apikey/apikey-list",
        "minimax" => "platform.minimax.io/user-center/basic-information/interface-key",
        _ => "",
    }
}

/// Whether a provider also sells coding plans.
pub fn has_plans(id: &str) -> bool {
    matches!(id, "moonshot" | "zai" | "minimax")
}

/// Whether a provider's free tier requires billing.
pub fn needs_billing(id: &str) -> bool {
    id == "gemini"
}

/// Whether a provider passes data to other providers.
pub fn has_privacy_notice(id: &str) -> bool {
    id == "openrouter"
}

/// Model choices available without keys (modelChoices).
pub fn model_choices(models: &[Value], credentials: &[Value]) -> Vec<Value> {
    models
        .iter()
        .map(|model| {
            let provider = model["provider"].as_str().unwrap_or("");
            let available = credentials.iter().any(|key| key["provider"].as_str() == Some(provider));
            let mut row = model.clone();
            row["available"] = Value::Bool(available);
            row
        })
        .collect()
}

/// Usable choices, reusing the shared count formatter.
pub fn usable_model_count(choices: &[Value]) -> usize {
    choices.iter().filter(|row| row["available"] == true).count()
}

/// Provider of a model id, when it appears in the catalog (apiProviderOf).
pub fn model_provider(model: Option<&str>, models: &[Value]) -> Option<String> {
    let model = model?;
    models
        .iter()
        .find(|row| row["id"].as_str() == Some(model))
        .and_then(|row| row["provider"].as_str())
        .map(str::to_owned)
}

/// SF Symbol for a provider key state.
pub fn key_icon(has_key: bool) -> &'static str {
    if has_key { "key.fill" } else { "key" }
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(providers: usize, keys: usize, models: usize, auto: bool) -> Vec<(&'static str, String)> {
    vec![
        ("providers", count_label(providers)),
        ("keys", count_label(keys)),
        ("models", count_label(models)),
        ("auto", if auto { "yes" } else { "no" }.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn catalog() {
        assert!(is_provider_id("anthropic"));
        assert!(!is_provider_id("x"));
        assert_eq!(provider_label("openai"), "OpenAI");
        assert_eq!(provider_label("x"), "x");
        assert_eq!(provider_by_label("Kimi"), Some("moonshot"));
        assert!(is_auto(Some("auto")));
        assert_eq!(key_placeholder("gemini"), "AQ.…");
    }

    #[test]
    fn keys() {
        assert!(is_anthropic_key("sk-ant-1234567890123456"));
        assert!(is_openai_key("sk-1234567890123456"));
        assert!(!is_openai_key("sk-ant-1234567890123456"));
        assert!(is_gemini_key("AIza123456789012345678901234567890"));
        assert!(key_looks_valid("openrouter", "sk-or-v1-12345678901234567890123456789012"));
        assert!(providers_for_key("sk-1234567890123456").contains(&"openai"));
        assert_eq!(provider_guess(Some("sk-ant-1234567890123456"), None), "anthropic");
        assert_eq!(provider_guess(None, Some("gemini")), "gemini");
    }

    #[test]
    fn provider_flags() {
        assert_eq!(provider_site("gemini"), "aistudio.google.com/api-keys");
        assert!(has_plans("zai"));
        assert!(needs_billing("gemini"));
        assert!(has_privacy_notice("openrouter"));
        assert_eq!(key_icon(true), "key.fill");
    }

    #[test]
    fn models_and_summary() {
        let models = vec![json!({"id":"m1","provider":"openai"}), json!({"id":"m2","provider":"gemini"})];
        let credentials = vec![json!({"provider":"openai"})];
        let choices = model_choices(&models, &credentials);
        assert_eq!(usable_model_count(&choices), 1);
        assert_eq!(model_provider(Some("m2"), &models).as_deref(), Some("gemini"));
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(8, 1, 2, true).len(), 4);
    }
}
