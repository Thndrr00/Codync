//! What a failed workflow step keeps: its full bounded error and a stable failure kind.
//! Ported from OpenBot 0.79.0's "show the full workflow step error and a what-to-do hint": a
//! provider's answer can put the useful line below the first three, so cutting there hides the
//! reason. The host keeps the whole text (bounded so a run row stays small) and names the kind for
//! the history and its filters. No content from the error is sent to analytics.

/// The most characters of one step's error the host keeps. A bound, not a line cut: every line of
/// the provider's answer survives up to this limit.
const MAX_ERROR_CHARS: usize = 8_000;

/// A failed step's error as it is stored and shown.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepError {
    /// The full error, with its line breaks, trimmed and bounded.
    pub text: String,
    /// A stable reason for the failure: `timeout`, `limit`, `auth`, `permission`, `network`,
    /// `model`, `cancelled` or `error`.
    pub kind: &'static str,
}

impl StepError {
    pub fn as_str(&self) -> &str {
        &self.text
    }
}

/// Bounds and classifies a failed step's error. Pure so the storage path and its tests agree; the
/// order matters — a sign-in error is an auth problem before it is a network one, and a cancellation
/// names the stop rather than whatever the agent said on the way out.
pub fn step_error(text: &str) -> StepError {
    let trimmed = text.trim();
    let count = trimmed.chars().count();
    let mut bounded: String = trimmed.chars().take(MAX_ERROR_CHARS).collect();
    if count > MAX_ERROR_CHARS {
        bounded.push_str("\n… (error trimmed)");
    }
    let lower = trimmed.to_ascii_lowercase();
    let kind = if ["cancelled", "canceled", "was stopped", "flow was cancelled"].iter().any(|w| lower.contains(w)) {
        "cancelled"
    } else if ["timed out", "timeout", "took too long", "did not answer in time"].iter().any(|w| lower.contains(w)) {
        "timeout"
    } else if ["usage", "quota", "credit", "out of limit", "limit reached", "rate limit", "insufficient balance"]
        .iter()
        .any(|w| lower.contains(w))
    {
        "limit"
    } else if ["sign-in", "sign in", "unauthor", "expired", "401", "log in again"].iter().any(|w| lower.contains(w)) {
        "auth"
    } else if ["permission", "denied", "not allowed", "forbidden"].iter().any(|w| lower.contains(w)) {
        "permission"
    } else if ["can't reach", "could not reach", "network", "offline", "connection"].iter().any(|w| lower.contains(w)) {
        "network"
    } else if ["model", "the agent made no file", "did not fit", "answer"].iter().any(|w| lower.contains(w)) {
        "model"
    } else {
        "error"
    };
    StepError { text: bounded, kind }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_long_multi_line_error_keeps_every_line_not_just_the_first_three() {
        let error = "line one\nline two\nline three\nthe reason: disk full\nline five";
        let seen = step_error(error);
        assert_eq!(seen.as_str(), error, "the fourth line is the one that explains the failure");
        assert_eq!(seen.kind, "error");
    }

    #[test]
    fn an_error_is_bounded_by_characters_and_names_its_kind() {
        let text = format!("{}\nlast line", "x".repeat(MAX_ERROR_CHARS + 100));
        let seen = step_error(&text);
        assert_eq!(seen.text.chars().count(), MAX_ERROR_CHARS + "\n… (error trimmed)".chars().count());
        assert!(seen.text.ends_with("\n… (error trimmed)"));
        for (detail, kind) in [
            ("making the picture took too long", "timeout"),
            ("rate limit reached", "limit"),
            ("the sign-in expired", "auth"),
            ("permission denied for that folder", "permission"),
            ("can't reach the provider", "network"),
            ("the model answered without a file", "model"),
            ("the flow was cancelled while the files were being made", "cancelled"),
            ("something odd happened", "error"),
        ] {
            assert_eq!(step_error(detail).kind, kind, "{detail}");
        }
        assert_eq!(step_error("  spaced out  ").as_str(), "spaced out");
    }
}
