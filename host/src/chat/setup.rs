//! A short setup checklist in a new bot's chat (agentes.chat ADR 0155/0164).
//!
//! A bot created in two steps gets a list of what is still missing — an agent, instructions, a
//! folder — so the person knows what to do next. The checklist is derived from what the bot already
//! has; it never claims more than the host can see.

use serde_json::{Value, json};

/// The setup steps a bot may still need. Every entry says what is done and what is left.
pub fn checklist(bot: &Value) -> Value {
    let mut steps = Vec::new();
    let has_agent = bot["backend"].as_str().is_some_and(|s| !s.is_empty());
    steps.push(json!({"id": "agent", "done": has_agent, "label": "Choose an agent"}));
    let has_instructions = bot["instructions"].as_str().is_some_and(|s| !s.trim().is_empty())
        || bot["about"].as_str().is_some_and(|s| !s.trim().is_empty());
    steps.push(json!({"id": "instructions", "done": has_instructions, "label": "Write what this bot does"}));
    let has_folder = bot["cwd"].as_str().is_some_and(|s| !s.is_empty());
    steps.push(json!({"id": "folder", "done": has_folder, "label": "Pick a working folder"}));
    let done = steps.iter().filter(|s| s["done"] == true).count();
    json!({"steps": steps, "done": done, "total": steps.len(), "complete": done == steps.len()})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_bot_sees_only_what_is_missing() {
        let bot = json!({"backend": "claude", "instructions": "", "cwd": ""});
        let list = checklist(&bot);
        assert_eq!(list["done"], 1);
        assert_eq!(list["total"], 3);
        assert_eq!(list["steps"][0]["done"], true);
        assert_eq!(list["steps"][1]["done"], false);
        assert_eq!(list["complete"], false);
    }

    #[test]
    fn a_configured_bot_is_complete() {
        let bot = json!({"backend": "claude", "about": "helps", "cwd": "/work"});
        assert_eq!(checklist(&bot)["complete"], true);
    }
}
