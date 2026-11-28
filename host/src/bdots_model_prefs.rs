#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from mercury-agent's `model-prefs.ts`: model refs, reasoning levels, effort
//! resolution and display names. Composes helpers already ported into this host. Only `botInsights`
//! calls this.

use serde_json::{Map, Value, json};

/// Reasoning levels a model may accept.
pub const REASONING_LEVELS: [&str; 6] = ["off", "low", "medium", "high", "xhigh", "max"];

/// Longest user-chosen display label.
pub const MAX_LABEL_LENGTH: usize = 80;

/// Longest provider or model ref used as a preference key.
pub const MAX_REF_LENGTH: usize = 256;

/// Most per-chat effort overrides kept.
pub const MAX_CHAT_OVERRIDES: usize = 1_000;

/// Providers with their own wire path (no OpenAI-style effort field).
pub const OWN_WIRE: [&str; 6] = ["anthropic", "deepseek", "mercuryCloud", "mimo", "mimoTokenPlan", "githubCopilot"];

/// Headings of provider groups in pickers.
pub const PROVIDER_LABELS: [(&str, &str); 8] = [
    ("openai", "OpenAI"),
    ("anthropic", "Anthropic"),
    ("ollamaLocal", "Ollama"),
    ("ollamaCloud", "Ollama Cloud"),
    ("openrouter", "OpenRouter"),
    ("gemini", "Gemini"),
    ("chatgptWeb", "ChatGPT"),
    ("mercuryCloud", "Mercury"),
];

/// Words whose casing is fixed in a model name.
const WORD_CASE: [(&str, &str); 18] = [
    ("gpt", "GPT"),
    ("oss", "OSS"),
    ("glm", "GLM"),
    ("qwen", "Qwen"),
    ("gemma", "Gemma"),
    ("kimi", "Kimi"),
    ("deepseek", "DeepSeek"),
    ("mistral", "Mistral"),
    ("minimax", "MiniMax"),
    ("nemotron", "Nemotron"),
    ("llama", "Llama"),
    ("phi", "Phi"),
    ("nsfw", "NSFW"),
    ("ai", "AI"),
    ("vl", "VL"),
    ("moe", "MoE"),
    ("it", "IT"),
    ("abliterated", "Abliterated"),
];

/// Whether a provider or model id is fit to key preferences (isValidModelRef).
pub fn is_valid_model_ref(value: Option<&str>) -> bool {
    value.is_some_and(|text| {
        !text.is_empty()
            && text.chars().count() <= MAX_REF_LENGTH
            && !text.chars().any(|c| (c as u32) < 32 || c as u32 == 127)
    })
}

/// Whether a string is a reasoning level (isReasoningLevel).
pub fn is_reasoning_level(value: Option<&str>) -> bool {
    value.is_some_and(|level| REASONING_LEVELS.contains(&level))
}

/// Preference key of a provider and model.
pub fn prefs_key(provider: &str, model: &str) -> String {
    format!("{provider}::{model}")
}

/// Keep only well-formed entries and fields from parsed preferences (sanitize).
pub fn sanitize_prefs(raw: &Value) -> Value {
    let mut clean = Map::new();
    let Some(entries) = raw.as_object() else {
        return Value::Object(clean);
    };
    for (key, value) in entries {
        let Some((provider, model)) = key.split_once("::") else {
            continue;
        };
        if !is_valid_model_ref(Some(provider)) || !is_valid_model_ref(Some(model)) || !value.is_object() {
            continue;
        }
        let mut pref = Map::new();
        if let Some(label) = value["label"].as_str() {
            let trimmed = label.trim();
            if !trimmed.is_empty() {
                pref.insert("label".to_owned(), Value::String(trimmed.chars().take(MAX_LABEL_LENGTH).collect()));
            }
        }
        if is_reasoning_level(value["reasoning"].as_str()) {
            pref.insert("reasoning".to_owned(), Value::String(value["reasoning"].as_str().unwrap_or("").to_owned()));
        }
        if !pref.is_empty() {
            clean.insert(key.clone(), Value::Object(pref));
        }
    }
    Value::Object(clean)
}

/// Preference of one model, or an empty object (getModelPref).
pub fn model_pref_of(all: &Value, provider: &str, model: &str) -> Value {
    all.get(prefs_key(provider, model)).filter(|value| value.is_object()).cloned().unwrap_or_else(|| json!({}))
}

/// Update a model's preference; empty label or reasoning clears that field.
pub fn apply_model_pref(
    all: &Value,
    provider: &str,
    model: &str,
    label: Option<&str>,
    reasoning: Option<&str>,
) -> Value {
    let mut next = all.clone();
    let key = prefs_key(provider, model);
    let mut pref = model_pref_of(&next, provider, model).as_object().cloned().unwrap_or_default();
    if let Some(label) = label {
        let trimmed = label.trim();
        if trimmed.is_empty() {
            pref.remove("label");
        } else {
            pref.insert("label".to_owned(), Value::String(trimmed.chars().take(MAX_LABEL_LENGTH).collect()));
        }
    }
    if let Some(level) = reasoning {
        if is_reasoning_level(Some(level)) {
            pref.insert("reasoning".to_owned(), Value::String(level.to_owned()));
        } else {
            pref.remove("reasoning");
        }
    }
    if pref.is_empty() {
        next.as_object_mut().map(|map| map.remove(&key));
    } else {
        next.as_object_mut().map(|map| map.insert(key, Value::Object(pref)));
    }
    next
}

/// Providers with their own request path (providerHasReasoningControl).
pub fn provider_has_reasoning_control(provider: &str) -> bool {
    !OWN_WIRE.contains(&provider)
}

/// OpenAI models that accept a reasoning effort (o-series and GPT-5).
pub fn openai_reasoning_model(model: &str) -> bool {
    let lower = model.to_ascii_lowercase();
    let rest = lower.strip_prefix("openai/").unwrap_or(&lower);
    (rest.starts_with("o") && rest.as_bytes().get(1).is_some_and(u8::is_ascii_digit)) || rest.starts_with("gpt-5")
}

/// Version of a Claude model id, as a decimal (major + minor/10).
fn claude_version(model: &str, family: &str) -> Option<f64> {
    let lower = model.to_ascii_lowercase();
    let needle = format!("claude-{family}-");
    let index = lower.find(&needle)?;
    let rest = &lower[index + needle.len()..];
    let major_text: String = rest.chars().take_while(char::is_ascii_digit).collect();
    if major_text.is_empty() {
        return None;
    }
    let major: f64 = major_text.parse().ok()?;
    let tail = &rest[major_text.len()..];
    let minor = tail
        .strip_prefix(['-', '.'])
        .and_then(|value| value.chars().next())
        .filter(|c| c.is_ascii_digit())
        .filter(|c| !tail.chars().nth(1).is_some_and(|next| next.is_ascii_digit()))
        .and_then(|c| c.to_digit(10))
        .unwrap_or(0);
    Some(major + f64::from(minor) / 10.0)
}

/// What a Claude model does with thinking, from its id (claudeThinking).
pub fn claude_thinking(model: &str) -> Value {
    let lower = model.to_ascii_lowercase();
    if lower.contains("claude-fable") || lower.contains("claude-mythos") {
        return json!({"thinksByDefault": true, "adaptive": true, "levels": ["low", "medium", "high", "xhigh", "max"]});
    }
    for family in ["opus", "sonnet", "haiku"] {
        let Some(version) = claude_version(model, family) else {
            continue;
        };
        if version >= 5.0 {
            return json!({"thinksByDefault": true, "adaptive": true, "levels": ["low", "medium", "high", "xhigh", "max"]});
        }
        if version >= 4.7 && family == "opus" {
            return json!({"thinksByDefault": false, "adaptive": true, "levels": ["low", "medium", "high", "xhigh", "max"]});
        }
        if version >= 4.6 && family != "haiku" {
            return json!({"thinksByDefault": false, "adaptive": true, "levels": ["low", "medium", "high", "max"]});
        }
        break;
    }
    json!({"thinksByDefault": false, "adaptive": false, "levels": []})
}

/// Levels a model can honour (reasoningLevelsFor).
pub fn reasoning_levels_for(provider: &str, model: Option<&str>) -> Vec<&'static str> {
    if provider == "chatgptWeb" {
        return vec!["low", "medium", "high"];
    }
    if provider == "anthropic" {
        return model.map_or(Vec::new(), |value| {
            claude_thinking(value)["levels"]
                .as_array()
                .map(|levels| levels.iter().filter_map(Value::as_str).collect())
                .unwrap_or_default()
        });
    }
    if OWN_WIRE.contains(&provider) {
        return Vec::new();
    }
    let ollama = provider == "ollamaLocal" || provider == "ollamaCloud";
    let Some(model) = model else {
        return if ollama { vec!["off", "low", "medium", "high"] } else { Vec::new() };
    };
    if provider == "openai" {
        return if openai_reasoning_model(model) { vec!["low", "medium", "high"] } else { Vec::new() };
    }
    if ollama {
        return if model.to_ascii_lowercase().contains("gpt-oss") {
            vec!["low", "medium", "high"]
        } else {
            vec!["off", "low", "medium", "high"]
        };
    }
    Vec::new()
}

/// How a level reads on screen (effortLabel).
pub fn reasoning_label(level: &str) -> &'static str {
    match level {
        "auto" => "Auto",
        "off" => "Off",
        "low" => "Low",
        "medium" => "Medium",
        "high" => "High",
        "xhigh" => "Extra high",
        "max" => "Max",
        _ => "Auto",
    }
}

/// The effort a request carries in this chat, and why (effortFor).
pub fn effort_for(provider: &str, model: &str, prefs: &Value, chat_level: Option<&str>) -> Value {
    let levels = reasoning_levels_for(provider, Some(model));
    let saved = model_pref_of(prefs, provider, model)["reasoning"].as_str().unwrap_or("");
    let model_default = if is_reasoning_level(Some(saved)) && levels.contains(&saved) { saved } else { "auto" };
    let mut out = json!({"modelDefault": model_default, "levels": levels, "effective": Value::Null, "source": "auto"});
    if let Some(chat) = chat_level {
        out["chat"] = Value::String(chat.to_owned());
        if chat == "auto" && !levels.is_empty() {
            out["source"] = Value::String("chat".to_owned());
            return out;
        }
        if chat != "auto" {
            if levels.contains(&chat) {
                out["effective"] = Value::String(chat.to_owned());
                out["source"] = Value::String("chat".to_owned());
                return out;
            }
            if !levels.is_empty() {
                out["effective"] =
                    if model_default == "auto" { Value::Null } else { Value::String(model_default.to_owned()) };
                out["source"] = if model_default == "auto" {
                    Value::String("auto".to_owned())
                } else {
                    Value::String("model".to_owned())
                };
                out["chatUnsupported"] = Value::Bool(true);
                return out;
            }
        }
    }
    out["effective"] = if model_default == "auto" { Value::Null } else { Value::String(model_default.to_owned()) };
    out["source"] =
        if model_default == "auto" { Value::String("auto".to_owned()) } else { Value::String("model".to_owned()) };
    out
}

/// Level a request carries (effectiveReasoning).
pub fn effective_reasoning(provider: &str, model: &str, prefs: &Value, chat_level: Option<&str>) -> Option<String> {
    effort_for(provider, model, prefs, chat_level)["effective"].as_str().map(str::to_owned)
}

/// Whether a base URL is OpenRouter (isOpenRouterUrl).
pub fn is_openrouter_url(base_url: Option<&str>) -> bool {
    base_url.is_some_and(|url| {
        let lower = url.to_ascii_lowercase();
        lower.contains("openrouter.ai/") || lower.contains("openrouter.ai:") || lower.contains("//openrouter.ai")
    })
}

/// Anthropic provider options for a thinking model (anthropicThinkingOptions).
pub fn anthropic_thinking_options(model: &str, level: Option<&str>) -> Option<Value> {
    let caps = claude_thinking(model);
    if caps["adaptive"] != true {
        return None;
    }
    let supported = level.filter(|value| {
        caps["levels"].as_array().is_some_and(|levels| levels.iter().any(|entry| entry.as_str() == Some(value)))
    });
    if caps["thinksByDefault"] != true && supported.is_none() {
        return None;
    }
    let mut out = json!({"thinking": {"type": "adaptive", "display": "summarized"}});
    if let Some(level) = supported {
        out["effort"] = Value::String(level.to_owned());
    }
    Some(out)
}

/// Apply a reasoning level to a request body (applyReasoningToBody).
pub fn apply_reasoning_to_body(body: &mut Value, level: Option<&str>, summary: bool, openrouter: bool) {
    if body["input"].is_array() {
        if level.is_none() && !summary {
            return;
        }
        let mut reasoning = body["reasoning"].as_object().cloned().unwrap_or_default();
        if let Some(level) = level {
            reasoning
                .insert("effort".to_owned(), Value::String(if level == "off" { "none" } else { level }.to_owned()));
        }
        if summary && level != Some("off") && !reasoning.contains_key("summary") {
            reasoning.insert("summary".to_owned(), Value::String("auto".to_owned()));
        }
        body.as_object_mut().map(|map| map.insert("reasoning".to_owned(), Value::Object(reasoning)));
    } else if let Some(level) = level {
        if body["messages"].is_array() {
            if openrouter {
                let mut reasoning = body["reasoning"].as_object().cloned().unwrap_or_default();
                reasoning
                    .insert("effort".to_owned(), Value::String(if level == "off" { "none" } else { level }.to_owned()));
                body.as_object_mut().map(|map| map.insert("reasoning".to_owned(), Value::Object(reasoning)));
            } else {
                body.as_object_mut().map(|map| {
                    map.insert(
                        "reasoning_effort".to_owned(),
                        Value::String(if level == "off" { "none" } else { level }.to_owned()),
                    )
                });
            }
        }
    }
}

/// Whether a model id is an Ollama cloud model (isOllamaCloudModel).
pub fn is_ollama_cloud_model(model: &str) -> bool {
    let lower = model.to_ascii_lowercase();
    lower.ends_with("cloud")
        && (lower.len() == 5 || lower.as_bytes().get(lower.len() - 6).is_some_and(|b| *b == b':' || *b == b'-'))
}

/// Heading a model is grouped under in pickers (modelGroupLabel).
pub fn model_group_label(provider: &str, model: &str) -> String {
    if provider == "ollamaLocal" && is_ollama_cloud_model(model) {
        return "Ollama Cloud · via this PC".to_owned();
    }
    PROVIDER_LABELS
        .iter()
        .find(|(id, _)| *id == provider)
        .map_or_else(|| provider.to_owned(), |(_, label)| (*label).to_owned())
}

/// Casing of one word of a model name (titleWord).
fn title_word(word: &str) -> String {
    let lower = word.to_ascii_lowercase();
    if let Some((_, proper)) = WORD_CASE.iter().find(|(key, _)| *key == lower) {
        return (*proper).to_owned();
    }
    let letters: String = lower.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
    let version = &lower[letters.len()..];
    if let Some((_, proper)) = WORD_CASE.iter().find(|(key, _)| *key == letters) {
        if !version.is_empty() && version.chars().all(|c| c.is_ascii_digit() || c == '.') {
            return format!("{proper} {version}");
        }
    }
    if lower.chars().all(|c| c.is_ascii_alphanumeric() || c == '.')
        && lower.chars().any(|c| c.is_ascii_digit())
        && lower.chars().any(|c| c.is_ascii_alphabetic())
        && lower.len() <= 8
    {
        return lower.to_ascii_uppercase();
    }
    if lower.starts_with(|c: char| c.is_ascii_digit()) {
        return word.to_owned();
    }
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
        None => String::new(),
    }
}

/// A readable model name (humanizeModelName).
pub fn humanize_model_name(model: &str) -> String {
    let trimmed = model.trim();
    if trimmed.is_empty() {
        return model.to_owned();
    }
    let mut id = trimmed.to_owned();
    if let Some(rest) = id.strip_prefix("hf.co/") {
        id = rest.split('/').skip(1).collect::<Vec<_>>().join("/");
    }
    if id.contains('/') {
        id = id.rsplit('/').next().unwrap_or(&id).to_owned();
    }
    let (raw_name, tag) = id.split_once(':').unwrap_or((&id, ""));
    let name = raw_name.strip_suffix("-gguf").unwrap_or(raw_name);
    let parts: Vec<&str> = name
        .split(|c| c == '-' || c.is_whitespace())
        .filter(|part| !part.is_empty())
        .enumerate()
        .filter(|(index, part)| !(*index > 0 && part.len() == 8 && part.chars().all(|c| c.is_ascii_digit())))
        .map(|(_, part)| part)
        .collect();
    let words: Vec<String> = parts.iter().map(|part| title_word(part)).collect();
    let mut result = words.join(" ");
    if !tag.is_empty() && tag.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        result.push_str(&format!(" ({})", tag.to_ascii_uppercase()));
    }
    result
}

/// Display name of a model, preferring a saved label (displayModelName).
pub fn display_model_name(label: Option<&str>, model: &str) -> String {
    label
        .filter(|value| !value.trim().is_empty())
        .map_or_else(|| humanize_model_name(model), |value| value.trim().to_owned())
}

/// Chat effort override, when valid (getChatEffort).
pub fn chat_effort_of(overrides: &Value, chat_key: &str) -> Option<String> {
    overrides
        .get(chat_key)
        .and_then(Value::as_str)
        .filter(|level| *level == "auto" || is_reasoning_level(Some(level)))
        .map(str::to_owned)
}

/// Set or clear a chat's effort override, trimming old entries (setChatEffort).
pub fn set_chat_effort(overrides: &Value, chat_key: &str, level: Option<&str>) -> Value {
    let mut next = overrides.clone();
    let Some(map) = next.as_object_mut() else {
        return json!({});
    };
    map.remove(chat_key);
    if let Some(level) = level.filter(|value| *value == "auto" || is_reasoning_level(Some(value))) {
        map.insert(chat_key.to_owned(), Value::String(level.to_owned()));
    }
    while map.len() > MAX_CHAT_OVERRIDES {
        if let Some(first) = map.keys().next().cloned() {
            map.remove(&first);
        }
    }
    Value::Object(map.clone())
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(levels: usize, providers: usize, adaptive: bool, chat: bool) -> Vec<(&'static str, String)> {
    vec![
        ("levels", count_label(levels)),
        ("providers", count_label(providers)),
        ("adaptive", if adaptive { "yes" } else { "no" }.to_owned()),
        ("chat", if chat { "yes" } else { "no" }.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refs_and_levels() {
        assert!(is_valid_model_ref(Some("openai/gpt-5")));
        assert!(!is_valid_model_ref(Some("")));
        assert!(!is_valid_model_ref(Some("bad\nname")));
        assert!(is_reasoning_level(Some("xhigh")));
        assert!(!is_reasoning_level(Some("none")));
        assert_eq!(prefs_key("openai", "gpt-5"), "openai::gpt-5");
        assert_eq!(reasoning_label("xhigh"), "Extra high");
        assert!(provider_has_reasoning_control("openai"));
        assert!(!provider_has_reasoning_control("anthropic"));
    }

    #[test]
    fn prefs_and_effort() {
        let raw = json!({"openai::gpt-5":{"label":"  My GPT  ","reasoning":"high"},"bad":{"label":"x"}});
        let clean = sanitize_prefs(&raw);
        assert!(clean.get("bad").is_none());
        assert_eq!(model_pref_of(&clean, "openai", "gpt-5")["label"], "My GPT");
        let updated = apply_model_pref(&clean, "openai", "gpt-5", None, Some("low"));
        assert_eq!(model_pref_of(&updated, "openai", "gpt-5")["reasoning"], "low");
        assert!(model_pref_of(&updated, "openai", "none").as_object().is_some_and(Map::is_empty));
        let effort = effort_for("openai", "gpt-5", &updated, Some("high"));
        assert_eq!(effort["effective"], "high");
        assert_eq!(effort["source"], "chat");
        assert_eq!(effective_reasoning("openai", "gpt-5", &updated, None).as_deref(), Some("low"));
        let overrides = set_chat_effort(&json!({}), "cli", Some("medium"));
        assert_eq!(chat_effort_of(&overrides, "cli").as_deref(), Some("medium"));
    }

    #[test]
    fn claude_and_openrouter() {
        assert!(openai_reasoning_model("gpt-5.1"));
        assert!(openai_reasoning_model("o3"));
        assert!(!openai_reasoning_model("gpt-4o"));
        let thinking = claude_thinking("claude-opus-4-7");
        assert_eq!(thinking["adaptive"], true);
        assert_eq!(thinking["levels"][0], "low");
        assert_eq!(claude_thinking("claude-haiku-4-5")["levels"].as_array().map_or(0, Vec::len), 0);
        assert_eq!(
            reasoning_levels_for("anthropic", Some("claude-opus-5")),
            vec!["low", "medium", "high", "xhigh", "max"]
        );
        assert_eq!(reasoning_levels_for("ollamaLocal", Some("llama3.2:cloud")), vec!["off", "low", "medium", "high"]);
        assert!(is_openrouter_url(Some("https://openrouter.ai/api/v1")));
        let options = anthropic_thinking_options("claude-opus-4-7", Some("high")).unwrap();
        assert_eq!(options["thinking"]["type"], "adaptive");
        assert!(anthropic_thinking_options("claude-3-5-sonnet", Some("high")).is_none());
    }

    #[test]
    fn bodies_and_names() {
        let mut body = json!({"messages": []});
        apply_reasoning_to_body(&mut body, Some("low"), false, false);
        assert_eq!(body["reasoning_effort"], "low");
        let mut body = json!({"messages": [], "reasoning": {"summary": "detailed"}});
        apply_reasoning_to_body(&mut body, Some("off"), false, true);
        assert_eq!(body["reasoning"]["effort"], "none");
        let mut body = json!({"input": []});
        apply_reasoning_to_body(&mut body, Some("high"), true, false);
        assert_eq!(body["reasoning"]["summary"], "auto");
        assert!(is_ollama_cloud_model("llama3:cloud"));
        assert_eq!(model_group_label("ollamaLocal", "llama3:cloud"), "Ollama Cloud · via this PC");
        assert_eq!(humanize_model_name("hf.co/user/gpt-oss-120b"), "GPT OSS 120B");
        assert_eq!(humanize_model_name("qwen3.8"), "Qwen 3.8");
        assert_eq!(display_model_name(Some("My model"), "gpt-oss-120b"), "My model");
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(6, 8, true, false).len(), 4);
    }
}
