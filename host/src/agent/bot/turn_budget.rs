//! Time budget for a turn the person is watching, from bdots' `guards.ts`.
//!
//! A chat expects a word within a minute and an answer within a few. Each nudge is given once
//! per turn; a visible message buys another minute. The notes are fixed text — no message
//! content ever leaves the host through this module.

use std::time::{Duration, Instant};

pub const UPDATE_AFTER: Duration = Duration::from_secs(60);
pub const WRAP_UP_AFTER: Duration = Duration::from_mins(3);
pub const STOP_AFTER: Duration = Duration::from_mins(5);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Nudge {
    Update,
    WrapUp,
    Stop,
}

impl Nudge {
    pub fn note(self) -> &'static str {
        match self {
            Self::Update => {
                "You have been quiet for over a minute. Before the next tool call, tell the user what you found so far, what is blocking you, and what you will try next."
            }
            Self::WrapUp => {
                "The user has been waiting several minutes. Do not start a new approach; finish with at most one or two more calls and report what you have, what blocked you, and what you would do next."
            }
            Self::Stop => {
                "You have worked long enough on this turn and tools are now off. Reply now with the answer so far (even partial), what blocked you, and one line offering the next step."
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct TurnBudget {
    started: Option<Instant>,
    last_spoke: Option<Instant>,
    given: [bool; 3],
}

impl TurnBudget {
    pub fn start(&mut self, now: Instant) {
        *self = Self { started: Some(now), last_spoke: Some(now), given: [false; 3] };
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// The bot wrote something the user can see; another minute of silence can be nudged again.
    pub fn spoke(&mut self, now: Instant) {
        self.last_spoke = Some(now);
        self.given[0] = false;
    }

    pub fn check(&mut self, now: Instant) -> Option<Nudge> {
        let started = self.started?;
        let worked = now.duration_since(started);
        let quiet = now.duration_since(self.last_spoke.unwrap_or(started));
        let (index, nudge) = [
            (2, Nudge::Stop, worked >= STOP_AFTER),
            (1, Nudge::WrapUp, worked >= WRAP_UP_AFTER),
            (0, Nudge::Update, quiet >= UPDATE_AFTER),
        ]
        .into_iter()
        .find(|(i, _, ready)| *ready && !self.given[*i])
        .map(|(i, nudge, _)| (i, nudge))?;
        self.given[index] = true;
        Some(nudge)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_nudge_is_given_once_and_a_message_buys_another_minute() {
        let t0 = Instant::now();
        let mut b = TurnBudget::default();
        assert_eq!(b.check(t0), None, "no turn yet");
        b.start(t0);
        assert_eq!(b.check(t0 + UPDATE_AFTER), Some(Nudge::Update));
        assert_eq!(b.check(t0 + UPDATE_AFTER), None);
        b.spoke(t0 + UPDATE_AFTER);
        assert_eq!(b.check(t0 + UPDATE_AFTER * 2), Some(Nudge::Update));
        assert_eq!(b.check(t0 + WRAP_UP_AFTER), Some(Nudge::WrapUp));
        assert_eq!(b.check(t0 + STOP_AFTER), Some(Nudge::Stop));
        assert_eq!(b.check(t0 + STOP_AFTER), None);
        b.clear();
        assert_eq!(b.check(t0 + STOP_AFTER), None);
    }

    #[test]
    fn a_stopping_turn_reports_even_if_it_never_went_quiet() {
        let t0 = Instant::now();
        let mut b = TurnBudget::default();
        b.start(t0);
        let mut now = t0;
        for _ in 0..30 {
            now += Duration::from_secs(10);
            b.spoke(now);
        }
        assert_eq!(b.check(now), Some(Nudge::Stop));
    }
}
