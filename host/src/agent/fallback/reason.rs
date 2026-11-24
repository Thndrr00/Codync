//! Which failures move a bot on to its next agent, read from what the agent answered over ACP: the JSON-RPC code,
//! the adapter's category (`errorKind`, codex-acp's `codexErrorInfo`), and the host's own `Failure`. The agents
//! retry what is worth retrying before they answer with an error, so Codync never asks the same agent again.

use crate::agent::acp::{AUTH_REQUIRED, Failure, FailureKind, RpcError};
use serde::{Deserialize, Serialize};

/// Why an agent couldn't answer, and how long it rests for it (see `rest`).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Reason {
    /// The plan's usage limit: it comes back when the limit resets.
    UsageLimit,
    /// No credits left, or the account is on hold.
    OutOfCredits,
    RateLimited,
    /// The provider is overloaded or failing.
    Overloaded,
    /// The connection to the provider dropped (OpenBot nightly triage-14512). A Codex stream that
    /// disconnects with no HTTP status is a network error even when its text names HTTP 503.
    Network,
    /// A request to the provider timed out after work began (triage-14512).
    Timeout,
    SignedOut,
    /// The provider refused a saved API key or sign-in; the user has to replace it in Settings
    /// (OpenBot nightly triage-14253). The rest backs off up to an hour and ends when the key changes.
    RefusedKey,
    /// The agent didn't start, stopped answering, or exited.
    NotStarting,
    /// The agent doesn't have the step's model.
    ModelMissing,
}

impl Reason {
    /// How the chat says it: "Claude Code reached its usage limit".
    pub fn phrase(self) -> &'static str {
        match self {
            Self::UsageLimit => "reached its usage limit",
            Self::OutOfCredits => "is out of credits",
            Self::RateLimited => "is rate limited",
            Self::Overloaded => "is overloaded",
            Self::Network => "lost its connection",
            Self::Timeout => "timed out",
            Self::SignedOut => "is signed out",
            Self::RefusedKey => "refused the saved API key",
            Self::NotStarting => "couldn't start",
            Self::ModelMissing => "doesn't have this bot's model",
        }
    }
}

/// How Claude's adapter words a usage limit (the Agent SDK's `USAGE_LIMIT_ERROR_PREFIXES`); it reports it as a
/// `rate_limit` or `billing_error` with this text in the message.
const USAGE_LIMIT_TEXT: [&str; 12] = [
    "You've hit your",
    "You've reached your",
    "You're out of usage credits",
    "Your org is out of usage",
    "Your seat type doesn't include usage credits",
    "Your seat type doesn't include usage",
    "Your usage allocation has been disabled by your admin",
    "Your group's usage limit is set to $0",
    "requires usage credits",
    "You're out of extra usage",
    "Your seat type doesn't include extra usage",
    "usage limit",
];

/// HTTP 410 Gone: a model the provider retired, so the turn moves to the next step's model. OpenBot 0.75.0
/// (#12998); the same cause as `model_not_found`, which — like every reason — moves on before work.
const GONE: i64 = 410;

/// Words an adapter uses for a model that no longer exists, besides `model_not_found`: Gemini's
/// "models/<id> is not found", "model … was retired", "model … is no longer available/supported".
const GONE_WORDS: [&str; 5] =
    ["retired", "no longer available", "no longer supported", "is not found", "does not exist"];

/// Whether the answer says the model is gone. A bare `410` (as a whole number) is HTTP 410 Gone; the words only
/// count when the message is about a model, so an unrelated "410" in an internal error isn't a fallback cause.
fn gone_text(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    let has_410 = lower.split(|c: char| !c.is_ascii_digit()).any(|part| part == "410");
    has_410 || (lower.contains("model") && GONE_WORDS.iter().any(|word| lower.contains(word)))
}

/// A dropped connection, named in the error text. A Codex stream that disconnects with no HTTP
/// status is a network error even when the text names HTTP 503 (triage-14512): a structured
/// `overloaded` kind is read first, so only a bare status falls through to this check.
const CONNECTION_WORDS: [&str; 12] = [
    "connection error",
    "connection reset",
    "connection closed",
    "disconnect",
    "network error",
    "socket hang up",
    "econnreset",
    "broken pipe",
    "unexpected eof",
    "eof occurred",
    "fetch failed",
    "stream ended unexpectedly",
];

/// Whether an error text names a dropped connection rather than a refusal or a usage limit.
pub fn network_text(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    CONNECTION_WORDS.iter().any(|word| lower.contains(word)) || lower.contains("http 503")
}

/// Whether an error text names a timeout (triage-14512).
pub fn timeout_text(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    ["timed out", "timeout", "time out", "deadline exceeded"].iter().any(|word| lower.contains(word))
}

/// Words a provider uses when it refuses a saved key or sign-in rather than just reporting that
/// nobody is signed in (triage-14253). Such a failure is told to the user and backed off, not
/// retried every ten minutes like an outage.
const REFUSED_KEY_WORDS: [&str; 10] = [
    "invalid api key",
    "invalid_api_key",
    "api key is invalid",
    "api key has been revoked",
    "please run /login",
    "bad credentials",
    "invalid credentials",
    "authentication failed",
    "credentials are invalid",
    "key was refused",
];

/// Whether an authentication error text names a refused saved credential.
pub fn refused_credential_text(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    REFUSED_KEY_WORDS.iter().any(|word| lower.contains(word))
}

/// The reasons that let a turn which already started work move on to the backup model
/// (OpenBot's `CONTINUE_CAUSES`, extended with network and timeout in triage-14512). A turn that
/// did nothing may be retried on the backup for any fallback reason; one that did work needs a
/// cause the backup can plausibly carry on from. Pure, so the set has a test.
pub fn continues_after_work(reason: Reason) -> bool {
    matches!(
        reason,
        Reason::UsageLimit
            | Reason::OutOfCredits
            | Reason::RateLimited
            | Reason::Overloaded
            | Reason::Network
            | Reason::Timeout
            | Reason::SignedOut
            | Reason::RefusedKey
            | Reason::NotStarting
    )
}

/// OpenBot 0.66.0: a backup that failed a short time ago is shown as "Not available now ·
/// skipped" instead of a count-down. A plan limit, a sign-out and a missing model have their own
/// explanations, so they keep the ordinary text. Pure, tested.
pub fn skip_reason(reason: Reason) -> Option<&'static str> {
    match reason {
        Reason::RateLimited
        | Reason::Overloaded
        | Reason::Network
        | Reason::Timeout
        | Reason::RefusedKey
        | Reason::NotStarting => Some("Not available now · skipped"),
        Reason::UsageLimit | Reason::OutOfCredits | Reason::SignedOut | Reason::ModelMissing => None,
    }
}

/// Why `error` moves the bot on, or `None` when it shouldn't (a stop, a refusal, a bug in the request).
pub fn of(error: &anyhow::Error) -> Option<Reason> {
    for cause in error.chain() {
        if let Some(f) = cause.downcast_ref::<Failure>() {
            return Some(match f.kind {
                FailureKind::NotSetUp | FailureKind::Exited => Reason::NotStarting,
                // triage-14512: a request that timed out after work began carries on on the backup,
                // so it needs its own reason rather than the start-time "couldn't start".
                FailureKind::Timeout => Reason::Timeout,
                FailureKind::NoModel => Reason::ModelMissing,
            });
        }
        if let Some(r) = cause.downcast_ref::<RpcError>() {
            return of_rpc(r);
        }
    }
    None
}

/// Why an agent's error answer moves the bot on (see [`of`]).
pub fn of_rpc(r: &RpcError) -> Option<Reason> {
    if r.code == AUTH_REQUIRED {
        return Some(Reason::SignedOut);
    }
    // A retired model answers 410 Gone (OpenBot 0.75.0, #12998). An adapter may report that as the
    // JSON-RPC code or only in its message, so check both before any other cause.
    if r.code == GONE || gone_text(&r.message) {
        return Some(Reason::ModelMissing);
    }
    let usage_text = USAGE_LIMIT_TEXT.iter().any(|t| r.message.contains(t));
    // Claude's adapter sends the Agent SDK's assistant error (`rate_limit`…); codex-acp its `codexErrorInfo`
    // (`usage_limit_exceeded`…); CodeBuddy its `category` (`auth`…).
    Some(match r.kind.as_deref() {
        Some("usage_limit_exceeded") => Reason::UsageLimit,
        Some("rate_limit" | "billing_error" | "account_on_hold") if usage_text => Reason::UsageLimit,
        Some("billing_error" | "account_on_hold") => Reason::OutOfCredits,
        Some("rate_limit" | "rate_limit_exceeded") => Reason::RateLimited,
        Some(
            "overloaded"
            | "server_error"
            | "server_overloaded"
            | "internal_server_error"
            | "response_too_many_failed_attempts",
        ) => Reason::Overloaded,
        Some("authentication_failed" | "unauthorized" | "auth" | "custom_model_auth")
            if refused_credential_text(&r.message) =>
        {
            Reason::RefusedKey
        }
        Some("authentication_failed" | "oauth_org_not_allowed" | "unauthorized" | "auth") => Reason::SignedOut,
        Some("model_not_found") => Reason::ModelMissing,
        _ if usage_text => Reason::UsageLimit,
        // triage-14512: a dropped connection (even one whose text names HTTP 503 with no status)
        // or a timeout is a fallback cause, so the backup carries the work the turn already did.
        _ if network_text(&r.message) => Reason::Network,
        _ if timeout_text(&r.message) => Reason::Timeout,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rpc(code: i64, message: &str, kind: Option<&str>) -> anyhow::Error {
        RpcError { code, message: message.into(), kind: kind.map(Into::into) }.into()
    }

    #[test]
    fn claude_and_codex_usage_limits_move_on() {
        let claude =
            rpc(-32603, "Internal error: You've hit your limit · resets 3pm (Asia/Taipei)", Some("rate_limit"));
        assert_eq!(of(&claude), Some(Reason::UsageLimit));
        assert_eq!(of(&rpc(-32603, "Internal error", Some("usage_limit_exceeded"))), Some(Reason::UsageLimit));
        assert_eq!(
            of(&rpc(-32603, "Internal error: Your org is out of usage · add funds to continue", Some("billing_error"))),
            Some(Reason::UsageLimit)
        );
        assert_eq!(
            of(&rpc(-32603, "Internal error: credit balance too low", Some("billing_error"))),
            Some(Reason::OutOfCredits)
        );
    }

    #[test]
    fn transient_and_access_failures_move_on() {
        assert_eq!(of(&rpc(-32603, "Internal error: 429", Some("rate_limit"))), Some(Reason::RateLimited));
        assert_eq!(of(&rpc(-32603, "Internal error", Some("overloaded"))), Some(Reason::Overloaded));
        assert_eq!(of(&rpc(-32603, "Internal error", Some("server_overloaded"))), Some(Reason::Overloaded));
        assert_eq!(of(&rpc(AUTH_REQUIRED, "Authentication required", None)), Some(Reason::SignedOut));
        assert_eq!(of(&rpc(-32603, "Internal error", Some("model_not_found"))), Some(Reason::ModelMissing));
        let gone = Failure::error(FailureKind::Exited, "agent process exited");
        assert_eq!(of(&gone.context("prompting")), Some(Reason::NotStarting));
        let no_model = Failure::error(FailureKind::NoModel, "Couldn't select model gpt-5.5: not supported");
        assert_eq!(of(&no_model), Some(Reason::ModelMissing));
    }

    #[test]
    fn a_retired_model_or_http_410_moves_on_to_the_next_model() {
        // OpenBot 0.75.0 (#12998): 410 Gone is a fallback cause; so are the words adapters use for a
        // model that was retired instead of returning a status code.
        assert_eq!(of(&rpc(GONE, "Gone", None)), Some(Reason::ModelMissing));
        assert_eq!(of(&rpc(-32603, "HTTP 410: model gpt-5.5 is gone", None)), Some(Reason::ModelMissing));
        assert_eq!(of(&rpc(-32603, "models/gemini-3-pro is not found", None)), Some(Reason::ModelMissing));
        assert_eq!(of(&rpc(-32603, "The model has been retired", None)), Some(Reason::ModelMissing));
        assert_eq!(of(&rpc(-32603, "model o3 is no longer supported", None)), Some(Reason::ModelMissing));
        // An unrelated number that merely contains 410, and a non-model error, still stay put.
        assert_eq!(of(&rpc(-32603, "request 1410 rejected", None)), None);
        assert_eq!(of(&rpc(-32603, "The file has been retired from the index", None)), None);
    }

    #[test]
    fn a_dropped_connection_or_a_timeout_carries_work_to_the_backup() {
        // triage-14512: with no structured status, a Codex stream that disconnects is a network
        // error even when it names HTTP 503.
        assert_eq!(of(&rpc(-32603, "API Error: Connection error.", None)), Some(Reason::Network));
        assert_eq!(of(&rpc(-32603, "HTTP 503: stream disconnected", None)), Some(Reason::Network));
        assert_eq!(of(&rpc(-32603, "Request timed out after 30s", None)), Some(Reason::Timeout));
        // A structured overload is read before the bare 503 text.
        assert_eq!(of(&rpc(-32603, "HTTP 503: overloaded", Some("overloaded"))), Some(Reason::Overloaded));
        // A host-side timeout is its own reason now, not a start failure.
        let timed_out = Failure::error(FailureKind::Timeout, "no answer");
        assert_eq!(of(&timed_out), Some(Reason::Timeout));
        for reason in [Reason::Network, Reason::Timeout, Reason::Overloaded, Reason::SignedOut, Reason::UsageLimit] {
            assert!(continues_after_work(reason), "{reason:?} continues a turn that did work");
        }
        assert!(!continues_after_work(Reason::ModelMissing), "a missing model is a setup issue");
    }

    #[test]
    fn a_refused_saved_key_is_told_to_the_user_not_treated_as_a_plain_sign_out() {
        // triage-14253: the e2e failure text is "Invalid API key · Please run /login".
        assert_eq!(
            of(&rpc(-32603, "Invalid API key · Please run /login", Some("authentication_failed"))),
            Some(Reason::RefusedKey)
        );
        assert_eq!(of(&rpc(AUTH_REQUIRED, "Authentication required", None)), Some(Reason::SignedOut));
        assert_eq!(of(&rpc(-32603, "Authentication failed", Some("oauth_org_not_allowed"))), Some(Reason::SignedOut));
        assert!(continues_after_work(Reason::RefusedKey));
    }

    #[test]
    fn only_a_recent_failure_is_shown_as_skipped() {
        // OpenBot 0.66.0: the label is about an error, not about a plan limit or a sign-out.
        for reason in [
            Reason::RateLimited,
            Reason::Overloaded,
            Reason::Network,
            Reason::Timeout,
            Reason::RefusedKey,
            Reason::NotStarting,
        ] {
            assert_eq!(skip_reason(reason), Some("Not available now · skipped"), "{reason:?}");
        }
        for reason in [Reason::UsageLimit, Reason::OutOfCredits, Reason::SignedOut, Reason::ModelMissing] {
            assert_eq!(skip_reason(reason), None, "{reason:?} has its own wording");
        }
    }

    #[test]
    fn a_bad_request_or_an_uncategorized_error_stays() {
        assert_eq!(of(&rpc(-32603, "Internal error", None)), None);
        assert_eq!(of(&rpc(-32603, "Internal error", Some("invalid_request"))), None);
        assert_eq!(of(&rpc(-32602, "Invalid params", None)), None);
        assert_eq!(of(&anyhow::anyhow!("no session")), None);
    }
}
