#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from agentes-chat's `ai-accounts.ts`: sign-in state, plan grouping,
//! account choices, reuse and titles. Composes helpers already ported into this host. Only
//! `botInsights` calls this.

use serde_json::Value;

/// Account kinds offered by the picker (ACCOUNT_KINDS).
pub const ACCOUNT_KINDS: [&str; 6] = ["claude_code", "codex", "cursor", "opencode", "grok", "antigravity"];

/// Sign-in state of a login (signInOf).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignIn {
    Active,
    SignedOut,
    Unknown,
}

/// Sign-in state of a row, from status and whether the computer knows it (signInOf).
pub fn sign_in_of(status: &str, status_known: bool) -> SignIn {
    if status == "active" {
        SignIn::Active
    } else if !status_known {
        SignIn::Unknown
    } else {
        SignIn::SignedOut
    }
}

/// Whether a login offers signing in again (offersSignInAgain).
pub fn offers_sign_in_again(status: &str) -> bool {
    status == "signed_out"
}

/// Name of a sign-in state, as a stable string.
pub fn sign_in_name(state: SignIn) -> &'static str {
    match state {
        SignIn::Active => "active",
        SignIn::SignedOut => "signed_out",
        SignIn::Unknown => "unknown",
    }
}

/// Human label of a sign-in state.
pub fn sign_in_label(state: SignIn) -> &'static str {
    match state {
        SignIn::Active => "Active",
        SignIn::SignedOut => "Signed out",
        SignIn::Unknown => "Unknown",
    }
}

/// SF Symbol for a sign-in state.
pub fn sign_in_icon(state: SignIn) -> &'static str {
    match state {
        SignIn::Active => "checkmark.circle.fill",
        SignIn::SignedOut => "xmark.circle.fill",
        SignIn::Unknown => "questionmark.circle",
    }
}

/// Semantic tone for a sign-in state.
pub fn sign_in_tone(state: SignIn) -> &'static str {
    match state {
        SignIn::Active => "good",
        SignIn::SignedOut => "attention",
        SignIn::Unknown => "secondary",
    }
}

/// Identity key of a login: machine, owner, kind, provider and email/key hint.
pub fn identity_key(row: &Value) -> String {
    let machine = row["machineId"].as_str().unwrap_or("");
    let owner = row["ownerId"].as_str().unwrap_or("");
    let kind = row["kind"].as_str().unwrap_or("");
    let provider = row["provider"].as_str().unwrap_or("");
    let identity = row["provider"]
        .as_str()
        .and_then(|_| row["keyHint"].as_str())
        .or_else(|| row["email"].as_str().map(str::to_ascii_lowercase).as_deref())
        .unwrap_or("");
    format!("{machine}|{owner}|{kind}|{provider}|{identity}")
}

/// Grouping key of a login, using the id when no identity is visible.
pub fn group_key(row: &Value) -> String {
    let identity = if row["provider"].as_str().is_some() { row["keyHint"].as_str() } else { row["email"].as_str() };
    identity.map_or_else(|| row["id"].as_str().unwrap_or("").to_owned(), crate::bdots_textfmt::slugify)
}

/// Whether two accounts belong to the same plan group.
pub fn same_group(a: &Value, b: &Value) -> bool {
    let org_a = a["providerOrgId"].as_str().unwrap_or("");
    let org_b = b["providerOrgId"].as_str().unwrap_or("");
    group_key(a) == group_key(b) && (org_a.is_empty() || org_b.is_empty() || org_a == org_b)
}

/// Group plan accounts by identity, preserving first-seen order (groupPlans).
pub fn group_plans(rows: &[Value]) -> Vec<Vec<Value>> {
    let mut groups: Vec<Vec<Value>> = Vec::new();
    for row in rows {
        if let Some(group) = groups.iter_mut().find(|group| group.first().is_some_and(|first| same_group(first, row))) {
            group.push(row.clone());
        } else {
            groups.push(vec![row.clone()]);
        }
    }
    groups
}

/// Distinct agent ids using any of a group's logins.
pub fn used_by(group: &[Value]) -> Vec<String> {
    let mut out = Vec::new();
    for row in group {
        if let Some(values) = row["usedBy"].as_array() {
            for agent in values.iter().filter_map(Value::as_str) {
                if !out.contains(&agent.to_owned()) {
                    out.push(agent.to_owned());
                }
            }
        }
    }
    out
}

/// Signed-out rows of a group.
pub fn signed_out(group: &[Value]) -> Vec<&Value> {
    group
        .iter()
        .filter(|row| {
            sign_in_of(row["status"].as_str().unwrap_or(""), row["statusKnown"] != false) == SignIn::SignedOut
        })
        .collect()
}

/// Rows whose last sign-in look was unknown.
pub fn unknown_rows(group: &[Value]) -> Vec<&Value> {
    group
        .iter()
        .filter(|row| sign_in_of(row["status"].as_str().unwrap_or(""), row["statusKnown"] != false) == SignIn::Unknown)
        .collect()
}

/// Whether an account is lendable to an agent (lendable).
pub fn lendable(status: &str, shared: bool, used_by_count: usize, agent_name: &str) -> bool {
    !agent_name.trim().is_empty() && status == "active" && (shared || used_by_count == 0)
}

/// Whether an account is on several computers (onSeveralComputers).
pub fn on_several_computers(machines: &[String]) -> bool {
    let mut distinct = Vec::new();
    for machine in machines {
        if !distinct.contains(machine) {
            distinct.push(machine);
        }
    }
    distinct.len() > 1
}

/// Title of an account row (accountTitle).
pub fn account_title(label: &str, plan: Option<&str>) -> String {
    match plan.filter(|value| !value.is_empty()) {
        Some(plan) => format!("{} · {}", label, crate::bdots_textfmt::upper_first(plan)),
        None => label.to_owned(),
    }
}

/// Whether a plan string is set.
pub fn has_plan(plan: Option<&str>) -> bool {
    plan.is_some_and(|value| !value.trim().is_empty())
}

/// Action offered for an existing account (ExistingAct).
pub fn existing_action(shared: bool, used_here: bool, signed_out: bool) -> &'static str {
    if signed_out {
        "again"
    } else if shared {
        "also"
    } else if used_here {
        "move"
    } else {
        "use"
    }
}

/// SF Symbol for an existing account action.
pub fn action_icon(action: &str) -> &'static str {
    match action {
        "use" => "plus.circle",
        "also" => "plus.circle.fill",
        "again" => "arrow.clockwise.circle",
        "move" => "arrow.right.circle",
        _ => "lock.circle",
    }
}

/// Semantic tone for an existing account action.
pub fn action_tone(action: &str) -> &'static str {
    if action == "locked" { "secondary" } else { "good" }
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(accounts: usize, groups: usize, active: usize, unknown: usize) -> Vec<(&'static str, String)> {
    vec![
        ("accounts", count_label(accounts)),
        ("plans", count_label(groups)),
        ("active", count_label(active)),
        ("unknown", count_label(unknown)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn row(id: &str, status: &str, email: &str) -> Value {
        json!({"id":id,"status":status,"statusKnown":true,"kind":"codex","email":email,"usedBy":["a"]})
    }

    #[test]
    fn sign_in_states() {
        assert_eq!(sign_in_of("active", true), SignIn::Active);
        assert_eq!(sign_in_of("signed_out", true), SignIn::SignedOut);
        assert_eq!(sign_in_of("signed_out", false), SignIn::Unknown);
        assert!(offers_sign_in_again("signed_out"));
        assert_eq!(sign_in_name(SignIn::Unknown), "unknown");
        assert_eq!(sign_in_label(SignIn::Active), "Active");
        assert_eq!(sign_in_icon(SignIn::SignedOut), "xmark.circle.fill");
        assert_eq!(sign_in_tone(SignIn::Unknown), "secondary");
    }

    #[test]
    fn grouping() {
        let rows =
            vec![row("1", "active", "a@x.com"), row("2", "signed_out", "a@x.com"), row("3", "active", "b@x.com")];
        assert_eq!(group_plans(&rows).len(), 2);
        assert_eq!(used_by(&rows[..2]), vec!["a"]);
        assert_eq!(signed_out(&rows[..2]).len(), 1);
        assert!(unknown_rows(&rows).is_empty());
        assert_eq!(identity_key(&rows[0]), identity_key(&rows[1]));
        assert_eq!(group_key(&rows[0]), group_key(&rows[1]));
        assert!(same_group(&rows[0], &rows[1]));
    }

    #[test]
    fn choices_and_titles() {
        assert!(lendable("active", false, 0, "Bot"));
        assert!(!lendable("signed_out", true, 0, "Bot"));
        assert!(on_several_computers(&["mac".to_owned(), "pc".to_owned(), "mac".to_owned()]));
        assert_eq!(account_title("Claude", Some("pro")), "Claude · Pro");
        assert_eq!(account_title("Claude", None), "Claude");
        assert!(has_plan(Some("pro")));
        assert_eq!(existing_action(false, false, false), "use");
        assert_eq!(existing_action(true, false, false), "also");
        assert_eq!(existing_action(false, true, true), "again");
        assert_eq!(action_icon("move"), "arrow.right.circle");
        assert_eq!(action_tone("locked"), "secondary");
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(4, 2, 3, 1).len(), 4);
    }
}
