#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from agentes-chat's `engine/reply-policy.ts`: sentinel lines, visible text and
//! reply verdicts. Composes helpers already ported into this host. Only `botInsights` calls this.

use serde_json::Value;

/// Word meaning "no reply" (NO_REPLY).
pub const NO_REPLY: &str = "NO_REPLY";

/// Tool used to pass on a message (PASS_TOOL).
pub const PASS_TOOL: &str = "pass";

/// Nudge when someone addresses an agent directly (ANSWER_NUDGE).
pub const ANSWER_NUDGE: &str = "You were addressed directly; answer the person.";

/// Minimum length before a lead is kept (PAUSE_LEAD_MIN_CHARS).
pub const PAUSE_LEAD_MIN_CHARS: usize = 80;

/// Decision kinds of a reply.
pub const VERDICTS: [&str; 4] = ["answer", "pass", "silent", "nudge"];

/// Sentinel markers stripped before showing text.
pub const SENTINELS: [&str; 4] = ["__NO_REPLY__", "<!--NO_REPLY-->", "[NO_REPLY]", "NO_REPLY"];

/// Whether a line is a sentinel (isSentinelLine).
pub fn is_sentinel_line(line: &str) -> bool {
    let trimmed = line.trim();
    SENTINELS.iter().any(|marker| trimmed == *marker)
}

/// Text a person actually sees; sentinels drop when final (visibleText).
pub fn visible_text(text: &str, final_answer: bool) -> String {
    let kept: Vec<&str> = text.lines().filter(|line| !is_sentinel_line(line)).collect();
    let joined = kept.join("\n").trim().to_owned();
    if final_answer { joined } else { joined }
}

/// Strip sentinels and trim (stripSentinel).
pub fn strip_sentinel(text: &str) -> String {
    visible_text(text, true)
}

/// Whether the text is only a sentinel (isSentinelOnly).
pub fn is_sentinel_only(text: &str) -> bool {
    !text.trim().is_empty() && strip_sentinel(text).is_empty()
}

/// Whether text ends with a sentinel (endsWithSentinel).
pub fn ends_with_sentinel(text: &str) -> bool {
    text.lines().last().is_some_and(is_sentinel_line)
}

/// A short lead of a long answer, or nothing (pauseLead).
pub fn pause_lead(text: &str) -> &str {
    if text.chars().count() >= PAUSE_LEAD_MIN_CHARS { text } else { "" }
}

/// Why an agent was called, from flags.
pub fn why_called(another_agent: bool, everyone: bool, direct: bool) -> &'static str {
    if direct {
        "addressed-directly"
    } else if everyone {
        "called-everyone"
    } else if another_agent {
        "called-by-agent"
    } else {
        "routine"
    }
}

/// Words for why a specific agent was called (whyYouWereCalled).
pub fn why_you_were_called(another_agent: bool, everyone: bool, direct: bool) -> &'static str {
    match why_called(another_agent, everyone, direct) {
        "addressed-directly" => ANSWER_NUDGE,
        "called-everyone" => "Everyone was called; answer if you can help.",
        "called-by-agent" => "Another agent asked for you.",
        _ => "A routine started this turn.",
    }
}

/// Words for why another agent was called (whyAnAgentCalled).
pub fn why_an_agent_called(direct: bool) -> &'static str {
    if direct { "The person asked for this agent." } else { "Another agent handed this over." }
}

/// Words for why everyone was called (whyEveryoneCalled).
pub fn why_everyone_called() -> &'static str {
    "The question was for every agent."
}

/// Verdict of a reply from its text and flags (replyPolicyFor).
pub fn reply_verdict(text: &str, addressed: bool, can_pass: bool) -> &'static str {
    if is_sentinel_only(text) {
        "silent"
    } else if addressed {
        "answer"
    } else if can_pass && text.trim().is_empty() {
        "pass"
    } else if addressed {
        "nudge"
    } else {
        "answer"
    }
}

/// Whether a verdict requires an answer (mustAnswer).
pub fn must_answer(verdict: &str) -> bool {
    matches!(verdict, "answer" | "nudge")
}

/// Whether a verdict is silent (isSilent).
pub fn is_silent(verdict: &str) -> bool {
    verdict == "silent"
}

/// Number of visible lines in a text (visibleLines).
pub fn visible_lines(text: &str) -> usize {
    text.lines().filter(|line| !is_sentinel_line(line)).count()
}

/// One line describing a verdict (verdictLine).
pub fn verdict_line(verdict: &str, reason: &str) -> String {
    format!("{verdict} · {reason}")
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(lines: usize, verdicts: usize, silent: bool, nudged: bool) -> Vec<(&'static str, String)> {
    vec![
        ("lines", count_label(lines)),
        ("verdicts", count_label(verdicts)),
        ("silent", if silent { "yes" } else { "no" }.to_owned()),
        ("nudged", if nudged { "yes" } else { "no" }.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sentinels() {
        assert!(is_sentinel_line("__NO_REPLY__"));
        assert!(is_sentinel_line(" NO_REPLY "));
        assert!(!is_sentinel_line("hello"));
        assert_eq!(visible_text("hi\nNO_REPLY\nthere", true), "hi\nthere");
        assert_eq!(strip_sentinel("NO_REPLY"), "");
        assert!(is_sentinel_only("__NO_REPLY__"));
        assert!(ends_with_sentinel("hello\nNO_REPLY"));
        assert!(!ends_with_sentinel("hello"));
        assert_eq!(visible_lines("a\nNO_REPLY\nb"), 2);
    }

    #[test]
    fn leads_and_reasons() {
        let short = "brief";
        assert_eq!(pause_lead(short), "");
        let long = "x".repeat(100);
        assert_eq!(pause_lead(&long), long.as_str());
        assert_eq!(why_called(true, false, false), "called-by-agent");
        assert_eq!(why_called(false, true, false), "called-everyone");
        assert_eq!(why_called(false, false, true), "addressed-directly");
        assert_eq!(why_you_were_called(false, false, true), ANSWER_NUDGE);
        assert!(why_an_agent_called(true).contains("person"));
        assert!(why_everyone_called().contains("every agent"));
    }

    #[test]
    fn verdicts_and_summary() {
        assert_eq!(reply_verdict("NO_REPLY", false, false), "silent");
        assert_eq!(reply_verdict("hi", true, false), "answer");
        assert_eq!(reply_verdict("", false, true), "pass");
        assert!(must_answer("nudge"));
        assert!(!must_answer("pass"));
        assert!(is_silent("silent"));
        assert_eq!(VERDICTS.len(), 4);
        assert_eq!(PASS_TOOL, "pass");
        assert!(verdict_line("answer", "direct").contains("answer"));
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(3, 2, true, false).len(), 4);
    }
}
