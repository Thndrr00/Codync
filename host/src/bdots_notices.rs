#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from agentes-chat's `engine/notices.ts`: notice kinds, quiet/agent-only
//! notices and handoff guidance. Composes helpers already ported into this host. Only `botInsights`
//! calls this.

use serde_json::Value;

/// Notices only an agent sees (AGENT_ONLY_NOTICES).
pub const AGENT_ONLY_NOTICES: [&str; 5] =
    ["run_started", "run_finished", "tool_error", "context_full", "retry_exhausted"];

/// Notices that never reach a person (QUIET_NOTICES).
pub const QUIET_NOTICES: [&str; 3] = ["run_stopped", "follow_up", "wake_skipped"];

/// Notices that are a bot's own report (SELF_REPORT_NOTICES).
pub const SELF_REPORT_NOTICES: [&str; 3] = ["progress", "result", "failed"];

/// Handed-back guidance for a report (HANDED_BACK_GUIDANCE).
pub const HANDED_BACK_GUIDANCE: &str = "Handed back to the person.";

/// Notice kinds in the catalog.
pub const NOTICE_KINDS: [&str; 8] =
    ["run_started", "run_finished", "run_stopped", "follow_up", "wake_skipped", "tool_error", "progress", "result"];

/// Metadata key of a notice.
pub const NOTICE_KEY: &str = "notice";

/// Whether a notice is agent-only (isAgentOnlyNotice).
pub fn is_agent_only_notice(meta: Option<&Value>) -> bool {
    meta.and_then(|value| value.get("kind"))
        .and_then(Value::as_str)
        .is_some_and(|kind| AGENT_ONLY_NOTICES.contains(&kind))
}

/// Whether a notice is quiet (isQuietNotice).
pub fn is_quiet_notice(meta: Option<&Value>) -> bool {
    meta.and_then(|value| value.get("kind")).and_then(Value::as_str).is_some_and(|kind| QUIET_NOTICES.contains(&kind))
}

/// Whether a notice is a self-report (isSelfReportNotice).
pub fn is_self_report_notice(meta: Option<&Value>) -> bool {
    meta.and_then(|value| value.get("kind"))
        .and_then(Value::as_str)
        .is_some_and(|kind| SELF_REPORT_NOTICES.contains(&kind))
}

/// Notice kind from metadata.
pub fn notice_kind(meta: Option<&Value>) -> &str {
    meta.and_then(|value| value.get(NOTICE_KEY)).and_then(|value| value["kind"].as_str()).unwrap_or("")
}

/// Guidance after a handoff report (handoffGuidance).
pub fn handoff_guidance(report: Option<&str>, personal: bool) -> String {
    let guidance = if personal { "Tell the person what you did." } else { "Report to the team." };
    report
        .filter(|value| !value.trim().is_empty())
        .map_or_else(|| guidance.to_owned(), |value| format!("{} {}", guidance, value.trim()))
}

/// Guidance when a task was handed back partly (handedBackPartly).
pub fn handed_back_partly(pending: &str) -> String {
    if pending.trim().is_empty() {
        HANDED_BACK_GUIDANCE.to_owned()
    } else {
        format!("{} Still pending: {}", HANDED_BACK_GUIDANCE, pending.trim())
    }
}

/// Words of a notice for a person (noticeText).
pub fn notice_text(kind: &str, personal: bool) -> String {
    let text = match kind {
        "run_started" => "Started working",
        "run_finished" => "Finished",
        "run_stopped" => "Stopped",
        "follow_up" => "Following up",
        "wake_skipped" => "Woke up but skipped",
        "tool_error" => "A tool failed",
        "progress" => "Progress update",
        "result" => "Result",
        _ => "Notice",
    };
    if personal { format!("{text}.") } else { format!("{text} — team notice.") }
}

/// Words of a notice shown to others (noticeForOthers).
pub fn notice_for_others(kind: &str, actor: &str) -> String {
    if is_quiet_notice(Some(&json!({"kind": kind}))) {
        return String::new();
    }
    if actor.is_empty() { notice_text(kind, false) } else { format!("{}: {}", actor, notice_text(kind, false)) }
}

/// Whether a notice should be published (shouldPublishNotice).
pub fn should_publish_notice(meta: Option<&Value>) -> bool {
    !is_agent_only_notice(meta) && !is_quiet_notice(meta) && notice_kind(meta) != "progress"
}

/// Priority of a notice kind (noticePriority).
pub fn notice_priority(kind: &str) -> u8 {
    match kind {
        "result" => 3,
        "tool_error" | "failed" => 2,
        "progress" => 1,
        _ => 0,
    }
}

/// Highest-priority notice kind (topNotice).
pub fn top_notice(kinds: &[String]) -> Option<String> {
    kinds.iter().max_by_key(|kind| notice_priority(kind)).cloned()
}

/// One line describing a notice (noticeLine).
pub fn notice_line(kind: &str, actor: &str, quiet: bool) -> String {
    format!("{kind} · {actor} · {}", if quiet { "quiet" } else { "visible" })
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(notices: usize, quiet: usize, agent_only: usize, reports: usize) -> Vec<(&'static str, String)> {
    vec![
        ("notices", count_label(notices)),
        ("quiet", count_label(quiet)),
        ("agentOnly", count_label(agent_only)),
        ("reports", count_label(reports)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn notice_kinds() {
        assert!(is_agent_only_notice(Some(&json!({"kind":"tool_error"}))));
        assert!(!is_agent_only_notice(Some(&json!({"kind":"result"}))));
        assert!(is_quiet_notice(Some(&json!({"kind":"follow_up"}))));
        assert!(is_self_report_notice(Some(&json!({"kind":"result"}))));
        assert_eq!(notice_kind(Some(&json!({"notice":{"kind":"result"}}))), "result");
        assert_eq!(notice_kind(None), "");
        assert_eq!(NOTICE_KINDS.len(), 8);
        assert_eq!(NOTICE_KEY, "notice");
    }

    #[test]
    fn guidance_and_text() {
        assert!(handoff_guidance(None, true).contains("person"));
        assert!(handoff_guidance(Some("done"), false).contains("done"));
        assert_eq!(handed_back_partly(""), HANDED_BACK_GUIDANCE);
        assert!(handed_back_partly("review").contains("review"));
        assert_eq!(notice_text("finished", true), "Finished.");
        assert!(notice_text("result", false).contains("team"));
        assert!(notice_for_others("follow_up", "Ada").is_empty());
        assert!(notice_for_others("result", "Ada").contains("Ada"));
    }

    #[test]
    fn publishing_and_summary() {
        assert!(!should_publish_notice(Some(&json!({"kind":"tool_error"}))));
        assert!(!should_publish_notice(Some(&json!({"kind":"follow_up"}))));
        assert!(should_publish_notice(Some(&json!({"kind":"result"}))));
        assert_eq!(notice_priority("result"), 3);
        assert_eq!(top_notice(&["progress".to_owned(), "result".to_owned()]).as_deref(), Some("result"));
        assert!(notice_line("result", "Ada", false).contains("visible"));
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(4, 2, 1, 1).len(), 4);
    }
}
