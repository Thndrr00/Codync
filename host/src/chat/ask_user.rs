//! `ask_user`: a bot asks a question with choices, and the answer comes back into its chat.
//!
//! The card is the same shape the terminal's `AskUserQuestion` uses (`external/question.rs`), so
//! every client already renders it. The person taps an option through `respondPermission`; the
//! host turns that tap into a user message in the bot's chat instead of an approval decision.
//! The tool returns at once — the answer arrives as the bot's next turn, like a mini-app `_tell`.

use crate::hub::Hub;
use crate::remote::push::{AlertKind, notify};
use crate::store::{EntryKind, Lane};
use anyhow::{Result, bail};
use serde_json::{Value, json};

pub const ASK_USER: &str = "ask_user";
pub const REQUEST_APPROVAL: &str = "request_approval";
pub const TOOLS: [&str; 2] = [ASK_USER, REQUEST_APPROVAL];

const MAX_QUESTION: usize = 400;
const MAX_HEADER: usize = 80;
const MAX_LABEL: usize = 120;
const MAX_DESCRIPTION: usize = 240;
const MIN_OPTIONS: usize = 2;
const MAX_OPTIONS: usize = 4;

pub fn tools() -> [Value; 2] {
    [
        json!({
            "name": "request_approval",
            "description": "Pause a workflow until a person approves or rejects a step. Your turn ends here; their decision arrives as their next message in the chat. Never use it for something you can safely do yourself.",
            "inputSchema": {
                "type": "object",
                "properties": {"question": {"type": "string", "description": "What needs a person's decision."}},
                "required": ["question"]
            }
        }),
        json!({
            "name": "ask_user",
            "description": "Ask the person a question with two to four choices. Your turn ends here; their answer arrives as their next message in the chat, so stop and wait instead of guessing.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "question": {"type": "string", "description": "What you need to know."},
                    "header": {"type": "string", "description": "A short label for the card."},
                    "options": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "label": {"type": "string", "description": "The choice the person taps."},
                                "description": {"type": "string", "description": "What this choice means."}
                            },
                            "required": ["label"]
                        }
                    }
                },
                "required": ["question", "options"]
            }
        }),
    ]
}

/// A choice as validated and stored on the card.
#[derive(Debug, PartialEq, Eq)]
pub struct Choice {
    pub label: String,
    pub description: String,
}

/// Validates the tool arguments and returns the option labels in card order.
pub fn choices(args: &Value) -> Result<Vec<Choice>> {
    let list = args["options"]
        .as_array()
        .filter(|list| (MIN_OPTIONS..=MAX_OPTIONS).contains(&list.len()))
        .ok_or_else(|| anyhow::anyhow!("ask_user needs between {MIN_OPTIONS} and {MAX_OPTIONS} options"))?;
    list.iter()
        .map(|o| {
            let label = clip(o["label"].as_str().unwrap_or_default(), MAX_LABEL);
            if label.is_empty() {
                bail!("every option needs a label");
            }
            Ok(Choice { label, description: clip(o["description"].as_str().unwrap_or_default(), MAX_DESCRIPTION) })
        })
        .collect()
}

fn clip(text: &str, max: usize) -> String {
    text.trim().chars().take(max).collect()
}

/// The card data, kept pure so the shape can be tested without a host.
pub fn card(question: &str, header: &str, choices: &[Choice], bot: &str, cwd: &str) -> Value {
    let detail = choices
        .iter()
        .filter(|c| !c.description.is_empty())
        .map(|c| format!("{}: {}", c.label, c.description))
        .collect::<Vec<_>>()
        .join("\n");
    let options = choices
        .iter()
        .enumerate()
        .map(|(i, c)| json!({"optionId": i.to_string(), "name": c.label, "kind": "allow_once"}))
        .collect::<Vec<_>>();
    json!({
        "title": clip(question, MAX_QUESTION),
        "toolKind": "question",
        "question": true,
        "header": clip(header, MAX_HEADER),
        "command": Value::Null,
        "detail": detail,
        "diffs": [],
        "cwd": cwd,
        "options": options,
        "status": "pending",
        "selected": Value::Null,
        "author": bot,
    })
}

pub fn call(hub: &Hub, bot: &str, name: &str, args: &Value) -> Result<Value> {
    if name != "ask_user" && name != "request_approval" {
        bail!("unknown ask_user tool: {name}");
    }
    let question = clip(args["question"].as_str().unwrap_or_default(), MAX_QUESTION);
    if question.is_empty() {
        bail!("{name} needs a question");
    }
    let choices = if name == "request_approval" {
        vec![
            Choice { label: "Approve".to_owned(), description: String::new() },
            Choice { label: "Reject".to_owned(), description: String::new() },
        ]
    } else {
        choices(args)?
    };
    let cwd = hub.store.bot(bot)?.map(|row| row.config.cwd).unwrap_or_default();
    let data = card(&question, args["header"].as_str().unwrap_or_default(), &choices, bot, &cwd);
    let entry = hub
        .add_entry(&Lane::main(bot), EntryKind::Permission, hub.store.max_turn(bot), &data)
        .ok_or_else(|| anyhow::anyhow!("couldn't save the question card"))?;
    if let Some(config) = hub.store.bot(bot)?
        && !config.deleted
    {
        notify(
            hub,
            &config.config,
            Some(bot),
            &format!("{} needs you", config.config.name),
            &question,
            AlertKind::NeedsInput,
        );
    }
    Ok(json!({"status": "asked", "entryId": entry.id, "question": question}))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args() -> Value {
        json!({
            "question": " Which database? ",
            "header": "DB",
            "options": [
                {"label": "SQLite", "description": "one file"},
                {"label": "Postgres", "description": "a server"}
            ]
        })
    }

    #[test]
    fn a_question_becomes_a_card_with_tappable_options() {
        let list = choices(&args()).unwrap();
        let data = card("Which database?", "DB", &list, "bot", "/work");
        assert_eq!(data["title"], "Which database?");
        assert_eq!(data["toolKind"], "question");
        assert_eq!(data["question"], true);
        assert_eq!(data["options"][0]["optionId"], "0");
        assert_eq!(data["options"][1]["name"], "Postgres");
        assert_eq!(data["detail"], "SQLite: one file\nPostgres: a server");
    }

    #[test]
    fn a_question_needs_two_to_four_labelled_options() {
        assert!(choices(&json!({"question": "q", "options": [{"label": "only"}]})).is_err());
        assert!(
            choices(&json!({"question": "q", "options": [
                {"label": "1"}, {"label": "2"}, {"label": "3"}, {"label": "4"}, {"label": "5"}
            ]}))
            .is_err()
        );
        assert!(choices(&json!({"question": "q", "options": [{"label": ""}, {"label": "2"}]})).is_err());
    }

    #[test]
    fn labels_and_descriptions_are_bounded() {
        let long = "x".repeat(500);
        let list = choices(&json!({"options": [{"label": long, "description": long}, {"label": "b"}]})).unwrap();
        assert_eq!(list[0].label.chars().count(), MAX_LABEL);
        assert_eq!(list[0].description.chars().count(), MAX_DESCRIPTION);
    }
}
