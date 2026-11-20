//! The management tool that lets an agent delete other agents. Ported from OpenBot 0.74.0 in
//! Codync's terms: one approval card names every agent to delete, even several at once, the card is
//! required even when the asking agent has Full access, and only the owner can approve it. The
//! bot's deletion path itself stays in `hub::delete_bot`.
use crate::agent::bot::Cmd;
use crate::chat::manage::APPROVAL_TIMEOUT;
use crate::hub::Hub;
use crate::store::BotConfig;
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::sync::Arc;
use tokio::sync::oneshot;

/// The name of the management tool.
pub const TOOL: &str = "delete_bots";

/// The most agents one card may name. Beyond this the agent should ask in smaller groups.
pub const MAX_AGENTS: usize = 20;

pub fn tool() -> Value {
    json!({
        "name": TOOL,
        "description": "Delete other Codync agents, several at once when needed. One approval card \
            names every agent and the owner must approve it, even when you have Full access. Only \
            the owner can approve a deletion.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "botIds": {"type": "array", "minItems": 1, "maxItems": MAX_AGENTS,
                    "items": {"type": "string"},
                    "description": "The ids of the agents to delete, from list_team. One card names them all."},
            },
            "required": ["botIds"],
        },
    })
}

/// The agent ids one request names: trimmed, in the order given and without repeats. An empty list
/// is refused; so is more than [`MAX_AGENTS`], so one card stays readable.
pub fn requested(args: &Value) -> Option<Vec<String>> {
    let ids = args["botIds"].as_array()?;
    let mut out: Vec<String> = Vec::new();
    for id in ids {
        let Some(id) = id.as_str().map(str::trim).filter(|id| !id.is_empty()) else { return None };
        if !out.iter().any(|seen| seen == id) {
            out.push(id.to_owned());
        }
    }
    (!out.is_empty() && out.len() <= MAX_AGENTS).then_some(out)
}

/// OpenBot 0.74.0: one approval card names all of the agents. Full access of the asking agent does
/// not skip it.
pub fn approval_card(agents: &[String]) -> Option<Value> {
    if agents.is_empty() || agents.len() > MAX_AGENTS {
        return None;
    }
    Some(json!({
        "kind": "deleteAgents",
        "title": (if agents.len() == 1 { "Delete an agent" } else { "Delete agents" }),
        "agents": agents,
        "note": "Only the owner can approve this deletion.",
    }))
}

/// Only the owner may approve, whoever asked and whatever access the asking agent has.
pub fn may_approve(approver_is_owner: bool, asking_agent_full_access: bool) -> bool {
    let _ = asking_agent_full_access; // Full access never substitutes for the owner's approval.
    approver_is_owner
}

/// Whether a deletion error is a locked folder rather than some other failure. Windows reports EBUSY (OS error
/// 32) when a program still holds a file in the agent's folder.
fn folder_locked(error: &str) -> bool {
    let lower = error.to_ascii_lowercase();
    lower.contains("ebusy")
        || lower.contains("resource busy")
        || lower.contains("os error 32")
        || lower.contains("in use")
        || lower.contains("locked")
}

/// What to tell the agent when a deletion fails. OpenBot 0.75.0 (#13220): on Windows, an EBUSY that no known
/// provider process causes means a program the agent started still holds its folder. Retrying fails the same
/// way, so name the step that frees it, and tell the agent to stop rather than delete the folder itself.
pub fn deletion_error(error: &str, is_windows: bool) -> String {
    if is_windows && folder_locked(error) {
        return "Another program still uses this agent's folder, for example a program that the agent started. \
                Close that program or restart the computer that this agent runs on, then delete the agent again. \
                Do not delete its folders or files another way."
            .into();
    }
    // Other systems can get EBUSY from a mount point, where "another program" would be wrong; keep their error,
    // but still stop the agent from trying to remove the folder with its own shell commands.
    format!("{error} Do not delete the agent's folders or files another way.")
}

/// Runs the management tool: one card naming every agent, asked even when the bot has Full access,
/// then the deletion. The card goes to the owner's chat, so only they can answer it.
pub async fn perform(hub: &Hub, source: &BotConfig, args: &Value) -> Result<Value> {
    let ids = requested(args).context("Give between 1 and 20 agent ids from list_team")?;
    let card = approval_card(&ids).context("Too many agents for one card")?;
    // Unlike `manage::approve`, do not skip the card under Auto: a deletion always needs its owner.
    let (reply, answer) = oneshot::channel();
    hub.send_cmd(
        &source.id,
        Cmd::Approve {
            title: card["title"].as_str().unwrap_or("Delete agents").to_owned(),
            detail: format!("{}\n\n{}", card["note"].as_str().unwrap_or_default(), ids.join("\n")),
            reply,
        },
    )?;
    let Ok(true) = tokio::time::timeout(APPROVAL_TIMEOUT, answer).await else {
        bail!("the owner didn't allow the deletion; nothing changed");
    };
    if !may_approve(true, source.permission == crate::store::Permission::Auto) {
        bail!("only the owner can approve a deletion");
    }
    for id in &ids {
        if let Err(error) = hub.delete_bot(id) {
            bail!("couldn't delete {id}: {}", deletion_error(&format!("{error:#}"), cfg!(windows)));
        }
    }
    Ok(json!({"deleted": ids}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_card_names_every_agent_even_with_full_access() {
        let agents = vec!["a".to_owned(), "b".to_owned(), "c".to_owned()];
        let card = approval_card(&agents).expect("a card for several agents");
        assert_eq!(card["kind"], "deleteAgents");
        assert_eq!(card["agents"], json!(agents));
        assert_eq!(card["title"], "Delete agents");
        assert_eq!(approval_card(&vec!["a".to_owned()]).unwrap()["title"], "Delete an agent");
        assert!(approval_card(&[]).is_none());
        assert!(may_approve(true, true), "the owner approves, even when the agent has Full access");
        assert!(!may_approve(false, true), "Full access never approves on the owner's behalf");
        assert!(!may_approve(false, false));
    }

    #[test]
    fn a_windows_locked_folder_says_what_frees_it() {
        // OpenBot 0.75.0 (#13220): EBUSY from a program the agent started told the agent only to retry, which
        // failed the same way. Name the step that frees the folder, and stop the agent deleting it itself.
        let locked = deletion_error("couldn't remove the agent folder: EBUSY (os error 32)", true);
        assert!(locked.contains("Another program still uses this agent's folder"));
        assert!(locked.contains("restart the computer that this agent runs on"));
        assert!(locked.contains("Do not delete its folders or files another way"));

        // Other systems keep their earlier error, and a non-lock Windows error isn't misreported as a lock.
        let other = deletion_error("permission denied", false);
        assert!(other.starts_with("permission denied"));
        assert!(other.contains("Do not delete the agent's folders or files another way"));
        assert!(deletion_error("permission denied", true).starts_with("permission denied"));
    }

    #[test]
    fn a_request_names_the_agents_without_repeats_or_blanks() {
        let args = json!({"botIds": [" a ", "b", "a", ""]});
        assert_eq!(requested(&args), Some(vec!["a".to_owned(), "b".to_owned()]));
        assert_eq!(requested(&json!({"botIds": ["a"]})), Some(vec!["a".to_owned()]));
        assert_eq!(requested(&json!({"botIds": []})), None);
        assert_eq!(requested(&json!({})), None);
        let too_many = (0..=MAX_AGENTS).map(|i| format!("b{i}")).collect::<Vec<_>>();
        assert_eq!(requested(&json!({"botIds": too_many.clone()})), None);
        assert!(approval_card(&too_many).is_none());
    }
}
