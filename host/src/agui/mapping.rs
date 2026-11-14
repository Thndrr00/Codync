//! Codync transcript entries as AG-UI events: streamed text, tool calls, approvals, notices, plans, and the run's end.

use super::events;
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};

/// Turns the entries the bot writes during a turn into the AG-UI events a client follows.
pub struct Mapper {
    thread_id: String,
    run_id: String,
    /// Text sent per entry: the message id is the entry id.
    text: HashMap<String, usize>,
    open: Option<String>,
    /// Tool calls whose result has already been reported.
    tools: HashSet<String>,
    /// The status last reported for an approval card.
    approvals: HashMap<String, String>,
    notices: HashSet<String>,
    plans: HashSet<String>,
}

impl Mapper {
    pub fn new(thread_id: &str, run_id: &str) -> Self {
        Self {
            thread_id: thread_id.to_owned(),
            run_id: run_id.to_owned(),
            text: HashMap::new(),
            open: None,
            tools: HashSet::new(),
            approvals: HashMap::new(),
            notices: HashSet::new(),
            plans: HashSet::new(),
        }
    }

    /// A whole state document, sent once at the start. Codync has no view state of its own yet (K4): the
    /// client's own state comes back untouched, and a change would follow as a `STATE_DELTA` patch.
    pub fn state_snapshot(&self, state: &Value) -> Value {
        events::event(events::STATE_SNAPSHOT, json!({"snapshot": state}))
    }

    /// One `entry` event's entry, as the events it means to an AG-UI client.
    pub fn entry(&mut self, entry: &Value) -> Vec<Value> {
        match entry["kind"].as_str().unwrap_or_default() {
            "agent" => self.text_entry(entry),
            "tool" => self.tool_entry(entry),
            "permission" => self.approval_entry(entry),
            "notice" => self.notice_entry(entry),
            "plan" => self.plan_entry(entry),
            _ => Vec::new(),
        }
    }

    /// The turn's result (`turnResult` on the user's message): closes what is open and ends the run.
    pub fn finish(&mut self, result: &Value) -> Vec<Value> {
        let mut out = self.close_text();
        let state = result["state"].as_str().unwrap_or("done");
        let text = result["text"].as_str().filter(|text| !text.trim().is_empty());
        if matches!(state, "failed" | "interrupted" | "waitingForSignIn") {
            out.push(events::run_error(failure_message(state, text), state));
        } else {
            out.push(events::run_finished(&self.thread_id, &self.run_id, text));
        }
        out
    }

    /// Closes an open stream when the client disappeared or the host stopped the turn.
    pub fn stopped(&mut self) -> Vec<Value> {
        let mut out = self.close_text();
        out.push(events::run_finished(&self.thread_id, &self.run_id, None));
        out
    }

    fn text_entry(&mut self, entry: &Value) -> Vec<Value> {
        let Ok(message_id) = id(entry) else { return Vec::new() };
        let text = entry["data"]["text"].as_str().unwrap_or_default();
        let final_message = entry["data"]["final"] == true;
        if text.is_empty() && !final_message {
            return Vec::new();
        }
        let sent = *self.text.get(message_id).unwrap_or(&0);
        let mut out = Vec::new();
        if sent == 0 {
            self.close_open(&mut out);
            self.open = Some(message_id.to_owned());
            out.push(events::text_start(message_id));
        }
        // Entries carry the whole segment so far; a client receives the part it hasn't seen.
        if text.len() > sent {
            let from = if text.is_char_boundary(sent) { sent } else { 0 };
            for chunk in events::chunks(&text[from..]) {
                out.push(events::text_content(message_id, &chunk));
            }
        }
        self.text.insert(message_id.to_owned(), text.len());
        if final_message && self.open.as_deref() == Some(message_id) {
            out.push(events::text_end(message_id));
            self.open = None;
        }
        out
    }

    fn tool_entry(&mut self, entry: &Value) -> Vec<Value> {
        let Ok(entry_id) = id(entry) else { return Vec::new() };
        let tool_call_id = entry["data"]["toolCallId"].as_str().filter(|id| !id.is_empty()).unwrap_or(entry_id);
        let title = entry["data"]["title"].as_str().unwrap_or("Tool");
        let status = entry["data"]["status"].as_str().unwrap_or("pending");
        let mut out = Vec::new();
        if self.tools.insert(format!("start:{tool_call_id}")) {
            out.push(events::event(
                events::TOOL_CALL_START,
                json!({"toolCallId": tool_call_id, "toolCallName": title}),
            ));
            let args = tool_arguments(&entry["data"]);
            out.push(events::event(
                events::TOOL_CALL_ARGS,
                json!({"toolCallId": tool_call_id, "delta": events::payload(&args.to_string())}),
            ));
            out.push(events::event(events::TOOL_CALL_END, json!({"toolCallId": tool_call_id})));
        }
        if matches!(status, "completed" | "failed") && self.tools.insert(format!("result:{tool_call_id}")) {
            let output = entry["data"]["output"].as_str().unwrap_or_default();
            let content = if output.trim().is_empty() {
                if status == "failed" { "The tool failed.".to_owned() } else { "Done.".to_owned() }
            } else {
                events::payload(output)
            };
            out.push(events::event(
                events::TOOL_CALL_RESULT,
                json!({
                    "messageId": entry_id,
                    "toolCallId": tool_call_id,
                    "content": content,
                    "role": "tool",
                }),
            ));
        }
        out
    }

    fn approval_entry(&mut self, entry: &Value) -> Vec<Value> {
        let Ok(entry_id) = id(entry) else { return Vec::new() };
        let status = entry["data"]["status"].as_str().unwrap_or("pending").to_owned();
        if self.approvals.get(entry_id) == Some(&status) {
            return Vec::new();
        }
        self.approvals.insert(entry_id.to_owned(), status.clone());
        vec![events::custom(
            events::APPROVAL,
            json!({
                "entryId": entry_id,
                "botId": entry["botId"],
                "title": entry["data"]["title"],
                "detail": entry["data"]["detail"],
                "toolKind": entry["data"]["toolKind"],
                "options": entry["data"]["options"],
                "selected": entry["data"]["selected"],
                "status": status,
                // How a client answers, through the host's existing permission method.
                "answer": {
                    "method": "respondPermission",
                    "params": {"botId": entry["botId"], "entryId": entry_id},
                },
            }),
        )]
    }

    fn notice_entry(&mut self, entry: &Value) -> Vec<Value> {
        let Ok(entry_id) = id(entry) else { return Vec::new() };
        if !self.notices.insert(entry_id.to_owned()) {
            return Vec::new();
        }
        vec![events::custom(
            events::NOTICE,
            json!({
                "entryId": entry_id,
                "text": entry["data"]["text"],
                "style": entry["data"]["style"],
            }),
        )]
    }

    fn plan_entry(&mut self, entry: &Value) -> Vec<Value> {
        let Ok(entry_id) = id(entry) else { return Vec::new() };
        if !self.plans.insert(entry_id.to_owned()) {
            return Vec::new();
        }
        vec![events::custom(events::PLAN, json!({"entryId": entry_id, "entries": entry["data"]["entries"]}))]
    }

    fn close_text(&mut self) -> Vec<Value> {
        let mut out = Vec::new();
        self.close_open(&mut out);
        out
    }

    fn close_open(&mut self, out: &mut Vec<Value>) {
        if let Some(message_id) = self.open.take() {
            out.push(events::text_end(&message_id));
        }
    }
}

fn id(entry: &Value) -> Result<&str, ()> {
    entry["id"].as_str().filter(|id| !id.is_empty()).ok_or(())
}

/// What the agent passed the tool: its `rawInput` when the entry kept one, else the command and paths it did keep.
fn tool_arguments(data: &Value) -> Value {
    if data["rawInput"].is_object() {
        return data["rawInput"].clone();
    }
    let mut args = serde_json::Map::new();
    if data["command"].is_string() {
        args.insert("command".into(), data["command"].clone());
    }
    if data["locations"].as_array().is_some_and(|locations| !locations.is_empty()) {
        args.insert("locations".into(), data["locations"].clone());
    }
    Value::Object(args)
}

fn failure_message(state: &str, text: Option<&str>) -> String {
    if let Some(text) = text {
        return text.to_owned();
    }
    match state {
        "interrupted" => "The host stopped before the turn finished.".into(),
        "waitingForSignIn" => "The agent needs a sign-in. Open the bot in Codync to sign in.".into(),
        _ => "The agent didn't finish.".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, kind: &str, data: Value) -> Value {
        json!({"id": id, "botId": "b", "kind": kind, "turn": 1, "data": data})
    }

    fn kinds(events: &[Value]) -> Vec<&str> {
        events.iter().filter_map(|event| event["type"].as_str()).collect()
    }

    #[test]
    fn streamed_text_becomes_start_delta_end() {
        let mut mapper = Mapper::new("t", "r");
        assert!(mapper.entry(&entry("e1", "agent", json!({"text": "", "final": false}))).is_empty());
        let first = mapper.entry(&entry("e1", "agent", json!({"text": "Hola", "final": false})));
        assert_eq!(kinds(&first), ["TEXT_MESSAGE_START", "TEXT_MESSAGE_CONTENT"]);
        assert_eq!(first[1]["delta"], "Hola");
        let second = mapper.entry(&entry("e1", "agent", json!({"text": "Hola, mundo", "final": true})));
        assert_eq!(kinds(&second), ["TEXT_MESSAGE_CONTENT", "TEXT_MESSAGE_END"]);
        assert_eq!(second[0]["delta"], ", mundo");
        assert_eq!(second[0]["messageId"], "e1");
    }

    #[test]
    fn a_new_text_segment_closes_the_open_one() {
        let mut mapper = Mapper::new("t", "r");
        mapper.entry(&entry("e1", "agent", json!({"text": "one", "final": false})));
        let next = mapper.entry(&entry("e2", "agent", json!({"text": "two", "final": true})));
        assert_eq!(
            kinds(&next),
            ["TEXT_MESSAGE_END", "TEXT_MESSAGE_START", "TEXT_MESSAGE_CONTENT", "TEXT_MESSAGE_END"]
        );
        assert_eq!(next[0]["messageId"], "e1");
        assert_eq!(next[1]["messageId"], "e2");
    }

    #[test]
    fn a_tool_call_reports_its_start_and_its_result_once() {
        let mut mapper = Mapper::new("t", "r");
        let started = mapper.entry(&entry(
            "e1",
            "tool",
            json!({"toolCallId": "call", "title": "Read file", "status": "in_progress", "command": "ls"}),
        ));
        assert_eq!(kinds(&started), ["TOOL_CALL_START", "TOOL_CALL_ARGS", "TOOL_CALL_END"]);
        assert_eq!(started[0]["toolCallName"], "Read file");
        assert_eq!(started[1]["delta"], "{\"command\":\"ls\"}");
        let done = mapper.entry(&entry(
            "e1",
            "tool",
            json!({"toolCallId": "call", "title": "Read file", "status": "completed", "output": "contents"}),
        ));
        assert_eq!(kinds(&done), ["TOOL_CALL_RESULT"]);
        assert_eq!(done[0]["content"], "contents");
        assert_eq!(done[0]["role"], "tool");
        assert!(mapper.entry(&entry("e1", "tool", json!({"toolCallId": "call", "status": "completed"}))).is_empty());
    }

    #[test]
    fn an_approval_carries_its_options_and_how_to_answer() {
        let mut mapper = Mapper::new("t", "r");
        let card = mapper.entry(&entry(
            "e1",
            "permission",
            json!({"title": "Run tests", "status": "pending", "options": [
                {"optionId": "allow", "name": "Allow", "kind": "allow_once"},
                {"optionId": "deny", "name": "Deny", "kind": "reject_once"},
            ]}),
        ));
        assert_eq!(kinds(&card), ["CUSTOM"]);
        assert_eq!(card[0]["name"], "codync.approval");
        assert_eq!(card[0]["value"]["options"][0]["optionId"], "allow");
        assert_eq!(card[0]["value"]["answer"]["method"], "respondPermission");
        assert_eq!(card[0]["value"]["answer"]["params"]["entryId"], "e1");
        let answered = mapper.entry(&entry("e1", "permission", json!({"status": "answered", "selected": "allow"})));
        assert_eq!(answered[0]["value"]["status"], "answered");
        assert_eq!(answered[0]["value"]["selected"], "allow");
        assert!(mapper.entry(&entry("e1", "permission", json!({"status": "answered"}))).is_empty());
    }

    #[test]
    fn a_finished_turn_carries_the_final_text() {
        let mut mapper = Mapper::new("t", "r");
        mapper.entry(&entry("e1", "agent", json!({"text": "done", "final": false})));
        let out = mapper.finish(&json!({"state": "done", "text": "done", "stopReason": "end_turn"}));
        assert_eq!(kinds(&out), ["TEXT_MESSAGE_END", "RUN_FINISHED"]);
        assert_eq!(out[1]["result"], "done");
        assert_eq!(out[1]["threadId"], "t");
        assert_eq!(out[1]["runId"], "r");
    }

    #[test]
    fn a_failed_turn_is_a_run_error() {
        let mut mapper = Mapper::new("t", "r");
        let out = mapper.finish(&json!({"state": "failed", "text": null, "stopReason": "error"}));
        assert_eq!(kinds(&out), ["RUN_ERROR"]);
        assert_eq!(out[0]["code"], "failed");
        assert_eq!(out[0]["message"], "The agent didn't finish.");

        let interrupted = Mapper::new("t", "r").finish(&json!({"state": "interrupted"}));
        assert_eq!(interrupted[0]["type"], "RUN_ERROR");
        assert!(interrupted[0]["message"].as_str().unwrap().contains("stopped"));
    }

    #[test]
    fn a_stopped_run_closes_what_is_open_and_finishes() {
        let mut mapper = Mapper::new("t", "r");
        mapper.entry(&entry("e1", "agent", json!({"text": "half", "final": false})));
        let out = mapper.stopped();
        assert_eq!(kinds(&out), ["TEXT_MESSAGE_END", "RUN_FINISHED"]);
        assert!(out[1]["result"].is_null());
    }
}
