//! A Mistral Vibe chat freezes its tool list when it opens. Connecting GitHub afterwards leaves that
//! chat unable to call the new tools, so every message fails; the host asks for it to be reopened.

use crate::LockExt;
use std::sync::Mutex;

/// The facts at the moment a GitHub connection is recorded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ChatState<'a> {
    pub backend: &'a str,
    /// The GitHub connection was just made.
    pub connected: bool,
    /// A chat was open when it was made.
    pub chat_open: bool,
}

/// Pure: reopen Mistral's chat so the connection's tools get declared there. Other backends, a
/// chat that isn't open, and a connection that never happened keep their session.
pub(crate) fn mistral_reopen(state: ChatState<'_>) -> bool {
    (state.backend == "vibe" || state.backend == "mistral-vibe") && state.connected && state.chat_open
}

/// The one backend whose chat is due a reopen. The harness list answers it once, so a client
/// starting a new session consumes the request instead of seeing it forever.
static PENDING: Mutex<Option<String>> = Mutex::new(None);

pub(crate) fn note(state: ChatState<'_>) {
    if mistral_reopen(state) {
        *PENDING.locked() = Some(state.backend.to_owned());
    }
}

pub(crate) fn pending(backend: &str) -> bool {
    let mut pending = PENDING.locked();
    if pending.as_deref() == Some(backend) {
        *pending = None;
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_an_open_mistral_chat_reopens_after_the_connect() {
        let state = |backend, connected, chat_open| ChatState { backend, connected, chat_open };
        assert!(mistral_reopen(state("vibe", true, true)));
        assert!(!mistral_reopen(state("vibe", true, false)), "no open chat to reopen");
        assert!(!mistral_reopen(state("vibe", false, true)), "nothing connected");
        assert!(mistral_reopen(state("mistral-vibe", true, true)), "its registry id is Mistral too");
        for backend in ["pi", "claude", "codex", "vibe-acp"] {
            assert!(!mistral_reopen(state(backend, true, true)), "{backend}");
        }
    }

    #[test]
    fn a_reopen_is_asked_for_once_and_only_for_mistral() {
        note(ChatState { backend: "claude", connected: true, chat_open: true });
        assert!(!pending("claude"));
        note(ChatState { backend: "vibe", connected: true, chat_open: true });
        assert!(pending("vibe"));
        assert!(!pending("vibe"), "the flag is consumed by the new session");
    }
}
