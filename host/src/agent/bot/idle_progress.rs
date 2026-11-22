//! Why an agent keeps working after a turn instead of waiting for the next message (OpenBot nightly
//! `t3-fix-agent-idle-progress`). Two turn-end gates used thread-wide state: a long background job (a dev server)
//! and a teammate still working on an older request each stopped the automatic continuation of *every* later turn.
//! Both gates now look only at work the running turn started, so one old item can no longer idle a whole thread.
//! The decisions are pure; the actor supplies the process and turn facts.

/// Whether detached work still holds this turn's automatic continuation. A job an earlier turn started keeps
/// running on its own and must not stop later turns; only work this turn started may hold the agent. An unknown
/// process state counts as busy (see `background_work`), so this is a hold only when the turn also owns the work.
pub(super) fn background_holds(started_this_turn: bool, detached_running: bool) -> bool {
    started_this_turn && detached_running
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_work_this_turn_started_holds_the_agent() {
        assert!(background_holds(true, true), "a dev server this turn started holds it");
        assert!(!background_holds(false, true), "an older dev server does not stop a later turn");
        assert!(!background_holds(true, false), "work that ended holds nothing");
        assert!(!background_holds(false, false));
    }
}
