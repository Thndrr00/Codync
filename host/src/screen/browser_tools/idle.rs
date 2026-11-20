//! Source: OpenBot 0.75 — "Unused browser tabs freeze to reduce resource use."
//!
//! The pure policy for the per-bot browser tab watcher. The watcher itself lives in `cdp.rs`, which
//! owns the CDP pipe; keeping the decision here makes it testable without a browser. A tab is frozen
//! only after a full idle window, and never while a person watches it, a tool call holds it, or its
//! bot has a running turn. Freezing is undone on the next frame, input event or tool call.

use std::time::Duration;

/// How long a tab may see no CDP activity before it can be frozen.
pub(super) const IDLE_AFTER: Duration = Duration::from_secs(5 * 60);
/// One scan of all sessions every 30 s: bounded work, never a busy loop.
pub(super) const SCAN_EVERY: Duration = Duration::from_secs(30);
/// Bounds the freeze request itself; a stuck CDP read cannot hold the watcher for ever.
pub(super) const FREEZE_TIMEOUT: Duration = Duration::from_secs(5);

/// What the watcher knows about one tab at one instant. A value, so the rule stays pure.
#[derive(Clone, Copy, Debug)]
pub(super) struct Idle {
    pub(super) idle_for: Duration,
    pub(super) live: bool,
    pub(super) pending: bool,
    pub(super) turn_running: bool,
    pub(super) frozen: bool,
}

/// Whether this tab may be frozen now. All four guards must pass, and it must have been idle for the
/// whole window. An already-frozen tab is left alone.
pub(super) fn should_freeze(state: Idle, idle_after: Duration) -> bool {
    !state.live && !state.pending && !state.turn_running && !state.frozen && state.idle_for >= idle_after
}

#[cfg(test)]
mod tests {
    use super::*;

    fn idle() -> Idle {
        Idle { idle_for: IDLE_AFTER, live: false, pending: false, turn_running: false, frozen: false }
    }

    #[test]
    fn a_tab_is_frozen_only_after_a_full_idle_window() {
        assert!(!should_freeze(Idle { idle_for: IDLE_AFTER - Duration::from_secs(1), ..idle() }, IDLE_AFTER));
        assert!(should_freeze(idle(), IDLE_AFTER));
        assert!(should_freeze(Idle { idle_for: IDLE_AFTER * 2, ..idle() }, IDLE_AFTER));
        assert!(IDLE_AFTER >= Duration::from_secs(60), "five minutes of quiet, not a few seconds");
        assert!(SCAN_EVERY < IDLE_AFTER && FREEZE_TIMEOUT < IDLE_AFTER, "the watcher is bounded");
    }

    #[test]
    fn a_live_watched_or_busy_tab_is_never_frozen() {
        assert!(!should_freeze(Idle { live: true, ..idle() }, IDLE_AFTER), "a person is watching");
        assert!(!should_freeze(Idle { pending: true, ..idle() }, IDLE_AFTER), "a tool call holds the tab");
        assert!(!should_freeze(Idle { turn_running: true, ..idle() }, IDLE_AFTER), "the bot is on a turn");
        assert!(!should_freeze(Idle { frozen: true, ..idle() }, IDLE_AFTER), "already frozen");
    }

    #[test]
    fn the_idle_window_is_a_parameter() {
        let short = Duration::from_secs(30);
        assert!(!should_freeze(Idle { idle_for: short, ..idle() }, IDLE_AFTER));
        assert!(should_freeze(Idle { idle_for: short, ..idle() }, short));
    }
}
