#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from agentes-chat's `approval-rules.ts`: "approve always" kinds, rule
//! scopes, SQL read targets and own-action validation. Composes helpers already ported into this
//! host. Only `botInsights` calls this.

use serde_json::Value;

/// Tool name used by command rules (COMMAND_RULE_TOOL).
pub const COMMAND_RULE_TOOL: &str = "command";

/// Maximum connected-app actions a routine may do on its own (MAX_OWN_ACTIONS).
pub const MAX_OWN_ACTIONS: usize = 20;

/// Prefix of a SQL read rule (SQL_READS).
pub const SQL_READS: &str = "sql-read:";

/// Audit key for a removed rule.
pub const RULE_REMOVED_AUDIT: &str = "agent.approval_rule_removed";

/// Minimum length of an own-action name.
pub const MIN_ACTION_LEN: usize = 1;

/// Maximum length of an own-action name.
pub const MAX_ACTION_LEN: usize = 128;

/// Whether an action name is valid (OWN_ACTION_NAME).
pub fn valid_action_name(name: &str) -> bool {
    (MIN_ACTION_LEN..=MAX_ACTION_LEN).contains(&name.len())
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b':' | b'-'))
}

/// Normalize remembered kind actions: empty means the tool itself (actionsOf).
pub fn actions_of(actions: &[String]) -> Vec<String> {
    if actions.is_empty() { vec![String::new()] } else { actions.to_vec() }
}

/// Read a remembered kind safely, or None (rememberedKindOf).
pub fn remembered_kind(value: &Value) -> Option<(String, Vec<String>, Option<String>)> {
    let tool = value["tool"].as_str()?;
    if tool.is_empty() {
        return None;
    }
    let actions = value["actions"].as_array()?;
    let list: Vec<String> = actions.iter().map(Value::as_str).collect::<Option<Vec<_>>>()?;
    if list.iter().any(String::is_empty) {
        return None;
    }
    let routine = value["routineId"].as_str().map(str::to_owned);
    Some((tool.to_owned(), list, routine))
}

/// The target of a SQL read call: sorted other arguments (sqlReadsTarget).
pub fn sql_reads_target(input: &Value, sql_arg: &str) -> String {
    let mut parts = Vec::new();
    if let Some(map) = input.as_object() {
        for (key, value) in map {
            if key == sql_arg {
                continue;
            }
            let rendered = match value {
                Value::String(text) => text.clone(),
                other => serde_json::to_string(other).unwrap_or_default(),
            };
            parts.push(format!("{key}={rendered}"));
        }
    }
    parts.sort();
    parts.join(" · ")
}

/// Whether a target is a SQL read rule's kind.
pub fn is_sql_read(kind: &str) -> bool {
    kind.starts_with(SQL_READS)
}

/// Build a SQL read rule action for a target.
pub fn sql_read_action(target: &str) -> String {
    format!("{SQL_READS}{}", crate::bdots_more::shorten(target.trim(), 120))
}

/// Rule key for a scope and kind, reusing the shared key formatter.
pub fn rule_key(org: &str, agent: &str, connection: Option<&str>, tool: &str, action: &str) -> String {
    let connection = connection.unwrap_or("command");
    crate::bdots_textfmt::slugify(&format!("{org}:{agent}:{connection}:{tool}:{action}"))
}

/// Whether a command rule has no connection.
pub fn is_command_rule(connection: Option<&str>, tool: &str) -> bool {
    connection.is_none() && tool == COMMAND_RULE_TOOL
}

/// Whether all wanted actions are covered by stored actions.
pub fn all_actions_covered(wanted: &[String], stored: &[String]) -> bool {
    wanted.iter().all(|action| stored.contains(action))
}

/// Rule ids that cover a kind, in order, or empty when any is missing.
pub fn covering_rule_ids(wanted: &[String], rows: &[(String, String)]) -> Vec<String> {
    if !all_actions_covered(wanted, &rows.iter().map(|row| row.1.clone()).collect::<Vec<_>>()) {
        return Vec::new();
    }
    rows.iter().map(|row| row.0.clone()).collect()
}

/// Whether a user decides an agent's approvals.
pub fn decides_approvals(owner: Option<&str>, user: &str, can_approve: bool) -> bool {
    owner == Some(user) || can_approve
}

/// Whether a routine owns an action within the limit.
pub fn within_own_actions(count: usize) -> bool {
    count <= MAX_OWN_ACTIONS
}

/// Whether an own action is a tool name or a Composio slug.
pub fn valid_own_action(name: &str) -> bool {
    valid_action_name(name)
}

/// Audit label for a removed rule.
pub fn removal_label(removed: bool) -> &'static str {
    if removed { "Rule removed" } else { "Rule kept" }
}

/// SF Symbol for a rule state.
pub fn rule_icon(active: bool) -> &'static str {
    if active { "checkmark.seal.fill" } else { "seal.slash" }
}

/// Semantic tone for a rule state.
pub fn rule_tone(active: bool) -> &'static str {
    if active { "good" } else { "secondary" }
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(actions: usize, rules: usize, sql_read: bool, command: bool) -> Vec<(&'static str, String)> {
    vec![
        ("actions", count_label(actions)),
        ("rules", count_label(rules)),
        ("sql", if sql_read { "yes" } else { "no" }.to_owned()),
        ("kind", if command { "command" } else { "tool" }.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn names_and_kinds() {
        assert!(valid_action_name("github.issues.create"));
        assert!(valid_action_name("a:b-c_d"));
        assert!(!valid_action_name(""));
        assert!(!valid_action_name("has space"));
        assert_eq!(actions_of(&[]), vec![""]);
        assert_eq!(actions_of(&["a".to_owned()]), vec!["a"]);
        assert!(is_sql_read("sql-read:project=x"));
        assert!(!is_sql_read("command"));
    }

    #[test]
    fn remembered_and_targets() {
        let kind = json!({"tool":"github","actions":["create","read"],"routineId":"r1"});
        let (tool, actions, routine) = remembered_kind(&kind).unwrap();
        assert_eq!(tool, "github");
        assert_eq!(actions, vec!["create", "read"]);
        assert_eq!(routine.as_deref(), Some("r1"));
        assert!(remembered_kind(&json!({"tool":"","actions":[]})).is_none());
        let input = json!({"sql":"select 1","project_id":"abc","schema":"public"});
        assert_eq!(sql_reads_target(&input, "sql"), "project_id=abc · schema=public");
        assert!(sql_read_action("project=x").starts_with(SQL_READS));
    }

    #[test]
    fn scopes_and_covering() {
        let key = rule_key("org", "bot", None, "command", "gh");
        assert!(key.contains("command"));
        assert!(is_command_rule(None, "command"));
        assert!(!is_command_rule(Some("c"), "command"));
        let wanted = vec!["a".to_owned(), "b".to_owned()];
        let rows = vec![("1".to_owned(), "a".to_owned()), ("2".to_owned(), "b".to_owned())];
        assert_eq!(covering_rule_ids(&wanted, &rows), vec!["1", "2"]);
        assert!(covering_rule_ids(&wanted, &rows[..1]).is_empty());
    }

    #[test]
    fn decisions_and_presentation() {
        assert!(decides_approvals(Some("u"), "u", false));
        assert!(decides_approvals(None, "u", true));
        assert!(within_own_actions(20));
        assert!(!within_own_actions(21));
        assert!(valid_own_action("composio.gmail.send"));
        assert_eq!(removal_label(true), "Rule removed");
        assert_eq!(rule_icon(true), "checkmark.seal.fill");
        assert_eq!(rule_tone(false), "secondary");
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(3, 2, true, false).len(), 4);
    }
}
