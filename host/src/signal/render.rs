//! An approval card drawn for Signal: the K5 blocks as plain text with numbered options. A person
//! answers by quoting the card and sending the number, since Signal has no buttons.

use crate::channels::approval::Approval;
use crate::channels::render::{self, Block};
use serde_json::Value;

/// The text of a card: its blocks, with English numbered options.
pub(super) fn card(blocks: &[Block]) -> String {
    render::signal(blocks)
}

/// The message an approval becomes: the card, plus the rule that makes the number a reply.
pub(super) fn approval_card(model: &Approval) -> String {
    card(&model.blocks(|_| None))
}

/// What the chat is told once a card is settled. Signal cannot edit a sent message, so the card
/// stays and this line says what it became.
pub(super) fn settled(entry: &Value) -> String {
    let selected = entry["data"]["selected"].as_str();
    let status = entry["data"]["status"].as_str().unwrap_or("answered");
    Approval::from_entry(entry)
        .map_or_else(|| format!("Approval {status}\n(closed)"), |model| model.settled(selected, status))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channels::approval::Choice;

    fn model() -> Approval {
        Approval {
            id: "e1".into(),
            title: "Run a command".into(),
            command: Some("cargo test".into()),
            kind: Some("execute".into()),
            detail: None,
            options: vec![
                Choice { id: "yes".into(), name: "Allow".into() },
                Choice { id: "no".into(), name: "Reject".into() },
            ],
        }
    }

    #[test]
    fn a_card_is_numbered_text_with_the_quote_rule() {
        let text = approval_card(&model());
        assert!(text.starts_with("Needs you: Run a command"));
        assert!(text.contains("\u{26a0}\u{fe0f} cargo test"));
        assert!(text.ends_with("Reply to this message with the number of an option:\n1) Allow\n2) Reject"), "{text}");
    }
}
