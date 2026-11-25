//! Subagents as a tool a bot can call: one bounded task runs in a short-lived child turn on the
//! bot's own harness (or a chosen agent), and the child's final reply comes back as the tool result.
//!
//! Ported from OpenBot nightly's subagent delegation: "When you ask an agent to use subagents, it
//! starts its own subagents and does not send the work to your other agents" (`CHANGELOG.md`).
//! Adapted to Codync's terms: the child is a throwaway ACP session on the parent's agent backend,
//! never one of the user's other bots. It reuses `chat::oneshot`, the host's existing throwaway-turn
//! runner, instead of a second agent loop. The child inherits the parent's browser/chat/own tools
//! so it can work on part of a task, but not the team tools (`delegate`, `message_bot`).

use crate::analytics::{self, Event};
use crate::chat::oneshot::{self, one_shot_with_servers};
use crate::hub::Hub;
use crate::store::BotConfig;
use anyhow::{Result, bail};
use serde_json::{Map, Value, json};
use std::sync::Arc;

/// Mirrors OpenBot's bounds: a child can be at most two levels below the bot, and a single turn may
/// start at most four children. Both are host-side caps, not instructions the bot can talk past.
pub const MAX_DEPTH: u8 = 2;
pub const MAX_CHILDREN_PER_TURN: usize = 4;

/// The tool name, shared by the schema and `chat::team`'s call dispatch.
pub const TOOL: &str = "delegate";

/// A task longer than this is refused before a child process starts.
const MAX_TASK_BYTES: usize = 32_000;

/// The MCP server whose tools are never handed to a child: it carries `delegate` and `message_bot`.
const TEAM_SERVER: &str = "team";

/// The MCP servers a helper child gets: the parent's browser/chat/own servers plus the child's own,
/// minus the team server. Names are deduped, and the parent's declaration wins. Pure so the
/// inheritance and the exclusion are tested without a host or a store (OpenBot 15942).
pub(crate) fn helper_tools(parent: &Value, child: &Value) -> Value {
    let mut kept: Vec<Value> = Vec::new();
    let mut names: Vec<String> = Vec::new();
    for server in parent.as_array().into_iter().flatten().chain(child.as_array().into_iter().flatten()) {
        let Some(name) = server["name"].as_str() else { continue };
        if name == TEAM_SERVER || names.iter().any(|known| known == name) {
            continue;
        }
        names.push(name.to_owned());
        kept.push(server.clone());
    }
    Value::Array(kept)
}

/// How one delegation attempt ended. Only these fixed values reach analytics; never the task text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Ok,
    DepthLimit,
    Cap,
    Failure,
}

impl Outcome {
    /// The fixed `outcome` property of `subagent_delegated`.
    pub fn name(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::DepthLimit => "depth_limit",
            Self::Cap => "cap",
            Self::Failure => "failure",
        }
    }

    /// The plain text a bot reads as the tool result when a child cannot run. Says which cap
    /// stopped it, with no task text echoed back.
    pub fn refusal(self) -> &'static str {
        match self {
            Self::Ok => "",
            Self::DepthLimit => "Subagent depth limit reached: this child cannot start another subagent.",
            Self::Cap => "Subagent limit reached: this turn has already started the maximum number of children.",
            Self::Failure => "The subagent could not finish; partial work may have happened. Do not blindly retry.",
        }
    }
}

/// One turn's delegation budget. Pure so both caps can be tested with no host, store or agent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Budget {
    pub children: usize,
}

impl Budget {
    /// Admits one child, or says which cap stopped it. Depth is checked first so a turn already
    /// too deep never spends a child slot; the cap counts children started in this turn.
    pub fn admit(&mut self, depth: u8) -> Outcome {
        if depth >= MAX_DEPTH {
            return Outcome::DepthLimit;
        }
        if self.children >= MAX_CHILDREN_PER_TURN {
            return Outcome::Cap;
        }
        self.children += 1;
        Outcome::Ok
    }
}

/// The `delegate` tool's schema. Listed with the other team tools; `call` refuses it unless the
/// bot's owner turned subagents on in the bot's settings.
pub fn tool() -> Value {
    json!({
        "name": TOOL,
        "description": "Hand one bounded task to a short-lived child turn on your own agent (or a chosen one) and wait for its final reply. The child works in a fresh session with your browser, chat and own tools, but no team tools: it cannot message the user, message other bots or start further subagents, and its work spends your tokens. At most 2 levels of children and 4 children per turn. Different from message_bot: this is your own subagent, not one of the user's bots.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "task": {"type": "string", "description": "Self-contained task and expected result; the child has no access to this chat."},
                "agent": {"type": "string", "description": "Agent backend id to run the child on; defaults to your own."},
                "depth": {"type": "integer", "minimum": 0, "maximum": MAX_DEPTH, "description": "Internal: nesting level reported by a calling subagent. Leave unset in a normal turn."},
            },
            "required": ["task"],
        },
        "annotations": {"readOnlyHint": false, "idempotentHint": false},
    })
}

/// Reports one attempt with only fixed properties: the outcome and the agent id, never the task.
fn report(hub: &Hub, outcome: Outcome, agent: &str) {
    let mut properties = Map::new();
    properties.insert("outcome".into(), outcome.name().into());
    properties.insert("agent".into(), agent.into());
    analytics::capture(hub, Event::SubagentDelegated, properties);
}

/// The child's fixed instructions. It has the parent's browser/chat/own tools (see `helper_tools`)
/// but no team tools, so these are a reminder of its shape, not the only boundary on delegation.
fn system_prompt(depth: u8) -> String {
    format!(
        "[Codync: you are a subagent, a short-lived child turn of another bot, at depth {} of {}. \
         Do the single task below and put the complete result in your final reply. You have your \
         parent's browser, chat and own tools, but no team tools: do not message the user, do not \
         message other bots, and do not try to start a further subagent. Your final reply is returned \
         to the bot that asked you.]",
        depth + 1,
        MAX_DEPTH
    )
}

/// Starts one child turn and waits for its final answer, returning it as the tool result.
pub async fn call(hub: &Arc<Hub>, source: &BotConfig, args: &Value) -> Result<Value> {
    if !source.subagents {
        bail!("subagents are off for this bot; the owner turns them on in the bot's settings");
    }
    let task = args["task"].as_str().unwrap_or_default().trim();
    if task.is_empty() || task.len() > MAX_TASK_BYTES {
        bail!("task must be nonempty and at most {MAX_TASK_BYTES} bytes");
    }
    let depth = args["depth"].as_u64().unwrap_or_default().min(u64::from(MAX_DEPTH));
    let depth = u8::try_from(depth).unwrap_or(MAX_DEPTH);
    let agent = args["agent"].as_str().filter(|agent| !agent.trim().is_empty()).unwrap_or(&source.backend).to_owned();
    let parent_servers = oneshot::bot_servers(hub, source)?;
    let servers = helper_tools(&parent_servers, &json!([]));

    let mut budget = hub.team.delegation(&source.id);
    let admitted = budget.admit(depth);
    hub.team.set_delegation(&source.id, budget);
    if admitted != Outcome::Ok {
        report(hub, admitted, &agent);
        bail!("{}", admitted.refusal());
    }

    let mut child = source.clone();
    child.backend = agent.clone();
    let system = system_prompt(depth);
    let user = format!("Subagent task:\n{task}");
    match one_shot_with_servers(hub, &child, &system, &user, servers).await {
        Ok(reply) => {
            report(hub, Outcome::Ok, &agent);
            Ok(json!({"reply": reply, "agent": agent, "depth": depth + 1}))
        }
        Err(error) => {
            tracing::warn!(%error, agent = %agent, "subagent turn failed");
            report(hub, Outcome::Failure, &agent);
            bail!("{}", Outcome::Failure.refusal());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_turn_can_start_at_most_four_children() {
        let mut budget = Budget::default();
        for _ in 0..MAX_CHILDREN_PER_TURN {
            assert_eq!(budget.admit(0), Outcome::Ok);
        }
        assert_eq!(budget.admit(0), Outcome::Cap);
        assert_eq!(budget.children, MAX_CHILDREN_PER_TURN);
        assert_eq!(
            Outcome::Cap.refusal(),
            "Subagent limit reached: this turn has already started the maximum number of children."
        );
    }

    #[test]
    fn depth_is_refused_before_a_child_slot_is_spent() {
        let mut budget = Budget::default();
        assert_eq!(budget.admit(MAX_DEPTH), Outcome::DepthLimit);
        assert_eq!(budget.children, 0, "a refused child does not use a slot");
        assert_eq!(
            Outcome::DepthLimit.refusal(),
            "Subagent depth limit reached: this child cannot start another subagent."
        );
        assert_eq!(budget.admit(MAX_DEPTH - 1), Outcome::Ok);
        assert_eq!(budget.children, 1);
    }

    #[test]
    fn outcome_names_are_fixed_and_never_echo_a_task() {
        assert_eq!(Outcome::Ok.name(), "ok");
        assert_eq!(Outcome::DepthLimit.name(), "depth_limit");
        assert_eq!(Outcome::Cap.name(), "cap");
        assert_eq!(Outcome::Failure.name(), "failure");
        for outcome in [Outcome::Ok, Outcome::DepthLimit, Outcome::Cap, Outcome::Failure] {
            assert!(!outcome.refusal().contains("secret task text"));
            assert!(outcome.name().chars().all(|c| c.is_ascii_lowercase() || c == '_'));
        }
        assert!(Outcome::Failure.refusal().contains("partial work"));
    }

    #[test]
    fn the_tool_schema_names_the_tool_and_its_bounded_arguments() {
        let schema = tool();
        assert_eq!(schema["name"], TOOL);
        let text = schema.to_string();
        assert!(text.contains("at most 2 levels") && text.contains("4 children"));
        assert!(text.contains("task") && text.contains("agent") && text.contains("depth"));
        assert_eq!(schema["inputSchema"]["required"][0], "task");
    }

    #[test]
    fn a_helper_inherits_the_parents_tools_but_not_the_team_server() {
        let parent = json!([
            {"name": "chat", "command": "host"},
            {"name": "team", "command": "host"},
            {"name": "my-connector", "command": "npx"}
        ]);
        let tools = helper_tools(&parent, &json!([]));
        let names: Vec<&str> =
            tools.as_array().expect("an array").iter().map(|s| s["name"].as_str().unwrap_or_default()).collect();
        assert_eq!(names, ["chat", "my-connector"]);
        assert_eq!(tools[0]["command"], "chat", "the parent's declaration is kept");
        assert!(!tools.to_string().contains("team"), "delegate and message_bot stay out");
    }

    #[test]
    fn a_server_declared_by_both_sides_is_declared_once() {
        let parent = json!([{"name": "chat", "command": "parent"}]);
        let child = json!([{"name": "chat", "command": "child"}, {"name": "memory"}]);
        let tools = helper_tools(&parent, &child);
        let names: Vec<&str> =
            tools.as_array().expect("an array").iter().map(|s| s["name"].as_str().unwrap_or_default()).collect();
        assert_eq!(names, ["chat", "memory"]);
        assert_eq!(tools[0]["command"], "parent", "the parent's definition wins");
    }

    #[test]
    fn a_child_without_a_parent_stays_tool_free() {
        assert_eq!(helper_tools(&json!([]), &json!([])), json!([]));
        assert_eq!(helper_tools(&Value::Null, &Value::Null), json!([]), "bad input never declares tools");
        let system = system_prompt(0);
        assert!(system.contains("browser") && system.contains("own tools"));
        assert!(system.contains("do not message other bots") && system.contains("further subagent"));
    }
}
