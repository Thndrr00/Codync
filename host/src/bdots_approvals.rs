#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers for a bot's pending approval cards. Reuses `bdots_pure` (choice options),
//! `bdots_text`, `bdots_more` and `bdots_activity` instead of duplicating them. Only
//! `botInsights` calls this.

/// One pending approval reduced to what the card shows.
#[derive(Clone, Debug, PartialEq)]
pub struct ApprovalLine {
    pub id: String,
    pub title: String,
    pub detail: String,
    pub age: String,
    pub icon: &'static str,
    pub tone: &'static str,
    pub urgent: bool,
}

/// SF Symbol for an approval's urgency.
pub fn urgency_icon(urgent: bool) -> &'static str {
    if urgent { "exclamationmark.triangle.fill" } else { "hand.raised.fill" }
}

/// Semantic tone for an approval's urgency.
pub fn urgency_tone(urgent: bool) -> &'static str {
    if urgent { "danger" } else { "attention" }
}

/// Short title for an approval card, reusing the shared summariser.
pub fn approval_title(text: &str) -> String {
    let first = crate::bdots_text::summarize(text);
    if first.is_empty() { "Approval needed".to_owned() } else { first }
}

/// Detail line from an approval's options, reusing the shared choice parser.
pub fn approval_detail(data: &serde_json::Value) -> String {
    let options = crate::bdots_pure::choice_options(data);
    let labels: Vec<String> = options.into_iter().map(|(label, _)| label).collect();
    if labels.is_empty() {
        return "Waiting for your answer".to_owned();
    }
    crate::bdots_more::shorten(&labels.join(" · "), 72)
}

/// Whether an approval has waited longer than the given minutes.
pub fn is_urgent(age_ms: i64, threshold_minutes: i64) -> bool {
    age_ms > threshold_minutes * 60_000
}

/// Human age for an approval, reusing the shared span formatter.
pub fn age_label(age_ms: i64) -> String {
    crate::bdots_text::format_span(age_ms.max(0))
}

/// Build a display line from an approval entry's fields.
pub fn approval_line(id: &str, text: &str, data: &serde_json::Value, age_ms: i64) -> ApprovalLine {
    let urgent = is_urgent(age_ms, 60);
    ApprovalLine {
        id: id.to_owned(),
        title: approval_title(text),
        detail: approval_detail(data),
        age: age_label(age_ms),
        icon: urgency_icon(urgent),
        tone: urgency_tone(urgent),
        urgent,
    }
}

/// Number of urgent approvals.
pub fn urgent_count(lines: &[ApprovalLine]) -> usize {
    lines.iter().filter(|l| l.urgent).count()
}

/// Number of approvals that are not urgent.
pub fn waiting_count(lines: &[ApprovalLine]) -> usize {
    lines.len() - urgent_count(lines)
}

/// Whether any approval is waiting.
pub fn has_any(lines: &[ApprovalLine]) -> bool {
    !lines.is_empty()
}

/// Oldest approval age in the window.
pub fn oldest_age(lines: &[ApprovalLine], ages: &[i64]) -> i64 {
    ages.iter().copied().max().unwrap_or(0)
}

/// A bar of filled cells for the urgent share.
pub fn urgency_bar(lines: &[ApprovalLine], width: usize) -> String {
    "▮".repeat(crate::bdots_activity::bar_width(urgent_count(lines), lines.len(), width))
}

/// Approval lines sorted by urgency, then by id.
pub fn sort_lines(lines: &[ApprovalLine]) -> Vec<ApprovalLine> {
    let mut out = lines.to_vec();
    out.sort_by(|a, b| b.urgent.cmp(&a.urgent).then_with(|| a.id.cmp(&b.id)));
    out
}

/// Up to `max` approvals, keeping the most urgent.
pub fn top_lines(lines: &[ApprovalLine], max: usize) -> Vec<ApprovalLine> {
    sort_lines(lines).into_iter().take(max).collect()
}

/// One plain sentence describing the approvals.
pub fn approval_headline(lines: &[ApprovalLine]) -> String {
    if lines.is_empty() {
        return "Nothing waiting on you.".to_owned();
    }
    let urgent = urgent_count(lines);
    if urgent == 0 {
        format!("{} approval{} waiting", lines.len(), if lines.len() == 1 { "" } else { "s" })
    } else {
        format!("{} approval{} waiting · {} urgent", lines.len(), if lines.len() == 1 { "" } else { "s" }, urgent)
    }
}

/// Semantic tone for the approval card.
pub fn approval_tone(lines: &[ApprovalLine]) -> &'static str {
    if lines.is_empty() {
        "good"
    } else if urgent_count(lines) > 0 {
        "danger"
    } else {
        "attention"
    }
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(lines: &[ApprovalLine]) -> String {
    crate::bdots_desk::compact_count(lines.len() as u64)
}

/// Summary rows for the card.
pub fn summary(lines: &[ApprovalLine]) -> Vec<(&'static str, String)> {
    vec![
        ("waiting", count_label(lines)),
        ("urgent", urgent_count(lines).to_string()),
        ("headline", approval_headline(lines)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn lines() -> Vec<ApprovalLine> {
        vec![
            approval_line("a", "Delete a file?", &json!({"choices":[{"label":"Yes"},{"label":"No"}]}), 120_000),
            approval_line("b", "Send email?", &json!({}), 10_000),
        ]
    }

    #[test]
    fn urgency_helpers() {
        assert_eq!(urgency_icon(true), "exclamationmark.triangle.fill");
        assert_eq!(urgency_tone(false), "attention");
        assert!(is_urgent(120_000, 60));
        assert!(!is_urgent(10_000, 60));
        assert_eq!(age_label(120_000), "2 min");
    }

    #[test]
    fn titles_and_details() {
        assert_eq!(approval_title("Delete a file?"), "Delete a file?");
        assert_eq!(approval_title(""), "Approval needed");
        let detail = approval_detail(&json!({"choices":[{"label":"Yes"},{"label":"No"}]}));
        assert!(!detail.is_empty());
        assert!(!approval_detail(&json!({})).is_empty());
    }

    #[test]
    fn lines_and_counts() {
        let l = lines();
        assert_eq!(l.len(), 2);
        assert!(l[0].urgent);
        assert!(!l[1].urgent);
        assert_eq!(urgent_count(&l), 1);
        assert_eq!(waiting_count(&l), 1);
        assert!(has_any(&l));
        assert!(!has_any(&[]));
    }

    #[test]
    fn sorting_bars_and_headlines() {
        let l = lines();
        let sorted = sort_lines(&l);
        assert!(sorted[0].urgent);
        assert_eq!(top_lines(&l, 1).len(), 1);
        assert!(!urgency_bar(&l, 8).is_empty());
        assert!(approval_headline(&l).contains("urgent"));
        assert!(approval_headline(&[]).contains("Nothing"));
        assert_eq!(approval_tone(&[]), "good");
        assert_eq!(approval_tone(&l), "danger");
        assert_eq!(count_label(&l), "2");
        assert_eq!(summary(&l).len(), 3);
    }

    #[test]
    fn oldest_age() {
        assert_eq!(oldest_age(&lines(), &[120_000, 10_000]), 120_000);
        assert_eq!(oldest_age(&[], &[]), 0);
    }
}
