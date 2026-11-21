//! Approval guards for turns nobody can answer (Idea 44 and bdots idea 23).
//!
//! A background routine or proactive check-in runs with no person at the keyboard. When
//! `appsInBackground` is off, only read-only app tools may run: a connected-app call that
//! reaches the permission gate is by definition one the agent did not auto-allow, so it is
//! refused instead of leaving the turn stuck waiting for a tap.

use super::approval_rules::Evaluation;

/// MCP title prefixes of connected apps and Composio tools. Those are the external
/// effects (send mail, open an issue) the setting is about — local chat, memory and
/// file tools keep their normal approval flow.
const APP_TOOL_PREFIXES: &[&str] =
    &["mcp__codync-connectors__", "mcp__codync-composio__", "mcp__composio__", "connectors.", "composio."];

/// Whether a tool call reaches a connected app.
pub(crate) fn is_app_tool(title: &str) -> bool {
    let title = title.trim();
    APP_TOOL_PREFIXES.iter().any(|prefix| title.starts_with(prefix))
}

/// The reason to refuse, or `None` when the request may follow its normal decision.
pub(crate) fn background_apps_reason(
    background: bool,
    apps_in_background: bool,
    _evaluation: &Evaluation,
    title: &str,
) -> Option<String> {
    (!background || apps_in_background || !is_app_tool(title)).then(|| {
        format!(
            "Blocked in a background run: \"{title}\" needs your approval, but connected apps in background work are off in this bot's settings."
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::approval_rules::Outcome;
    use crate::store::Permission;

    fn evaluation(permission: Permission) -> Evaluation {
        Evaluation { outcome: Outcome::Setting { permission }, chained: false, allow_passed_over: None }
    }

    #[test]
    fn a_background_run_refuses_every_app_tool_reaching_the_gate() {
        for permission in [Permission::Ask, Permission::Auto] {
            let reason =
                background_apps_reason(true, false, &evaluation(permission), "mcp__codync-connectors__send_email");
            assert!(reason.is_some_and(|r| r.contains("background") && r.contains("send_email")));
        }
    }

    #[test]
    fn a_foreground_run_or_an_enabled_app_keeps_its_normal_flow() {
        let ask = evaluation(Permission::Ask);
        assert!(background_apps_reason(false, false, &ask, "mcp__codync-connectors__send_email").is_none());
        assert!(background_apps_reason(true, true, &ask, "mcp__codync-connectors__send_email").is_none());
    }

    #[test]
    fn only_connected_app_titles_are_guarded() {
        let ask = evaluation(Permission::Ask);
        for title in ["Write /repo/src/main.rs", "mcp__chat__send_message", "Terminal", "mcp__memory__edit_memory"] {
            assert!(!is_app_tool(title), "{title}");
            assert!(background_apps_reason(true, false, &ask, title).is_none(), "{title}");
        }
        for title in ["mcp__codync-composio__GMAIL_SEND_EMAIL", "connectors.github.create_issue", "composio.send"] {
            assert!(is_app_tool(title), "{title}");
        }
    }
}
