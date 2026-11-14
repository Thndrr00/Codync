//! The `show_block` tool: a bot shows a form, an approval, a table or a chart in the chat, and a card
//! with actions comes back with the person's choice and the form's fields.

use super::schema::{self, Block};
use crate::agent::bot::Cmd;
use crate::hub::Hub;
use anyhow::{Result, anyhow, bail};
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::oneshot;

pub const TOOLS: [&str; 1] = ["show_block"];

/// How long a card with actions waits for the person before the tool gives up; the card closes.
pub const ANSWER_TIMEOUT: Duration = Duration::from_mins(10);

pub fn tool() -> Value {
    json!({
        "name": "show_block",
        "description": "Show the person a small piece of interface in your chat: a form, an approval with buttons, a \
            table or a simple chart. The host checks it against Codync's catalogue before anything is drawn, so an \
            invalid block is refused with the exact reason (fix it and call again). Every client draws it with its own \
            controls; Android draws the buttons and text only for now. A block with actions waits for the person and \
            returns their choice and the form's field values. Plain text only: no HTML, Markdown or scripts. \
            Shape: {\"version\": 1, \"title\": \"…\", \"body\": [components], \"actions\": [{id, label, kind}]}. \
            Components: heading {text}; text {text}; divider; callout {tone: info|warning, text}; \
            field {field: {id, label, type: text|multiline|number|date|time|select|toggle, required, placeholder, \
            options: [{value, label}], value}}; table {columns: [text], rows: [[text|number|true/false|null]]}; \
            chart {chartType: bar|line, labels: [text], values: [number]}. Limits: 40 body components, 20 fields, \
            8 table columns, 50 rows, 12 chart points, 4 actions. Actions default to one Submit when a form declares \
            none; kind is primary, secondary or destructive. Use it instead of asking the person to type formatted \
            answers: a field's id comes back in the result's values.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "block": {
                    "type": "object",
                    "description": "The block document (see the tool description).",
                    "required": ["version", "title", "body"],
                    "properties": {
                        "version": {"type": "integer", "enum": [1]},
                        "title": {"type": "string"},
                        "body": {"type": "array", "maxItems": schema::MAX_BODY, "items": {"type": "object"}},
                        "actions": {"type": "array", "maxItems": schema::MAX_ACTIONS, "items": {"type": "object"}}
                    }
                }
            },
            "required": ["block"]
        },
        "annotations": {"readOnlyHint": false, "idempotentHint": false}
    })
}

/// The `chat` server's block tool.
pub async fn call(hub: &Arc<Hub>, bot: &str, name: &str, args: &Value) -> Result<Value> {
    if !TOOLS.contains(&name) {
        bail!("unknown block tool: {name}");
    }
    let block = parse(args)?;
    let interactive = !block.actions.is_empty();
    let (reply, answer) = oneshot::channel();
    let cmd = Cmd::Block { data: card(&block), reply: interactive.then_some(reply) };
    hub.send_cmd(bot, cmd).map_err(|error| anyhow!("the block couldn't be shown: {error:#}"))?;
    if !interactive {
        return Ok(json!({"shown": true}));
    }
    let answer = tokio::time::timeout(ANSWER_TIMEOUT, answer)
        .await
        .map_err(|_| {
            anyhow!("the person didn't answer this block within {} minutes; it closed", ANSWER_TIMEOUT.as_secs() / 60)
        })?
        .map_err(|_| anyhow!("the block closed before the person answered it (its turn ended or was stopped)"))?;
    Ok(answer_json(answer))
}

/// What the tool answers the agent: the chosen action with the form's fields, or that it closed.
fn answer_json(answer: Option<Value>) -> Value {
    match answer {
        Some(answer) => json!({
            "answered": true,
            "optionId": answer["optionId"],
            "values": answer["values"],
        }),
        None => json!({"answered": false, "cancelled": true}),
    }
}

/// The answers a client sent for a card's block, checked against the block's own schema. A plain
/// approval (no block) or a cancel (`None`) passes through.
pub fn answers(block: &Value, values: Option<Value>) -> Result<Option<Value>> {
    match (block, values) {
        (block, Some(values)) if block.is_object() => Ok(Some(schema::parse(block)?.answers(&values)?)),
        (_, values) => Ok(values),
    }
}

/// The tool's block document, checked; a refusal names what broke and what's allowed.
fn parse(args: &Value) -> Result<Block> {
    schema::parse(&args["block"]).map_err(|error| anyhow!("{error:#}\n\n{}", super::guide()))
}

/// The card's data: a permission entry every client already routes to its approval card, with the
/// block (and, once answered, the form's values) beside what older clients need to fall back to.
fn card(block: &Block) -> Value {
    let options: Vec<Value> = block
        .actions
        .iter()
        .map(|action| json!({"optionId": action.id, "name": action.label, "kind": action.kind}))
        .collect();
    json!({
        "title": block.doc["title"],
        "toolKind": "block",
        "command": Value::Null,
        "detail": "",
        "diffs": [],
        "options": options,
        "status": if options.is_empty() { "shown" } else { "pending" },
        "selected": Value::Null,
        "block": block.doc,
        "values": Value::Null,
        "inputEntryIds": [],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_card_offers_its_actions_and_keeps_the_block_for_every_client() {
        let block = schema::parse(&json!({
            "version": 1,
            "title": "Approve the trip",
            "body": [
                {"type": "heading", "text": "Madrid"},
                {"type": "field", "field": {"id": "note", "label": "Note", "type": "text"}}
            ],
            "actions": [{"id": "approve", "label": "Approve", "kind": "primary"}]
        }))
        .unwrap();
        let card = card(&block);
        assert_eq!(card["status"], "pending");
        assert_eq!(card["options"][0]["optionId"], "approve");
        assert_eq!(card["block"]["body"][1]["field"]["id"], "note");
    }

    #[test]
    fn an_invalid_block_is_refused_with_the_catalogue() {
        let error = parse(&json!({"block": {"version": 1, "title": "x", "body": [{"type": "iframe"}]}})).unwrap_err();
        assert!(error.to_string().contains("the components are"), "{error}");
        assert!(error.to_string().contains("Allowed shape"), "{error}");
    }

    #[test]
    fn a_form_answer_round_trips_through_the_host_checks() {
        let block = schema::parse(&json!({
            "version": 1,
            "title": "Book the trip",
            "body": [
                {"type": "field", "field": {"id": "nights", "label": "Nights", "type": "number", "required": true}},
                {"type": "field", "field": {
                    "id": "room", "label": "Room", "type": "select", "required": true,
                    "options": [{"value": "double", "label": "Double"}]
                }}
            ],
            "actions": [{"id": "book", "label": "Book", "kind": "primary"}]
        }))
        .unwrap();
        // What the client sends: the chosen action and the filled fields.
        let accepted = answers(&block.doc, Some(json!({"nights": 2, "room": "double"}))).unwrap().unwrap();
        let answer = answer_json(Some(json!({"optionId": "book", "values": accepted})));
        assert_eq!(answer["answered"], true);
        assert_eq!(answer["optionId"], "book");
        assert_eq!(answer["values"], json!({"nights": 2, "room": "double"}));

        // The host refuses a required field left blank, a value the select never offered, and junk.
        assert!(answers(&block.doc, Some(json!({"room": "double"}))).is_err());
        assert!(answers(&block.doc, Some(json!({"nights": 2, "room": "suite"}))).is_err());
        assert!(answers(&block.doc, Some(json!({"nights": "two", "room": "double"}))).is_err());
        // A cancel, and a card that isn't a block, pass through.
        assert!(answers(&block.doc, None).unwrap().is_none());
        assert_eq!(answers(&json!(null), Some(json!({"x": 1}))).unwrap(), Some(json!({"x": 1})));
        assert_eq!(answer_json(None), json!({"answered": false, "cancelled": true}));
    }

    #[test]
    fn a_display_only_block_is_shown_without_waiting() {
        let block = schema::parse(&json!({
            "version": 1,
            "title": "Yesterday",
            "body": [{"type": "table", "columns": ["Bot", "Runs"], "rows": [["Digest", 2]]}]
        }))
        .unwrap();
        let card = card(&block);
        assert_eq!(card["status"], "shown");
        assert!(card["options"].as_array().unwrap().is_empty());
    }
}
