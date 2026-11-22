//! A linked number can keep saying "connected" while WhatsApp stops handing it messages.
//!
//! The host notices two stalls (OpenBot nightly triage-12196): a message that could not be
//! finished (an inbox entry left queued) and a post WhatsApp never confirmed (a receipt that
//! timed out or came back without a message id). Each stall reconnects the link so held messages
//! come in; three stops inside one hour ask the owner for attention instead of looping forever.
//! The decision is pure so it can be tested without a link, a child process or the clock.

use std::collections::VecDeque;

/// Stops inside this window count together.
pub(super) const WINDOW_MS: u64 = 60 * 60 * 1000;
/// A queued message older than this has stopped, even though the link says it is connected. Long
/// agent turns are normal, so this is deliberately patient: only one that never moves is a stop.
pub(super) const QUEUED_STALL_MS: u64 = 30 * 60 * 1000;
/// After this many stops in an hour the number needs the owner instead of another silent retry.
pub(super) const ATTENTION_AFTER: usize = 3;

/// What one stop means for the link.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Decision {
    /// Not enough stops in the window to ask the owner; reconnect and go on.
    Reconnect,
    /// Reconnect, and show "Needs attention" until the owner or a quiet hour clears it.
    NeedsAttention,
}

/// The stops seen recently, oldest first.
#[derive(Default)]
pub(super) struct Stalls(VecDeque<u64>);

impl Stalls {
    /// Notes a stop at `now` (epoch milliseconds) and says what to do about it.
    pub fn note(&mut self, now: u64) -> Decision {
        while self.0.front().is_some_and(|at| now.saturating_sub(*at) >= WINDOW_MS) {
            self.0.pop_front();
        }
        self.0.push_back(now);
        if self.0.len() >= ATTENTION_AFTER { Decision::NeedsAttention } else { Decision::Reconnect }
    }

    /// Forgets the stops: a fresh link, or an hour without another one, starts over.
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// How many stops are inside the window at `now`.
    pub fn count(&self, now: u64) -> usize {
        self.0.iter().filter(|at| now.saturating_sub(**at) < WINDOW_MS).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_stops_reconnect_and_the_third_asks_for_attention() {
        let mut stalls = Stalls::default();
        assert_eq!(stalls.note(1_000), Decision::Reconnect);
        assert_eq!(stalls.note(61_000), Decision::Reconnect);
        assert_eq!(stalls.note(121_000), Decision::NeedsAttention);
    }

    #[test]
    fn stops_outside_the_hour_do_not_add_up() {
        let mut stalls = Stalls::default();
        stalls.note(0);
        stalls.note(WINDOW_MS - 1);
        assert_eq!(stalls.count(WINDOW_MS - 1), 2);
        // The first stop has left the window, so this is only the second one now.
        assert_eq!(stalls.note(WINDOW_MS), Decision::Reconnect);
        assert_eq!(stalls.count(WINDOW_MS), 2);
    }

    #[test]
    fn clearing_the_stops_for_a_new_link_starts_over() {
        let mut stalls = Stalls::default();
        stalls.note(10_000);
        stalls.note(20_000);
        stalls.clear();
        assert_eq!(stalls.count(20_000), 0);
        assert_eq!(stalls.note(20_000), Decision::Reconnect);
    }

    #[test]
    fn a_clock_that_moves_backwards_never_reports_a_future_stop() {
        let mut stalls = Stalls::default();
        stalls.note(10_000);
        assert_eq!(stalls.count(9_000), 0);
        assert_eq!(stalls.note(9_000), Decision::Reconnect);
    }
}
