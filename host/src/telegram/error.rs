//! A Telegram post can fail before it ever reaches Telegram.
//!
//! OpenBot nightly triage-13542: when the service the host talks through is offline, a failed
//! Telegram post used to reach the agent as "The platform refused the call: telegram_503", so it
//! blamed Telegram and checked the wrong things. Here the failure is told apart from a real Bot API
//! refusal, and the agent gets the check it can actually do. The classifier is pure so the wording
//! can be tested without a service, a token or a network call.

use anyhow::Error;

/// What caused a Telegram call to fail.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Failure {
    /// The host never reached Telegram: its own service or egress was down.
    ServiceOffline,
    /// Telegram answered and refused (or the call failed on Telegram's side).
    Platform,
}

/// What the agent is told when the host's service is the cause (never the user's data).
pub(super) const SERVICE_MESSAGE: &str = "Codync is not connected to its service, so Telegram could not be called. Telegram is not the cause. Check this computer's connection to Codync, then try again.";

/// Classifies a call's error text. Service failures are the ones that never got a Bot API answer:
/// the egress guard, an offline relay or a proxy. A `ok: false` body stays a platform failure.
pub(super) fn classify(error: &str) -> Failure {
    let lower = error.to_ascii_lowercase();
    let service = [
        "service is unavailable",
        "not connected to its service",
        "service offline",
        "could not reach the service",
        "no route to service",
        "egress policy",
        "relay is offline",
        "codync is offline",
    ];
    if service.iter().any(|needle| lower.contains(needle)) { Failure::ServiceOffline } else { Failure::Platform }
}

/// Rewrites only a service failure for the agent; a Telegram refusal keeps its own detail.
pub(super) fn agent_error(error: Error) -> Error {
    if classify(&format!("{error:#}")) == Failure::ServiceOffline { anyhow::anyhow!(SERVICE_MESSAGE) } else { error }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_offline_service_is_not_blamed_on_telegram() {
        let error = anyhow::anyhow!("Telegram sendMessage: the service is unavailable (egress policy)");
        assert_eq!(classify(&format!("{error:#}")), Failure::ServiceOffline);
        let message = format!("{:#}", agent_error(error));
        assert!(message.contains("not connected to its service"));
        assert!(message.contains("Telegram is not the cause"));
        assert!(message.contains("Check"));
    }

    #[test]
    fn a_telegram_refusal_keeps_its_own_words() {
        for text in [
            "Telegram sendMessage: chat not found",
            "Telegram sendMessage: 429 Too Many Requests",
            "Telegram sendMessage: bot token revoked",
        ] {
            assert_eq!(classify(text), Failure::Platform);
            assert_eq!(format!("{:#}", agent_error(anyhow::anyhow!(text))), text);
        }
    }

    #[test]
    fn an_old_telegram_503_without_a_service_cause_is_still_a_platform_failure() {
        // The point of the fix: don't guess from the status code alone.
        assert_eq!(classify("The platform refused the call: telegram_503"), Failure::Platform);
    }
}
