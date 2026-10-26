//! What a turn actually did, judged from the tools the agent ran, not from what it says it did: it changed or ran
//! something, only looked, or did nothing. A turn that did nothing but says it's done is a made-up report.
//!
//! Idea from mercury-agent's `bot-turn.ts` (MIT); no code copied.

use serde::Serialize;

#[derive(Serialize, Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum Outcome {
    /// No tool ran.
    #[default]
    Nothing,
    /// Tools only read, searched or fetched.
    Inspected,
    /// A tool edited, moved, deleted or ran something.
    Acted,
}

/// The finished tool calls of one turn.
#[derive(Default)]
pub(super) struct Trace {
    outcome: Outcome,
    calls: u32,
}

impl Trace {
    /// Notes one finished tool call by its ACP `kind` (`edit`, `execute`, `read`…). A failed call changed nothing.
    pub(super) fn finished(&mut self, kind: Option<&str>, failed: bool) {
        if failed {
            return;
        }
        self.calls += 1;
        let this = match kind {
            Some("edit" | "delete" | "move" | "execute") => Outcome::Acted,
            // An unlabeled tool might do anything, but this only counts what the agent says it is.
            _ => Outcome::Inspected,
        };
        self.outcome = self.outcome.max(this);
    }

    pub(super) fn outcome(&self) -> Outcome {
        self.outcome
    }

    /// Tool calls that finished without failing.
    pub(super) fn calls(&self) -> u32 {
        self.calls
    }
}

/// Whether `text` reports finished work ("done", "I've created…", "listo"): the claim a turn that did nothing can't back.
pub fn claims_done(text: &str) -> bool {
    const CLAIMS: [&str; 12] = [
        "done",
        "completed",
        "finished",
        "i've created",
        "i have created",
        "i've updated",
        "i updated",
        "i created",
        "i've fixed",
        "all set",
        "listo",
        "terminado",
    ];
    let text = text.to_lowercase();
    CLAIMS.iter().any(|claim| text.contains(claim))
}

/// A turn that reports work and ran no tool that could have done it.
pub fn claimed_without_action(outcome: Outcome, text: &str) -> bool {
    outcome == Outcome::Nothing && claims_done(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_strongest_call_decides() {
        let mut trace = Trace::default();
        assert_eq!(trace.outcome(), Outcome::Nothing);
        trace.finished(Some("read"), false);
        assert_eq!(trace.outcome(), Outcome::Inspected);
        trace.finished(Some("execute"), false);
        trace.finished(Some("read"), false);
        assert_eq!(trace.outcome(), Outcome::Acted);
    }

    #[test]
    fn a_failed_call_changed_nothing() {
        let mut trace = Trace::default();
        trace.finished(Some("edit"), true);
        assert_eq!(trace.outcome(), Outcome::Nothing);
        assert_eq!(trace.calls(), 0);
        trace.finished(Some("read"), false);
        trace.finished(None, false);
        assert_eq!(trace.calls(), 2);
    }

    #[test]
    fn only_a_turn_with_no_tools_can_claim_without_action() {
        assert!(claimed_without_action(Outcome::Nothing, "All done, I've created the file."));
        assert!(!claimed_without_action(Outcome::Acted, "Done."));
        assert!(!claimed_without_action(Outcome::Nothing, "Which folder do you mean?"));
    }
}
