//! Every agent's hook payload mapped onto the event names and fields of Claude Code's.
//!
//! Adapted from the MIT-licensed `coucou-hook` (github.com/Louis-CFM/coucou, `windows/hook/src/normalize.rs`).

use serde_json::{Map, Value};

/// The canonical name of an agent's event. Unknown names pass through.
pub fn event(name: &str) -> &str {
    match name {
        "BeforeTool" | "BeforeToolSelection" | "pre_tool_use" | "preToolUse" => "PreToolUse",
        "AfterTool" | "AfterModel" | "PostInvocation" | "post_tool_use" | "postToolUse" => "PostToolUse",
        "BeforeAgent" | "PreInvocation" | "user_prompt_submit" | "userPromptSubmitted" | "beforeSubmitPrompt" => {
            "UserPromptSubmit"
        }
        "AfterAgent" | "stop" | "agentStop" => "Stop",
        "startup" | "session_start" | "sessionStart" => "SessionStart",
        "exit" | "session_end" | "sessionEnd" => "SessionEnd",
        "notification" => "Notification",
        "permissionRequest" => "PermissionRequest",
        "postToolUseFailure" => "PostToolUseFailure",
        "subagentStart" => "SubagentStart",
        "subagentStop" => "SubagentStop",
        other => other,
    }
}

/// Keys of Antigravity's `toolCall.args` that Codync reads under another name.
const ARG_ALIASES: &[(&str, &str)] = &[
    ("CommandLine", "command"),
    ("FilePath", "file_path"),
    ("Path", "path"),
    ("Url", "url"),
    ("Query", "query"),
    ("Pattern", "pattern"),
];

fn non_empty<'a>(map: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    map.get(key).and_then(Value::as_str).filter(|s| !s.is_empty())
}

/// Fills `tool_name`, `tool_input`, `session_id` and `cwd` from wherever the agent put them. A field the payload
/// already has is never overwritten. `env` reads an environment variable (a parameter so tests stay pure).
pub fn fields(map: &mut Map<String, Value>, env: &dyn Fn(&str) -> Option<String>) {
    let tool_call = map.get("toolCall").and_then(Value::as_object).cloned();

    if !map.contains_key("tool_name") {
        let name = non_empty(map, "toolName")
            .or_else(|| tool_call.as_ref().and_then(|t| non_empty(t, "name")))
            .or_else(|| non_empty(map, "tool"))
            .map(str::to_owned);
        if let Some(name) = name {
            map.insert("tool_name".into(), Value::String(name));
        }
    }

    if !map.contains_key("tool_input") {
        // Copilot sends toolArgs as an object, or as the JSON text of one.
        let args = match map.get("toolArgs") {
            Some(Value::Object(o)) => Some(o.clone()),
            Some(Value::String(s)) => serde_json::from_str::<Value>(s).ok().and_then(|v| v.as_object().cloned()),
            _ => None,
        };
        let input = args.or_else(|| {
            let mut flat = tool_call.as_ref()?.get("args")?.as_object()?.clone();
            for (from, to) in ARG_ALIASES {
                if let Some(v) = flat.get(*from).cloned() {
                    flat.insert((*to).to_owned(), v);
                }
            }
            Some(flat)
        });
        if let Some(input) = input {
            map.insert("tool_input".into(), Value::Object(input));
        }
    }

    if non_empty(map, "session_id").is_none() {
        let sid = ["conversationId", "conversation_id", "sessionId"]
            .iter()
            .find_map(|k| non_empty(map, k))
            .map(str::to_owned)
            .or_else(|| env("GEMINI_SESSION_ID").filter(|s| !s.is_empty()));
        if let Some(sid) = sid {
            map.insert("session_id".into(), Value::String(sid));
        }
    }

    if non_empty(map, "cwd").is_none() {
        let cwd = non_empty(map, "workdir").map(str::to_owned).or_else(|| {
            ["workspacePaths", "workspace_roots"]
                .iter()
                .find_map(|k| map.get(*k)?.as_array()?.first()?.as_str().filter(|s| !s.is_empty()).map(str::to_owned))
        });
        if let Some(cwd) = cwd {
            map.insert("cwd".into(), Value::String(cwd));
        }
    }
}

/// The event once the payload is known: a stop that reports an error (Cursor's `status: "error"`) is a failure.
pub fn refine(event: &str, map: &Map<String, Value>) -> String {
    if event == "Stop" && map.get("status").and_then(Value::as_str) == Some("error") {
        return "StopFailure".into();
    }
    event.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn obj(v: &Value) -> Map<String, Value> {
        v.as_object().cloned().unwrap_or_default()
    }

    #[test]
    fn every_agents_event_names_map_onto_claude_codes() {
        for (raw, canonical) in [
            ("BeforeTool", "PreToolUse"),
            ("AfterTool", "PostToolUse"),
            ("BeforeAgent", "UserPromptSubmit"),
            ("AfterAgent", "Stop"),
            ("startup", "SessionStart"),
            ("exit", "SessionEnd"),
            ("PreInvocation", "UserPromptSubmit"),
            ("PostInvocation", "PostToolUse"),
            ("agentStop", "Stop"),
            ("permissionRequest", "PermissionRequest"),
            ("beforeSubmitPrompt", "UserPromptSubmit"),
        ] {
            assert_eq!(event(raw), canonical, "{raw}");
        }
        for same in ["PreToolUse", "PermissionRequest", "Stop", "Interrupt", "somethingNew"] {
            assert_eq!(event(same), same);
        }
    }

    #[test]
    fn antigravity_and_gemini_tool_calls_become_tool_name_and_input() {
        let mut m = obj(&json!({
            "conversationId": "conv-1",
            "workspacePaths": ["C:\\Projects\\test"],
            "toolCall": { "name": "run_command", "args": { "CommandLine": "cargo check" } }
        }));
        fields(&mut m, &|_| None);
        assert_eq!(m["tool_name"], "run_command");
        assert_eq!(m["tool_input"]["command"], "cargo check");
        assert_eq!(m["session_id"], "conv-1");
        assert_eq!(m["cwd"], "C:\\Projects\\test");
    }

    #[test]
    fn copilot_arguments_may_arrive_as_json_text() {
        let mut m = obj(&json!({"toolName": "bash", "toolArgs": "{\"command\":\"npm test\"}", "workdir": "/app"}));
        fields(&mut m, &|_| None);
        assert_eq!(m["tool_name"], "bash");
        assert_eq!(m["tool_input"]["command"], "npm test");
        assert_eq!(m["cwd"], "/app");
    }

    #[test]
    fn what_the_payload_already_says_is_never_overwritten() {
        let mut m = obj(&json!({
            "tool_name": "Bash", "tool_input": {"command": "ls"}, "session_id": "keep", "cwd": "/keep",
            "toolName": "other", "conversationId": "no", "workdir": "/no"
        }));
        fields(&mut m, &|_| Some("env".into()));
        assert_eq!(
            (m["tool_name"].as_str(), m["session_id"].as_str(), m["cwd"].as_str()),
            (Some("Bash"), Some("keep"), Some("/keep"))
        );
    }

    #[test]
    fn gemini_session_comes_from_the_environment_last() {
        let mut m = obj(&json!({}));
        fields(&mut m, &|k| (k == "GEMINI_SESSION_ID").then(|| "g-7".to_owned()));
        assert_eq!(m["session_id"], "g-7");
    }

    #[test]
    fn a_stop_with_an_error_status_is_a_failure() {
        assert_eq!(refine("Stop", &obj(&json!({"status": "error"}))), "StopFailure");
        assert_eq!(refine("Stop", &obj(&json!({"status": "completed"}))), "Stop");
        assert_eq!(refine("PreToolUse", &obj(&json!({"status": "error"}))), "PreToolUse");
    }
}
