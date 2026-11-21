//! Fast mode in a session's config options (ACP `configOptions`), and the request a custom
//! OpenAI-compatible endpoint gets.

use serde_json::{Value, json};
use std::path::Path;

/// OpenBot 0.74.0: with "Use my Claude Code setup" off, Claude agents must not load the user's
/// `~/.claude/CLAUDE.md` or the rules in `~/.claude/rules` as project files on each turn (even the
/// ones the user excluded with `claudeMdExcludes`). Codync's switch is the bot's `harnessMemory`
/// (`agent::own_memory::kept`); when it is off these are the exclusions the launch's Claude Code
/// settings need. Turn the switch on to load the files, with the user's own exclusions applying.
pub fn claude_md_excludes(home: &Path, use_setup: bool) -> Option<Value> {
    (!use_setup).then(|| {
        json!([
            home.join(".claude").join("CLAUDE.md").to_string_lossy(),
            home.join(".claude").join("rules").join("**").to_string_lossy(),
        ])
    })
}

/// OpenBot 0.70.0: the host's Claude usage check is a `claude -p /usage` run. With "Use my Claude
/// Code setup" on it keeps the home folder, where the user's own setup applies (old behavior). With
/// it off it runs from the host's probe folder, so a slow user setup can't hold the check to its
/// deadline. Pure so the cwd decision is testable without starting Claude.
pub fn claude_check_dir<'a>(home: &'a Path, probe: &'a Path, use_setup: bool) -> &'a Path {
    if use_setup { home } else { probe }
}

/// A custom endpoint a bot points at, as far as request building is concerned. The key is only a
/// flag here: it is sent as a header by the agent CLI, never in a request body or a log.
pub struct CustomEndpoint<'a> {
    pub base_url: &'a str,
    pub api_key: Option<&'a str>,
}

/// OpenBot 0.74.0 (#7476): OpenAI's own v1 API refuses function tools together with
/// `reasoning_effort` on `/chat/completions` for newer models such as GPT-6 Astra, so each turn
/// failed with "Function tools with reasoning_effort are not supported". With a key, its v1 URL
/// uses the Responses adapter (`/responses`, `store: false`), which accepts both. A keyless or
/// other endpoint keeps the compatible adapter: the OpenAI adapter would otherwise read an
/// inherited `OPENAI_API_KEY` and send a key the user never gave this endpoint.
pub fn uses_responses_api(endpoint: &CustomEndpoint<'_>) -> bool {
    endpoint.api_key.is_some_and(|key| !key.trim().is_empty())
        && endpoint.base_url.trim().trim_end_matches('/').eq_ignore_ascii_case("https://api.openai.com/v1")
}

/// The path the agent CLI calls for `endpoint`.
pub fn request_path(endpoint: &CustomEndpoint<'_>) -> &'static str {
    if uses_responses_api(endpoint) { "/v1/responses" } else { "/v1/chat/completions" }
}

/// The request body for one turn on a custom endpoint: function tools and reasoning effort are
/// both kept, whichever adapter the endpoint uses. The key is not part of the body.
pub fn custom_request(endpoint: &CustomEndpoint<'_>, model: &str, effort: Option<&str>, tools: &[Value]) -> Value {
    let mut body = json!({"model": model, "stream": true, "tools": tools});
    if uses_responses_api(endpoint) {
        body["store"] = json!(false);
    }
    if let Some(effort) = effort.filter(|effort| !effort.trim().is_empty()) {
        body["reasoning_effort"] = json!(effort);
    }
    body
}

/// Fast mode's option IDs: Claude's adapter (`fast`) and Codex's (`fast-mode`).
const FAST_MODE_IDS: [&str; 2] = ["fast", "fast-mode"];

/// The Fast mode option. Agents offer it only while the session's model supports it.
pub fn fast_mode_config(session: &Value) -> Option<&Value> {
    session["configOptions"]
        .as_array()?
        .iter()
        .find(|v| v["id"].as_str().is_some_and(|id| FAST_MODE_IDS.contains(&id)) && is_switch(v))
}

/// A boolean, or a select listing `on` and `off`, as Claude and Codex offer it. Cursor's `fast`
/// (`true` / `false`) is one of its model's options instead, set from the bot's `modelOptions`.
fn is_switch(option: &Value) -> bool {
    let lists = |value: &str| option["options"].as_array().is_some_and(|o| o.iter().any(|v| v["value"] == value));
    option["type"] == "boolean" || (option["type"] == "select" && lists("on") && lists("off"))
}

/// Whether the Fast mode option is on.
pub fn fast_mode_on(config: &Value) -> bool {
    config["currentValue"] == true || config["currentValue"] == "on"
}

/// Why the agent turned Fast mode off by itself, when it says: Claude's adapter appends it to
/// the option's description ("Faster responses on supported models — requires extra usage…").
pub fn fast_mode_refusal(config: &Value) -> Option<String> {
    let description = config["description"].as_str()?;
    let (_, why) = description.split_once(" — ")?;
    Some(why.trim()).filter(|why| !why.is_empty()).map(str::to_owned)
}

/// What a bot's agent says about Fast mode (wire `fastModeSupport` and `fastModeIssue`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum FastModeSupport {
    /// No session has listed its options yet.
    #[default]
    Unknown,
    Offered,
    /// The session's model doesn't offer it.
    Unsupported,
    /// The agent turned it off after the bot asked for it (Claude: the account can't use it),
    /// with its reason when it gave one. Asked again only once the user changes the setting.
    Refused(Option<String>),
}

impl FastModeSupport {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Offered => "offered",
            Self::Unsupported => "unsupported",
            Self::Refused(_) => "refused",
        }
    }

    pub fn issue(&self) -> Option<&str> {
        match self {
            Self::Refused(why) => why.as_deref(),
            _ => None,
        }
    }

    /// Kept across host restarts (`fast-mode.<bot>`), so clients can offer the switch before
    /// the first turn. A refusal isn't kept: the next host asks again.
    pub fn stored(&self) -> Option<&'static str> {
        match self {
            Self::Unknown => None,
            Self::Offered | Self::Refused(_) => Some("offered"),
            Self::Unsupported => Some("unsupported"),
        }
    }

    pub fn from_stored(v: Option<&str>) -> Self {
        match v {
            Some("offered") => Self::Offered,
            Some("unsupported") => Self::Unsupported,
            _ => Self::Unknown,
        }
    }
}

/// The value that turns `config` on or off: a boolean, or the `on` / `off` select entries.
/// `None` when it already is.
pub fn fast_mode_change(config: &Value, on: bool) -> Option<Value> {
    let value = if config["type"] == "boolean" { json!(on) } else { json!(if on { "on" } else { "off" }) };
    (config["currentValue"] != value).then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_custom_openai_v1_keeps_function_tools_and_reasoning_effort() {
        let tools = [json!({"type": "function", "function": {"name": "search"}})];
        let openai = CustomEndpoint { base_url: "https://api.openai.com/v1", api_key: Some("sk-test") };
        assert!(uses_responses_api(&openai));
        assert_eq!(request_path(&openai), "/v1/responses");
        let body = custom_request(&openai, "gpt-6-astra", Some("high"), &tools);
        assert_eq!(body["tools"], json!(tools), "function tools stay with a reasoning effort");
        assert_eq!(body["reasoning_effort"], "high");
        assert_eq!(body["store"], false);
        assert!(!body.to_string().contains("sk-test"), "the key is a header, not the body");

        let keyless = CustomEndpoint { base_url: "https://api.openai.com/v1", api_key: None };
        let other = CustomEndpoint { base_url: "https://llm.example.com/v1", api_key: Some("sk-test") };
        for endpoint in [&keyless, &other] {
            assert!(!uses_responses_api(endpoint));
            assert_eq!(request_path(endpoint), "/v1/chat/completions");
            let body = custom_request(endpoint, "gpt-6-astra", Some("high"), &tools);
            assert_eq!(body["tools"], json!(tools));
            assert_eq!(body["reasoning_effort"], "high");
            assert!(body.get("store").is_none());
        }
        let no_effort = custom_request(&openai, "gpt-6-astra", None, &tools);
        assert_eq!(no_effort["tools"], json!(tools));
        assert!(no_effort.get("reasoning_effort").is_none());
    }

    #[test]
    fn claude_setup_off_excludes_the_home_instruction_files() {
        let home = Path::new("/home/someone");
        let off = claude_md_excludes(home, false).expect("switch off excludes files");
        let list = off.as_array().expect("a list of paths");
        assert_eq!(list.len(), 2);
        assert!(
            list[0].as_str().unwrap().ends_with(".claude/CLAUDE.md")
                || list[0].as_str().unwrap().ends_with(".claude\\CLAUDE.md")
        );
        let rules = list[1].as_str().unwrap();
        assert!(rules.ends_with("rules/**") || rules.ends_with("rules\\**"), "{rules}");
        assert_eq!(claude_md_excludes(home, true), None, "switch on loads the user's own setup");
    }

    #[test]
    fn claude_checks_leave_the_home_folder_when_the_setup_switch_is_off() {
        let home = Path::new("/home/someone");
        let probe = Path::new("/data/codync/probe");
        assert_eq!(claude_check_dir(home, probe, true), home, "switch on keeps the old behavior");
        assert_eq!(claude_check_dir(home, probe, false), probe, "switch off avoids a slow user setup");
    }

    #[test]
    fn fast_mode_is_claudes_and_codexs_on_off_option() {
        let on_off = json!([{"value": "on"}, {"value": "off"}]);
        for id in ["fast", "fast-mode"] {
            let session = json!({"configOptions": [
                {"id": "model", "category": "model", "type": "select", "options": []},
                {"id": id, "category": "model_config", "type": "select", "currentValue": "off", "options": on_off}
            ]});
            assert_eq!(fast_mode_config(&session).unwrap()["id"], id);
        }
        let boolean = json!({"configOptions": [{"id": "fast", "type": "boolean", "currentValue": false}]});
        assert!(fast_mode_config(&boolean).is_some());
        let other = json!({"configOptions": [{"id": "effort", "category": "thought_level", "type": "select"}]});
        assert!(fast_mode_config(&other).is_none());
        // Cursor's `fast` is a model option the bot sets through `modelOptions`.
        let cursor = json!({"configOptions": [{"id": "fast", "type": "select", "currentValue": "true",
            "options": [{"value": "false"}, {"value": "true"}]}]});
        assert!(fast_mode_config(&cursor).is_none());
    }

    #[test]
    fn fast_mode_refusal_reads_the_reason_from_the_description() {
        let refused = json!({"id": "fast", "currentValue": "off",
            "description": "Faster responses on supported models — requires extra usage to be enabled for this account"});
        assert_eq!(fast_mode_refusal(&refused).as_deref(), Some("requires extra usage to be enabled for this account"));
        assert!(!fast_mode_on(&refused));
        assert!(fast_mode_refusal(&json!({"description": "1.5x speed, increased usage"})).is_none());
        assert!(fast_mode_on(&json!({"currentValue": true})));
    }

    #[test]
    fn fast_mode_support_keeps_only_what_the_agent_offers_across_restarts() {
        let refused = FastModeSupport::Refused(Some("no extra usage".into()));
        assert_eq!(refused.as_str(), "refused");
        assert_eq!(refused.issue(), Some("no extra usage"));
        assert_eq!(FastModeSupport::from_stored(refused.stored()), FastModeSupport::Offered);
        assert_eq!(FastModeSupport::from_stored(FastModeSupport::Unknown.stored()), FastModeSupport::Unknown);
    }

    #[test]
    fn fast_mode_change_sends_only_a_different_value() {
        let select = json!({"id": "fast", "type": "select", "currentValue": "off"});
        assert_eq!(fast_mode_change(&select, true), Some(json!("on")));
        assert_eq!(fast_mode_change(&select, false), None);
        let boolean = json!({"id": "fast-mode", "type": "boolean", "currentValue": true});
        assert_eq!(fast_mode_change(&boolean, false), Some(json!(false)));
        assert_eq!(fast_mode_change(&boolean, true), None);
    }
}
