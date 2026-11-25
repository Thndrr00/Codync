#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers for a compact executive digest of a bot. Every value is derived from findings
//! the host already computes for `botInsights`; this module only shapes and ranks them.
//! Reuses `bdots_health`, `bdots_advice`, `bdots_week`, `bdots_session` and `bdots_text`.

/// One digest stat: a label, a value and how to render it.
#[derive(Clone, Debug, PartialEq)]
pub struct DigestStat {
    pub id: &'static str,
    pub label: String,
    pub value: String,
    pub icon: &'static str,
    pub tone: &'static str,
}

fn stat(id: &'static str, label: &str, value: String, icon: &'static str, tone: &'static str) -> DigestStat {
    DigestStat { id, label: label.to_owned(), value, icon, tone }
}

/// SF Symbol for a digest stat.
pub fn stat_icon(id: &str) -> &'static str {
    match id {
        "health" => "heart.fill",
        "work" => "wrench.fill",
        "week" => "calendar",
        "routine" => "clock.badge.checkmark",
        "memory" => "brain",
        "sync" => "arrow.triangle.2.circlepath",
        "advice" => "lightbulb.fill",
        _ => "info.circle",
    }
}

/// Semantic tone for a digest stat.
pub fn stat_tone(id: &str, good: bool) -> &'static str {
    if !good {
        return "attention";
    }
    match id {
        "health" | "advice" => "good",
        "week" | "routine" | "sync" => "accent",
        _ => "secondary",
    }
}

/// Format a health score for the digest.
pub fn health_value(score: u32) -> String {
    format!("{score}/100")
}

/// Whether a health score is good.
pub fn health_good(score: u32) -> bool {
    score >= 70
}

/// Format a step count, reusing the shared compact formatter.
pub fn work_value(steps: usize) -> String {
    crate::bdots_desk::compact_count(steps as u64)
}

/// Format a weekly total with its trend label.
pub fn week_value(total: usize, trend: &str) -> String {
    if total == 0 {
        "quiet".to_owned()
    } else {
        format!("{} · {}", crate::bdots_desk::compact_count(total as u64), trend)
    }
}

/// Format a routine count.
pub fn routine_value(active: usize, total: usize) -> String {
    format!("{active}/{total}")
}

/// Format a memory score.
pub fn memory_value(score: u32) -> String {
    crate::bdots_advice::readiness_label(score).to_owned()
}

/// Format a sync score.
pub fn sync_value(score: u32) -> String {
    crate::bdots_session::sync_label(score).to_owned()
}

/// Format an advice count.
pub fn advice_value(count: usize) -> String {
    if count == 0 { "all clear".to_owned() } else { format!("{count} to review") }
}

/// Build every digest stat from already-computed values.
pub fn build_stats(
    health_score: u32,
    steps: usize,
    week_total: usize,
    week_trend: &str,
    routines_active: usize,
    routines_total: usize,
    memory_score: u32,
    sync_score: u32,
    advice_count: usize,
) -> Vec<DigestStat> {
    vec![
        stat(
            "health",
            "Health",
            health_value(health_score),
            stat_icon("health"),
            stat_tone("health", health_good(health_score)),
        ),
        stat("work", "Recent work", work_value(steps), stat_icon("work"), stat_tone("work", steps > 0)),
        stat(
            "week",
            "This week",
            week_value(week_total, week_trend),
            stat_icon("week"),
            stat_tone("week", week_total > 0),
        ),
        stat(
            "routine",
            "Routines",
            routine_value(routines_active, routines_total),
            stat_icon("routine"),
            stat_tone("routine", routines_active > 0),
        ),
        stat(
            "memory",
            "Memory",
            memory_value(memory_score),
            stat_icon("memory"),
            stat_tone("memory", memory_score >= 30),
        ),
        stat("sync", "Sync", sync_value(sync_score), stat_icon("sync"), stat_tone("sync", sync_score >= 80)),
        stat(
            "advice",
            "Suggestions",
            advice_value(advice_count),
            stat_icon("advice"),
            stat_tone("advice", advice_count == 0),
        ),
    ]
}

/// Count stats rendered in a good tone.
pub fn good_count(stats: &[DigestStat]) -> usize {
    stats.iter().filter(|s| s.tone == "good" || s.tone == "accent").count()
}

/// Count stats needing attention.
pub fn attention_count(stats: &[DigestStat]) -> usize {
    stats.len() - good_count(stats)
}

/// A 0–100 overall score from the stat tones.
pub fn overall_score(stats: &[DigestStat]) -> u32 {
    if stats.is_empty() {
        return 0;
    }
    ((good_count(stats) as u32 * 100) / stats.len() as u32).min(100)
}

/// Semantic tone for the overall score.
pub fn overall_tone(score: u32) -> &'static str {
    if score >= 80 {
        "good"
    } else if score >= 50 {
        "attention"
    } else {
        "danger"
    }
}

/// Short label for the overall score.
pub fn overall_label(score: u32) -> &'static str {
    if score >= 80 {
        "Looking good"
    } else if score >= 50 {
        "Getting there"
    } else {
        "Needs attention"
    }
}

/// One plain sentence for the digest.
pub fn digest_headline(stats: &[DigestStat]) -> String {
    if stats.is_empty() {
        return "Nothing to show yet.".to_owned();
    }
    let score = overall_score(stats);
    format!("{} · {} of {} areas on track", overall_label(score), good_count(stats), stats.len())
}

/// The two stats with the most urgent tone, for a top strip.
pub fn highlights(stats: &[DigestStat], max: usize) -> Vec<&DigestStat> {
    let mut out: Vec<&DigestStat> = stats.iter().collect();
    out.sort_by(|a, b| {
        let rank = |s: &DigestStat| {
            if s.tone == "attention" {
                0
            } else if s.tone == "accent" {
                1
            } else {
                2
            }
        };
        rank(a).cmp(&rank(b)).then_with(|| a.id.cmp(b.id))
    });
    out.into_iter().take(max).collect()
}

/// Stats sorted by id, for a stable list across clients.
pub fn sorted(stats: &[DigestStat]) -> Vec<&DigestStat> {
    let mut out: Vec<&DigestStat> = stats.iter().collect();
    out.sort_by(|a, b| a.id.cmp(b.id));
    out
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(stats: &[DigestStat]) -> String {
    crate::bdots_desk::compact_count(stats.len() as u64)
}

/// A one-line summary built from selected stat values.
pub fn one_line(stats: &[DigestStat]) -> String {
    let pick = |id: &str| stats.iter().find(|s| s.id == id).map_or_else(|| "-".to_owned(), |s| s.value.clone());
    format!("{} · {} · {}", pick("health"), pick("week"), pick("advice"))
}

/// Summary rows for the card.
pub fn summary(stats: &[DigestStat]) -> Vec<(&'static str, String)> {
    vec![
        ("areas", count_label(stats)),
        ("onTrack", good_count(stats).to_string()),
        ("attention", attention_count(stats).to_string()),
        ("headline", digest_headline(stats)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stats() -> Vec<DigestStat> {
        build_stats(85, 40, 120, "More active", 3, 4, 90, 100, 2)
    }

    #[test]
    fn icons_and_tones() {
        assert_eq!(stat_icon("health"), "heart.fill");
        assert_eq!(stat_icon("advice"), "lightbulb.fill");
        assert_eq!(stat_tone("health", true), "good");
        assert_eq!(stat_tone("health", false), "attention");
    }

    #[test]
    fn value_formatting() {
        assert_eq!(health_value(85), "85/100");
        assert!(health_good(85));
        assert!(!health_good(50));
        assert_eq!(work_value(1500), "1.5K");
        assert_eq!(week_value(0, "quiet"), "quiet");
        assert!(week_value(10, "More active").contains("More active"));
        assert_eq!(routine_value(3, 4), "3/4");
        assert_eq!(advice_value(0), "all clear");
        assert_eq!(advice_value(2), "2 to review");
    }

    #[test]
    fn stats_are_built_for_every_area() {
        let s = stats();
        assert_eq!(s.len(), 7);
        assert!(s.iter().any(|x| x.id == "health"));
        assert!(s.iter().any(|x| x.id == "sync"));
    }

    #[test]
    fn scores_labels_and_headlines() {
        let s = stats();
        assert!(overall_score(&s) > 0);
        assert_eq!(overall_score(&[]), 0);
        assert_eq!(overall_tone(90), "good");
        assert_eq!(overall_label(40), "Needs attention");
        assert!(digest_headline(&[]).contains("Nothing"));
        assert!(digest_headline(&s).contains("on track"));
        assert_eq!(count_label(&s), "7");
    }

    #[test]
    fn ordering_and_one_line() {
        let s = stats();
        assert_eq!(sorted(&s)[0].id, "advice");
        assert_eq!(highlights(&s, 2).len(), 2);
        assert!(one_line(&s).contains("85/100"));
        assert_eq!(summary(&s).len(), 4);
        assert_eq!(good_count(&s) + attention_count(&s), s.len());
    }
}
