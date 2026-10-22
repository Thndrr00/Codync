//! The user's rules for what a bot's agent may do without asking: each permission request the
//! agent sends is matched, first rule first, before the bot's general setting (`Permission`)
//! applies. `allow` answers it with the agent's allow option, `deny` with its reject option (or a
//! cancel), and `ask` always shows the card, even for a bot that approves everything.
//!
//! A rule matches the request's command (shell tools) or else its title, as a pattern where `*`
//! is any text, ignoring case; `kind` narrows it to one tool kind (`execute`, `edit`, `read`,
//! `fetch`…). An `allow` rule never approves a command that chains others (`;`, `&&`, `|`,
//! `` ` ``, `$(`, redirections, a newline): `git status*` must not let `git status && rm -rf ~`
//! through, so such a command falls to the card instead. `deny` and `ask` rules still apply to it.
//!
//! Idea from rakazo's `packages/core/src/action-approval.ts` (Apache-2.0); no code copied.
//! Its automated review of low-risk requests isn't here: no model decides for the user.

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

/// Rules one bot may have.
const MAX_RULES: usize = 100;
/// Longest pattern, in characters.
const MAX_PATTERN: usize = 500;
/// What makes one command line run more than one command.
const CHAINING: [&str; 9] = [";", "&&", "||", "|", "`", "$(", ">", "<", "\n"];

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Decision {
    Allow,
    Ask,
    Deny,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalRule {
    /// The command or title it matches; `*` is any text.
    pub pattern: String,
    /// Only requests of this tool kind; any kind when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    pub decision: Decision,
}

pub fn validate(rules: &[ApprovalRule]) -> Result<()> {
    if rules.len() > MAX_RULES {
        bail!("A bot keeps at most {MAX_RULES} approval rules");
    }
    for rule in rules {
        let chars = rule.pattern.trim().chars().count();
        if chars == 0 || chars > MAX_PATTERN {
            bail!("An approval rule's pattern is 1 to {MAX_PATTERN} characters");
        }
    }
    Ok(())
}

/// The rule that decides a request of `kind` doing `subject` (its command, or its title).
pub fn decide<'a>(rules: &'a [ApprovalRule], kind: &str, subject: &str) -> Option<&'a ApprovalRule> {
    let chained = CHAINING.iter().any(|c| subject.contains(c));
    rules.iter().find(|rule| {
        rule.kind.as_deref().is_none_or(|k| k.eq_ignore_ascii_case(kind))
            && matches(&rule.pattern, subject)
            && !(rule.decision == Decision::Allow && chained)
    })
}

/// Whether `pattern` (with `*` for any text) covers all of `text`, ignoring case.
fn matches(pattern: &str, text: &str) -> bool {
    let pattern: Vec<char> = pattern.trim().to_lowercase().chars().collect();
    let text: Vec<char> = text.trim().to_lowercase().chars().collect();
    // Two cursors; on a mismatch, go back to just after the last `*` and let it take one more char.
    let (mut p, mut t) = (0, 0);
    let mut star: Option<(usize, usize)> = None;
    while t < text.len() {
        if p < pattern.len() && pattern[p] == '*' {
            star = Some((p, t));
            p += 1;
        } else if p < pattern.len() && pattern[p] == text[t] {
            p += 1;
            t += 1;
        } else if let Some((sp, st)) = star {
            p = sp + 1;
            t = st + 1;
            star = Some((sp, st + 1));
        } else {
            return false;
        }
    }
    pattern[p..].iter().all(|c| *c == '*')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(pattern: &str, decision: Decision) -> ApprovalRule {
        ApprovalRule { pattern: pattern.into(), kind: None, decision }
    }

    #[test]
    fn patterns_cover_the_whole_text() {
        assert!(matches("git status*", "git status --short"));
        assert!(matches("GIT *", "git push origin main"));
        assert!(matches("*.env", "cat .env"));
        assert!(!matches("git status", "git status --short"));
        assert!(!matches("git push*", "git pull"));
        assert!(matches("*", "anything"));
    }

    #[test]
    fn the_first_rule_decides_and_chained_commands_are_never_allowed() {
        let rules = [rule("git push*", Decision::Ask), rule("git *", Decision::Allow), rule("rm *", Decision::Deny)];
        assert_eq!(decide(&rules, "execute", "git push origin").unwrap().decision, Decision::Ask);
        assert_eq!(decide(&rules, "execute", "git status").unwrap().decision, Decision::Allow);
        assert!(decide(&rules, "execute", "git status && rm -rf ~").is_none(), "falls to the card");
        assert_eq!(decide(&rules, "execute", "rm -rf build; ls").unwrap().decision, Decision::Deny);
        assert!(decide(&rules, "execute", "ls").is_none());
        let only_edits = [ApprovalRule { pattern: "*".into(), kind: Some("edit".into()), decision: Decision::Deny }];
        assert!(decide(&only_edits, "execute", "ls").is_none());
        assert!(decide(&only_edits, "edit", "Edit main.rs").is_some());
    }

    #[test]
    fn rules_are_bounded() {
        assert!(validate(&[rule(" ", Decision::Allow)]).is_err());
        let many: Vec<ApprovalRule> = std::iter::repeat_n(rule("x", Decision::Ask), MAX_RULES + 1).collect();
        assert!(validate(&many).is_err());
        assert!(validate(&[rule("npm test", Decision::Allow)]).is_ok());
    }
}
