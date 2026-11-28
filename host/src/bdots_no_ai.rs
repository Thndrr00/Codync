#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from agentes-chat's `no-ai.ts`: cards for an agent with no AI, kept-message
//! state, plan lines, connect intents and plan choices. Composes helpers already ported into this
//! host. Only `botInsights` calls this.

use serde_json::Value;

/// Reason code when nobody had an AI to think with (NO_AI).
pub const NO_AI: &str = "no_ai";

/// Why a right-hand plan can't think (RightHandPlanLine).
pub const RIGHT_HAND_PLAN_LINES: [&str; 13] = [
    "computer_away",
    "computer_failed",
    "cli_failed",
    "plan_limit",
    "model_limit",
    "model_region",
    "model_off",
    "plan_inactive",
    "commands_off",
    "plan_not_shared",
    "plan_unattended",
    "runner_outdated",
    "plan_tools",
];

/// Connect intents a card may carry (ConnectIntent).
pub const INTENTS: [&str; 5] = ["claude", "chatgpt", "api_key", "any", "task"];

/// The brain a plan thinks with, on the agent's computer (PLAN_BRAIN).
pub const PLAN_BRAIN: [(&str, &str); 6] = [
    ("claude", "claude_code"),
    ("chatgpt", "codex"),
    ("cursor", "cursor"),
    ("zen", "opencode"),
    ("grok", "grok"),
    ("gemini", "antigravity"),
];

/// Every plan picker's order, most used first (PLAN_CHOICES).
pub const PLAN_CHOICES: [&str; 6] = ["claude", "chatgpt", "gemini", "grok", "cursor", "zen"];

/// Maker marks of plans (PLAN_MARKS).
pub const PLAN_MARKS: [&str; 4] = ["claude", "openai", "zai", "kimi"];

/// Maker marks of API keys (KEY_MARKS).
pub const KEY_MARKS: [&str; 4] = ["claude", "openai", "gemini", "openrouter"];

/// How long a cloud install may wait (CLOUD_INSTALL_WAIT_MS).
pub const CLOUD_INSTALL_WAIT_MS: u64 = 20 * 60_000;

/// How long a banner catches up with reads (CATCH_UP_MS).
pub const CATCH_UP_MS: u64 = 8_000;

/// Coding plan ids a provider may name.
pub fn is_plan_id(provider: &str) -> bool {
    matches!(provider, "glm" | "kimi" | "zai" | "minimax" | "glm-coding" | "kimi-coding")
}

/// Whether a reason is one of the right-hand plan lines.
pub fn is_plan_line(reason: &str) -> bool {
    RIGHT_HAND_PLAN_LINES.contains(&reason)
}

/// Whether a string is a connect intent.
pub fn is_intent(value: &str) -> bool {
    INTENTS.contains(&value)
}

/// The card in a failed run's `noAi` metadata (noAiCardOf).
pub fn no_ai_card_of(meta: Option<&Value>) -> Value {
    let Some(raw) = meta.and_then(|value| value.get("noAi")).filter(|value| value.is_object()) else {
        return Value::Null;
    };
    let Some(agent_id) = raw["agentId"].as_str() else {
        return Value::Null;
    };
    let reason = raw["reason"].as_str().unwrap_or("");
    let signed_out = reason == "not_signed_in";
    let own = if signed_out {
        PLAN_BRAIN.iter().find(|(_, kind)| *kind == raw["own"].as_str().unwrap_or("")).map(|(_, kind)| *kind)
    } else {
        None
    };
    let mut card = serde_json::Map::new();
    card.insert("agentId".to_owned(), Value::String(agent_id.to_owned()));
    card.insert(
        "forUserId".to_owned(),
        raw["forUserId"].as_str().map_or(Value::Null, |id| Value::String(id.to_owned())),
    );
    card.insert(
        "intent".to_owned(),
        Value::String(raw["intent"].as_str().filter(|value| is_intent(value)).unwrap_or("task").to_owned()),
    );
    if let Some(provider) = raw["provider"].as_str() {
        card.insert("provider".to_owned(), Value::String(provider.to_owned()));
    }
    if raw["autoOpen"] == true {
        card.insert("autoOpen".to_owned(), Value::Bool(true));
    }
    if let Some(kind) = own {
        card.insert("own".to_owned(), Value::String(kind.to_owned()));
    } else if signed_out {
        card.insert("signedOut".to_owned(), Value::Bool(true));
    }
    if is_plan_line(reason) {
        card.insert("planLine".to_owned(), Value::String(reason.to_owned()));
    }
    if let Some(model) = raw["model"].as_str().filter(|value| !value.is_empty()) {
        card.insert("model".to_owned(), Value::String(model.to_owned()));
    }
    if reason == "cli_failed" {
        if let Some(why) = raw["why"].as_str().filter(|value| !value.is_empty()) {
            card.insert("why".to_owned(), Value::String(why.to_owned()));
        }
    }
    if matches!(reason, "cli_missing" | "no_computer") {
        card.insert("missing".to_owned(), Value::String(reason.to_owned()));
    }
    if reason == "plan_hold" {
        card.insert("held".to_owned(), Value::Bool(true));
    }
    Value::Object(card)
}

/// Where a kept message stands, from the plan to its answer (keptCardState).
pub fn kept_card_state(hold: bool, paid: bool, ready: bool) -> &'static str {
    if hold {
        if paid { "paid" } else { "held" }
    } else if ready {
        "answer"
    } else {
        "connect"
    }
}

/// Key of a plan line's sentence (planLineKey).
pub fn plan_line_key(line: &str, why: Option<&str>) -> String {
    if line == "cli_failed" && why.is_some_and(|value| !value.is_empty()) {
        "noAi.rightHand.cli_failedWhy".to_owned()
    } else {
        format!("noAi.rightHand.{line}")
    }
}

/// A plan line that passes by itself once the agent can think again (planLinePasses).
pub fn plan_line_passes(line: Option<&str>) -> bool {
    matches!(line, Some("computer_away" | "computer_failed" | "cli_failed"))
}

/// Key of a card's line while the agent still can't think (noAiLineKey).
pub fn no_ai_line_key(own: Option<&str>, signed_out: bool, missing: Option<&str>) -> String {
    if let Some(kind) = own {
        format!("noAi.notSignedIn.{kind}")
    } else if signed_out {
        "noAi.signedOut".to_owned()
    } else {
        match missing {
            Some("cli_missing") => "noAi.cliMissing".to_owned(),
            Some("no_computer") => "noAi.noComputer".to_owned(),
            _ => "noAi.agent".to_owned(),
        }
    }
}

/// Whether the connect dialog opens by itself (opensByItself).
pub fn opens_by_itself(intent: Option<&str>, auto_open: bool) -> bool {
    auto_open && matches!(intent, Some("claude" | "chatgpt" | "api_key"))
}

/// The brain a plan thinks with (PLAN_BRAIN lookup).
pub fn brain_of_plan(plan: &str) -> &'static str {
    PLAN_BRAIN.iter().find(|(name, _)| *name == plan).map_or("claude_code", |(_, kind)| *kind)
}

/// The plan a CLI signs in to by itself (planOfKind).
pub fn plan_of_kind(kind: &str) -> &'static str {
    match kind {
        "codex" => "chatgpt",
        "cursor" => "cursor",
        "opencode" => "zen",
        "grok" => "grok",
        "antigravity" => "gemini",
        _ => "claude",
    }
}

/// An agent's state in the AI status (aiStateOf).
pub fn ai_state_of(status: Option<&Value>, agent_id: &str) -> Option<String> {
    let agents = status?.get("agents")?.as_array()?;
    agents
        .iter()
        .find(|entry| entry["agentId"].as_str() == Some(agent_id))
        .and_then(|entry| entry["state"].as_str())
        .map(str::to_owned)
}

/// Account kind an agent's plan choice signs in as (kindOf).
pub fn kind_of_choice(choice: &str) -> &'static str {
    if matches!(choice, "chatgpt" | "cursor" | "zen" | "grok" | "gemini") {
        brain_of_plan(choice)
    } else {
        "claude_code"
    }
}

/// Plan of an account, from its provider or kind (planOfAccount).
pub fn plan_of_account(account: &Value) -> String {
    let provider = account["provider"].as_str().unwrap_or("");
    if is_plan_id(provider) {
        provider.to_owned()
    } else {
        plan_of_kind(account["kind"].as_str().unwrap_or("")).to_owned()
    }
}

/// What an agent's brain view thinks with, as the picker's choice (aiChoiceOf).
pub fn ai_choice_of(view: &Value) -> String {
    if view["brain"].as_str() == Some("model") {
        return "key".to_owned();
    }
    let provider = view["accounts"]
        .as_array()
        .and_then(|accounts| {
            let account_id = view["account"].as_str()?;
            accounts
                .iter()
                .find(|account| account["id"].as_str() == Some(account_id))
                .and_then(|account| account["provider"].as_str())
        })
        .unwrap_or("");
    plan_of_account(&serde_json::json!({"kind": view["brain"].as_str().unwrap_or(""), "provider": provider}))
}

/// A plan's maker mark (planMark).
pub fn plan_mark(choice: &str) -> &'static str {
    match choice {
        "chatgpt" => "openai",
        "zen" => "opencode",
        other => other,
    }
}

/// A plan's short name as its tile says it (planLabel).
pub fn plan_choice_label(choice: &str) -> String {
    if is_plan_id(choice) {
        return crate::bdots_textfmt::upper_first(choice);
    }
    match choice {
        "chatgpt" => "ChatGPT".to_owned(),
        "gemini" => "Gemini".to_owned(),
        "grok" => "Grok".to_owned(),
        "cursor" => "Cursor".to_owned(),
        "zen" => "OpenCode".to_owned(),
        _ => "Claude".to_owned(),
    }
}

/// Other names people search a plan by (planSearchNames).
pub fn plan_search_names(choice: &str) -> &'static str {
    match choice {
        "claude" => "Claude Code",
        "chatgpt" => "Codex",
        "gemini" => "Gemini CLI Antigravity",
        "grok" => "Grok Build",
        "zen" => "OpenCode Zen",
        _ => "",
    }
}

/// Key of an agent's line from its AI status (noAiStatusKey).
pub fn no_ai_status_key(status: Option<&Value>, agent_id: &str) -> String {
    let reason = status
        .and_then(|value| value.get("agents"))
        .and_then(Value::as_array)
        .and_then(|agents| agents.iter().find(|entry| entry["agentId"].as_str() == Some(agent_id)))
        .and_then(|entry| entry["reason"].as_str())
        .unwrap_or("");
    no_ai_line_key(
        None,
        reason == "not_signed_in",
        if matches!(reason, "cli_missing" | "no_computer") { Some(reason) } else { None },
    )
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(lines: usize, choices: usize, opens: bool, held: bool) -> Vec<(&'static str, String)> {
    vec![
        ("lines", count_label(lines)),
        ("choices", count_label(choices)),
        ("opens", if opens { "yes" } else { "no" }.to_owned()),
        ("held", if held { "yes" } else { "no" }.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn codes_and_lines() {
        assert_eq!(NO_AI, "no_ai");
        assert!(is_plan_line("computer_away"));
        assert!(!is_plan_line("plan_hold"));
        assert!(is_intent("api_key"));
        assert!(!is_intent("task_x"));
        assert_eq!(kept_card_state(true, false, true), "held");
        assert_eq!(kept_card_state(true, true, false), "paid");
        assert_eq!(kept_card_state(false, false, true), "answer");
        assert_eq!(kept_card_state(false, false, false), "connect");
        assert_eq!(plan_line_key("cli_failed", Some("boom")), "noAi.rightHand.cli_failedWhy");
        assert_eq!(plan_line_key("plan_limit", None), "noAi.rightHand.plan_limit");
        assert!(plan_line_passes(Some("cli_failed")));
        assert!(!plan_line_passes(Some("plan_limit")));
    }

    #[test]
    fn cards() {
        let meta = json!({"noAi":{"agentId":"a1","intent":"api_key","reason":"not_signed_in","own":"codex","provider":"openai","autoOpen":true}});
        let card = no_ai_card_of(Some(&meta));
        assert_eq!(card["agentId"], "a1");
        assert_eq!(card["own"], "codex");
        assert_eq!(card["intent"], "api_key");
        assert_eq!(no_ai_card_of(None), Value::Null);
        let meta = json!({"noAi":{"agentId":"a1","reason":"plan_hold"}});
        assert_eq!(no_ai_card_of(Some(&meta))["held"], true);
    }

    #[test]
    fn line_keys_and_auto_open() {
        assert_eq!(no_ai_line_key(Some("codex"), false, None), "noAi.notSignedIn.codex");
        assert_eq!(no_ai_line_key(None, true, None), "noAi.signedOut");
        assert_eq!(no_ai_line_key(None, false, Some("cli_missing")), "noAi.cliMissing");
        assert_eq!(no_ai_line_key(None, false, None), "noAi.agent");
        assert!(opens_by_itself(Some("claude"), true));
        assert!(!opens_by_itself(Some("any"), true));
        assert!(!opens_by_itself(Some("claude"), false));
        let status = json!({"agents":[{"agentId":"a1","reason":"not_signed_in","state":"no_ai"}]});
        assert_eq!(no_ai_status_key(Some(&status), "a1"), "noAi.signedOut");
        assert_eq!(ai_state_of(Some(&status), "a1").as_deref(), Some("no_ai"));
    }

    #[test]
    fn plans_and_choices() {
        assert_eq!(brain_of_plan("chatgpt"), "codex");
        assert_eq!(plan_of_kind("antigravity"), "gemini");
        assert_eq!(kind_of_choice("zen"), "opencode");
        assert_eq!(plan_of_account(&json!({"kind":"codex","provider":null})), "chatgpt");
        let view = json!({"brain":"model","account":"a1","accounts":[{"id":"a1","provider":"openai"}]});
        assert_eq!(ai_choice_of(&view), "key");
        let view = json!({"brain":"claude_code","account":"a1","accounts":[{"id":"a1","provider":"claude_code"}]});
        assert_eq!(ai_choice_of(&view), "claude");
        assert_eq!(plan_mark("chatgpt"), "openai");
        assert_eq!(plan_choice_label("chatgpt"), "ChatGPT");
        assert_eq!(plan_search_names("grok"), "Grok Build");
        assert_eq!(CLOUD_INSTALL_WAIT_MS, 1_200_000);
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(13, 6, true, false).len(), 4);
    }
}
