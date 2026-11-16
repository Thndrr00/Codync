//! The variables a policy expression sees, and the redacted snapshot a client shows for them.
//!
//! Every variable is a string, and all of them are always there: an empty string means the request
//! doesn't have it (a tool with no command, a call that isn't about a file). That way `tool.name ==
//! "shell"` never fails on a request that isn't a tool call, and a typo in a field name still can't
//! read anything it shouldn't.
//!
//! The rules see the request as it is (`truth`). A client sees `snapshot`: the same groups, with
//! credentials masked out of a command, a page's query string dropped and the home folder shortened
//! to `~`. The snapshot is for people reading a simulation, never for deciding.

use crate::store::BotConfig;
use cel::{Context, Value as CelValue};
use serde_json::Value;
use std::collections::HashMap;

/// Longest value a snapshot shows.
const SNAPSHOT_TEXT: usize = 200;

/// What a rule is evaluated against: one proposed or real tool call.
#[derive(Clone, Debug, Default)]
pub struct Request {
    /// The agent's tool name (`shell`, `read_file`, a connector's tool…).
    pub tool: String,
    /// Its kind (`execute`, `edit`, `read`, `fetch`, `other`…); inferred from `tool` when empty.
    pub kind: String,
    /// What the call says it does: its title.
    pub intent: String,
    pub bot: String,
    pub bot_name: String,
    /// The person the bot belongs to; empty for the owner.
    pub actor: String,
    /// `owner`, `admin` or `member`.
    pub actor_role: String,
    pub command: String,
    pub file_path: String,
    pub file_op: String,
    pub mcp_server: String,
    pub mcp_tool: String,
    pub page_url: String,
    /// `person`, `routine`, `channel` or `network`: who set the turn off.
    pub initiator: String,
    /// The key a keyboard action presses.
    pub key: String,
}

impl Request {
    /// A tool call by name, its kind inferred the way the approval cards infer it.
    pub fn tool(name: &str) -> Self {
        Self {
            tool: name.to_owned(),
            kind: crate::agent::approval_preview::kind_of(name).to_owned(),
            intent: name.to_owned(),
            ..Self::default()
        }
    }

    /// An agent's permission request (`session/request_permission`'s `toolCall`) for `config`'s bot.
    ///
    /// An ACP call carries no tool name of its own: its title's first word stands in (`shell`, `Read`),
    /// and a connector tool named `mcp__server__tool` says which connector and tool it is.
    pub fn from_tool_call(
        call: &Value,
        title: &str,
        config: &BotConfig,
        command: Option<&str>,
        initiator: &str,
    ) -> Self {
        let named = call["name"].as_str().unwrap_or_default().trim();
        let tool = if named.is_empty() { first_word(title) } else { named.to_owned() };
        let raw = &call["rawInput"];
        let path = ["path", "file_path", "filePath", "target_file", "notebook_path"]
            .iter()
            .find_map(|key| raw[*key].as_str())
            .unwrap_or_default()
            .to_owned();
        let url = ["url", "uri", "href"].iter().find_map(|key| raw[*key].as_str()).unwrap_or_default().to_owned();
        let explicit_kind = call["kind"].as_str().unwrap_or_default().trim();
        let kind = if explicit_kind.is_empty() {
            crate::agent::approval_preview::kind_of(&tool).to_owned()
        } else {
            explicit_kind.to_owned()
        };
        let (mcp_server, mcp_tool) = connector(raw, &tool);
        Self {
            tool,
            kind,
            intent: title.to_owned(),
            bot: config.id.clone(),
            bot_name: config.name.clone(),
            command: command.unwrap_or_default().to_owned(),
            file_path: path,
            file_op: String::new(),
            mcp_server,
            mcp_tool,
            page_url: url,
            initiator: initiator.to_owned(),
            key: raw["key"].as_str().unwrap_or_default().to_owned(),
            ..Self::default()
        }
    }

    /// A request with every variable filled in: what a rule is tried against when it is saved, and what
    /// the editors show as an example.
    pub fn sample(initiator: &str) -> Self {
        Self {
            tool: "shell".into(),
            kind: "execute".into(),
            intent: "Run the tests".into(),
            bot: "bot_sample".into(),
            bot_name: "Sample bot".into(),
            actor: String::new(),
            actor_role: "owner".into(),
            command: "cargo test".into(),
            file_path: "~/project/src/main.rs".into(),
            file_op: "edit".into(),
            mcp_server: "github".into(),
            mcp_tool: "create_issue".into(),
            page_url: "https://example.com/project/issues".into(),
            initiator: initiator.to_owned(),
            key: "Enter".into(),
        }
    }

    /// The groups and fields, as the rules read them.
    pub fn truth(&self) -> Value {
        let file_op = if self.file_op.is_empty() && !self.file_path.is_empty() {
            self.kind.as_str()
        } else {
            self.file_op.as_str()
        };
        let host = self.page_url.split('/').nth(2).unwrap_or_default();
        serde_json::json!({
            "tool": {"name": self.tool, "kind": self.kind},
            "intent": self.intent,
            "bot": {"id": self.bot, "name": self.bot_name},
            "actor": {"id": self.actor, "role": self.actor_role},
            "command": self.command,
            "file": {"path": self.file_path, "op": file_op},
            "mcp": {"server": self.mcp_server, "tool": self.mcp_tool},
            "page": {"url": self.page_url, "host": host},
            "initiator": {"kind": self.initiator},
            "key": self.key,
        })
    }

    /// The same groups with secrets taken out, for the simulator to show.
    pub fn snapshot(&self) -> Value {
        let mut shown = self.truth();
        redact(&mut shown, "");
        shown
    }
}

fn redact(value: &mut Value, field: &str) {
    match value {
        Value::Object(map) => {
            for (key, inner) in map.iter_mut() {
                redact(inner, key);
            }
        }
        Value::String(text) => {
            let redacted = match field {
                "command" => mask_command(text),
                "url" => without_query(text),
                "path" => shortened_home(text),
                _ => text.clone(),
            };
            *text = cap(&redacted, SNAPSHOT_TEXT);
        }
        _ => {}
    }
}

/// A command with what names a credential masked: `--token abc` and `API_KEY=abc` become `[redacted]`.
fn mask_command(command: &str) -> String {
    const NAMES: [&str; 8] = ["password", "passwd", "token", "secret", "api_key", "apikey", "authorization", "bearer"];
    let names = |word: &str| {
        let lower = word.to_ascii_lowercase();
        NAMES.iter().any(|name| lower.contains(name))
    };
    let mut out: Vec<String> = Vec::new();
    let mut mask_next = false;
    for word in command.split_whitespace() {
        if mask_next {
            out.push("[redacted]".into());
            mask_next = false;
            continue;
        }
        if names(word) {
            match word.split_once('=') {
                Some((name, _)) => out.push(format!("{name}=[redacted]")),
                None => {
                    out.push(word.to_owned());
                    // `--password hunter2`: the value after a flag that names one.
                    mask_next = word.starts_with('-');
                }
            }
            continue;
        }
        out.push(word.to_owned());
    }
    out.join(" ")
}

/// A URL without its query string or fragment, which is where tokens travel.
fn without_query(url: &str) -> String {
    url.split(['?', '#']).next().unwrap_or_default().to_owned()
}

/// A call's title's first word, lowercased and stripped of punctuation: the tool's stand-in name.
fn first_word(title: &str) -> String {
    title
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .trim_matches(|c: char| !c.is_alphanumeric() && c != '_')
        .to_lowercase()
        .chars()
        .take(40)
        .collect()
}

/// The connector a tool belongs to: `rawInput.server`/`.tool`, else a name in the
/// `mcp__server__tool` shape Claude gives connector tools.
fn connector(raw: &Value, tool: &str) -> (String, String) {
    let explicit = (
        raw["mcpServer"].as_str().or_else(|| raw["server"].as_str()),
        raw["mcpTool"].as_str().or_else(|| raw["serverTool"].as_str()),
    );
    if let (Some(server), Some(inner)) = explicit {
        return (server.to_owned(), inner.to_owned());
    }
    match tool.strip_prefix("mcp__").and_then(|rest| rest.split_once("__")) {
        Some((server, inner)) => (server.to_owned(), inner.to_owned()),
        None => (String::new(), String::new()),
    }
}

/// A path with the person's home folder as `~`.
fn shortened_home(path: &str) -> String {
    let Some(home) = dirs::home_dir() else { return path.to_owned() };
    let home = home.to_string_lossy().replace('\\', "/");
    let shown = path.replace('\\', "/");
    match shown.strip_prefix(&home) {
        Some(rest) => format!("~{rest}"),
        None => shown,
    }
}

fn cap(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_owned();
    }
    let mut cut: String = text.chars().take(max).collect();
    cut.push('…');
    cut
}

fn cel(value: &Value) -> CelValue {
    match value {
        Value::Null => CelValue::Null,
        Value::Bool(flag) => CelValue::from(*flag),
        Value::Number(number) => {
            number.as_i64().map_or_else(|| CelValue::from(number.as_f64().unwrap_or_default()), CelValue::from)
        }
        Value::String(text) => CelValue::from(text.clone()),
        Value::Array(items) => CelValue::from(items.iter().map(cel).collect::<Vec<CelValue>>()),
        Value::Object(map) => {
            let pairs: HashMap<String, CelValue> = map.iter().map(|(key, inner)| (key.clone(), cel(inner))).collect();
            CelValue::from(pairs)
        }
    }
}

/// One CEL context with the request's groups (`tool.name`, `file.path`…). Built per evaluation: a
/// permission request is a human-scale event, and compiling then is what keeps a rule's text and what
/// runs the same thing.
pub fn context(request: &Request) -> Context<'static, 'static> {
    let mut context = Context::default();
    if let Value::Object(groups) = request.truth() {
        for (group, fields) in groups {
            context.add_variable_from_value(group, cel(&fields));
        }
    }
    context
}

/// The variables a rule may use, as an editor lists them.
pub const VARIABLES: [(&str, &str); 16] = [
    ("tool.name", "The agent's tool name"),
    ("tool.kind", "Its kind: execute, edit, delete, move, read, search, fetch, think, other"),
    ("intent", "What the call says it does"),
    ("bot.id", "The bot making the call"),
    ("bot.name", "Its name"),
    ("actor.id", "The person whose bot it is; empty for the owner"),
    ("actor.role", "owner, admin or member"),
    ("command", "The shell command, when there is one"),
    ("file.path", "The file the call is about"),
    ("file.op", "read, edit, delete or move"),
    ("mcp.server", "The connector, for a connector tool"),
    ("mcp.tool", "The connector's tool"),
    ("page.url", "The page, for a browser action"),
    ("page.host", "Its host"),
    ("initiator.kind", "person, routine, channel or network"),
    ("key", "The key a keyboard action presses"),
];
