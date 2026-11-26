#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from hermes-agent's eval metrics and judge thresholds
//! (`evals/codebase_navigability/static_metrics.py`, `goal_judge_wait.py`): bounded scores,
//! verdicts, grades and aggregate pass rates. Composes helpers already ported into this host.
//! Only `botInsights` calls this.

/// Clamp a score into the 0–100 range.
pub fn clamp_score(score: i64) -> u32 {
    score.clamp(0, 100) as u32
}

/// Normalise a raw score against its maximum into a 0–100 percentage.
pub fn normalize_score(raw: f64, max: f64) -> u32 {
    if !raw.is_finite() || !max.is_finite() || max <= 0.0 {
        return 0;
    }
    clamp_score((raw / max * 100.0).round() as i64)
}

/// Passing threshold for an eval level.
pub fn threshold_for(level: &str) -> u32 {
    match level.to_ascii_lowercase().as_str() {
        "strict" => 90,
        "high" => 80,
        "normal" | "default" => 70,
        "loose" => 50,
        _ => 70,
    }
}

/// Verdict of a score against a threshold (judge wait).
pub fn verdict(score: u32, threshold: u32) -> &'static str {
    let score = clamp_score(i64::from(score));
    if score >= threshold.saturating_add(10) {
        "pass"
    } else if score >= threshold {
        "borderline"
    } else {
        "fail"
    }
}

/// Semantic tone for a verdict.
pub fn verdict_tone(verdict: &str) -> &'static str {
    match verdict {
        "pass" => "good",
        "borderline" => "attention",
        _ => "danger",
    }
}

/// Letter grade for a score.
pub fn grade(score: u32) -> &'static str {
    match clamp_score(i64::from(score)) {
        90..=100 => "A",
        80..=89 => "B",
        70..=79 => "C",
        60..=69 => "D",
        _ => "F",
    }
}

/// SF Symbol for a grade.
pub fn grade_icon(grade: &str) -> &'static str {
    match grade {
        "A" | "B" => "rosette",
        "C" => "medal",
        "D" | "F" => "xmark.circle.fill",
        _ => "questionmark.circle",
    }
}

/// Mean of a set of scores, rounded.
pub fn average(scores: &[u32]) -> u32 {
    if scores.is_empty() {
        return 0;
    }
    let sum: u64 = scores.iter().map(|s| u64::from(clamp_score(i64::from(*s)))).sum();
    clamp_score((sum / scores.len() as u64) as i64)
}

/// Median of a set of scores, rounded.
pub fn median(scores: &[u32]) -> u32 {
    if scores.is_empty() {
        return 0;
    }
    let mut sorted: Vec<u32> = scores.iter().map(|s| clamp_score(i64::from(*s))).collect();
    sorted.sort_unstable();
    let mid = sorted.len() / 2;
    if sorted.len() % 2 == 0 { clamp_score(i64::from(sorted[mid - 1] + sorted[mid]) / 2) } else { sorted[mid] }
}

/// Pass rate as a whole percent, reusing the shared ratio helper.
pub fn pass_rate(passed: usize, total: usize) -> u32 {
    crate::bdots_activity::percent(passed, total)
}

/// Whether two measurements agree within a tolerance.
pub fn is_within_tolerance(a: f64, b: f64, tolerance: f64) -> bool {
    (a - b).abs() <= tolerance.abs()
}

/// Whether a sample count is large enough for a stable verdict.
pub fn enough_samples(samples: usize) -> bool {
    samples >= 3
}

/// Summary rows for the card.
pub fn summary(score: u32, threshold: u32, samples: usize) -> Vec<(&'static str, String)> {
    vec![
        ("score", score.to_string()),
        ("grade", grade(score).to_owned()),
        ("threshold", threshold.to_string()),
        ("samples", crate::bdots_desk::compact_count(samples as u64)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamping_and_normalising() {
        assert_eq!(clamp_score(-5), 0);
        assert_eq!(clamp_score(150), 100);
        assert_eq!(normalize_score(37.5, 50.0), 75);
        assert_eq!(normalize_score(1.0, 0.0), 0);
    }

    #[test]
    fn thresholds_and_verdicts() {
        assert_eq!(threshold_for("strict"), 90);
        assert_eq!(threshold_for("weird"), 70);
        assert_eq!(verdict(95, 80), "pass");
        assert_eq!(verdict(85, 80), "borderline");
        assert_eq!(verdict(50, 80), "fail");
        assert_eq!(verdict_tone("borderline"), "attention");
    }

    #[test]
    fn grades_and_icons() {
        assert_eq!(grade(95), "A");
        assert_eq!(grade(75), "C");
        assert_eq!(grade(20), "F");
        assert_eq!(grade_icon("A"), "rosette");
        assert_eq!(grade_icon("F"), "xmark.circle.fill");
    }

    #[test]
    fn aggregates() {
        assert_eq!(average(&[]), 0);
        assert_eq!(average(&[60, 80, 100]), 80);
        assert_eq!(median(&[90, 60, 80]), 80);
        assert_eq!(median(&[60, 80]), 70);
        assert_eq!(pass_rate(1, 4), 25);
        assert!(is_within_tolerance(1.01, 1.0, 0.02));
        assert!(!is_within_tolerance(1.1, 1.0, 0.02));
        assert!(enough_samples(3));
        assert!(!enough_samples(2));
    }

    #[test]
    fn summary_rows() {
        assert_eq!(crate::bdots_desk::compact_count(1500), "1.5K");
        assert_eq!(summary(85, 70, 4).len(), 4);
    }
}
