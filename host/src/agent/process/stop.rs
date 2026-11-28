//! Pure decision for a provider check that ran out of time or is still running at quit (#14).

use std::time::Duration;

/// Whether a check that is past its budget must have its whole child tree stopped, rather than
/// just the start script it was launched with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TreeStop {
    /// Still inside the budget, or nothing left running: no escalation.
    Wait,
    /// Time is up and children may still be running: end every process in the tree.
    StopTree,
}

impl TreeStop {
    pub(crate) fn stop_tree(self) -> bool {
        self == Self::StopTree
    }
}

/// Decides from a check's elapsed time, its limit and whether anything is still running whether
/// the whole tree must be stopped. A check that finished within its budget needs no escalation;
/// one that is still running once the budget is up (including at quit, where the budget is the
/// shutdown grace) has descendants that outlive the start script on macOS/Linux.
pub(crate) fn process_tree_stop(elapsed: Duration, limit: Duration, still_running: bool) -> TreeStop {
    if still_running && elapsed >= limit { TreeStop::StopTree } else { TreeStop::Wait }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_check_inside_its_budget_is_left_running() {
        assert!(!process_tree_stop(Duration::from_secs(1), Duration::from_secs(5), true).stop_tree());
        assert!(!process_tree_stop(Duration::from_secs(1), Duration::from_secs(5), false).stop_tree());
    }

    #[test]
    fn a_finished_check_needs_no_tree_stop_even_past_its_budget() {
        assert!(!process_tree_stop(Duration::from_secs(10), Duration::from_secs(5), false).stop_tree());
    }

    #[test]
    fn a_timed_out_check_thats_still_running_stops_its_tree() {
        assert_eq!(process_tree_stop(Duration::from_secs(5), Duration::from_secs(5), true), TreeStop::StopTree);
        assert!(process_tree_stop(Duration::from_secs(6), Duration::from_secs(5), true).stop_tree());
    }

    #[test]
    fn an_immediate_quit_with_a_zero_grace_stops_the_tree() {
        assert!(process_tree_stop(Duration::ZERO, Duration::ZERO, true).stop_tree());
        assert!(!process_tree_stop(Duration::ZERO, Duration::ZERO, false).stop_tree());
    }
}
