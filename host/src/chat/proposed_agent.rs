//! Agents propose agents; a person creates them (agentes.chat ADR 0081).
//!
//! An agent may propose a new bot, but never create it: the proposal is a card with the proposed
//! name, agent and instructions, and the person decides. This is the shape of that proposal.

use anyhow::{Result, bail};
use serde_json::{Value, json};

const MAX_INSTRUCTIONS: usize = 2_000;

/// A proposal from an agent, validated before it becomes a card.
pub fn proposal(from: &str, name: &str, agent: &str, instructions: &str) -> Result<Value> {
    let name = name.trim();
    if name.is_empty() {
        bail!("a proposed bot needs a name");
    }
    if instructions.chars().count() > MAX_INSTRUCTIONS {
        bail!("proposed instructions must be at most {MAX_INSTRUCTIONS} characters");
    }
    Ok(json!({
        "type": "agent_proposal",
        "by": from,
        "name": name,
        "agent": agent,
        "instructions": instructions.trim(),
        "status": "pending",
    }))
}

/// Accepting a proposal hands the fields to the host's normal create-bot path.
pub fn accept(mut proposal: Value) -> Value {
    proposal["status"] = "accepted".into();
    proposal
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_proposal_is_bounded_and_needs_a_name() {
        assert!(proposal("a", "", "claude", "helps").is_err());
        assert!(proposal("a", "New", "claude", &"x".repeat(MAX_INSTRUCTIONS + 1)).is_err());
        let p = proposal("a", "New", "claude", "  helps  ").unwrap();
        assert_eq!(p["status"], "pending");
        assert_eq!(accept(p)["status"], "accepted");
    }
}
