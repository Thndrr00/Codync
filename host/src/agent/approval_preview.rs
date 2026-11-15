//! What a proposed tool call would get, without doing it: the same approval rules a real request goes
//! through (`approval_rules::evaluate`), the rule that decided, or the bot's default, and a sentence a
//! person can read like an audit entry. No policy language is added here.

use super::approval_rules::{ApprovalRule, Decision, Evaluation, Outcome};
use crate::store::{BotConfig, Permission};
use serde_json::{Value, json};

/// The tool kinds the approval cards use. A tool name Codync doesn't recognize is `other`.
pub fn kind_of(tool: &str) -> &'static str {
    let name = tool.trim().to_ascii_lowercase();
    if ["shell", "bash", "command", "terminal", "exec", "run"].iter().any(|word| name.contains(word)) {
        "execute"
    } else if ["read", "view", "open_file", "cat"].iter().any(|word| name.contains(word)) {
        "read"
    } else if ["edit", "write", "patch", "replace"].iter().any(|word| name.contains(word)) {
        "edit"
    } else if ["delete", "remove", "trash"].iter().any(|word| name.contains(word)) {
        "delete"
    } else if ["move", "rename"].iter().any(|word| name.contains(word)) {
        "move"
    } else if ["fetch", "http", "web", "browse", "url"].iter().any(|word| name.contains(word)) {
        "fetch"
    } else {
        "other"
    }
}

/// The preview as a client shows it: the decision, the rule (or the default) that made it, and why.
#[allow(clippy::too_many_arguments, reason = "one proposed call's fields, kept explicit")]
pub fn preview(
    bot: &BotConfig,
    rules: &[ApprovalRule],
    quick: &[ApprovalRule],
    tool: &str,
    command: Option<&str>,
    input: Option<&Value>,
    initiator: &str,
    kind: Option<&str>,
) -> Value {
    let kind = kind.filter(|kind| !kind.trim().is_empty()).map_or_else(|| kind_of(tool), str::trim);
    let command = command.filter(|text| !text.trim().is_empty());
    let input_text = input.and_then(Value::as_str).filter(|text| !text.trim().is_empty());
    let subject = command.or(input_text).unwrap_or(tool);
    let evaluation = super::approval_rules::evaluate(rules, quick, bot.permission, kind, command, subject);
    let decision = match &evaluation.outcome {
        Outcome::Forbidden { .. } => Decision::Deny,
        Outcome::Rule { decision, .. } => *decision,
        Outcome::Setting { permission } => match permission {
            Permission::Auto => Decision::Allow,
            Permission::Ask => Decision::Ask,
        },
    };
    let rule = match &evaluation.outcome {
        Outcome::Rule { index, decision, pattern, kind, quick } => Some(json!({
            "index": index,
            "decision": decision,
            "pattern": pattern,
            "kind": kind,
            "quick": quick,
        })),
        _ => None,
    };
    let source = match &evaluation.outcome {
        Outcome::Forbidden { .. } => "forbidden",
        Outcome::Rule { quick, .. } if *quick => "quickRule",
        Outcome::Rule { .. } => "rule",
        Outcome::Setting { .. } => "default",
    };
    json!({
        "decision": decision,
        "source": source,
        "rule": rule,
        "default": bot.permission,
        "chained": evaluation.chained,
        "allowPassedOver": evaluation.allow_passed_over,
        "tool": tool,
        "kind": kind,
        "initiator": initiator,
        "explanation": explain(bot, tool, kind, subject, command, input, initiator, &evaluation, decision),
    })
}

#[allow(clippy::too_many_arguments, reason = "the same proposed call the preview describes")]
fn explain(
    bot: &BotConfig,
    tool: &str,
    kind: &str,
    subject: &str,
    command: Option<&str>,
    input: Option<&Value>,
    initiator: &str,
    evaluation: &Evaluation,
    decision: Decision,
) -> String {
    let call = match command {
        Some(command) => format!("run `{}`", shorten(command, 180)),
        None => format!("use {tool}"),
    };
    let detail = input
        .filter(|input| !input.is_null())
        .map(|input| format!(" with input `{}`", shorten(&input.to_string(), 180)))
        .unwrap_or_default();
    let head = format!("{} asks {} to {call}{detail} (tool kind {kind}).", shorten(initiator, 120), bot.name);
    let why = match &evaluation.outcome {
        Outcome::Forbidden { pattern } => format!(
            "A command matching \"{pattern}\" is forbidden on this computer: no rule or Approvals setting can allow it. Nothing ran."
        ),
        Outcome::Rule { index, decision, pattern, kind, quick } => {
            let label = if *quick { format!("Quick rule {}", index + 1) } else { format!("Rule {}", index + 1) };
            let narrow = kind.as_deref().map_or_else(String::new, |kind| format!(" for {kind}"));
            format!(
                "{label} ({}, pattern \"{pattern}\"{narrow}) decides {}. Nothing ran.",
                match decision {
                    Decision::Allow => "allow",
                    Decision::Ask => "ask",
                    Decision::Deny => "deny",
                },
                match decision {
                    Decision::Allow => "allow",
                    Decision::Ask => "ask",
                    Decision::Deny => "deny",
                },
            )
        }
        Outcome::Setting { permission } => format!(
            "No rule matches, so the bot's Approvals setting decides {}: {}. Nothing ran.",
            match decision {
                Decision::Allow => "allow",
                Decision::Ask => "ask",
                Decision::Deny => "deny",
            },
            match permission {
                Permission::Auto => "Approve automatically",
                Permission::Ask => "Ask me",
            },
        ),
    };
    let chained = (evaluation.chained && !matches!(evaluation.outcome, Outcome::Forbidden { .. }))
        .then(|| " The command runs more than one command, so no allow rule approves it.".to_owned());
    let passed = evaluation
        .allow_passed_over
        .map(|index| format!(" An allow rule ({}) matched but was passed over for that reason.", index + 1));
    format!("{head} {why}{}{}", chained.unwrap_or_default(), passed.unwrap_or_default())
}

fn shorten(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_owned();
    }
    let mut cut: String = text.chars().take(max).collect();
    cut.push('…');
    cut
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn bot(rules: Vec<ApprovalRule>, permission: Permission) -> BotConfig {
        serde_json::from_value(json!({
            "id": "bot", "name": "Researcher", "backend": "fixture", "command": "true", "cwd": "/tmp",
            "approvalRules": rules, "permission": permission,
        }))
        .unwrap()
    }

    fn rule(pattern: &str, decision: Decision) -> ApprovalRule {
        ApprovalRule { pattern: pattern.into(), kind: None, decision }
    }

    #[test]
    fn a_rule_that_decides_is_named_with_its_decision_and_explanation() {
        let cfg = bot(vec![rule("git push*", Decision::Ask), rule("git *", Decision::Allow)], Permission::Auto);
        let preview = preview(
            &cfg,
            &cfg.approval_rules,
            &[],
            "shell",
            Some("git push origin main"),
            None,
            "member Ana",
            Some("execute"),
        );
        assert_eq!(preview["decision"], "ask");
        assert_eq!(preview["source"], "rule");
        assert_eq!(preview["rule"]["index"], 0);
        assert_eq!(preview["rule"]["pattern"], "git push*");
        let explanation = preview["explanation"].as_str().unwrap();
        assert!(explanation.contains("Rule 1"), "{explanation}");
        assert!(explanation.contains("Nothing ran"), "{explanation}");
        assert!(explanation.contains("member Ana"), "{explanation}");
    }

    #[test]
    fn no_matching_rule_falls_back_to_the_bots_setting() {
        let cfg = bot(vec![rule("git *", Decision::Allow)], Permission::Ask);
        let preview = preview(&cfg, &cfg.approval_rules, &[], "shell", Some("ls -la"), None, "owner", Some("execute"));
        assert_eq!(preview["decision"], "ask");
        assert_eq!(preview["source"], "default");
        assert!(preview["rule"].is_null());
        assert!(preview["explanation"].as_str().unwrap().contains("Approvals setting decides ask"));
    }

    #[test]
    fn forbidden_commands_are_denied_before_any_rule_and_a_chained_command_is_explained() {
        let cfg = bot(vec![rule("git status*", Decision::Allow)], Permission::Auto);
        let forbidden =
            preview(&cfg, &cfg.approval_rules, &[], "shell", Some("rm -rf /"), None, "owner", Some("execute"));
        assert_eq!((forbidden["decision"].as_str(), forbidden["source"].as_str()), (Some("deny"), Some("forbidden")));
        let chained = preview(
            &cfg,
            &cfg.approval_rules,
            &[],
            "shell",
            Some("git status && rm x"),
            None,
            "owner",
            Some("execute"),
        );
        assert_eq!(chained["decision"], "allow", "the setting allows it, the rule was passed over");
        assert_eq!(chained["chained"], true);
        assert!(chained["explanation"].as_str().unwrap().contains("passed over"));
    }

    #[test]
    fn a_tool_name_suggests_its_kind_and_an_unknown_one_is_other() {
        assert_eq!(kind_of("shell"), "execute");
        assert_eq!(kind_of("read_file"), "read");
        assert_eq!(kind_of("edit_file"), "edit");
        assert_eq!(kind_of("github_create_issue"), "other");
    }
}
