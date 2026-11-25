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
    /// ChatGPT has no capacity for the request right now; an automatic backup can take it at once
    /// (OpenBot nightly triage-15582).
    Capacity,
    /// The provider this bot was using is no longer configured (its API address was removed). The chat
    /// carries on on the setup saved now, with the earlier conversation summed up (OpenBot 0.78.0).
    Unconfigured,
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
            Self::Capacity => "is at capacity",
            Self::Unconfigured => "is no longer configured",
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

/// Words a provider uses when it has no capacity for a request right now, as distinct from this
/// account's usage limit (OpenBot nightly triage-15582).
const CAPACITY_WORDS: [&str; 3] = ["at capacity", "capacity limit", "capacity reached"];

/// Whether an error text names capacity rather than a quota the account spent.
pub fn capacity_text(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    CAPACITY_WORDS.iter().any(|word| lower.contains(word))
}

/// Whether a capacity failure moves straight to the next backup model or account. Only ChatGPT's
/// provider does, and only with automatic backups configured and at least one backup to run on.
/// Pure, so the rule is tested without a bot or a store.
pub fn capacity_next(provider: &str, backups: usize, auto: bool) -> bool {
    auto && backups > 0 && matches!(provider.trim().to_ascii_lowercase().as_str(), "codex" | "chatgpt")
}

/// Adapter categories for a reply the bot cannot use, not a user-visible answer (OpenBot triage-15839,
/// `changelog.d/triage-15839.md`). `rpc_error` keeps only lowercase identifiers, so the spellings
/// adapters use are normalised to one set here.
const INVALID_REPLY_KINDS: [&str; 4] = ["invalid_reply", "invalid_response", "non_valid_reply", "not_valid_reply"];

/// Whether an unusable reply moves a backup-model agent to its next backup before it starts work.
/// Only with automatic backups and a backup left: a plain answer stays, and an internal/server error
/// keeps the existing `Reason::Overloaded` path. Pure, so the rule is tested without a bot or a store.
pub fn invalid_reply_fallback(error_kind: &str, auto: bool, backups: usize) -> bool {
    let kind = error_kind.trim().to_ascii_lowercase().replace('-', "_").replace(' ', "_");
    auto && backups > 0 && INVALID_REPLY_KINDS.contains(&kind.as_str())
}

/// Adapter categories that mean the provider the bot was using is no longer configured (its API
/// address was removed), so the chat should carry on with the setup saved now (OpenBot 0.78.0).
const UNCONFIGURED_KINDS: [&str; 6] = [
    "unconfigured",
    "not_configured",
    "no_provider",
    "provider_not_configured",
    "no_base_url",
    "custom_endpoint_missing",
];

/// Whether a failure kind is a removed configuration, not a refusal, a limit or a missing model.
pub fn unconfigured_recovery(kind: &str) -> bool {
    UNCONFIGURED_KINDS.iter().any(|name| kind.trim().eq_ignore_ascii_case(name))
}

/// The same failure named in text by an adapter that has no category for it.
fn unconfigured_text(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    ["no provider configured", "provider is not configured", "api address is not configured", "no base url"]
        .iter()
        .any(|word| lower.contains(word))
}

/// The adapter's own category on an error, so a caller can tell why it moved on without reading text.
pub fn error_kind(error: &anyhow::Error) -> &str {
    error.chain().find_map(|cause| cause.downcast_ref::<RpcError>()).and_then(|r| r.kind.as_deref()).unwrap_or_default()
}

/// Codex's settings reader can fail with a serde detail like "invalid data". That tells the person
/// nothing they can do: name the fix instead (OpenBot 0.78.0). Other errors keep their detail.
pub fn settings_failure_message(raw: &str) -> String {
    const PLAIN: &str = "couldn't load your ChatGPT settings; reconnect in Providers and try again";
    let lower = raw.to_ascii_lowercase();
    let settings_failure = lower.contains("invalid data")
        || lower.contains("invalid settings")
        || lower.contains("couldn't load model settings")
        || (lower.contains("settings") && lower.contains("expected value"))
        || (lower.contains("settings") && lower.contains("failed to parse"));
    if settings_failure { PLAIN.to_owned() } else { raw.to_owned() }
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
            | Reason::Capacity
            | Reason::Unconfigured
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
        | Reason::NotStarting
        | Reason::Capacity => Some("Not available now · skipped"),
        Reason::UsageLimit | Reason::OutOfCredits | Reason::SignedOut | Reason::ModelMissing | Reason::Unconfigured => {
            None
        }
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
    // A provider whose API address was removed is not a refusal to answer: the turn continues on the
    // setup saved now (OpenBot 0.78.0). Check the adapter's category and the text it sends.
    let kind = r.kind.as_deref().unwrap_or_default();
    if unconfigured_recovery(kind) || unconfigured_text(&r.message) {
        return Some(Reason::Unconfigured);
    }
    if capacity_text(&r.message) {
        return Some(Reason::Capacity);
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

    #[test]
    fn chatgpt_at_capacity_moves_on_only_with_automatic_backups() {
        // triage-15582: a capacity error is its own reason, and the next backup can take the turn
        // at once when the bot has backups configured.
        assert_eq!(of(&rpc(-32603, "ChatGPT is at capacity right now", None)), Some(Reason::Capacity));
        assert_eq!(of(&rpc(-32603, "The model has reached its capacity limit", None)), Some(Reason::Capacity));
        assert_eq!(of(&rpc(-32603, "Internal error: capacity reached", None)), Some(Reason::Capacity));
        assert_eq!(of(&rpc(-32603, "usage limit", None)), Some(Reason::UsageLimit), "a quota is not capacity");
        assert_eq!(skip_reason(Reason::Capacity), Some("Not available now · skipped"));
        assert!(continues_after_work(Reason::Capacity), "work already done can be carried on");

        assert!(capacity_next("codex", 2, true), "Codex with backups moves straight on");
        assert!(capacity_next("ChatGPT", 1, true));
        assert!(!capacity_next("codex", 0, true), "no backup to move to");
        assert!(!capacity_next("codex", 2, false), "backups off wait as before");
        assert!(!capacity_next("claude", 2, true), "only ChatGPT's capacity behaves this way");
    }

    #[test]
    fn an_invalid_reply_moves_on_only_with_automatic_backups() {
        // triage-15839: the provider's unusable reply used to leave the bot asking the person to
        // choose another model, even with backups set to switch without asking.
        assert_eq!(of(&rpc(-32603, "Internal error", Some("invalid_reply"))), None, "it has no reason today");
        for kind in ["invalid_reply", "INVALID_REPLY", " non-valid reply ", "not_valid_reply", "invalid_response"] {
            assert!(invalid_reply_fallback(kind, true, 1), "{kind}");
            assert!(invalid_reply_fallback(kind, true, 2), "{kind}");
        }
        assert!(!invalid_reply_fallback("invalid_reply", false, 2), "backups off keep the answer");
        assert!(!invalid_reply_fallback("invalid_reply", true, 0), "no backup keeps the original error");
        for kind in ["", "answer", "refusal", "invalid_request"] {
            assert!(!invalid_reply_fallback(kind, true, 2), "{kind} is a user-visible answer or a bad request");
        }
        for kind in ["internal_server_error", "server_error", "server_overloaded", "overloaded"] {
            assert!(!invalid_reply_fallback(kind, true, 2), "{kind} keeps its existing path");
            assert_eq!(of(&rpc(-32603, "Internal error", Some(kind))), Some(Reason::Overloaded), "{kind}");
        }
    }

    #[test]
    fn a_removed_provider_recovers_on_the_setup_saved_now() {
        for kind in ["unconfigured", "NOT_CONFIGURED", "no_provider", "provider_not_configured", "no_base_url"] {
            assert!(unconfigured_recovery(kind), "{kind}");
            assert_eq!(of(&rpc(-32603, "Internal error", Some(kind))), Some(Reason::Unconfigured), "{kind}");
        }
        assert_eq!(of(&rpc(-32603, "No provider configured for this model", None)), Some(Reason::Unconfigured));
        assert_eq!(of(&rpc(-32603, "The API address is not configured", None)), Some(Reason::Unconfigured));
        assert!(!unconfigured_recovery("invalid_request"));
        assert!(!unconfigured_recovery(""));
        assert_eq!(error_kind(&rpc(-32603, "Internal error", Some("unconfigured"))), "unconfigured");
        assert_eq!(error_kind(&anyhow::anyhow!("no session")), "");
        assert!(continues_after_work(Reason::Unconfigured));
    }

    #[test]
    fn a_codex_settings_load_error_says_what_to_do_not_the_serde_detail() {
        let plain = "couldn't load your ChatGPT settings; reconnect in Providers and try again";
        for raw in [
            "invalid data",
            "Failed to parse settings: expected value at line 1 column 4",
            "invalid settings file",
            "Couldn't load model settings: invalid data",
        ] {
            assert_eq!(settings_failure_message(raw), plain, "{raw}");
        }
        assert_eq!(settings_failure_message("The agent timed out"), "The agent timed out");
        assert_eq!(settings_failure_message(""), "");
    }
}
