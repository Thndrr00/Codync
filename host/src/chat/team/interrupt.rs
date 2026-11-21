//! `interrupt_bot`: stop another bot's running turn, with a reason. Ported from OpenBot nightly
//! in Codync's terms: it reuses `hub.stop_bot`, the team tools' visibility rules and the owner-only
//! approval card that `market::bot_deletion` uses. No new loop or store.

use crate::agent::bot::Cmd;
use crate::chat::manage::APPROVAL_TIMEOUT;
use crate::hub::Hub;
use anyhow::{Result, anyhow, bail};
use serde_json::{Value, json};
use std::sync::Arc;
use tokio::sync::oneshot;

/// The team tool's name (also the new label string clients may show).
pub(super) const TOOL: &str = "interrupt_bot";
/// The fixed answer once an interruption is accepted, whatever the target was doing.
pub(super) const OUTCOME: &str = "Interrupted the agent's running turn.";

pub(super) fn tool() -> Value {
    json!({
        "name": TOOL,
        "description": "Stop another Codync bot's running turn. Give a short reason. You may stop a bot you created; stopping one you don't own asks the owner on a card. The reply comes as a new turn in that bot's chat, where it answers you. Don't poll for it.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "botId": {"type": "string", "description": "The id of the agent to stop, from list_bots."},
                "reason": {"type": "string", "description": "Why you are stopping it; shown on the owner's approval card."},
            },
            "required": ["botId"],
        },
        "annotations": {"readOnlyHint": false, "idempotentHint": true},
    })
}

/// The authorization decision. Pure, so every stopping rule has a test without a host.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Decision {
    /// The caller owns the target (or is the owner) and may stop it now.
    Allow,
    /// The caller doesn't own the target: only the owner, on a card, may allow it.
    OwnerApproval,
    /// Not a bot the caller may act on at all.
    Deny,
}

/// Whoever calls, a hidden, deleted or unknown bot is never actionable. The caller may stop itself
/// and the bots it created; the owner may stop any bot; anyone else needs the owner's approval.
pub(super) fn decide(actionable: bool, caller_owns_target: bool, caller_is_owner: bool) -> Decision {
    if !actionable {
        Decision::Deny
    } else if caller_is_owner || caller_owns_target {
        Decision::Allow
    } else {
        Decision::OwnerApproval
    }
}

/// Stops the named bot. A bot the caller doesn't own first waits on the same owner-only card a
/// deletion uses; the stop itself always goes through `hub.stop_bot`.
pub(super) async fn perform(hub: &Arc<Hub>, from: &str, args: &Value) -> Result<Value> {
    let to = args["botId"].as_str().ok_or_else(|| anyhow!("botId is required"))?;
    let reason = args["reason"].as_str().unwrap_or_default().trim();
    let source = super::visible_bot(hub, from)?;
    let row = hub.store.bot(to)?;
    let actionable = row.as_ref().is_some_and(super::visible);
    let caller_owns_target =
        row.as_ref().is_some_and(|r| r.config.id == from || r.config.created_by.as_deref() == Some(from));

    match decide(actionable, caller_owns_target, false) {
        Decision::Deny => bail!("unknown or hidden bot"),
        Decision::OwnerApproval => {
            let target = row.as_ref().map(|r| r.config.name.as_str()).unwrap_or(to);
            let detail = if reason.is_empty() {
                format!("{} wants to stop {}.", source.name, target)
            } else {
                format!("{} wants to stop {}.\nReason: {reason}", source.name, target)
            };
            let (reply, answer) = oneshot::channel();
            hub.send_cmd(&source.id, Cmd::Approve { title: "Stop another agent?".to_owned(), detail, reply })?;
            let Ok(true) = tokio::time::timeout(APPROVAL_TIMEOUT, answer).await else {
                bail!("the owner didn't allow the interruption; nothing was stopped");
            };
            // Full access never substitutes for the owner's approval, as with a deletion.
            if !crate::market::bot_deletion::may_approve(true, source.permission == crate::store::Permission::Auto) {
                bail!("only the owner can allow this interruption");
            }
        }
        Decision::Allow => {}
    }
    hub.stop_bot(to)?;
    Ok(json!({"stopped": true, "botId": to, "outcome": OUTCOME}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bot_may_stop_itself_and_the_bots_it_created() {
        assert_eq!(decide(true, true, false), Decision::Allow, "the caller owns the target");
        assert_eq!(decide(true, false, true), Decision::Allow, "the owner may stop any bot");
        assert_eq!(decide(true, false, false), Decision::OwnerApproval, "anyone else needs the owner");
    }

    #[test]
    fn a_bot_the_caller_cannot_act_on_is_never_stopped() {
        assert_eq!(decide(false, true, true), Decision::Deny, "hidden or deleted beats ownership");
        assert_eq!(decide(false, false, false), Decision::Deny);
    }

    #[test]
    fn the_tool_declares_an_optional_reason_and_a_fixed_outcome() {
        let tool = tool();
        assert_eq!(tool["name"], TOOL);
        let schema = &tool["inputSchema"];
        assert_eq!(schema["required"], json!(["botId"]));
        assert!(schema["properties"]["reason"].is_object(), "the reason is optional");
        assert_eq!(OUTCOME, "Interrupted the agent's running turn.");
    }
}
