//! Pi's signed-in model can refuse a chat that no longer fits its context (OpenBot 0.70). Retrying
//! the same session only repeats the refusal, so the host opens a fresh one carrying the history.

/// Plain markers a model uses when a chat is too long. No error text is logged or sent anywhere.
const OVERFLOW: &[&str] = &[
    "context length",
    "context limit",
    "context overflow",
    "context window",
    "maximum context",
    "max context",
    "too many tokens",
    "token limit exceeded",
    "request too large",
    "prompt too long",
    "history is too long",
];

/// Pure: does `error` say a signed-in model refused an over-long chat?
pub(crate) fn context_overflow(error: &str) -> bool {
    let lower = error.to_ascii_lowercase();
    OVERFLOW.iter().any(|marker| lower.contains(marker))
}

/// Pure: for this backend, does the error ask for a fresh session that carries the history?
/// Only Pi's own model recovers this way; other agents keep their own retry and fallback rules.
pub fn restart_with_history(backend: &str, error: &str) -> bool {
    (backend == "pi" || backend == "pi-acp") && context_overflow(error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_over_long_chat_is_kept_apart_from_other_errors() {
        for error in [
            "400 context length exceeded: 128000 > 64000",
            "This model's maximum context length is 200000 tokens",
            "Context overflow — start a new conversation",
            "the request too large, shorten the prompt",
            "PROMPT TOO LONG",
            "chat history is too long to fit",
        ] {
            assert!(context_overflow(error), "{error}");
        }
        for error in ["", "not signed in", "network timeout", "model not found", "insufficient quota"] {
            assert!(!context_overflow(error), "{error}");
        }
    }

    #[test]
    fn only_pi_restarts_with_history() {
        let error = "context length exceeded";
        assert!(restart_with_history("pi", error));
        assert!(restart_with_history("pi-acp", error), "its registry id is Pi too");
        for backend in ["codex", "claude", "vibe", "custom"] {
            assert!(!restart_with_history(backend, error), "{backend}");
        }
        assert!(!restart_with_history("pi", "model not found"));
    }
}
