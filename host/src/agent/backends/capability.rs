//! Why a provider's CLI cannot start here, as opposed to a failed download.
//!
//! Ported from OpenBot nightly `triage-15358`: a binary built for a processor without SSE4.1
//! dies with SIGILL before it prints its "compiled with" line, so every retry looked like a
//! generic download failure. The original failure is still logged; only this classification
//! becomes the text on the Providers row.

/// Fixed text shown on the row when this computer's processor cannot run the provider.
pub const UNSUPPORTED_CPU: &str = "The processor of this computer cannot run this provider. Use a different provider.";

/// OpenBot triage-16354: a CLI that could not be downloaded used to show a bare "couldn't start".
/// The plain advice names the thing that helps, without a vendor name or an error dump.
pub const DOWNLOAD_FAILURE: &str =
    "The provider could not be downloaded. Check your internet connection and try again.";

/// The narrow signal that a CLI died before it could run, as opposed to failing to download.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StartFailure {
    IllegalInstruction,
    DownloadOrNetwork,
    /// Pi's signed-in model refused an over-long chat: the session must be reopened with history,
    /// not reported as a processor or download failure.
    ContextOverflow,
    Other,
}

/// Reads the reason a CLI start ended out of its last stderr lines. A download or network
/// failure is kept separate on purpose: it must never be shown as a processor problem.
pub(crate) fn start_failure(stderr_tail: &str) -> StartFailure {
    if stderr_tail.lines().map(str::trim).any(|line| line.contains("Illegal instruction") || line.contains("SIGILL")) {
        StartFailure::IllegalInstruction
    } else if stderr_tail
        .lines()
        .map(str::trim)
        .any(|line| line.contains("couldn't download") || line.contains("network") || line.contains("timed out"))
    {
        StartFailure::DownloadOrNetwork
    } else if super::pi::restart_with_history("pi", stderr_tail) {
        StartFailure::ContextOverflow
    } else {
        StartFailure::Other
    }
}

/// Whether this computer's processor can run `provider`'s CLI at all. Only a native build with
/// a known instruction requirement is affected; other architectures and other providers are
/// left alone (OpenBot nightly `triage-15358`).
pub(crate) fn provider_capability(provider: &str, sse41: bool) -> bool {
    !sse41 && matches!(provider, "gemini" | "antigravity")
}

/// The plain message for a CLI that died while downloading itself, if the stderr said so. A
/// processor problem is a different cause and keeps its own text (OpenBot triage-16354).
pub(crate) fn download_error(failure: StartFailure) -> Option<&'static str> {
    (failure == StartFailure::DownloadOrNetwork).then_some(DOWNLOAD_FAILURE)
}

/// The message to show for a failed CLI start, if the failure really was the processor or a
/// download. A crash while a bot is running keeps its own error.
pub(crate) fn capability_failure(provider: &str, sse41: bool, failure: StartFailure) -> Option<&'static str> {
    if download_error(failure).is_some() {
        return download_error(failure);
    }
    (failure == StartFailure::IllegalInstruction && provider_capability(provider, sse41)).then_some(UNSUPPORTED_CPU)
}

/// SSE4.1 is an x86 feature; builds for other architectures do not require it.
pub(crate) fn sse41() -> bool {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        std::arch::is_x86_feature_detected!("sse4.1")
    }
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_gemini_needs_sse41_on_this_processor() {
        assert!(!provider_capability("gemini", true));
        assert!(provider_capability("gemini", false));
        assert!(provider_capability("antigravity", false));
        assert!(!provider_capability("claude", false));
        assert!(!provider_capability("grok", false));
    }

    #[test]
    fn a_download_or_network_failure_is_never_called_a_processor_problem() {
        assert_eq!(capability_failure("gemini", false, StartFailure::IllegalInstruction), Some(UNSUPPORTED_CPU));
        assert_eq!(capability_failure("gemini", false, StartFailure::Other), None);
        assert_eq!(capability_failure("gemini", true, StartFailure::IllegalInstruction), None);
        assert_eq!(capability_failure("claude", false, StartFailure::IllegalInstruction), None);
    }

    #[test]
    fn a_download_failure_advises_checking_the_connection() {
        assert_eq!(download_error(StartFailure::DownloadOrNetwork), Some(DOWNLOAD_FAILURE));
        assert!(DOWNLOAD_FAILURE.contains("Check your internet connection"));
        assert_eq!(download_error(StartFailure::IllegalInstruction), None);
        assert_eq!(download_error(StartFailure::Other), None);
        assert_eq!(capability_failure("gemini", false, StartFailure::DownloadOrNetwork), Some(DOWNLOAD_FAILURE));
        assert_eq!(capability_failure("claude", true, StartFailure::DownloadOrNetwork), Some(DOWNLOAD_FAILURE));
        assert_eq!(capability_failure("gemini", false, StartFailure::IllegalInstruction), Some(UNSUPPORTED_CPU));
    }

    #[test]
    fn a_context_overflow_asks_for_a_fresh_session_not_a_start_error() {
        assert_eq!(start_failure("error: context length exceeded (128000 > 64000)"), StartFailure::ContextOverflow);
        assert_eq!(capability_failure("pi", false, StartFailure::ContextOverflow), None);
        assert_eq!(start_failure("agent said no\n"), StartFailure::Other);
    }

    #[test]
    fn an_illegal_instruction_is_kept_apart_from_a_download_failure() {
        assert_eq!(
            start_failure("sh: line 1: 12345 Illegal instruction (core dumped) gemini --acp"),
            StartFailure::IllegalInstruction
        );
        assert_eq!(
            start_failure("couldn't download gemini with npm: network unreachable"),
            StartFailure::DownloadOrNetwork
        );
        assert_eq!(start_failure("agent said no\n"), StartFailure::Other);
        assert_eq!(start_failure(""), StartFailure::Other);
    }
}
