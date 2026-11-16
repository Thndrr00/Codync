//! The approval card every messaging channel shows, built once and rendered by [`super::render`].
//!
//! A permission entry is shown natively by each channel: Slack Block Kit buttons, Discord components,
//! Telegram's inline keyboard and WhatsApp's numbered options. All of them answer through the host's
//! existing `respondPermission` method, so the answer path does not change with the channel.

use super::render::{Block, Button, Field, Tone};
use serde_json::Value;
use sha2::{Digest as _, Sha256};

/// One choice an approval offers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Choice {
    pub id: String,
    pub name: String,
}

/// What an approval entry carries that a channel card shows.
#[derive(Clone, Debug)]
pub struct Approval {
    pub id: String,
    pub title: String,
    pub command: Option<String>,
    pub kind: Option<String>,
    pub detail: Option<String>,
    pub options: Vec<Choice>,
}

impl Approval {
    /// The approval entry `entry` holds, or `None` when it is not one.
    pub fn from_entry(entry: &Value) -> Option<Self> {
        let data = &entry["data"];
        let options = data["options"]
            .as_array()?
            .iter()
            .filter_map(|option| {
                Some(Choice { id: option["optionId"].as_str()?.to_owned(), name: option["name"].as_str()?.to_owned() })
            })
            .collect();
        Some(Self {
            id: entry["id"].as_str()?.to_owned(),
            title: data["title"].as_str().unwrap_or("Approval").to_owned(),
            command: data["command"].as_str().filter(|command| !command.is_empty()).map(str::to_owned),
            kind: data["toolKind"].as_str().filter(|kind| !kind.is_empty() && *kind != "other").map(str::to_owned),
            detail: data["detail"].as_str().filter(|detail| !detail.trim().is_empty()).map(str::to_owned),
            options,
        })
    }

    /// The card's blocks. `action` turns an option id into the code that channel's button carries; a
    /// channel that answers by number (WhatsApp) passes `None` for every option.
    pub fn blocks(&self, action: impl Fn(&str) -> Option<String>) -> Vec<Block> {
        let mut blocks = vec![Block::Text(format!("Needs you: {}", self.title))];
        if let Some(command) = &self.command {
            blocks.push(Block::Callout { tone: Tone::Warning, text: command.clone() });
        }
        let mut fields = Vec::new();
        if let Some(kind) = &self.kind {
            fields.push(Field::new("Tool", kind.clone()));
        }
        if let Some(detail) = &self.detail {
            fields.push(Field::new("Details", detail.chars().take(400).collect::<String>()));
        }
        if !fields.is_empty() {
            blocks.push(Block::Fields(fields));
        }
        if !self.options.is_empty() {
            blocks.push(Block::Buttons(
                self.options.iter().map(|option| Button::new(&option.name, action(&option.id))).collect(),
            ));
        }
        blocks
    }

    /// What the card says once it is settled: the choice, or that it closed.
    pub fn settled(&self, selected: Option<&str>, status: &str) -> String {
        let chosen = self.options.iter().find(|option| Some(option.id.as_str()) == selected);
        match (status, chosen) {
            ("answered", Some(choice)) => format!("{}\n✓ {}", self.title, choice.name),
            _ => format!("{}\n(closed)", self.title),
        }
    }
}

/// The print of a card: a digest of its title and options, so a button of a card that changed cannot
/// answer another.
pub fn print(card: &Value) -> Option<String> {
    let mut digest = Sha256::new();
    digest.update(card["data"]["title"].as_str().unwrap_or_default());
    for option in card["data"]["options"].as_array().into_iter().flatten() {
        digest.update(option["optionId"].as_str().unwrap_or_default());
    }
    Some(crate::chat::files::digest_hex(&digest.finalize()).chars().take(6).collect())
}

/// The value a Slack, Discord or Telegram button sends back: `p|<entry id>|<option>|<print>`.
pub fn action(card: &Value, option: &str) -> Option<String> {
    Some(format!("p|{}|{option}|{}", card["id"].as_str()?, print(card)?))
}

/// `action` split back into its id, option and print.
pub fn parse_action(value: &str) -> Option<(&str, &str, &str)> {
    let mut parts = value.split('|');
    let (Some("p"), Some(id), Some(option), Some(print)) = (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return None;
    };
    (parts.next().is_none() && !id.is_empty() && !option.is_empty() && !print.is_empty()).then_some((id, option, print))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channels::render;
    use serde_json::json;

    fn entry() -> Value {
        json!({
            "id": "e1",
            "kind": "permission",
            "data": {
                "title": "Run a command",
                "toolKind": "execute",
                "command": "cargo test",
                "detail": "in the workspace",
                "status": "pending",
                "options": [
                    {"optionId": "yes", "name": "Allow"},
                    {"optionId": "always", "name": "Always allow"},
                    {"optionId": "no", "name": "Reject"},
                ],
            },
        })
    }

    #[test]
    fn the_same_approval_becomes_each_channels_buttons() {
        let card = entry();
        let approval = Approval::from_entry(&card).expect("an approval");
        let blocks = approval.blocks(|option| action(&card, option));
        let slack = render::slack(&blocks);
        let expected = format!("p|e1|always|{}", print(&card).expect("a print"));
        assert_eq!(slack["blocks"][3]["elements"][1]["value"], expected);
        let telegram = render::telegram(&blocks);
        assert_eq!(telegram["reply_markup"]["inline_keyboard"][1][0]["text"], "Reject");
        let text = render::whatsapp(&blocks);
        assert!(text.contains("1) Allow"));
    }

    #[test]
    fn actions_round_trip_and_a_changed_card_gets_another_print() {
        let card = entry();
        let value = action(&card, "yes").expect("an action");
        let printed = print(&card).expect("a print");
        assert_eq!(parse_action(&value), Some(("e1", "yes", printed.as_str())));
        let mut changed = card.clone();
        changed["data"]["options"] = json!([{"optionId": "no", "name": "Reject"}]);
        assert_ne!(action(&changed, "yes"), Some(value));
        assert_eq!(parse_action("p|e1|yes"), None);
        assert_eq!(parse_action("q|e1|yes|abcdef"), None);
    }

    #[test]
    fn a_settled_card_names_the_choice() {
        let approval = Approval::from_entry(&entry()).expect("an approval");
        assert_eq!(approval.settled(Some("always"), "answered"), "Run a command\n✓ Always allow");
        assert_eq!(approval.settled(None, "cancelled"), "Run a command\n(closed)");
    }
}
