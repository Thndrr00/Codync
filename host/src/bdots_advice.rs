#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers that turn the bot's existing findings into a short, prioritised advice list.
//! Every input is derived from data the host already computes for `botInsights`; this module only
//! shapes it. Reuses `bdots_health`, `bdots_profile`, `bdots_activity` and `bdots_text`.

/// One piece of advice.
#[derive(Clone, Debug, PartialEq)]
pub struct Advice {
    pub id: &'static str,
    pub title: String,
    pub detail: String,
    pub icon: &'static str,
    pub tone: &'static str,
    pub priority: u8,
}

fn advice(
    id: &'static str,
    title: &str,
    detail: String,
    icon: &'static str,
    tone: &'static str,
    priority: u8,
) -> Advice {
    Advice { id, title: title.to_owned(), detail, icon, tone, priority }
}

/// Priority label for a number (1 is highest).
pub fn priority_label(priority: u8) -> &'static str {
    match priority {
        0 => "Now",
        1 => "Soon",
        _ => "Optional",
    }
}

/// SF Symbol for a priority.
pub fn priority_icon(priority: u8) -> &'static str {
    match priority {
        0 => "exclamationmark.circle.fill",
        1 => "clock.badge.exclamationmark",
        _ => "lightbulb",
    }
}

/// Semantic tone for a priority.
pub fn priority_tone(priority: u8) -> &'static str {
    match priority {
        0 => "danger",
        1 => "attention",
        _ => "good",
    }
}

/// Advice for a missing name.
pub fn name_advice(name: &str) -> Option<Advice> {
    if name.trim().is_empty() {
        Some(advice(
            "name",
            "Name this bot",
            "A name makes it easy to find in the roster.".into(),
            "pencil",
            "attention",
            1,
        ))
    } else {
        None
    }
}

/// Advice for a missing description.
pub fn description_advice(description: &str) -> Option<Advice> {
    if description.trim().is_empty() {
        Some(advice(
            "description",
            "Add a description",
            "Other bots and you can see what it does.".into(),
            "text.badge.xmark",
            "attention",
            1,
        ))
    } else {
        None
    }
}

/// Advice for a missing workspace folder.
pub fn folder_advice(cwd: &str) -> Option<Advice> {
    if cwd.trim().is_empty() {
        Some(advice(
            "folder",
            "Pick a folder",
            "Give this bot a workspace for its files.".into(),
            "folder.badge.questionmark",
            "danger",
            0,
        ))
    } else {
        None
    }
}

/// Advice for a missing model.
pub fn model_advice(model: Option<&str>) -> Option<Advice> {
    if !crate::bdots_desk::is_model_id(model.unwrap_or("")) {
        Some(advice("model", "Choose a model", "Pick a model so the bot can think.".into(), "cpu", "attention", 1))
    } else {
        None
    }
}

/// Advice when no routines are scheduled.
pub fn routine_advice(count: usize) -> Option<Advice> {
    if count == 0 {
        Some(advice(
            "routines",
            "Add a routine",
            "Schedule work so it runs without you.".into(),
            "calendar.badge.clock",
            "good",
            2,
        ))
    } else {
        None
    }
}

/// Advice when no connectors are attached.
pub fn connector_advice(count: usize) -> Option<Advice> {
    if count == 0 {
        Some(advice(
            "connectors",
            "Connect an account",
            "Let this bot reach the apps it needs.".into(),
            "plugs",
            "good",
            2,
        ))
    } else {
        None
    }
}

/// Advice when no skills are installed.
pub fn skill_advice(count: usize) -> Option<Advice> {
    if count == 0 {
        Some(advice(
            "skills",
            "Add a skill",
            "Skills give the bot repeatable procedures.".into(),
            "puzzlepiece.extension",
            "good",
            2,
        ))
    } else {
        None
    }
}

/// Advice when no fallback agent is configured.
pub fn fallback_advice(count: usize, self_fallbacks: bool) -> Option<Advice> {
    if count == 0 && !self_fallbacks {
        Some(advice(
            "fallbacks",
            "Add a fallback",
            "Another agent can take over if this one stops.".into(),
            "arrow.triangle.turn.up.right.diamond",
            "attention",
            1,
        ))
    } else {
        None
    }
}

/// Advice when memory is off.
pub fn memory_advice(shared: Option<bool>, harness: bool, skills: &[String]) -> Option<Advice> {
    if !crate::bdots_memory::has_any(shared, harness, skills) {
        Some(advice("memory", "Turn on memory", "Let the bot remember between turns.".into(), "brain", "good", 2))
    } else {
        None
    }
}

/// Advice for a permissive approval mode.
pub fn trust_advice(mode: &str) -> Option<Advice> {
    if mode == "auto" {
        Some(advice(
            "trust",
            "Review approvals",
            "This bot approves requests on its own.".into(),
            "hand.raised",
            "attention",
            1,
        ))
    } else {
        None
    }
}

/// Advice when the daily budget is high.
pub fn budget_advice(used: u64, limit: Option<u64>) -> Option<Advice> {
    let percent = crate::bdots_activity::percent(used as usize, limit.unwrap_or(0) as usize);
    if limit.is_some_and(|l| l > 0) && percent >= 85 {
        Some(advice(
            "budget",
            "Watch the budget",
            "Today's tokens are nearly spent.".into(),
            "gauge.badge.exclamationmark",
            "attention",
            1,
        ))
    } else {
        None
    }
}

/// Advice when the reader is behind.
pub fn sync_advice(rev: i64, read_rev: i64) -> Option<Advice> {
    if crate::bdots_session::unread_revisions(rev, read_rev) > 20 {
        Some(advice(
            "sync",
            "Catch up on chat",
            "Many entries are still unread.".into(),
            "envelope.badge",
            "secondary",
            2,
        ))
    } else {
        None
    }
}

/// Count advice by priority.
pub fn count_by_priority(items: &[Advice]) -> Vec<(u8, usize)> {
    let mut out = Vec::new();
    for p in [0, 1, 2] {
        let n = items.iter().filter(|a| a.priority == p).count();
        if n > 0 {
            out.push((p, n));
        }
    }
    out
}

/// Number of high-priority items.
pub fn urgent_count(items: &[Advice]) -> usize {
    items.iter().filter(|a| a.priority == 0).count()
}

/// 0–100 readiness score: start full, subtract per suggestion.
pub fn readiness_score(items: &[Advice]) -> u32 {
    let penalty: u32 = items
        .iter()
        .map(|a| match a.priority {
            0 => 25,
            1 => 12,
            _ => 5,
        })
        .sum();
    100u32.saturating_sub(penalty)
}

/// Semantic tone for a readiness score.
pub fn readiness_tone(score: u32) -> &'static str {
    if score >= 80 {
        "good"
    } else if score >= 50 {
        "attention"
    } else {
        "danger"
    }
}

/// Short label for a readiness score.
pub fn readiness_label(score: u32) -> &'static str {
    if score >= 80 {
        "Ready"
    } else if score >= 50 {
        "Nearly ready"
    } else {
        "Needs setup"
    }
}

/// One plain sentence for the advice card.
pub fn advice_headline(items: &[Advice]) -> String {
    if items.is_empty() {
        return "Nothing to improve right now.".to_owned();
    }
    let score = readiness_score(items);
    format!("{} · {} suggestion{}", readiness_label(score), items.len(), if items.len() == 1 { "" } else { "s" })
}

/// Advice sorted by priority, then title.
pub fn sort_advice(items: &[Advice]) -> Vec<Advice> {
    let mut out = items.to_vec();
    out.sort_by(|a, b| a.priority.cmp(&b.priority).then_with(|| a.title.cmp(&b.title)));
    out
}

/// Up to `max` suggestions, keeping the most urgent.
pub fn top_advice(items: &[Advice], max: usize) -> Vec<Advice> {
    sort_advice(items).into_iter().take(max).collect()
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(items: &[Advice]) -> String {
    crate::bdots_desk::compact_count(items.len() as u64)
}

/// Build the whole advice list from the bot's current state.
pub fn build_all(
    name: &str,
    description: &str,
    cwd: &str,
    model: Option<&str>,
    routines: usize,
    connectors: usize,
    skills: usize,
    fallbacks: usize,
    self_fallbacks: bool,
    shared: Option<bool>,
    harness: bool,
    memory_skills: &[String],
    mode: &str,
    used: u64,
    limit: Option<u64>,
    rev: i64,
    read_rev: i64,
) -> Vec<Advice> {
    [
        folder_advice(cwd),
        name_advice(name),
        description_advice(description),
        model_advice(model),
        trust_advice(mode),
        budget_advice(used, limit),
        fallback_advice(fallbacks, self_fallbacks),
        routine_advice(routines),
        connector_advice(connectors),
        skill_advice(skills),
        memory_advice(shared, harness, memory_skills),
        sync_advice(rev, read_rev),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// Summary rows for the card.
pub fn summary(items: &[Advice]) -> Vec<(&'static str, String)> {
    vec![
        ("total", count_label(items)),
        ("urgent", urgent_count(items).to_string()),
        ("score", readiness_score(items).to_string()),
        ("headline", advice_headline(items)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty() -> Vec<Advice> {
        build_all(
            "Ada",
            "helps",
            "/work",
            Some("claude-sonnet-4"),
            2,
            1,
            1,
            1,
            false,
            Some(true),
            true,
            &[],
            "ask",
            10,
            Some(100),
            5,
            5,
        )
    }

    #[test]
    fn priority_helpers() {
        assert_eq!(priority_label(0), "Now");
        assert_eq!(priority_icon(2), "lightbulb");
        assert_eq!(priority_tone(1), "attention");
    }

    #[test]
    fn individual_advice() {
        assert!(name_advice("").is_some());
        assert!(name_advice("Ada").is_none());
        assert!(description_advice("").is_some());
        assert!(folder_advice("").is_some());
        assert!(model_advice(None).is_some());
        assert!(routine_advice(0).is_some());
        assert!(connector_advice(0).is_some());
        assert!(skill_advice(0).is_some());
        assert!(fallback_advice(0, false).is_some());
        assert!(memory_advice(None, false, &[]).is_some());
        assert!(trust_advice("auto").is_some());
        assert!(budget_advice(90, Some(100)).is_some());
        assert!(sync_advice(50, 5).is_some());
    }

    #[test]
    fn no_advice_when_configured() {
        assert!(empty().is_empty());
    }

    #[test]
    fn counts_scores_and_labels() {
        let items = build_all("", "", "", None, 0, 0, 0, 0, false, None, false, &[], "auto", 95, Some(100), 50, 5);
        assert_eq!(items.len(), 12);
        assert!(urgent_count(&items) > 0);
        assert_eq!(readiness_score(&[]), 100);
        assert_eq!(readiness_tone(90), "good");
        assert_eq!(readiness_label(40), "Needs setup");
        assert!(advice_headline(&[]).contains("Nothing"));
        assert_eq!(count_label(&items), "12");
        assert_eq!(summary(&items).len(), 4);
    }

    #[test]
    fn ordering_and_counts() {
        let items = build_all("", "", "", None, 0, 0, 0, 0, false, None, false, &[], "auto", 95, Some(100), 50, 5);
        let sorted = sort_advice(&items);
        assert_eq!(sorted[0].priority, 0);
        assert_eq!(top_advice(&items, 1).len(), 1);
        assert_eq!(count_by_priority(&items)[0].0, 0);
    }
}
