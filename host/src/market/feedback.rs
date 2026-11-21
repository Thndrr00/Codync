//! The tool an agent uses to send a bug report or a feature idea to GitHub as a public issue.
//! Ported from OpenBot 0.74.0 in Codync's terms: the report names Codync, and the user reads and
//! approves the card before anything is sent. The prompt is built here, in one place, so the
//! language rule can be tested without a network call or an approval card.
use crate::agent::bot::Cmd;
use crate::chat::manage::APPROVAL_TIMEOUT;
use crate::hub::Hub;
use crate::store::BotConfig;
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::sync::Arc;
use tokio::sync::oneshot;

/// The name of the reporting tool.
pub const TOOL: &str = "send_feedback";

/// OpenBot 0.74.0: an agent writes every bug report and feature idea it sends to GitHub in English,
/// also when the user wrote to it in another language. The rule applies to every part of the report,
/// not just the body: a report is public and the maintainers read it in English.
pub const ENGLISH_RULE: &str =
    "Write the title, text and details in English, even when the user writes to you in another language.";

/// The tool definition handed to the agent. `title`, `text` and `details` all carry the English
/// rule, so a report in another language is never sent after the user approves the card.
pub fn tool() -> Value {
    json!({
        "name": TOOL,
        "description": "Send a bug report or a feature idea for Codync itself as a public GitHub \
            issue. Codync adds the version, system, model and errors. The user reads and approves \
            the text before it is sent.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "kind": {"type": "string", "enum": ["bug", "idea"],
                    "description": "bug: something in Codync does not work. idea: a feature request or another change the user wants."},
                "title": {"type": "string", "minLength": 1, "maxLength": 200,
                    "description": format!("A short summary, with no prefix: the feature and the symptom. {ENGLISH_RULE}")},
                "text": {"type": "string", "minLength": 1, "maxLength": 1000,
                    "description": format!("In one to three plain sentences. For a bug: what went wrong and what the user expected, with the exact error text. For an idea: what the user wants Codync to do. {ENGLISH_RULE}")},
                "details": {"type": "string", "minLength": 1, "maxLength": 1000,
                    "description": format!("For a bug: what the user did before it, as short numbered steps. For an idea: the problem it solves. Leave it out when you do not know. {ENGLISH_RULE}")},
            },
            "required": ["kind", "title", "text"],
        },
    })
}

/// Codync's own repository, where these reports go.
const REPO: &str = "Thndrr00/Codync";

/// How the person's answer on the card ended. Ported from OpenBot nightly: dismissing a bug report or
/// idea card withdraws it before anything is sent, just like denying it or letting it time out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    /// The person tapped Allow: send the report.
    Allowed,
    /// The person tapped Deny or dismissed the card: withdraw the send.
    Withdrawn,
    /// Nobody answered before the request timed out or the tool stopped waiting: withdraw the send.
    Expired,
}

/// Whether the report may be sent. Pure, so every way of withdrawing it has a test without a host or a
/// network call: only an explicit Allow sends, everything else leaves the card's report unsent.
pub(crate) fn may_send(outcome: Outcome) -> bool {
    outcome == Outcome::Allowed
}

/// Shows the whole report on a card and opens the public issue only after the person allows it. The
/// card is asked for even under Auto: a public post is never automatic. Dismissing or letting the card
/// lapse withdraws the send; this reuses the same `Cmd::Approve` answer/oneshot as any other approval.
pub async fn perform(hub: &Hub, source: &BotConfig, args: &Value) -> Result<Value> {
    let kind = args["kind"].as_str().filter(|k| *k == "bug" || *k == "idea").context("kind must be bug or idea")?;
    let title = args["title"].as_str().map(str::trim).filter(|t| !t.is_empty()).context("a title is required")?;
    let text = args["text"].as_str().map(str::trim).filter(|t| !t.is_empty()).context("a report is required")?;
    let detail = format!("{kind}: {title}\n\n{text}\n\n{}", args["details"].as_str().unwrap_or_default());
    let (reply, answer) = oneshot::channel();
    hub.send_cmd(&source.id, Cmd::Approve { title: "Send a public bug report or idea?".to_owned(), detail, reply })?;
    // A dropped oneshot (card withdrawn, tool stopped waiting) and a timeout both withdraw the send,
    // the same way `manage::approve` treats them as "not allowed".
    let outcome = match tokio::time::timeout(APPROVAL_TIMEOUT, answer).await {
        Ok(Ok(true)) => Outcome::Allowed,
        Ok(Ok(false)) => Outcome::Withdrawn,
        Ok(Err(_)) | Err(_) => Outcome::Expired,
    };
    if !may_send(outcome) {
        bail!("the report was withdrawn; nothing was sent");
    }
    let token = crate::github::connection(&hub.store)?.context("GitHub is not configured")?.token;
    let url = format!("https://api.github.com/repos/{REPO}/issues");
    let client = crate::egress::client(crate::egress::Policy::Only(&["api.github.com"]), &url).await?;
    let response = client
        .post(&url)
        .bearer_auth(token)
        .header("accept", "application/vnd.github+json")
        .header("user-agent", "codync")
        .json(&json!({"title": format!("[{kind}] {title}"), "body": text}))
        .timeout(std::time::Duration::from_secs(20))
        .send()
        .await?;
    if !response.status().is_success() {
        bail!("GitHub answered {}", response.status());
    }
    let issue: Value = response.json().await.unwrap_or_else(|_| json!({}));
    Ok(json!({"url": issue["html_url"].clone()}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_part_of_a_github_report_asks_for_english() {
        let tool = tool();
        assert_eq!(tool["name"], TOOL);
        let properties = &tool["inputSchema"]["properties"];
        for field in ["title", "text", "details"] {
            let description = properties[field]["description"].as_str().unwrap();
            assert!(
                description.contains("in English") && description.contains("another language"),
                "{field} must ask for English: {description}"
            );
        }
        assert_eq!(tool["inputSchema"]["required"], json!(["kind", "title", "text"]));
    }

    #[test]
    fn only_an_explicit_allow_sends_a_dismissed_report_is_withdrawn() {
        assert!(may_send(Outcome::Allowed));
        assert!(!may_send(Outcome::Withdrawn), "dismissing the card cancels the send");
        assert!(!may_send(Outcome::Expired), "a lapsed card cancels the send too");
    }
}
