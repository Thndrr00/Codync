//! One hook event as the host reads it: the canonical payload `codync-host hook` posts, trusted for nothing.

use serde_json::Value;

/// Longest text kept for a prompt or a message; the chat shows far less.
const MAX_TEXT: usize = 4_000;

pub struct Event {
    /// `""` for Claude Code, otherwise the `--agent` the hook was installed with.
    pub agent: String,
    /// Canonical (Claude Code's) event name.
    pub name: String,
    /// Identifies the terminal session across events.
    pub session: String,
    pub cwd: String,
    pub tool: String,
    pub input: Value,
    pub tool_use_id: String,
    pub prompt: String,
    /// The agent's last reply, on `Stop`.
    pub message: String,
    /// The hook had to cut an edit's text: its line counts cannot be trusted.
    pub truncated: bool,
}

fn text(payload: &Value, key: &str) -> String {
    payload.get(key).and_then(Value::as_str).map(|s| crate::agent::acp::truncate(s, MAX_TEXT)).unwrap_or_default()
}

/// An agent name is a short, plain tag: anything else is not one of ours.
fn agent_tag(raw: &str) -> Option<String> {
    let tag = raw.to_ascii_lowercase();
    let ok = tag.len() <= 24 && tag.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
    ok.then_some(tag)
}

impl Event {
    pub fn parse(body: &Value) -> Option<Self> {
        let payload = body.get("payload").filter(|p| p.is_object())?;
        let agent = agent_tag(body.get("agent").and_then(Value::as_str).unwrap_or_default())?;
        let name = text(payload, "hook_event_name");
        if name.is_empty() {
            return None;
        }
        let cwd = text(payload, "cwd");
        let id = text(payload, "session_id");
        let terminal = body["terminal"]["session"].as_str().unwrap_or_default();
        // Without a session id a terminal and a folder still tell sessions apart.
        let tail = if id.is_empty() { format!("{cwd}|{terminal}") } else { id };
        Some(Self {
            session: format!("{agent}:{tail}"),
            agent,
            name,
            cwd,
            tool: text(payload, "tool_name"),
            input: payload.get("tool_input").cloned().unwrap_or(Value::Null),
            tool_use_id: text(payload, "tool_use_id"),
            prompt: text(payload, "prompt"),
            message: text(payload, "last_assistant_message"),
            truncated: payload.get("diffTruncated").and_then(Value::as_bool).unwrap_or(false),
        })
    }

    /// The changed lines of an edit, unless the hook had to cut its text: counts from cut text would lie.
    pub fn diffs(&self) -> Vec<Value> {
        if self.truncated { Vec::new() } else { super::tool::diffs(&self.tool, &self.input) }
    }

    /// The folder's own name: all a roster row needs to tell sessions apart.
    pub fn project(&self) -> String {
        self.cwd.trim_end_matches(['/', '\\']).rsplit(['/', '\\']).next().unwrap_or_default().to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_canonical_payload_is_read() {
        let body = json!({"agent": "", "payload": {
            "hook_event_name": "PreToolUse", "session_id": "s1", "cwd": "/home/me/app/",
            "tool_name": "Bash", "tool_input": {"command": "ls"}, "tool_use_id": "t1"}});
        let e = Event::parse(&body).unwrap();
        assert_eq!((e.name.as_str(), e.session.as_str(), e.tool.as_str()), ("PreToolUse", ":s1", "Bash"));
        assert_eq!(e.project(), "app");
        assert_eq!(e.input["command"], "ls");
    }

    #[test]
    fn without_a_session_id_the_terminal_and_folder_tell_sessions_apart() {
        let body = |term: &str| {
            json!({"agent": "gemini", "terminal": {"session": term},
            "payload": {"hook_event_name": "Stop", "cwd": "C:\\work\\api"}})
        };
        let (a, b) = (Event::parse(&body("1")).unwrap(), Event::parse(&body("2")).unwrap());
        assert_ne!(a.session, b.session);
        assert_eq!(a.project(), "api");
    }

    #[test]
    fn what_is_not_a_hook_event_is_refused() {
        for body in [
            json!({}),
            json!({"payload": []}),
            json!({"payload": {}}),
            json!({"agent": "Evil Agent!", "payload": {"hook_event_name": "Stop"}}),
        ] {
            assert!(Event::parse(&body).is_none(), "{body}");
        }
    }
}
