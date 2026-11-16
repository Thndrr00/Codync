//! One card, drawn the way each messaging channel draws it.
//!
//! A card is a list of [`Block`]s: text, fields, a table, a divider, a callout and buttons. Slack, Discord,
//! Telegram and WhatsApp each get a serializer that maps those blocks onto what the channel can express, and
//! Teams gets an Adaptive Card (defined here; its transport arrives with the Teams channel). A channel that
//! cannot express a block shows its plain-text form instead ([`fallback`]), which every serializer also puts
//! in the payload field the channel shows where rich content is unavailable (Slack's notification text,
//! Discord's `content`).
//!
//! Payload text is what the person reads in the channel, not host UI copy: it is not localized.

use serde_json::{Value, json};

/// What a callout is saying, which decides its mark and color.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Info,
    Success,
    Warning,
    Danger,
}

impl Tone {
    /// The mark a text-only channel puts in front of the callout.
    fn mark(self) -> &'static str {
        match self {
            Self::Info => "ℹ️",
            Self::Success => "✅",
            Self::Warning => "⚠️",
            Self::Danger => "⛔",
        }
    }

    /// Adaptive Cards' text color for this tone.
    fn adaptive(self) -> &'static str {
        match self {
            Self::Info => "default",
            Self::Success => "good",
            Self::Warning => "warning",
            Self::Danger => "attention",
        }
    }
}

/// One way to answer a card.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Button {
    /// What the person reads.
    pub label: String,
    /// What pressing it sends back, already in the channel's own encoding. `None` when this card is only
    /// offered as numbered options (WhatsApp), or when the caller has no short code for it.
    pub action: Option<String>,
}

impl Button {
    pub fn new(label: impl Into<String>, action: Option<String>) -> Self {
        Self { label: label.into(), action }
    }
}

/// A label and its value, shown as facts or key/value lines.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field {
    pub label: String,
    pub value: String,
}

impl Field {
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self { label: label.into(), value: value.into() }
    }
}

/// One piece of a card.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Block {
    Text(String),
    Fields(Vec<Field>),
    Table { columns: Vec<String>, rows: Vec<Vec<String>> },
    Divider,
    Callout { tone: Tone, text: String },
    Buttons(Vec<Button>),
}

/// The plain text every channel can show: what a block a channel cannot express falls back to.
pub fn fallback(card: &[Block]) -> String {
    let mut lines: Vec<String> = Vec::new();
    for block in card {
        match block {
            Block::Text(text) => lines.push(text.clone()),
            Block::Fields(fields) => {
                lines.extend(fields.iter().map(|field| format!("{}: {}", field.label, field.value)));
            }
            Block::Table { columns, rows } => lines.push(table_text(columns, rows)),
            Block::Divider => lines.push(String::new()),
            Block::Callout { tone, text } => lines.push(format!("{} {text}", tone.mark())),
            Block::Buttons(buttons) => {
                lines.extend(
                    buttons.iter().enumerate().map(|(index, button)| format!("{}) {}", index + 1, button.label)),
                );
            }
        }
    }
    lines.join("\n")
}

/// A Slack `chat.postMessage` body: Block Kit blocks, with the fallback as its notification text.
pub fn slack(card: &[Block]) -> Value {
    let mut blocks: Vec<Value> = Vec::new();
    for block in card {
        match block {
            Block::Text(text) => {
                blocks.push(json!({"type": "section", "text": {"type": "mrkdwn", "text": clip(text, SLACK_TEXT)}}))
            }
            Block::Fields(fields) if !fields.is_empty() => {
                let fields: Vec<Value> = fields
                    .iter()
                    .map(|field| {
                        json!({"type": "mrkdwn",
                            "text": format!("*{}*\n{}", field.label, clip(&field.value, SLACK_TEXT))})
                    })
                    .collect();
                blocks.push(json!({"type": "section", "fields": fields}));
            }
            Block::Fields(_) => {}
            Block::Table { columns, rows } => blocks.push(json!({
                "type": "section",
                "text": {"type": "mrkdwn", "text": format!("```{}```", clip(&table_text(columns, rows), SLACK_TEXT))},
            })),
            Block::Divider => blocks.push(json!({"type": "divider"})),
            Block::Callout { tone, text } => blocks.push(json!({
                "type": "context",
                "elements": [{"type": "mrkdwn", "text": format!("{} {}", tone.mark(), clip(text, SLACK_TEXT))}],
            })),
            Block::Buttons(buttons) => {
                let elements: Vec<Value> = buttons
                    .iter()
                    .enumerate()
                    .filter_map(|(index, button)| {
                        // A Slack button needs what it sends back; a label alone is shown as fallback text.
                        let action = button.action.as_ref()?;
                        Some(json!({
                            "type": "button",
                            "action_id": format!("codync_{index}"),
                            "text": {"type": "plain_text", "text": clip(&button.label, SLACK_LABEL)},
                            "value": clip(action, SLACK_VALUE),
                        }))
                    })
                    .collect();
                if !elements.is_empty() {
                    blocks.push(json!({"type": "actions", "elements": elements}));
                }
            }
        }
    }
    json!({"text": fallback(card), "blocks": blocks})
}

/// A Discord message body: fields become an embed, buttons action rows, everything else content.
pub fn discord(card: &[Block]) -> Value {
    let mut content: Vec<String> = Vec::new();
    let mut fields: Vec<Value> = Vec::new();
    let mut components: Vec<Value> = Vec::new();
    for block in card {
        match block {
            Block::Text(text) => content.push(text.clone()),
            Block::Fields(list) => fields.extend(list.iter().map(|field| {
                json!({"name": clip(&field.label, DISCORD_LABEL),
                        "value": clip(&field.value, DISCORD_VALUE), "inline": true})
            })),
            Block::Table { columns, rows } => {
                content.push(format!("```\n{}```", clip(&table_text(columns, rows), DISCORD_TEXT)));
            }
            Block::Divider => content.push(String::new()),
            Block::Callout { tone, text } => content.push(format!("{} {text}", tone.mark())),
            Block::Buttons(buttons) => {
                let row: Vec<Value> = buttons
                    .iter()
                    .filter_map(|button| {
                        let action = button.action.as_ref()?;
                        Some(json!({"type": 2, "style": 1, "label": clip(&button.label, DISCORD_LABEL),
                            "custom_id": clip(action, DISCORD_ID)}))
                    })
                    .collect();
                components.extend(row.chunks(5).map(|row| json!({"type": 1, "components": row})));
            }
        }
    }
    let mut body = json!({"content": content.join("\n\n"), "allowed_mentions": {"parse": []}});
    if !fields.is_empty() {
        body["embeds"] = json!([{"fields": fields}]);
    }
    if !components.is_empty() {
        body["components"] = json!(components);
    }
    body
}

/// A Telegram `sendMessage` payload: HTML text and an inline keyboard, whose callback data must fit 64 bytes.
pub fn telegram(card: &[Block]) -> Value {
    let mut lines: Vec<String> = Vec::new();
    let mut rows: Vec<Vec<Value>> = Vec::new();
    for block in card {
        match block {
            Block::Text(text) => lines.push(escape_html(text)),
            Block::Fields(fields) => lines.extend(
                fields
                    .iter()
                    .map(|field| format!("<b>{}</b>: {}", escape_html(&field.label), escape_html(&field.value))),
            ),
            Block::Table { columns, rows: body } => {
                lines.push(format!("<pre>{}</pre>", escape_html(&table_text(columns, body))));
            }
            Block::Divider => lines.push(String::new()),
            Block::Callout { tone, text } => lines.push(format!("{} {}", tone.mark(), escape_html(text))),
            Block::Buttons(buttons) => {
                let buttons: Vec<Value> = buttons
                    .iter()
                    .filter_map(|button| {
                        let action = button.action.as_ref()?;
                        // Telegram refuses callback data over 64 bytes; that option stays a plain label.
                        (action.len() <= TELEGRAM_CALLBACK)
                            .then(|| json!({"text": clip(&button.label, TELEGRAM_LABEL), "callback_data": action}))
                    })
                    .collect();
                rows.extend(buttons.chunks(2).map(<[Value]>::to_vec));
            }
        }
    }
    let mut payload = json!({"text": clip(&lines.join("\n"), TELEGRAM_TEXT), "parse_mode": "HTML"});
    if !rows.is_empty() {
        payload["reply_markup"] = json!({"inline_keyboard": rows});
    }
    payload
}

/// A WhatsApp message body: text, with the buttons as numbered options answered by number.
pub fn whatsapp(card: &[Block]) -> String {
    let mut lines: Vec<String> = Vec::new();
    for block in card {
        match block {
            Block::Text(text) => lines.push(text.clone()),
            Block::Fields(fields) => {
                lines.extend(fields.iter().map(|field| format!("*{}*: {}", field.label, field.value)));
            }
            Block::Table { columns, rows } => lines.push(table_text(columns, rows)),
            Block::Divider => lines.push(String::new()),
            Block::Callout { tone, text } => lines.push(format!("{} {text}", tone.mark())),
            Block::Buttons(buttons) if !buttons.is_empty() => {
                lines.push(String::new());
                lines.push("Responde con el número de una opción:".into());
                lines.extend(
                    buttons.iter().enumerate().map(|(index, button)| format!("{}) {}", index + 1, button.label)),
                );
            }
            Block::Buttons(_) => {}
        }
    }
    lines.join("\n")
}

/// A Signal message body: text, with the buttons as numbered options answered by a quoted reply.
pub fn signal(card: &[Block]) -> String {
    let mut lines: Vec<String> = Vec::new();
    for block in card {
        match block {
            Block::Text(text) => lines.push(text.clone()),
            Block::Fields(fields) => {
                lines.extend(fields.iter().map(|field| format!("{}: {}", field.label, field.value)));
            }
            Block::Table { columns, rows } => lines.push(table_text(columns, rows)),
            Block::Divider => lines.push(String::new()),
            Block::Callout { tone, text } => lines.push(format!("{} {text}", tone.mark())),
            Block::Buttons(buttons) if !buttons.is_empty() => {
                lines.push(String::new());
                lines.push("Reply to this message with the number of an option:".into());
                lines.extend(
                    buttons.iter().enumerate().map(|(index, button)| format!("{}) {}", index + 1, button.label)),
                );
            }
            Block::Buttons(_) => {}
        }
    }
    lines.join("\n")
}

/// A Teams bot message carrying an Adaptive Card. The Teams transport itself is K7; this is the wire shape.
pub fn teams(card: &[Block]) -> Value {
    let mut body: Vec<Value> = Vec::new();
    let mut actions: Vec<Value> = Vec::new();
    for block in card {
        match block {
            Block::Text(text) => body.push(json!({"type": "TextBlock", "text": text, "wrap": true})),
            Block::Fields(fields) => body.push(json!({
                "type": "FactSet",
                "facts": fields.iter().map(|field| json!({"title": field.label, "value": field.value})).collect::<Vec<_>>(),
            })),
            Block::Table { columns, rows } => body.push(json!({
                "type": "Table",
                "columns": columns.iter().map(|_| json!({"width": 1})).collect::<Vec<_>>(),
                "rows": rows.iter().map(|row| json!({
                    "type": "TableRow",
                    "cells": row.iter().map(|cell| json!({
                        "type": "TableCell",
                        "items": [{"type": "TextBlock", "text": cell, "wrap": true}],
                    })).collect::<Vec<_>>(),
                })).collect::<Vec<_>>(),
            })),
            Block::Divider => body.push(json!({"type": "TextBlock", "text": " ", "separator": true})),
            Block::Callout { tone, text } => body.push(json!({
                "type": "Container",
                "style": "emphasis",
                "items": [{"type": "TextBlock", "text": text, "wrap": true, "color": tone.adaptive()}],
            })),
            Block::Buttons(buttons) => actions.extend(buttons.iter().filter_map(|button| {
                let action = button.action.as_ref()?;
                Some(json!({"type": "Action.Submit", "title": button.label, "data": {"codyncAction": action}}))
            })),
        }
    }
    json!({
        "type": "message",
        "attachments": [{
            "contentType": "application/vnd.microsoft.card.adaptive",
            "contentUrl": Value::Null,
            "content": {
                "$schema": "https://adaptivecards.io/schemas/adaptive-card.json",
                "type": "AdaptiveCard",
                "version": "1.5",
                "body": body,
                "actions": actions,
            },
        }],
    })
}

const SLACK_TEXT: usize = 2900;
const SLACK_LABEL: usize = 75;
const SLACK_VALUE: usize = 2000;
const DISCORD_TEXT: usize = 1900;
const DISCORD_LABEL: usize = 80;
const DISCORD_VALUE: usize = 1000;
const DISCORD_ID: usize = 100;
const TELEGRAM_TEXT: usize = 4000;
const TELEGRAM_LABEL: usize = 64;
const TELEGRAM_CALLBACK: usize = 64;

/// `text` in at most `max` characters.
fn clip(text: &str, max: usize) -> String {
    if text.chars().count() <= max { text.to_owned() } else { text.chars().take(max).collect() }
}

/// The table's rows as one line per row, columns separated by a pipe.
fn table_text(columns: &[String], rows: &[Vec<String>]) -> String {
    let mut lines = vec![columns.join(" | ")];
    lines.extend(rows.iter().map(|row| row.join(" | ")));
    lines.join("\n")
}

/// Telegram's HTML mode escapes these three.
fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<Block> {
        vec![
            Block::Text("Needs you: Run a command".into()),
            Block::Callout { tone: Tone::Warning, text: "cargo test".into() },
            Block::Fields(vec![Field::new("Tool", "execute"), Field::new("Details", "1 file <2>")]),
            Block::Buttons(vec![
                Button::new("Allow", Some("p|e1|allow|a1b2c3".into())),
                Button::new("Reject", Some("p|e1|reject|a1b2c3".into())),
            ]),
        ]
    }

    fn labels(card: &Value) -> Vec<String> {
        card["components"]
            .as_array()
            .into_iter()
            .flatten()
            .flat_map(|row| row["components"].as_array().cloned().unwrap_or_default())
            .filter_map(|button| button["label"].as_str().map(str::to_owned))
            .collect()
    }

    #[test]
    fn slack_shows_sections_fields_and_buttons() {
        let card = slack(&sample());
        assert_eq!(
            card["text"],
            "Needs you: Run a command\n⚠️ cargo test\nTool: execute\nDetails: 1 file <2>\n1) Allow\n2) Reject"
        );
        let blocks = card["blocks"].as_array().expect("blocks");
        assert_eq!(blocks[0]["type"], "section");
        assert_eq!(blocks[0]["text"]["text"], "Needs you: Run a command");
        assert_eq!(blocks[1]["type"], "context");
        assert_eq!(blocks[2]["fields"][0]["text"], "*Tool*\nexecute");
        assert_eq!(blocks[3]["type"], "actions");
        assert_eq!(blocks[3]["elements"][0]["value"], "p|e1|allow|a1b2c3");
        assert_eq!(blocks[3]["elements"][1]["text"]["text"], "Reject");
    }

    #[test]
    fn discord_shows_fields_in_an_embed_and_buttons_in_an_action_row() {
        let card = discord(&sample());
        assert!(card["content"].as_str().expect("content").contains("Needs you: Run a command"));
        assert!(card["content"].as_str().expect("content").contains("⚠️ cargo test"));
        assert_eq!(card["embeds"][0]["fields"][1]["value"], "1 file <2>");
        assert_eq!(card["components"][0]["type"], 1);
        assert_eq!(labels(&card), ["Allow", "Reject"]);
        assert_eq!(card["components"][0]["components"][0]["custom_id"], "p|e1|allow|a1b2c3");
    }

    #[test]
    fn telegram_escapes_html_and_sends_an_inline_keyboard() {
        let card = telegram(&sample());
        let text = card["text"].as_str().expect("text");
        assert!(text.contains("Details</b>: 1 file &lt;2&gt;"), "{text}");
        assert_eq!(card["parse_mode"], "HTML");
        assert_eq!(card["reply_markup"]["inline_keyboard"][0][0]["callback_data"], "p|e1|allow|a1b2c3");
        assert_eq!(card["reply_markup"]["inline_keyboard"][0][1]["text"], "Reject");
    }

    #[test]
    fn telegram_leaves_out_callback_data_it_cannot_carry() {
        let card = vec![Block::Buttons(vec![
            Button::new("Allow", Some("x".repeat(TELEGRAM_CALLBACK + 1))),
            Button::new("Reject", Some("p|e1|reject|a1b2c3".into())),
        ])];
        let payload = telegram(&card);
        assert_eq!(payload["reply_markup"]["inline_keyboard"][0].as_array().expect("row").len(), 1);
        assert_eq!(payload["reply_markup"]["inline_keyboard"][0][0]["text"], "Reject");
    }

    #[test]
    fn whatsapp_numbers_the_options() {
        let text = whatsapp(&sample());
        assert!(text.starts_with("Needs you: Run a command\n⚠️ cargo test"));
        assert!(text.contains("*Tool*: execute"));
        assert!(text.ends_with("Responde con el número de una opción:\n1) Allow\n2) Reject"), "{text}");
    }

    #[test]
    fn signal_numbers_its_options_in_english() {
        let text = signal(&sample());
        assert!(text.starts_with("Needs you: Run a command\n\u{26a0}\u{fe0f} cargo test"), "{text}");
        assert!(text.ends_with("Reply to this message with the number of an option:\n1) Allow\n2) Reject"), "{text}");
    }

    #[test]
    fn teams_sends_an_adaptive_card() {
        let card = teams(&sample());
        let attachment = &card["attachments"][0];
        assert_eq!(attachment["contentType"], "application/vnd.microsoft.card.adaptive");
        assert_eq!(attachment["content"]["body"][1]["style"], "emphasis");
        assert_eq!(attachment["content"]["body"][2]["type"], "FactSet");
        assert_eq!(attachment["content"]["actions"][0]["data"]["codyncAction"], "p|e1|allow|a1b2c3");
    }

    #[test]
    fn a_block_a_channel_cannot_show_falls_back_to_text() {
        let card = vec![
            Block::Table {
                columns: vec!["Option".into(), "Effect".into()],
                rows: vec![vec!["allow".into(), "run it".into()]],
            },
            Block::Divider,
            Block::Buttons(vec![Button::new("Allow", None), Button::new("Reject", None)]),
        ];
        let text = fallback(&card);
        assert!(text.contains("Option | Effect\nallow | run it"), "{text}");
        assert!(text.ends_with("1) Allow\n2) Reject"), "{text}");
        // A channel that can't press a button still names both as text.
        let discord = discord(&card);
        assert!(discord["content"].as_str().expect("content").contains("Option | Effect"));
        assert!(discord.get("components").is_none(), "no action row without an action");
    }
}
