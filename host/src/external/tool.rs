//! A terminal agent's tool call, as the chat shows it: a title, an ACP-style kind and the changed lines.

use crate::agent::acp::truncate;
use crate::agent::bot::diff_summary;
use serde_json::{Value, json};

const MAX_TITLE: usize = 120;

fn field<'a>(input: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter().find_map(|k| input.get(*k)?.as_str()).filter(|s| !s.is_empty())
}

pub fn path(input: &Value) -> Option<&str> {
    field(input, &["file_path", "path", "notebook_path"])
}

fn base_name(path: &str) -> &str {
    path.trim_end_matches(['/', '\\']).rsplit(['/', '\\']).next().unwrap_or(path)
}

/// The ACP `ToolKind` a tool maps onto. Tool names are Claude Code's, which the other agents are mapped onto.
pub fn kind(tool: &str) -> &'static str {
    match tool {
        "Bash" | "run_command" | "shell" | "bash" => "execute",
        "Edit" | "MultiEdit" | "Write" | "NotebookEdit" => "edit",
        "Read" | "NotebookRead" => "read",
        "Grep" | "Glob" | "LS" => "search",
        "WebFetch" | "WebSearch" => "fetch",
        _ => "other",
    }
}

/// "Run npm test", "Edit app.rs", or the tool's own name.
pub fn title(tool: &str, input: &Value) -> String {
    let tool = if tool.is_empty() { "Tool" } else { tool };
    let detail = match kind(tool) {
        "execute" => field(input, &["command", "CommandLine"]).map(|c| c.lines().next().unwrap_or(c).to_owned()),
        "edit" | "read" => path(input).map(|p| base_name(p).to_owned()),
        "search" => field(input, &["pattern", "query", "path"]).map(str::to_owned),
        "fetch" => field(input, &["url", "query"]).map(str::to_owned),
        _ => None,
    };
    let verb = match kind(tool) {
        "execute" => "Run",
        _ => tool,
    };
    truncate(&detail.map_or_else(|| tool.to_owned(), |d| format!("{verb} {d}")), MAX_TITLE)
}

/// The changed regions of an edit tool's input, in the shape cards and tool rows show.
pub fn diffs(tool: &str, input: &Value) -> Vec<Value> {
    let Some(file) = path(input) else { return Vec::new() };
    let text = |k: &str| input.get(k).and_then(Value::as_str).unwrap_or_default().to_owned();
    match tool {
        "Edit" => vec![diff_summary(file, &text("old_string"), &text("new_string"))],
        "Write" => vec![diff_summary(file, "", &text("content"))],
        "MultiEdit" => input["edits"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|e| {
                let side = |k: &str| e.get(k).and_then(Value::as_str).unwrap_or_default();
                diff_summary(file, side("old_string"), side("new_string"))
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// The entry data of a tool call that has started.
pub fn entry_data(id: &str, tool: &str, input: &Value, author: &str) -> Value {
    let locations = path(input).map_or_else(|| json!([]), |p| json!([{"path": p}]));
    json!({
        "toolCallId": id,
        "title": title(tool, input),
        "toolKind": kind(tool),
        "status": "in_progress",
        "output": "",
        "diffs": [],
        "locations": locations,
        "author": author,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles_name_what_the_tool_does() {
        assert_eq!(title("Bash", &json!({"command": "npm test\nnpm run lint"})), "Run npm test");
        assert_eq!(title("Edit", &json!({"file_path": "C:\\p\\src\\app.rs"})), "Edit app.rs");
        assert_eq!(title("Grep", &json!({"pattern": "TODO"})), "Grep TODO");
        assert_eq!(title("Mystery", &json!({})), "Mystery");
        assert_eq!(title("", &Value::Null), "Tool");
        assert!(title("Bash", &json!({"command": "x".repeat(500)})).len() <= MAX_TITLE + 3);
    }

    #[test]
    fn edits_report_their_changed_lines() {
        let edit = diffs("Edit", &json!({"file_path": "a.rs", "old_string": "a\nb\n", "new_string": "a\nB\nC\n"}));
        assert_eq!((edit[0]["added"].as_u64(), edit[0]["removed"].as_u64()), (Some(2), Some(1)));
        let multi = diffs(
            "MultiEdit",
            &json!({"file_path": "a.rs", "edits": [{"old_string": "x", "new_string": "y"}, {"old_string": "p", "new_string": "q"}]}),
        );
        assert_eq!(multi.len(), 2);
        assert_eq!(diffs("Bash", &json!({"command": "ls"})).len(), 0, "only edits change lines");
        assert!(diffs("Edit", &json!({})).is_empty(), "no file, nothing to show");
    }
}
