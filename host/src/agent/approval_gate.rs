//! Approval guards for turns nobody can answer (Idea 44).
//!
//! A background routine or proactive check-in runs with no person at the keyboard.
//! When `appsInBackground` is off, an external-app tool that would normally show an
//! approval card is refused instead of leaving the run stuck waiting for a tap.

use super::approval_rules::{Decision, Evaluation, Outcome};
use crate::store::Permission;

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
    evaluation: &Evaluation,
    title: &str,
) -> Option<String> {
    if !background || apps_in_background || !is_app_tool(title) {
        return None;
    }
    let would_ask = match &evaluation.outcome {
        Outcome::Rule { decision: Decision::Ask, .. } => true,
        Outcome::Setting { permission: Permission::Ask } => true,
        _ => false,
    };
    would_ask.then(|| {
        format!(
            "Blocked in a background run: \"{title}\" needs your approval, but connected apps in background work are off in this bot's settings."
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evaluation(permission: Permission) -> Evaluation {
        Evaluation { outcome: Outcome::Setting { permission }, chained: false, allow_passed_over: None }
    }

    #[test]
    fn a_background_run_refuses_an_app_that_would_ask() {
        let reason =
            background_apps_reason(true, false, &evaluation(Permission::Ask), "mcp__codync-connectors__send_email");
        assert!(reason.is_some_and(|r| r.contains("background") && r.contains("send_email")));
    }

    #[test]
    fn a_foreground_run_or_an_allowed_app_keeps_its_normal_flow() {
        let ask = evaluation(Permission::Ask);
        assert!(background_apps_reason(false, false, &ask, "mcp__codync-connectors__send_email").is_none());
        assert!(background_apps_reason(true, true, &ask, "mcp__codync-connectors__send_email").is_none());
        assert!(
            background_apps_reason(true, false, &evaluation(Permission::Auto), "mcp__codync-connectors__send_email")
                .is_none()
        );
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
