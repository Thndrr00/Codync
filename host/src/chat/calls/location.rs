//! The voice's `request_location` tool (OpenBot nightly v0.77.0): the agent gets the shared place only
//! after the person answers the card. The reason is bounded here and never sent to analytics.

use crate::hub::Hub;
use crate::store::BotConfig;
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;

/// How long the tool waits for an answer. Shorter than the client's 60 s `callTool` budget
/// (`apps/desktop/src/renderer/views/call/call-link.ts`). A timeout leaves the card answerable.
pub const WAIT: Duration = Duration::from_secs(30);

/// Longest reason kept on the card; longer text is cut, not logged.
const MAX_PURPOSE: usize = 200;

/// The pure decision for a `request_location` call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Request {
    /// Show a card with this bounded reason.
    Ask(String),
    /// No card. The code is fixed, never the reason's text, so analytics cannot carry content.
    Refuse(&'static str),
}

/// Bounds the agent's reason before it becomes a card title. Pure, tested.
pub fn validate_request(purpose: &str) -> Request {
    let trimmed = purpose.trim();
    if trimmed.is_empty() {
        return Request::Refuse("missing reason");
    }
    if trimmed.chars().any(char::is_control) {
        return Request::Refuse("unusable reason");
    }
    Request::Ask(trimmed.chars().take(MAX_PURPOSE).collect())
}

/// Asks through the existing card path and returns the shared place to the voice. An answer after the wait
/// is delivered by `external::location::share` as the user's next message instead (the CAPTCHA-card fix).
pub async fn perform(hub: &Arc<Hub>, call: &str, cfg: &BotConfig, purpose: &str) -> Value {
    let purpose = match validate_request(purpose) {
        Request::Ask(purpose) => purpose,
        Request::Refuse(code) => return json!({"error": format!("No location card: {code}.")}),
    };
    let Some(pending) = crate::external::location::request(hub, call, &purpose, cfg) else {
        return json!({"error": "This location was already asked for and answered."});
    };
    match tokio::time::timeout(WAIT, pending.answer).await {
        Ok(Ok(text)) => json!({"location": text}),
        Ok(Err(_)) => json!({"error": "The location card was closed before it was answered."}),
        Err(_) => json!({
            "location": null,
            "note": "The card stays in the chat; a later answer reaches the agent as the next message.",
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reason_is_trimmed_and_bounded() {
        let long = "x".repeat(MAX_PURPOSE + 50);
        let Request::Ask(purpose) = validate_request(&format!("  {long}  ")) else { panic!("an answerable card") };
        assert_eq!(purpose.chars().count(), MAX_PURPOSE);
        assert_eq!(Request::Ask("find a café".into()), validate_request("  find a café \n"));
    }

    #[test]
    fn an_unusable_reason_leaves_no_card_and_no_content_to_log() {
        assert_eq!(validate_request("   "), Request::Refuse("missing reason"));
        assert_eq!(validate_request("\u{0}"), Request::Refuse("unusable reason"));
        for Request::Refuse(code) in [validate_request(""), validate_request("here\u{0}")] {
            assert!(code.chars().all(|c| c.is_ascii_lowercase() || c == ' '), "{code}");
        }
    }
}
