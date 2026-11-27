#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from agentes-chat's `plans.ts`: coding plans, model choices, key patterns
//! and plan aliases. Composes helpers already ported into this host. Only `botInsights` calls this.

use serde_json::Value;

/// Name of the OpenCode Go plan.
pub const OPENCODE_GO: &str = "opencode";

/// Prefix of an OpenCode Go model id (opencodeGoModel).
pub fn opencode_go_model(id: &str) -> String {
    format!("opencode-go/{id}")
}

/// Plan name from a provider id, or None for the CLI's own sign-in (planOf).
pub fn plan_of(provider: Option<&str>) -> Option<&'static str> {
    match provider.unwrap_or("").to_ascii_lowercase().as_str() {
        "claude_code" | "anthropic" => Some("claude"),
        "codex" | "openai" => Some("chatgpt"),
        "cursor" => Some("cursor"),
        "zen" => Some("opencode"),
        "grok" | "xai" => Some("grok"),
        "gemini" | "google" => Some("gemini"),
        _ => None,
    }
}

/// Plan of an account row (accountPlan).
pub fn account_plan(account: Option<&Value>) -> Option<&'static str> {
    plan_of(account.and_then(|row| row["provider"].as_str()))
}

/// Plan of the account an agent thinks with (brainPlan).
pub fn brain_plan(view: &Value) -> Option<&'static str> {
    let account_id = view["account"].as_str()?;
    let accounts = view["accounts"].as_array()?;
    accounts.iter().find(|row| row["id"].as_str() == Some(account_id)).and_then(|row| account_plan(Some(row)))
}

/// Capitalized plan name (planName).
pub fn plan_name(plan: &str) -> String {
    crate::bdots_textfmt::upper_first(plan)
}

/// Whether a plan has its own free tier hint (planFreeHint).
pub fn has_free_hint(plan: Option<&str>) -> bool {
    plan == Some(OPENCODE_GO)
}

/// Whether a plan is a key-based account rather than a sign-in.
pub fn is_key_plan(plan: &str) -> bool {
    matches!(plan.to_ascii_lowercase().as_str(), "cursor" | "opencode")
}

/// Key pattern used by Cursor and OpenCode key accounts.
pub const KEY_PATTERN: &str = "^[\\x21-\\x7e]{16,512}$";

/// Whether a key matches the printable-ASCII 16..512 pattern.
pub fn valid_key(key: &str) -> bool {
    (16..=512).contains(&key.len()) && key.bytes().all(|b| (0x21..=0x7e).contains(&b))
}

/// Placeholder shown in a key input.
pub fn key_placeholder(plan: &str) -> &'static str {
    if plan.eq_ignore_ascii_case("cursor") { "key_" } else { "sk-" }
}

/// Model choice for a stored brain model (brainModelChoice).
pub fn brain_model_choice(kind: &str, plan: Option<&str>, value: &str) -> String {
    if kind != "claude_code" || value.eq_ignore_ascii_case("auto") {
        return value.to_owned();
    }
    match plan {
        Some(plan) => format!("{plan}:{value}"),
        None => value.to_owned(),
    }
}

/// Model choices for a computer brain (brainModelChoices).
pub fn brain_model_choices(plan: Option<&str>, models: &[Value], provider: Option<&str>) -> Vec<Value> {
    if plan == Some("claude") {
        return vec![
            serde_json::json!({"value":"sonnet","label":"Sonnet"}),
            serde_json::json!({"value":"opus","label":"Opus"}),
            serde_json::json!({"value":"haiku","label":"Haiku"}),
        ];
    }
    let mut out: Vec<Value> = models.to_vec();
    if provider == Some("zen") {
        let go = serde_json::json!({"value": opencode_go_model("glm"), "label": "GLM (Go)"});
        if !out.iter().any(|row| row["value"] == go["value"]) {
            out.push(go);
        }
    }
    out
}

/// Sign-in key for a provider and kind (signInKey).
pub fn sign_in_key(provider: Option<&str>, kind: &str, cursor_key: bool) -> String {
    let provider = provider.unwrap_or("");
    if cursor_key { "cursor:key".to_owned() } else { format!("{provider}:{kind}") }
}

/// Whether an account is a personal Gemini sign-in (geminiPersonal).
pub fn gemini_personal(account: Option<&Value>) -> bool {
    account
        .and_then(|row| row["provider"].as_str())
        .is_some_and(|provider| provider.eq_ignore_ascii_case("gemini") || provider.eq_ignore_ascii_case("google"))
}

/// Whether a brain uses a named coding plan.
pub fn uses_named_plan(plan: Option<&str>) -> bool {
    plan.is_some_and(|value| {
        plan_of(Some(value)).is_some()
            || matches!(value, "claude" | "chatgpt" | "cursor" | "opencode" | "grok" | "gemini")
    })
}

/// Human label for a coding plan, reusing the text formatter.
pub fn plan_label(plan: &str) -> String {
    crate::bdots_textfmt::upper_first(plan)
}

/// SF Symbol for a plan.
pub fn plan_icon(plan: &str) -> &'static str {
    match plan.to_ascii_lowercase().as_str() {
        "claude" => "sparkles",
        "chatgpt" => "circle.hexagongrid.fill",
        "cursor" => "cursorarrow",
        "opencode" => "terminal.fill",
        "gemini" => "star.fill",
        _ => "key.fill",
    }
}

/// Semantic tone for a plan.
pub fn plan_tone(plan: &str) -> &'static str {
    if is_key_plan(plan) { "accent" } else { "good" }
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(plan: &str, models: usize, key: bool, free_hint: bool) -> Vec<(&'static str, String)> {
    vec![
        ("plan", plan_label(plan)),
        ("models", count_label(models)),
        ("key", if key { "yes" } else { "no" }.to_owned()),
        ("free", if free_hint { "yes" } else { "no" }.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn plans_and_names() {
        assert_eq!(opencode_go_model("m"), "opencode-go/m");
        assert_eq!(plan_of(Some("claude_code")), Some("claude"));
        assert_eq!(plan_of(Some("zen")), Some("opencode"));
        assert_eq!(plan_of(None), None);
        assert_eq!(plan_name("pro"), "Pro");
        assert!(has_free_hint(Some("opencode")));
        assert!(is_key_plan("cursor"));
        assert!(!is_key_plan("claude"));
    }

    #[test]
    fn keys_and_sign_in() {
        let key = "a".repeat(20);
        assert!(valid_key(&key));
        assert!(!valid_key("short"));
        assert_eq!(key_placeholder("cursor"), "key_");
        assert_eq!(key_placeholder("opencode"), "sk-");
        assert_eq!(sign_in_key(Some("openai"), "codex", false), "openai:codex");
        assert_eq!(sign_in_key(None, "x", true), "cursor:key");
        assert!(gemini_personal(Some(&json!({"provider":"gemini"}))));
    }

    #[test]
    fn brains_and_accounts() {
        let account = json!({"id":"a1","provider":"claude_code"});
        assert_eq!(account_plan(Some(&account)), Some("claude"));
        let view = json!({"account":"a1","accounts":[account.clone()]});
        assert_eq!(brain_plan(&view), Some("claude"));
        assert_eq!(brain_model_choice("claude_code", Some("claude"), "sonnet"), "claude:sonnet");
        assert_eq!(brain_model_choice("claude_code", None, "auto"), "auto");
        let own = vec![json!({"value":"a","label":"A"})];
        assert_eq!(brain_model_choices(Some("claude"), &own, None).len(), 3);
        assert_eq!(brain_model_choices(None, &own, Some("zen")).len(), 2);
    }

    #[test]
    fn presentation() {
        assert!(uses_named_plan(Some("claude")));
        assert!(!uses_named_plan(Some("unknown")));
        assert_eq!(plan_label("chatgpt"), "Chatgpt");
        assert_eq!(plan_icon("claude"), "sparkles");
        assert_eq!(plan_tone("cursor"), "accent");
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary("claude", 3, false, false).len(), 4);
    }
}
