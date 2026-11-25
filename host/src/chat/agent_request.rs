//! Approval requests by agents (agentes.chat ADR 0126).
//!
//! When an agent needs a person's decision, the request is a bounded card: who asked, why, and a
//! small set of options. Codync already answers ACP permission requests; this adds the structured
//! form the clients can draw and the single-use answer.

use anyhow::{Result, bail};
use serde_json::{Value, json};

const MAX_REASON_CHARS: usize = 400;

/// The options an agent may offer; always at least an allow and a deny.
pub fn options() -> Value {
    json!([
        {"id": "allow", "label": "Allow", "kind": "allow_once"},
        {"id": "deny", "label": "Deny", "kind": "reject_once"},
    ])
}

/// A request card, with the reason clipped so a client cannot be flooded.
pub fn card(agent: &str, reason: &str, request_id: &str) -> Result<Value> {
    let reason = reason.trim();
    if reason.is_empty() {
        bail!("a request needs a reason");
    }
    let clipped: String = reason.chars().take(MAX_REASON_CHARS).collect();
    Ok(json!({
        "type": "approval_request",
        "requestId": request_id,
        "agent": agent,
        "reason": clipped,
        "options": options(),
        "status": "pending",
    }))
}

/// Who asked for the decision: the agent (a bot) or a person. A bot's own agent id is always a
/// bot; this is the field the client shows so an agent's request isn't mistaken for the person's.
pub fn request(agent: &str, reason: &str) -> Value {
    json!({"kind": "agent", "id": agent, "reason": reason})
}

/// Applies an answer once: the card moves from pending to decided with the chosen option.
pub fn decide(mut card: Value, option: &str) -> Result<Value> {
    if card["status"] != "pending" {
        bail!("this request was already decided");
    }
    let allowed = card["options"].as_array().is_some_and(|o| o.iter().any(|v| v["id"] == option));
    if !allowed {
        bail!("that option is not on this request");
    }
    card["status"] = "decided".into();
    card["selected"] = json!(option);
    Ok(card)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_is_answered_once() {
        let card = card("Ada", "needs to send an email", "r1").unwrap();
        assert_eq!(card["status"], "pending");
        let decided = decide(card.clone(), "allow").unwrap();
        assert_eq!(decided["selected"], "allow");
        assert!(decide(decided, "deny").is_err());
        assert!(decide(card, "other").is_err());
    }

    #[test]
    fn a_long_reason_is_clipped() {
        let card = card("Ada", &"x".repeat(MAX_REASON_CHARS + 50), "r2").unwrap();
        assert_eq!(card["reason"].as_str().unwrap().chars().count(), MAX_REASON_CHARS);
    }
}
