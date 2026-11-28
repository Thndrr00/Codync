#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers for a bot's trust posture: its permission mode and approval rules, shaped into
//! a read-only card. Composes `bdots_more`, `bdots_text` and `bdots_profile` instead of copying
//! their validators. Only the `botInsights` endpoint calls this, so every helper is live.

/// One approval rule reduced to what the card shows.
#[derive(Clone, Debug, PartialEq)]
pub struct RuleLine {
    pub pattern: String,
    pub kind: String,
    pub decision: &'static str,
    pub icon: &'static str,
    pub tone: &'static str,
    pub broad: bool,
}

/// Symbol for a rule decision, shared by the four clients.
pub fn decision_icon(decision: &str) -> &'static str {
    match decision {
        "allow" => "checkmark.circle.fill",
        "deny" => "xmark.octagon.fill",
        _ => "questionmark.circle.fill",
    }
}

/// Semantic tone for a rule decision.
pub fn decision_tone(decision: &str) -> &'static str {
    match decision {
        "allow" => "good",
        "deny" => "danger",
        _ => "attention",
    }
}

/// Human label for a permission mode.
pub fn permission_label(mode: &str) -> &'static str {
    match mode {
        "auto" => "Automatic",
        "ask" => "Ask each time",
        _ => "Unknown",
    }
}

/// Symbol for a permission mode.
pub fn permission_icon(mode: &str) -> &'static str {
    if mode == "ask" { "hand.raised.fill" } else { "bolt.circle.fill" }
}

/// Semantic tone for a permission mode.
pub fn permission_tone(mode: &str) -> &'static str {
    if mode == "ask" { "good" } else { "attention" }
}

/// One plain sentence about the bot's permission mode.
pub fn permission_detail(mode: &str) -> &'static str {
    match mode {
        "auto" => "Approves requests on its own",
        "ask" => "Sends requests to you to approve",
        _ => "No permission mode set",
    }
}

/// Whether a rule pattern matches everything.
pub fn is_broad(pattern: &str) -> bool {
    pattern.trim() == "*"
}

/// Whether a pattern looks like a shell command rather than a title.
pub fn is_command(pattern: &str) -> bool {
    let lower = pattern.to_ascii_lowercase();
    ["rm ", "sudo", "curl", "wget", "git ", "npm ", "python", "bash", "sh "].iter().any(|c| lower.contains(c))
}

/// Short display text for a pattern, reused from the shared shortening helper.
pub fn pattern_text(pattern: &str) -> String {
    crate::bdots_more::shorten(pattern.trim(), 48)
}

/// Count rules by decision, in a fixed order: allow, ask, deny.
pub fn count_decisions(decisions: &[&str]) -> Vec<(&'static str, usize)> {
    let mut out = Vec::new();
    for want in ["allow", "ask", "deny"] {
        let n = decisions.iter().filter(|d| **d == want).count();
        if n > 0 {
            out.push((want, n));
        }
    }
    out
}

/// Number of broad rules; these override the mode for everything.
pub fn broad_count(patterns: &[&str]) -> usize {
    patterns.iter().filter(|p| is_broad(p)).count()
}

/// Number of rules scoped to a tool kind.
pub fn scoped_count(kinds: &[&str]) -> usize {
    kinds.iter().filter(|k| !k.is_empty()).count()
}

/// Number of rules naming a shell command.
pub fn command_count(patterns: &[&str]) -> usize {
    patterns.iter().filter(|p| is_command(p)).count()
}

/// Total number of rules.
pub fn total_rules(lines: &[RuleLine]) -> usize {
    lines.len()
}

/// Whether any rule denies something.
pub fn has_deny(lines: &[RuleLine]) -> bool {
    lines.iter().any(|r| r.decision == "deny")
}

/// Whether any rule allows something broadly.
pub fn has_broad_allow(lines: &[RuleLine]) -> bool {
    lines.iter().any(|r| r.decision == "allow" && r.broad)
}

/// Whether the bot asks for every request (mode `ask`, no broad allow).
pub fn asks_for_everything(mode: &str, lines: &[RuleLine]) -> bool {
    mode == "ask" && !has_broad_allow(lines)
}

/// A 0–100 trust score: asking is safer than auto; denials help; broad allows hurt.
pub fn trust_score(mode: &str, lines: &[RuleLine]) -> u32 {
    let mut score = if mode == "ask" { 70 } else { 40 };
    score += (has_deny(lines) as u32) * 10;
    score += (scoped_count(&lines.iter().map(|r| r.kind.as_str()).collect::<Vec<_>>()) as u32).min(3) * 5;
    if has_broad_allow(lines) {
        score = score.saturating_sub(25);
    }
    score.min(100)
}

/// Semantic tone for a trust score.
pub fn trust_tone(score: u32) -> &'static str {
    if score >= 70 {
        "good"
    } else if score >= 45 {
        "attention"
    } else {
        "danger"
    }
}

/// Short label for a trust score.
pub fn trust_label(score: u32) -> &'static str {
    if score >= 70 {
        "Careful"
    } else if score >= 45 {
        "Balanced"
    } else {
        "Permissive"
    }
}

/// One plain sentence summarising the trust posture.
pub fn trust_headline(mode: &str, lines: &[RuleLine]) -> String {
    let score = trust_score(mode, lines);
    format!("{} · {} rules · {}", trust_label(score), lines.len(), permission_label(mode))
}

/// Rules sorted: denies first, then asks, then allows, then by pattern.
pub fn sort_rules(lines: &[RuleLine]) -> Vec<RuleLine> {
    let rank = |d: &str| match d {
        "deny" => 0,
        "ask" => 1,
        _ => 2,
    };
    let mut out = lines.to_vec();
    out.sort_by(|a, b| rank(a.decision).cmp(&rank(b.decision)).then_with(|| a.pattern.cmp(&b.pattern)));
    out
}

/// Up to `max` rules for a card, keeping the strictest.
pub fn strictest(lines: &[RuleLine], max: usize) -> Vec<RuleLine> {
    sort_rules(lines).into_iter().take(max).collect()
}

/// Compact count label for the rules card, reusing the shared formatter.
pub fn rule_count_label(lines: &[RuleLine]) -> String {
    crate::bdots_desk::compact_count(lines.len() as u64)
}

/// Build a display line from a rule's raw fields.
pub fn rule_line(pattern: &str, kind: &str, decision: &str) -> RuleLine {
    RuleLine {
        pattern: pattern_text(pattern),
        kind: kind.to_owned(),
        decision,
        icon: decision_icon(decision),
        tone: decision_tone(decision),
        broad: is_broad(pattern),
    }
}

/// Assemble the whole trust summary.
pub fn summary(mode: &str, lines: &[RuleLine]) -> Vec<(&'static str, String)> {
    vec![
        ("mode", permission_label(mode).to_owned()),
        ("rules", rule_count_label(lines)),
        ("broad", broad_count(&lines.iter().map(|r| r.pattern.as_str()).collect::<Vec<_>>()).to_string()),
        ("headline", trust_headline(mode, lines)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines() -> Vec<RuleLine> {
        vec![rule_line("*", "shell", "deny"), rule_line("npm test", "shell", "allow"), rule_line("git push", "", "ask")]
    }

    #[test]
    fn decision_helpers_are_stable() {
        assert_eq!(decision_icon("allow"), "checkmark.circle.fill");
        assert_eq!(decision_icon("deny"), "xmark.octagon.fill");
        assert_eq!(decision_tone("ask"), "attention");
        assert_eq!(permission_label("auto"), "Automatic");
        assert_eq!(permission_icon("ask"), "hand.raised.fill");
        assert_eq!(permission_tone("auto"), "attention");
        assert!(permission_detail("ask").contains("approve"));
    }

    #[test]
    fn patterns_and_counts() {
        assert!(is_broad(" * "));
        assert!(!is_broad("npm test"));
        assert!(is_command("sudo rm x"));
        assert!(!is_command("read a file"));
        assert_eq!(pattern_text("  npm test  "), "npm test");
        assert_eq!(count_decisions(&["allow", "deny", "allow"]), vec![("allow", 2), ("deny", 1)]);
        assert_eq!(broad_count(&["*", "x"]), 1);
        assert_eq!(scoped_count(&["shell", ""]), 1);
        assert_eq!(command_count(&["sudo x", "note"]), 1);
    }

    #[test]
    fn posture_detectors_and_score() {
        let l = lines();
        assert_eq!(total_rules(&l), 3);
        assert!(has_deny(&l));
        assert!(!has_broad_allow(&l));
        assert!(asks_for_everything("ask", &l));
        assert!(trust_score("ask", &l) > trust_score("auto", &l));
        assert_eq!(trust_tone(80), "good");
        assert_eq!(trust_label(50), "Balanced");
        assert!(trust_headline("ask", &l).contains("rules"));
    }

    #[test]
    fn ordering_and_strips() {
        let l = lines();
        let sorted = sort_rules(&l);
        assert_eq!(sorted[0].decision, "deny");
        assert_eq!(strictest(&l, 1)[0].decision, "deny");
        assert_eq!(rule_count_label(&l), "3");
        assert_eq!(summary("ask", &l).len(), 4);
    }

    #[test]
    fn broad_allow_lowers_the_score() {
        let safe = vec![rule_line("*", "shell", "deny")];
        let loose = vec![rule_line("*", "", "allow")];
        assert!(trust_score("auto", &safe) > trust_score("auto", &loose));
        assert!(has_broad_allow(&loose));
    }
}
