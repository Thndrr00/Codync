//! The subagents an agent started in one turn: one `subagents` entry per turn, which updates in
//! place while they work.
//!
//! A managed agent reaches the host over ACP, which reports a subagent as the tool call that started
//! it: Claude Code's `Task` and `Agent` calls, Codex's `spawnAgent`, and the calls of Codync's own
//! team tools (`ask_bot`, `delegate_batch`, `run_pipeline`). A terminal session says so with Claude
//! Code's `SubagentStart` and `SubagentStop` hooks too. Neither reports the work inside a subagent,
//! so a card shows its task, its state, the newest step the tool call named, and its time.
//!
//! The entry's data is what every client renders:
//! `{ "subagents": [{ "id", "task", "state", "step", "startedAt", "endedAt" }], "author", "text" }`.
//! `state` is `running`, `done`, `failed` or `stopped`; times are epoch milliseconds.

use serde_json::{Value, json};

/// The longest task or step a card keeps: the chat shows a line of it.
const MAX_TEXT: usize = 200;

/// How one subagent is doing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Running,
    Done,
    Failed,
    Stopped,
}

impl State {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Done => "done",
            Self::Failed => "failed",
            Self::Stopped => "stopped",
        }
    }
}

#[derive(Clone, Debug)]
struct Item {
    id: String,
    task: String,
    state: State,
    step: String,
    started_at: i64,
    ended_at: Option<i64>,
}

/// The subagents of one turn, and the entry that holds them.
#[derive(Debug, Default)]
pub struct Card {
    entry: Option<String>,
    items: Vec<Item>,
}

impl Card {
    /// The tool call is one of the agent's subagents: its task is what the call asked for.
    /// Returns false for every other tool.
    pub fn start_with_tool(&mut self, id: &str, title: &str, input: &Value, now: i64) -> bool {
        match task_of(title, input) {
            Some(task) => self.start(id, &task, now),
            None => false,
        }
    }

    /// Starts one subagent under its call's id; a call already counted changes nothing.
    pub fn start(&mut self, id: &str, task: &str, now: i64) -> bool {
        if self.items.iter().any(|item| item.id == id) {
            return false;
        }
        self.items.push(Item {
            id: id.to_owned(),
            task: clipped(task),
            state: State::Running,
            step: String::new(),
            started_at: now,
            ended_at: None,
        });
        true
    }

    pub fn has(&self, id: &str) -> bool {
        self.items.iter().any(|item| item.id == id)
    }

    /// What the subagent does now: the newest step its call named. False when it is ended.
    pub fn step(&mut self, id: &str, step: &str) -> bool {
        let step = clipped(step);
        let Some(item) = self.items.iter_mut().find(|item| item.id == id && item.state == State::Running) else {
            return false;
        };
        if step.is_empty() || item.step == step || item.task.eq_ignore_ascii_case(&step) {
            return false;
        }
        item.step = step;
        true
    }

    /// Ends a subagent once: a later report keeps how it first ended.
    pub fn finish(&mut self, id: &str, state: State, now: i64) -> bool {
        let Some(item) = self.items.iter_mut().find(|item| item.id == id && item.state == State::Running) else {
            return false;
        };
        item.state = state;
        item.ended_at = Some(now);
        true
    }

    /// Ends everything still running, as when the turn that started them ends.
    pub fn stop_running(&mut self, now: i64) -> bool {
        let mut changed = false;
        for item in &mut self.items {
            if item.state == State::Running {
                item.state = State::Stopped;
                item.ended_at = Some(now);
                changed = true;
            }
        }
        changed
    }

    pub fn running(&self) -> bool {
        self.items.iter().any(|item| item.state == State::Running)
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The entry holding this card, once it is saved.
    pub fn entry(&self) -> Option<&str> {
        self.entry.as_deref()
    }

    pub fn set_entry(&mut self, id: String) {
        self.entry = Some(id);
    }

    /// The entry's data: the whole list each time.
    pub fn data(&self) -> Value {
        let subagents: Vec<Value> = self
            .items
            .iter()
            .map(|item| {
                json!({
                    "id": item.id,
                    "task": item.task,
                    "state": item.state.as_str(),
                    "step": item.step,
                    "startedAt": item.started_at,
                    "endedAt": item.ended_at,
                })
            })
            .collect();
        let tasks: Vec<&str> = self.items.iter().map(|item| item.task.as_str()).collect();
        let text = if tasks.is_empty() { String::new() } else { format!("Subagents: {}", tasks.join(", ")) };
        json!({"subagents": subagents, "text": text})
    }
}

/// The task of a tool call that started a subagent, or `None` for every other call.
pub fn task_of(title: &str, input: &Value) -> Option<String> {
    let name = tool_name(title);
    let field = |key: &str| input[key].as_str().map(str::trim).filter(|value| !value.is_empty());
    let first_line = |key: &str| field(key).and_then(|text| text.lines().next()).map(clipped);
    match name.as_str() {
        "task" | "agent" | "spawnagent" | "spawn_agent" => first_line("description")
            .or_else(|| first_line("prompt"))
            .or_else(|| first_line("message"))
            // Some adapters name the call without its arguments: the title still says it is one.
            .or_else(|| Some(clipped(title))),
        "ask_bot" => first_line("message"),
        "delegate_batch" => goals(&input["tasks"]),
        "run_pipeline" => goals(&input["stages"]),
        _ => None,
    }
}

/// The first lines of the goals of a batch or a pipeline, at most two of them.
fn goals(list: &Value) -> Option<String> {
    let goals: Vec<String> = list
        .as_array()?
        .iter()
        .filter_map(|task| task["goal"].as_str())
        .filter_map(|goal| goal.lines().next())
        .map(str::trim)
        .filter(|goal| !goal.is_empty())
        .map(clipped)
        .collect();
    match goals.len() {
        0 => None,
        1 => goals.into_iter().next(),
        count => Some(format!("{} (+{})", goals[0], count - 1)),
    }
}

/// A tool title as the name the host knows: `mcp__team__delegate_batch` and
/// `team.delegate_batch` are both `delegate_batch`.
fn tool_name(title: &str) -> String {
    let name = title.rsplit("__").next().unwrap_or(title);
    let name = name.rsplit('.').next().unwrap_or(name);
    name.trim().to_ascii_lowercase()
}

/// One line, cut to what a card shows.
fn clipped(text: &str) -> String {
    let one_line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    one_line.chars().take(MAX_TEXT).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_subagent_tool_call_names_its_task() {
        let claude = json!({"description": "Review the upload API", "prompt": "Go read it."});
        assert_eq!(task_of("Task", &claude).as_deref(), Some("Review the upload API"));
        assert_eq!(task_of("Agent", &claude).as_deref(), Some("Review the upload API"));
        let codex = json!({"prompt": "Find the flaky test\nand fix it", "receiverThreadIds": ["t1"]});
        assert_eq!(task_of("spawnAgent", &codex).as_deref(), Some("Find the flaky test"));
        let team = json!({"tasks": [{"botId": "a", "goal": "Fix the login"}, {"botId": "b", "goal": "Ship it"}]});
        assert_eq!(task_of("mcp__team__delegate_batch", &team).as_deref(), Some("Fix the login (+1)"));
        assert_eq!(task_of("team.run_pipeline", &json!({"stages": [{"goal": "Audit"}]})).as_deref(), Some("Audit"));
        assert_eq!(task_of("ask_bot", &json!({"message": "Is it green?"})).as_deref(), Some("Is it green?"));
    }

    #[test]
    fn other_tools_are_not_subagents() {
        for title in ["Read", "Bash", "mcp__chat__send_message", "WebFetch"] {
            assert!(task_of(title, &json!({"command": "ls"})).is_none(), "{title}");
        }
    }

    #[test]
    fn a_card_lists_its_subagents_and_ends_them_once() {
        let mut card = Card::default();
        assert!(card.start_with_tool("t1", "Task", &json!({"description": "Read the tests"}), 1_000));
        assert!(!card.start("t1", "Read the tests", 2_000), "a call is counted once");
        assert!(card.start("t2", "Fix them", 1_500));
        assert!(card.running());
        assert!(card.step("t1", "Read src/lib.rs"));
        assert!(!card.step("t1", "Read src/lib.rs"), "the same step changes nothing");
        assert!(card.finish("t1", State::Done, 3_000));
        assert!(!card.finish("t1", State::Failed, 4_000), "how it first ended stands");
        let data = card.data();
        assert_eq!(data["subagents"][0]["id"], "t1");
        assert_eq!(data["subagents"][0]["task"], "Read the tests");
        assert_eq!(data["subagents"][0]["state"], "done");
        assert_eq!(data["subagents"][0]["step"], "Read src/lib.rs");
        assert_eq!(data["subagents"][0]["startedAt"], 1_000);
        assert_eq!(data["subagents"][0]["endedAt"], 3_000);
        assert_eq!(data["subagents"][1]["state"], "running");
        assert_eq!(data["text"], "Subagents: Read the tests, Fix them");
        assert!(card.stop_running(5_000));
        assert!(!card.running());
        assert_eq!(card.data()["subagents"][1]["state"], "stopped");
    }

    #[test]
    fn a_card_with_no_subagent_has_no_task() {
        let card = Card::default();
        assert!(card.is_empty());
        assert_eq!(card.data()["text"], "");
        assert!(card.data()["subagents"].as_array().is_some_and(Vec::is_empty));
    }
}
