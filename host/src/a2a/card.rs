//! The A2A agent card of one of this computer's bots.
//!
//! A card is built from the bot's own name and description plus the address it has on the network.
//! It is served at `GET /a2a/{bot}/.well-known/agent-card.json` on the local API and, for another
//! computer, compiled from what that computer offers in the network directory (the authoritative
//! card stays on the computer that runs the bot).

use crate::store::BotConfig;
use serde_json::{Value, json};

/// The protocol version this adapter speaks. It implements `message/send` and `message/stream`.
pub const PROTOCOL: &str = "0.3.0";

/// The card of `bot`, reachable at `url`. `handle` and `computer` are its network address parts,
/// empty when this computer isn't in a network.
pub fn agent_card(bot: &BotConfig, handle: &str, computer: &str, url: &str) -> Value {
    let name = bot.name.trim();
    let description = bot.description.trim();
    let id = if handle.is_empty() { bot.id.as_str() } else { handle };
    json!({
        "name": if name.is_empty() { id } else { name },
        "description": if description.is_empty() {
            format!("The Codync bot {id}.")
        } else {
            description.to_owned()
        },
        "url": url,
        "version": env!("CARGO_PKG_VERSION"),
        "protocolVersion": PROTOCOL,
        "preferredTransport": "JSONRPC",
        "capabilities": {"streaming": true, "pushNotifications": false},
        "defaultInputModes": ["text/plain"],
        "defaultOutputModes": ["text/plain"],
        "skills": [{
            "id": id,
            "name": if name.is_empty() { id } else { name },
            "description": if description.is_empty() {
                format!("Ask the Codync bot {id} for a bounded result.")
            } else {
                description.to_owned()
            },
            "tags": ["codync"],
        }],
        "metadata": {"computer": computer, "agent": id},
    })
}

/// The card another computer advertises, compiled from the network directory. Its `url` is the
/// local host's address only as a label; requests to another computer travel over codync-network.
pub fn remote_card(handle: &str, name: &str, about: &str, computer: &str, network: &str) -> Value {
    let name = if name.trim().is_empty() { handle } else { name.trim() };
    json!({
        "name": name,
        "description": if about.trim().is_empty() {
            format!("The Codync bot {handle} on {computer}.")
        } else {
            about.trim().to_owned()
        },
        "url": format!("codync://{handle}@{computer}"),
        "version": env!("CARGO_PKG_VERSION"),
        "protocolVersion": PROTOCOL,
        "preferredTransport": "codync-network",
        "capabilities": {"streaming": false, "pushNotifications": false},
        "defaultInputModes": ["text/plain"],
        "defaultOutputModes": ["text/plain"],
        "skills": [{
            "id": handle,
            "name": name,
            "description": if about.trim().is_empty() {
                format!("Ask the Codync bot {handle} on {computer} for a bounded result.")
            } else {
                about.trim().to_owned()
            },
            "tags": ["codync", "network"],
        }],
        "metadata": {"computer": computer, "agent": handle, "network": network},
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::BotConfig;
    use serde_json::json;

    fn bot() -> BotConfig {
        let mut bot: BotConfig = serde_json::from_value(json!({"id": "b1", "name": "Reviewer"})).expect("a bot");
        bot.description = "Reviews diffs.".into();
        bot
    }

    #[test]
    fn a_card_names_the_bot_and_its_streaming_skill() {
        let card = agent_card(&bot(), "reviewer", "desk", "http://127.0.0.1:19222/a2a/b1");
        assert_eq!(card["name"], "Reviewer");
        assert_eq!(card["description"], "Reviews diffs.");
        assert_eq!(card["protocolVersion"], PROTOCOL);
        assert_eq!(card["capabilities"]["streaming"], true);
        assert_eq!(card["skills"][0]["id"], "reviewer");
        assert_eq!(card["metadata"]["agent"], "reviewer");
        assert_eq!(card["metadata"]["computer"], "desk");
        assert_eq!(card["url"], "http://127.0.0.1:19222/a2a/b1");
    }

    #[test]
    fn a_remote_card_names_the_computer_and_the_network() {
        let card = remote_card("reviewer", "Reviewer", "", "desk", "home");
        assert_eq!(card["url"], "codync://reviewer@desk");
        assert_eq!(card["metadata"]["network"], "home");
        assert!(card["description"].as_str().unwrap_or_default().contains("desk"));
    }
}
