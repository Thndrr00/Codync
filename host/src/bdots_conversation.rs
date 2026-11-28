#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers for the rhythm of a bot's chat: the mix of user, agent and tool entries in a
//! recent window. Reuses `bdots_text`, `bdots_activity` and `bdots_desk`. Only `botInsights` calls this.

/// One kind of entry reduced to what the card shows.
#[derive(Clone, Debug, PartialEq)]
pub struct KindLine {
    pub kind: &'static str,
    pub label: String,
    pub count: usize,
    pub icon: &'static str,
    pub tone: &'static str,
    pub percent: u32,
    pub bar: String,
}

/// Human label for an entry kind.
pub fn kind_label(kind: &str) -> &'static str {
    match kind {
        "user" => "Your messages",
        "agent" => "Replies",
        "thought" => "Thinking",
        "tool" => "Tool steps",
        "plan" => "Plans",
        "permission" => "Approvals",
        "notice" => "Notices",
        _ => "Other",
    }
}

/// SF Symbol for an entry kind, shared by the four clients.
pub fn kind_icon(kind: &str) -> &'static str {
    match kind {
        "user" => "person.fill",
        "agent" => "bubble.left.fill",
        "thought" => "brain",
        "tool" => "wrench.fill",
        "plan" => "list.bullet.rectangle",
        "permission" => "hand.raised.fill",
        "notice" => "bell.fill",
        _ => "circle.fill",
    }
}

/// Semantic tone for an entry kind.
pub fn kind_tone(kind: &str) -> &'static str {
    match kind {
        "user" => "accent",
        "agent" => "good",
        "tool" => "secondary",
        "permission" => "attention",
        "thought" | "plan" => "tertiary",
        _ => "secondary",
    }
}

/// Count each entry kind in a window, in a fixed order.
pub fn count_kinds(kinds: &[&str]) -> Vec<(&'static str, usize)> {
    let mut out = Vec::new();
    for want in ["user", "agent", "thought", "tool", "plan", "permission", "notice"] {
        let n = kinds.iter().filter(|k| **k == want).count();
        if n > 0 {
            out.push((want, n));
        }
    }
    out
}

/// Total entries in the window.
pub fn total(kinds: &[&str]) -> usize {
    kinds.len()
}

/// Percent of entries of one kind.
pub fn kind_percent(kinds: &[&str], kind: &str) -> u32 {
    let n = kinds.iter().filter(|k| **k == kind).count();
    crate::bdots_activity::percent(n, kinds.len())
}

/// Bar of filled cells for one kind's share.
pub fn kind_bar(kinds: &[&str], kind: &str, width: usize) -> String {
    let n = kinds.iter().filter(|k| **k == kind).count();
    "▮".repeat(crate::bdots_activity::bar_width(n, kinds.len(), width))
}

/// Build a display line for one entry kind.
pub fn kind_line(kinds: &[&str], kind: &str) -> KindLine {
    let n = kinds.iter().filter(|k| **k == kind).count();
    KindLine {
        kind,
        label: kind_label(kind).to_owned(),
        count: n,
        icon: kind_icon(kind),
        tone: kind_tone(kind),
        percent: kind_percent(kinds, kind),
        bar: kind_bar(kinds, kind, 8),
    }
}

/// All kinds as display lines, in fixed order.
pub fn all_lines(kinds: &[&str]) -> Vec<KindLine> {
    count_kinds(kinds).into_iter().map(|(k, _)| kind_line(kinds, k)).collect()
}

/// Number of user messages.
pub fn user_messages(kinds: &[&str]) -> usize {
    kinds.iter().filter(|k| **k == "user").count()
}

/// Number of agent replies.
pub fn agent_replies(kinds: &[&str]) -> usize {
    kinds.iter().filter(|k| **k == "agent").count()
}

/// Number of tool steps.
pub fn tool_steps(kinds: &[&str]) -> usize {
    kinds.iter().filter(|k| **k == "tool").count()
}

/// Replies per user message, as a whole number (0 when none).
pub fn reply_ratio(kinds: &[&str]) -> u32 {
    let users = user_messages(kinds);
    if users == 0 {
        return 0;
    }
    (agent_replies(kinds) as u32) / users as u32
}

/// Tool steps per user message, as a whole number (0 when none).
pub fn work_ratio(kinds: &[&str]) -> u32 {
    let users = user_messages(kinds);
    if users == 0 {
        return 0;
    }
    (tool_steps(kinds) as u32) / users as u32
}

/// The most common kind, or "user" for an empty window.
pub fn dominant_kind(kinds: &[&str]) -> &'static str {
    count_kinds(kinds).into_iter().max_by_key(|(_, n)| *n).map_or("user", |(k, _)| k)
}

/// Whether the bot did more work than it talked.
pub fn work_heavy(kinds: &[&str]) -> bool {
    tool_steps(kinds) > agent_replies(kinds)
}

/// Whether the person led the conversation (more messages than replies).
pub fn user_led(kinds: &[&str]) -> bool {
    user_messages(kinds) > agent_replies(kinds)
}

/// One plain sentence describing the chat rhythm.
pub fn rhythm_headline(kinds: &[&str]) -> String {
    if total(kinds) == 0 {
        return "No conversation yet.".to_owned();
    }
    let dominant = kind_label(dominant_kind(kinds));
    format!("{} entries · mostly {dominant}", total(kinds))
}

/// Semantic tone for the conversation card.
pub fn rhythm_tone(kinds: &[&str]) -> &'static str {
    if total(kinds) == 0 { "secondary" } else { "good" }
}

/// A one-line summary combining the two ratios.
pub fn rhythm_line(kinds: &[&str]) -> String {
    format!("{} replies · {} steps per message", reply_ratio(kinds), work_ratio(kinds))
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(kinds: &[&str]) -> String {
    crate::bdots_desk::compact_count(total(kinds) as u64)
}

/// Summary rows for the card.
pub fn summary(kinds: &[&str]) -> Vec<(&'static str, String)> {
    vec![
        ("entries", count_label(kinds)),
        ("user", user_messages(kinds).to_string()),
        ("agent", agent_replies(kinds).to_string()),
        ("headline", rhythm_headline(kinds)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds() -> Vec<&'static str> {
        vec!["user", "agent", "tool", "tool", "agent", "user", "thought"]
    }

    #[test]
    fn labels_icons_tones() {
        assert_eq!(kind_label("user"), "Your messages");
        assert_eq!(kind_icon("tool"), "wrench.fill");
        assert_eq!(kind_tone("permission"), "attention");
        assert_eq!(kind_label("unknown"), "Other");
    }

    #[test]
    fn counts_and_shares() {
        let k = kinds();
        assert_eq!(total(&k), 7);
        assert_eq!(count_kinds(&k).len(), 4);
        assert_eq!(kind_percent(&k, "user"), 28);
        assert!(!kind_bar(&k, "user", 8).is_empty());
        assert_eq!(count_label(&k), "7");
    }

    #[test]
    fn lines_and_detectors() {
        let k = kinds();
        let lines = all_lines(&k);
        assert_eq!(lines.len(), 4);
        assert_eq!(lines[0].kind, "user");
        assert_eq!(dominant_kind(&k), "user");
        assert!(!work_heavy(&k));
        assert!(!user_led(&k));
    }

    #[test]
    fn ratios() {
        let k = kinds();
        assert_eq!(user_messages(&k), 2);
        assert_eq!(agent_replies(&k), 2);
        assert_eq!(tool_steps(&k), 2);
        assert_eq!(reply_ratio(&k), 1);
        assert_eq!(work_ratio(&k), 1);
        assert_eq!(reply_ratio(&[]), 0);
        assert_eq!(work_ratio(&[]), 0);
    }

    #[test]
    fn headlines_and_summary() {
        let k = kinds();
        assert!(rhythm_headline(&k).contains("entries"));
        assert!(rhythm_headline(&[]).contains("No conversation"));
        assert_eq!(rhythm_tone(&[]), "secondary");
        assert!(rhythm_line(&k).contains("replies"));
        assert_eq!(summary(&k).len(), 4);
    }

    #[test]
    fn work_heavy_window() {
        let k = vec!["user", "tool", "tool", "tool", "agent"];
        assert!(work_heavy(&k));
        assert!(user_led(&k));
        assert_eq!(dominant_kind(&k), "tool");
    }
}
