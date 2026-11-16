//! The policy engine: this computer's list of named rules, each a CEL expression with an effect, and
//! the one place that decides what it says about a request.
//!
//! Precedence (see `docs/features/policy-engine.md`):
//!
//! 1. The forbidden commands (`agent::blocklist`) are checked before anything here, in the approval
//!    seam itself.
//! 2. Every enabled `deny` rule, in the order the person arranged them.
//! 3. Every enabled `allow` rule, in that order.
//! 4. Nothing matched: with at least one enabled `allow` rule saved, the list is an allowlist and
//!    nothing is permitted; with only `deny` rules, or none at all, the bot's approval rules and its
//!    Approvals setting decide as they always did.
//!
//! A rule that does not compile or does not evaluate to `true` or `false` refuses the request: the
//! engine fails closed and names the rule and the error, never treating a broken rule as a pass.

use super::variables::{Request, context};
use crate::store::{MAX_EXPRESSION, MAX_MESSAGE, MAX_NAME, MAX_POLICIES, Policy, PolicyEffect};
use anyhow::{Result, bail};
use cel::{Program, Value as CelValue};
use serde::Serialize;

/// A rule that decided, as a client shows it. `index` counts from 0 in the saved order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Rule {
    pub id: String,
    pub name: String,
    pub effect: PolicyEffect,
    pub index: usize,
    pub message: Option<String>,
}

impl Rule {
    fn of(policy: &Policy, index: usize) -> Self {
        Self {
            id: policy.id.clone(),
            name: policy.name.trim().to_owned(),
            effect: policy.effect,
            index,
            message: Some(policy.message.trim().to_owned()).filter(|message| !message.is_empty()),
        }
    }
}

/// What the policy list says about one request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "outcome", rename_all = "camelCase")]
pub enum Decision {
    /// A rule decided: a `deny` refuses the request, an `allow` permits it.
    Rule { rule: Rule, allowlist: bool },
    /// Nothing matched. `allowlist` means at least one allow rule is saved, so nothing is permitted;
    /// otherwise the approval rules decide, as before.
    NoMatch { allowlist: bool },
    /// No enabled rule: the approval rules and the Approvals setting decide, as before.
    Unset,
    /// A rule could not be compiled or evaluated. Refused, naming the rule and the error.
    Error { name: String, error: String },
}

impl Decision {
    /// The rule that decided, when one did.
    pub fn rule(&self) -> Option<&Rule> {
        match self {
            Self::Rule { rule, .. } => Some(rule),
            _ => None,
        }
    }

    /// Whether the request is refused: a deny rule, an allowlist with no match, or a broken rule.
    pub fn denied(&self) -> bool {
        match self {
            Self::Rule { rule, .. } => rule.effect == PolicyEffect::Deny,
            Self::NoMatch { allowlist } => *allowlist,
            Self::Error { .. } => true,
            Self::Unset => false,
        }
    }

    /// Whether a rule permits it.
    pub fn allowed(&self) -> bool {
        matches!(self, Self::Rule { rule, .. } if rule.effect == PolicyEffect::Allow)
    }

    /// Why it is refused, in one sentence naming the rule, or `None` when it isn't.
    pub fn refusal(&self) -> Option<String> {
        if !self.denied() {
            return None;
        }
        Some(match self {
            Self::Rule { rule, .. } => match &rule.message {
                Some(message) => format!("Refused by the policy \"{}\": {message}", rule.name),
                None => format!("Refused by the policy \"{}\"", rule.name),
            },
            Self::NoMatch { .. } => {
                "Refused: no policy on this computer allows it, and a policy list that allows anything only lets through what it names".into()
            }
            Self::Error { name, error } => {
                format!("Refused: the policy \"{name}\" could not be evaluated ({error}), so nothing is allowed")
            }
            Self::Unset => String::new(),
        })
    }

    /// A short name for where the decision came from: `policy`, `noMatch`, `unset` or `error`.
    pub fn source(&self) -> &'static str {
        match self {
            Self::Rule { .. } => "policy",
            Self::NoMatch { .. } => "noMatch",
            Self::Error { .. } => "error",
            Self::Unset => "unset",
        }
    }
}

/// What the policy list says about `request`. Deny rules are all tried before any allow rule.
pub fn evaluate(policies: &[Policy], request: &Request) -> Decision {
    let enabled: Vec<(usize, &Policy)> = policies.iter().enumerate().filter(|(_, policy)| policy.enabled).collect();
    if enabled.is_empty() {
        return Decision::Unset;
    }
    let allowlist = enabled.iter().any(|(_, policy)| policy.effect == PolicyEffect::Allow);
    let scope = context(request);
    for effect in [PolicyEffect::Deny, PolicyEffect::Allow] {
        for (index, policy) in enabled.iter().filter(|(_, policy)| policy.effect == effect) {
            match holds(policy, &scope) {
                Ok(true) => return Decision::Rule { rule: Rule::of(policy, *index), allowlist },
                Ok(false) => {}
                Err(error) => return Decision::Error { name: policy.name.trim().to_owned(), error },
            }
        }
    }
    Decision::NoMatch { allowlist }
}

/// Whether one rule holds for `scope`: its expression compiles and is `true`.
fn holds(policy: &Policy, scope: &cel::Context<'_, '_>) -> Result<bool, String> {
    let program = Program::compile(policy.expression.trim()).map_err(|errors| brief(&errors.to_string()))?;
    match program.execute(scope) {
        Ok(CelValue::Bool(it_holds)) => Ok(it_holds),
        Ok(other) => Err(format!("it is {}, not true or false", other.type_of())),
        Err(error) => Err(brief(&error.to_string())),
    }
}

/// One rule's own answer about a request, for the what-if's list of candidates.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleTrace {
    pub id: String,
    pub name: String,
    pub effect: PolicyEffect,
    /// Its place in the saved list, counting from 0.
    pub index: usize,
    pub matched: bool,
    /// Why it could not be tried; such a rule refuses in a real request.
    pub error: Option<String>,
}

/// Every enabled rule's own answer, in the order they are tried: deny rules first. What the what-if
/// lists beside its verdict; nothing here decides anything.
pub fn trace(policies: &[Policy], request: &Request) -> Vec<RuleTrace> {
    let scope = context(request);
    let mut traced = Vec::new();
    for effect in [PolicyEffect::Deny, PolicyEffect::Allow] {
        let rules = policies.iter().enumerate().filter(|(_, policy)| policy.enabled && policy.effect == effect);
        for (index, policy) in rules {
            let (matched, error) = match holds(policy, &scope) {
                Ok(true) => (true, None),
                Ok(false) => (false, None),
                Err(error) => (false, Some(error)),
            };
            traced.push(RuleTrace {
                id: policy.id.clone(),
                name: policy.name.trim().to_owned(),
                effect,
                index,
                matched,
                error,
            });
        }
    }
    traced
}

/// A rule must be saved only when it compiles and evaluates, on a sample of every initiator.
pub fn validate(policy: &Policy) -> Result<()> {
    let program = Program::compile(policy.expression.trim())
        .map_err(|errors| anyhow::anyhow!("This expression doesn't compile: {}", brief(&errors.to_string())))?;
    for initiator in ["person", "routine", "channel", "network"] {
        let sample = Request::sample(initiator);
        match program.execute(&context(&sample)) {
            Ok(CelValue::Bool(_)) => {}
            Ok(other) => bail!("This expression gives {}, not true or false (tried as {initiator})", other.type_of()),
            Err(error) => bail!("This expression doesn't evaluate (as {initiator}): {}", brief(&error.to_string())),
        }
    }
    Ok(())
}

/// A rule's error as one short line: what a person can act on, without the parser's position table.
fn brief(error: &str) -> String {
    let line = error.lines().map(str::trim).find(|line| !line.is_empty()).unwrap_or("it failed");
    let line: String = line.chars().take(200).collect();
    line
}

/// The limits an editor enforces, as the API returns them.
pub fn limits() -> serde_json::Value {
    serde_json::json!({
        "policies": MAX_POLICIES,
        "name": MAX_NAME,
        "expression": MAX_EXPRESSION,
        "message": MAX_MESSAGE,
    })
}

/// One sentence for a person: the request, the rule that decides it and why, ending "Nothing ran."
pub fn explain(decision: &Decision, request: &Request) -> String {
    let call = if request.command.is_empty() {
        format!("use {}", request.tool)
    } else {
        format!("run `{}`", brief(&request.command))
    };
    let head = format!("{} asks to {call} (tool kind {}).", who(request), request.kind);
    let why = match decision {
        Decision::Rule { rule, .. } => {
            let effect = match rule.effect {
                PolicyEffect::Allow => "allow",
                PolicyEffect::Deny => "deny",
            };
            let message = rule.message.as_deref().map_or_else(String::new, |message| format!(": {message}"));
            format!("Policy rule \"{}\" ({effect}) decides {effect}{message}.", rule.name)
        }
        Decision::NoMatch { allowlist: true } => {
            "No policy matches. This computer has allow rules, so only what one names goes through, and nothing does."
                .into()
        }
        Decision::NoMatch { allowlist: false } => {
            "No policy matches. This computer has only deny rules, so the bot's approval rules and its Approvals setting decide."
                .into()
        }
        Decision::Unset => {
            "No policy is set on this computer, so the bot's approval rules and its Approvals setting decide.".into()
        }
        Decision::Error { name, error } => {
            format!("Policy rule \"{name}\" could not be evaluated ({error}), so nothing is allowed.")
        }
    };
    format!("{head} {why} Nothing ran.")
}

fn who(request: &Request) -> String {
    if request.actor.is_empty() {
        return "the owner".into();
    }
    match request.actor_role.as_str() {
        "admin" => format!("admin {}", request.actor),
        "member" => format!("member {}", request.actor),
        _ => request.actor.clone(),
    }
}
