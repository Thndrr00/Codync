//! A policy engine for this computer: a list of named rules, each a CEL expression with an effect
//! (`allow` or `deny`) and a message, evaluated at the same seam as the approval rules and the
//! forbidden commands. The full guide (variables, precedence, presets, simulator, limits) is
//! `docs/features/policy-engine.md`.
//!
//! What it decides, in order, for one tool call:
//!
//! 1. The forbidden commands (`agent::blocklist`) still come first, in the seam.
//! 2. Every enabled `deny` rule, in the order the person arranged them.
//! 3. Every enabled `allow` rule, in that order.
//! 4. Nothing matched: with an allow rule saved the list is an allowlist and the call is refused;
//!    with only deny rules — or no rules at all — the bot's approval rules and its Approvals setting
//!    decide exactly as before.
//!
//! A rule that doesn't compile, or doesn't evaluate to `true` or `false`, refuses the call and names
//! the rule and the error: a broken rule is never a pass. A rule is tried before it is saved, on a
//! sample of every initiator, so most broken rules never get that far.

pub mod api;
mod engine;
mod presets;
mod variables;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

pub use api::METHODS;
pub use engine::{Decision, Rule, evaluate, explain, limits, trace, validate};
pub use variables::{Request, VARIABLES};

use crate::hub::Hub;

/// What this computer's saved policies say about `request`.
///
/// A list that can't be read refuses: a database problem must not become a permission.
pub fn saved(hub: &Hub, request: &Request) -> Decision {
    match hub.store.policies() {
        Ok(policies) => evaluate(&policies, request),
        Err(error) => {
            Decision::Error { name: "the policy list".into(), error: format!("it could not be read: {error}") }
        }
    }
}

/// The sentence to refuse a call with, when the policy list refuses it, or `None` when it doesn't.
pub fn refusal(hub: &Hub, request: &Request) -> Option<String> {
    saved(hub, request).refusal()
}
