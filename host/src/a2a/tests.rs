use super::*;
use crate::store::BotConfig;
use serde_json::json;

fn bot(offered: bool) -> BotConfig {
    let mut bot: BotConfig =
        serde_json::from_value(json!({"id": "b", "name": "Reviewer", "description": "Reviews."})).expect("a bot");
    bot.offered_to_network = offered;
    bot
}

#[test]
fn a_bot_kept_off_the_network_is_not_an_a2a_agent() {
    assert!(agent_available(&bot(true)));
    assert!(!agent_available(&bot(false)));
    assert!(client::admit(&bot(true)).is_ok());
    assert!(client::admit(&bot(false)).unwrap_err().to_string().contains("isn't offered"));
    let mut hidden = bot(true);
    hidden.hidden = true;
    assert!(!agent_available(&hidden));
}

#[test]
fn a_waiter_is_taken_exactly_once() {
    let waiting = Waiting::default();
    let (sender, _receiver) = tokio::sync::oneshot::channel();
    waiting.insert("letter-1".into(), sender);
    assert!(waiting.take("letter-1").is_some());
    assert!(waiting.take("letter-1").is_none());
}

#[test]
fn the_tool_names_ask_bot_as_its_sibling() {
    let text = tool()["description"].as_str().unwrap_or_default();
    assert!(text.contains("ask_bot"));
    assert!(text.contains("approvals"));
}

#[test]
fn the_tool_schema_asks_for_an_address_and_a_task() {
    let tool = tool();
    assert_eq!(tool["name"], "a2a_ask");
    assert_eq!(tool["inputSchema"]["required"], json!(["agent", "message"]));
    assert!(tool["description"].as_str().unwrap_or_default().contains("network rules"));
}
