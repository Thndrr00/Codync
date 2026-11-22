//! Pure rules for a teammate's turn: a temporary failure earns one retry, and an identical result repeated does not
//! open another turn. Ported from OpenBot's nightly teammate handling (triage-13054, -13130, -13131, -14330); the
//! code is Codync's.

/// Why a teammate's request failed, as far as the requester can tell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FailureKind {
    /// The provider stopped answering, the call timed out, the process went away, or the answer was empty: all
    /// short-lived, so the request goes out once more before anyone gives up.
    Temporary,
    /// A real cause the requester or the user has to fix.
    Permanent,
}

/// A briefly failing teammate request is retried once, never in a loop.
pub(super) fn retry_once(kind: FailureKind, attempts: u32) -> bool {
    kind == FailureKind::Temporary && attempts == 0
}

/// What makes two teammate answers the same result: the same teammate, the same text and the same files.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ResultFingerprint {
    pub agent: String,
    pub text: String,
    pub files: Vec<String>,
}

/// A teammate repeating its own result ends the turn quietly; the second message stays in the chat. A progress note
/// and a later result, or a failure note and a later result, are not the same and keep the retry.
pub(super) fn repeats_result(previous: Option<&ResultFingerprint>, next: &ResultFingerprint) -> bool {
    previous.is_some_and(|p| p.agent == next.agent && p.text.trim() == next.text.trim() && p.files == next.files)
}

/// What must be true for a teammate result re-delivered by restart recovery to end quietly
/// (OpenBot nightly triage-14330).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ReplayedResult {
    /// Recovery gave this result to the agent again after a restart.
    pub restarted: bool,
    /// The result was in a provider turn when the restart happened, not merely starting.
    pub had_provider_turn: bool,
    /// An earlier result of the same request had already been handled before the restart.
    pub answers_request: bool,
}

/// A result recovery gave again ends quietly when its empty turn has nothing new to say: the
/// requester already took the earlier answer in the provider turn that the restart interrupted.
/// A result that was only starting, or that never answered the request, keeps its retry and its
/// "no answer" error (triage-14330).
pub(super) fn replays_started_result(r: &ReplayedResult) -> bool {
    r.restarted && r.had_provider_turn && r.answers_request
}

/// An internal result between teammates is not a message for a contact in an external chat. Only the text the agent
/// wrote for that contact is posted; a teammate-only turn has no "Working on it…" post either (triage-13131).
pub(super) fn internal_teammate_result(external_chat: bool, teammate_request: bool) -> bool {
    external_chat && teammate_request
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_once_only_the_first_temporary_failure() {
        assert!(retry_once(FailureKind::Temporary, 0), "a short failure goes out once more");
        assert!(!retry_once(FailureKind::Temporary, 1), "never a second retry");
        assert!(!retry_once(FailureKind::Permanent, 0), "a real cause is not retried");
        assert!(!retry_once(FailureKind::Permanent, 1));
    }

    #[test]
    fn repeated_result_needs_the_same_teammate_text_and_files() {
        let first = ResultFingerprint {
            agent: "researcher".into(),
            text: "  Status: done, Result: found it  ".into(),
            files: vec!["sha-1".into()],
        };
        let same = ResultFingerprint { text: "Status: done, Result: found it".into(), ..first.clone() };
        assert!(repeats_result(Some(&first), &same), "padding and wording can't hide a repeat");
        let new_text = ResultFingerprint { text: "Status: done, Result: more".into(), ..first.clone() };
        assert!(!repeats_result(Some(&first), &new_text));
        let new_files = ResultFingerprint { files: vec!["sha-2".into()], ..first.clone() };
        assert!(!repeats_result(Some(&first), &new_files));
        let other = ResultFingerprint { agent: "other".into(), ..first.clone() };
        assert!(!repeats_result(Some(&first), &other));
        assert!(!repeats_result(None, &same), "the first result is never a repeat");
    }

    #[test]
    fn only_a_result_replayed_after_its_provider_turn_ends_quietly() {
        let started = ReplayedResult { restarted: true, had_provider_turn: true, answers_request: true };
        assert!(replays_started_result(&started), "already handled: nothing new to say, end quietly");
        assert!(
            !replays_started_result(&ReplayedResult { had_provider_turn: false, ..started }),
            "a result only starting at the restart never reached the provider: keep the retry and error"
        );
        assert!(
            !replays_started_result(&ReplayedResult { answers_request: false, ..started }),
            "a result that never answered the request keeps its error"
        );
        assert!(
            !replays_started_result(&ReplayedResult { restarted: false, ..started }),
            "a first delivery is not a replay"
        );
    }

    #[test]
    fn only_a_teammate_turn_in_an_external_chat_is_internal() {
        assert!(internal_teammate_result(true, true));
        assert!(!internal_teammate_result(true, false), "a message written for the contact is posted");
        assert!(!internal_teammate_result(false, true), "inside Codync the result is an ordinary reply");
        assert!(!internal_teammate_result(false, false));
    }
}
