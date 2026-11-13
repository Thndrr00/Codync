//! Why a voice request failed, in `<data>/logs/voice.log`: one line with the operation, the
//! provider, the model and the chat turn when known, with secrets and the spoken text removed.
//! OpenBot keeps the same record for agent failures in `logs/providers/providers.log` (0.63);
//! Codync has no provider log, so this is the smallest one beside the tool logs that share the
//! folder and their writer (`crate::tools::log`), instead of a second log format.

use super::Provider;
use crate::tools::log::Log;
use std::sync::LazyLock;

/// The longest reason kept: a provider's message explains a failure; a page of it does not.
const MAX_REASON: usize = 300;

static LOG: LazyLock<Log> = LazyLock::new(|| Log::open(&crate::service::data_dir().join("logs"), "voice"));

/// One failed request. The text and audio are never part of it, only what failed and why.
pub(super) struct Failure<'a> {
    /// `transcribe` or `speak`.
    pub(super) operation: &'a str,
    pub(super) provider: Provider,
    pub(super) model: &'a str,
    /// The chat turn the request belongs to, when the caller knows it.
    pub(super) turn: Option<&'a str>,
    pub(super) reason: &'a str,
}

/// Appends the failure to the voice log. A log that can't be written never fails the request.
pub(super) async fn record(failure: Failure<'_>) {
    LOG.note(&line(&failure)).await;
}

fn line(failure: &Failure<'_>) -> String {
    let reason = crate::diagnostics::redact(&one_line(failure.reason));
    let reason = reason.chars().take(MAX_REASON).collect::<String>();
    format!(
        "{} voice {} provider={} model={} turn={} reason=\"{reason}\"",
        chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        failure.operation,
        failure.provider.id(),
        failure.model,
        failure.turn.unwrap_or("-"),
    )
}

/// A provider's message may carry line breaks; one failure is one line.
fn one_line(reason: &str) -> String {
    reason.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn failure(reason: &str) -> Failure<'_> {
        Failure { operation: "transcribe", provider: Provider::Groq, model: "whisper-large-v3", turn: None, reason }
    }

    #[test]
    fn a_line_names_the_request_and_never_the_reason_it_was_refused_over() {
        let line =
            line(&failure("Groq answered 401. Incorrect API key provided: gsk_live_9f8e7d6c5b4a39281706f5e4d3c2b1a0"));
        assert!(line.contains("voice transcribe provider=groq model=whisper-large-v3 turn=-"));
        assert!(line.contains("Incorrect API key"));
        assert!(!line.contains("gsk_live_"), "{line}");
        assert!(!line.contains("9f8e7d6c5b4a39281706f5e4d3c2b1a0"), "{line}");
        assert!(!line.contains('\n'));
    }

    #[test]
    fn a_reason_keeps_its_turn_and_one_line() {
        let line = line(&Failure {
            operation: "speak",
            provider: Provider::Xai,
            model: "xai-tts",
            turn: Some("1786000000000"),
            reason: "xAI is busy.\nTry again.",
        });
        assert!(line.contains("voice speak provider=xai model=xai-tts turn=1786000000000"));
        assert!(line.contains("reason=\"xAI is busy. Try again.\""));
    }

    #[test]
    fn a_long_reason_is_shortened() {
        let line = line(&failure(&"a ".repeat(400)));
        assert!(line.len() < 400, "{}", line.len());
    }
}
