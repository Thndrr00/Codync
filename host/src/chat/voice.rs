//! How agents talk to you (agentes.chat ADR 0147): simple by default, technical by choice.
//!
//! A per-bot setting shapes one line added to the agent's instructions. It changes only wording:
//! the tools, permissions and behavior are the same either way.

use crate::store::Store;
use serde_json::{Value, json};

fn key(bot: &str) -> String {
    format!("agent-voice:{bot}")
}

const SIMPLE: &str = "Write in plain, short sentences. Skip jargon and internal names unless the person asks for them.";
const TECHNICAL: &str =
    "You may use precise technical terms, tool names and commands when they make the answer clearer.";

/// Reads the setting: a bot that never chose one is simple.
pub fn get(store: &Store, bot: &str) -> String {
    match store.kv_get(&key(bot)).as_deref() {
        Some("technical") => "technical".to_owned(),
        _ => "simple".to_owned(),
    }
}

/// Sets the voice and returns the line to add to the instructions.
pub fn set(store: &Store, bot: &str, voice: &str) -> Result<Value, anyhow::Error> {
    let voice = if voice == "technical" { "technical" } else { "simple" };
    store.kv_set(&key(bot), voice)?;
    Ok(json!({"voice": voice, "instruction": instruction(voice)}))
}

/// The instruction line for a voice.
pub fn instruction(voice: &str) -> &'static str {
    if voice == "technical" { TECHNICAL } else { SIMPLE }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_is_the_default_and_technical_is_a_choice() {
        assert!(instruction("simple").contains("plain"));
        assert!(instruction("technical").contains("technical"));
        assert!(instruction("anything else").contains("plain"));
    }
}
