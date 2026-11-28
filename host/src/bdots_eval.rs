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

// ===== New pure ports from hermes-agent evals =====
// `evals/codebase_navigability/static_metrics.py` (static metrics + thresholds +
// first-party import-graph resolution + Tarjan SCC cycle detection) and
// `evals/postmortem/live_ab/goal_judge_wait.py` (`n` rounds default 3; verdict
// collection / `SUMMARY`; `judge_goal` verdicts `done/blocked/continue/wait/skipped`,
// fail-open to `continue`). 100% pure & deterministic; no I/O; no new crates.

use std::collections::{BTreeMap, BTreeSet};

/// Total source lines, matching hermes `src.count("\n") + (0 if src.endswith("\n") else 1)`.
pub fn count_lines(src: &str) -> usize {
    let newlines = src.matches('\n').count();
    if src.ends_with('\n') { newlines } else { newlines + 1 }
}

/// Approximate hermes `code_lines` fallback: non-blank lines that are not `#` comments.
/// (Pure; does not run Python `tokenize`/`ast`, so it approximates "code" without docstring removal.)
pub fn approx_code_lines(src: &str) -> usize {
    src.split('\n')
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with('#')
        })
        .count()
}

/// Count values strictly greater than `threshold` (hermes `files_gt_*`, `funcs_gt_*`, `cc_gt_*`, `mi`... `L > t`).
pub fn count_above(values: &[f64], threshold: f64) -> usize {
    values.iter().filter(|v| **v > threshold).count()
}

/// Count values greater than or equal to `threshold` (hermes `k >= 4`, `d >= 5`, `elif_chains_ge*`, `nesting_ge5`).
pub fn count_at_least(values: &[usize], threshold: usize) -> usize {
    values.iter().filter(|v| **v >= threshold).count()
}

/// Count values strictly below `threshold` (hermes `mi_lt_20`, `mi_lt_10`, `v < t`).
pub fn count_below(values: &[f64], threshold: f64) -> usize {
    values.iter().filter(|v| **v < threshold).count()
}

/// hermes `pct(a, q)`: `a[int(len(a) * q) - 1] if a else 0`, Python-style negative index `-1` => last.
/// `sorted` must already be ascending (caller sorts; `funcs` in hermes is `funcs.sort()`ed).
pub fn percentile(sorted: &[f64], q: f64) -> f64 {
    let n = sorted.len();
    if n == 0 {
        return 0.0;
    }
    let raw = (n as f64 * q) as i64 - 1;
    let idx = if raw < 0 { n as i64 + raw } else { raw };
    let idx = idx.clamp(0, n as i64 - 1) as usize;
    sorted[idx]
}

/// Round to two decimals like hermes `round(x, 2)`.
pub fn round2(x: f64) -> f64 {
    if !x.is_finite() {
        return x;
    }
    (x * 100.0).round() / 100.0
}

/// hermes `resolve(name)`: shrink a dotted module name to the longest first-party prefix in `first_party`.
pub fn resolve_module_name(name: &str, first_party: &BTreeSet<String>) -> String {
    let mut current = name;
    while !current.is_empty() && !first_party.contains(current) {
        current = match current.rfind('.') {
            Some(dot) => &current[..dot],
            None => "",
        };
    }
    current.to_owned()
}

/// Tarjan strongly-connected components over a `module -> imports` graph (hermes `strong` + `sccs`).
/// Deterministic: neighbors are visited in sorted order; components and members are sorted.
pub fn strongly_connected_components(edges: &BTreeMap<String, BTreeSet<String>>) -> Vec<Vec<String>> {
    struct Tarjan<'a> {
        edges: &'a BTreeMap<String, BTreeSet<String>>,
        index: BTreeMap<String, usize>,
        low: BTreeMap<String, usize>,
        on_stack: BTreeSet<String>,
        stack: Vec<String>,
        counter: usize,
        sccs: Vec<Vec<String>>,
    }

    impl<'a> Tarjan<'a> {
        fn strong(&mut self, v: &str) {
            let idx = self.counter;
            self.index.insert(v.to_owned(), idx);
            self.low.insert(v.to_owned(), idx);
            self.counter += 1;
            self.stack.push(v.to_owned());
            self.on_stack.insert(v.to_owned());

            let neighbors: Vec<String> = self.edges.get(v).map(|s| s.iter().cloned().collect()).unwrap_or_default();
            for w in neighbors {
                if !self.index.contains_key(&w) {
                    self.strong(&w);
                    let lw = self.low[&w];
                    let lv = self.low[v];
                    self.low.insert(v.to_owned(), lw.min(lv));
                } else if self.on_stack.contains(&w) {
                    let iw = self.index[&w];
                    let lv = self.low[v];
                    self.low.insert(v.to_owned(), iw.min(lv));
                }
            }

            if self.low[v] == self.index[v] {
                let mut comp: Vec<String> = Vec::new();
                while let Some(w) = self.stack.pop() {
                    self.on_stack.remove(&w);
                    let done = w == v;
                    comp.push(w);
                    if done {
                        break;
                    }
                }
                comp.sort();
                self.sccs.push(comp);
            }
        }
    }

    let mut tarjan = Tarjan {
        edges,
        index: BTreeMap::new(),
        low: BTreeMap::new(),
        on_stack: BTreeSet::new(),
        stack: Vec::new(),
        counter: 0,
        sccs: Vec::new(),
    };
    for v in edges.keys() {
        if !tarjan.index.contains_key(v) {
            tarjan.strong(v);
        }
    }
    let mut sccs = tarjan.sccs;
    sccs.sort();
    sccs
}

/// Normalise a goal-judge verdict string (hermes `_parse_judge_response` + `judge_goal`):
/// trim + lowercase; `done/blocked/wait/skipped` pass through; anything else => `continue`.
pub fn normalize_judge_verdict(raw: &str) -> &'static str {
    match raw.trim().to_ascii_lowercase().as_str() {
        "done" => "done",
        "blocked" => "blocked",
        "wait" => "wait",
        "skipped" => "skipped",
        _ => "continue",
    }
}

/// `goal_judge_wait.py`: `n = int(argv[2]) if len(argv) > 2 else 3` rounds. Invalid => 3. `n` is not clamped.
pub fn judge_wait_rounds(arg: Option<&str>) -> i64 {
    match arg {
        Some(s) => s.trim().parse::<i64>().unwrap_or(3),
        None => 3,
    }
}

/// `goal_judge_wait.py` `SUMMARY`: count verdicts in fixed order `[done, blocked, continue, wait, skipped]`,
/// normalising each with `normalize_judge_verdict`. Unknown/empty verdicts count as `continue`.
pub fn summarize_verdicts(verdicts: &[&str]) -> Vec<(&'static str, usize)> {
    const ORDER: [&str; 5] = ["done", "blocked", "continue", "wait", "skipped"];
    let mut counts = [0usize; 5];
    for verdict in verdicts {
        match normalize_judge_verdict(verdict) {
            "done" => counts[0] += 1,
            "blocked" => counts[1] += 1,
            "continue" => counts[2] += 1,
            "wait" => counts[3] += 1,
            "skipped" => counts[4] += 1,
            _ => {}
        }
    }
    ORDER.iter().copied().zip(counts).collect()
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

    // ----- new hermes-agent eval ports -----

    #[test]
    fn count_lines_matches_hermes() {
        assert_eq!(count_lines("a\nb\n"), 2);
        assert_eq!(count_lines("a\nb"), 2);
        assert_eq!(count_lines("a\n\n"), 2);
        assert_eq!(count_lines(""), 1);
        assert_eq!(count_lines("x"), 1);
    }

    #[test]
    fn approx_code_lines_skips_blanks_and_comments() {
        assert_eq!(approx_code_lines("a\n\n# c\nb\n   # indented\nc"), 3);
        assert_eq!(approx_code_lines(""), 0);
        assert_eq!(approx_code_lines("# only a comment"), 0);
    }

    #[test]
    fn count_thresholds() {
        assert_eq!(count_above(&[100.0, 200.0, 1001.0], 1000.0), 1);
        assert_eq!(count_above(&[100.0, 1000.0], 1000.0), 0);
        assert_eq!(count_at_least(&[1, 3, 4, 8], 4), 2);
        assert_eq!(count_at_least(&[1, 2, 3], 4), 0);
        assert_eq!(count_below(&[19.0, 20.0, 10.0], 20.0), 2);
        assert_eq!(count_below(&[20.0, 21.0], 20.0), 0);
    }

    #[test]
    fn percentile_matches_pct() {
        assert_eq!(percentile(&[], 0.5), 0.0);
        assert_eq!(percentile(&[1.0, 2.0, 3.0, 4.0], 0.5), 2.0);
        assert_eq!(percentile(&[1.0, 2.0, 3.0, 4.0], 0.95), 3.0);
        assert_eq!(percentile(&[1.0, 2.0, 3.0], 0.5), 1.0);
        assert_eq!(percentile(&[5.0], 0.95), 5.0); // int(1*.95)=0 -> -1 -> last
        assert_eq!(percentile(&[1.0, 2.0], 0.5), 1.0); // int(2*.5)=1 -> a[0]
    }

    #[test]
    fn round2_two_decimals() {
        assert_eq!(round2(1.23456), 1.23);
        assert_eq!(round2(9.8765), 9.88);
        assert_eq!(round2(1.23456 * 100.0 / 100.0), 1.23);
        assert!(round2(f64::NAN).is_nan());
    }

    #[test]
    fn resolve_module_name_first_party() {
        let fp: BTreeSet<String> = ["a".to_owned(), "a.b".to_owned()].into_iter().collect();
        assert_eq!(resolve_module_name("a.b.c", &fp), "a.b");
        assert_eq!(resolve_module_name("a.b", &fp), "a.b");
        assert_eq!(resolve_module_name("a", &fp), "a");
        assert_eq!(resolve_module_name("x.y", &fp), "");
        assert_eq!(resolve_module_name("x", &fp), "");
    }

    #[test]
    fn scc_finds_cycles() {
        let mut edges: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        edges.entry("a".to_owned()).or_default().insert("b".to_owned());
        edges.entry("b".to_owned()).or_default().insert("a".to_owned());
        edges.entry("b".to_owned()).or_default().insert("c".to_owned());
        edges.entry("c".to_owned()).or_default().insert("c".to_owned());
        edges.entry("d".to_owned()).or_default().insert("a".to_owned());
        let sccs = strongly_connected_components(&edges);
        assert!(sccs.contains(&vec!["a".to_owned(), "b".to_owned()]));
        assert!(sccs.contains(&vec!["c".to_owned()]));
        assert!(sccs.contains(&vec!["d".to_owned()]));
        let cycles = sccs.iter().filter(|c| c.len() > 1).count();
        assert_eq!(cycles, 1);
    }

    #[test]
    fn scc_acyclic_singletons() {
        let mut edges: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        edges.entry("a".to_owned()).or_default().insert("b".to_owned());
        edges.entry("b".to_owned()).or_default();
        let sccs = strongly_connected_components(&edges);
        assert_eq!(sccs, vec![vec!["a".to_owned()], vec!["b".to_owned()]]);
    }

    #[test]
    fn normalize_judge_verdict_cases() {
        assert_eq!(normalize_judge_verdict(" DONE "), "done");
        assert_eq!(normalize_judge_verdict("blocked"), "blocked");
        assert_eq!(normalize_judge_verdict("Wait"), "wait");
        assert_eq!(normalize_judge_verdict("skipped"), "skipped");
        assert_eq!(normalize_judge_verdict("continue"), "continue");
        assert_eq!(normalize_judge_verdict("garbage"), "continue");
        assert_eq!(normalize_judge_verdict(""), "continue");
    }

    #[test]
    fn judge_wait_rounds_default_and_parse() {
        assert_eq!(judge_wait_rounds(None), 3);
        assert_eq!(judge_wait_rounds(Some("5")), 5);
        assert_eq!(judge_wait_rounds(Some(" 4 ")), 4);
        assert_eq!(judge_wait_rounds(Some("nope")), 3);
        assert_eq!(judge_wait_rounds(Some("-1")), -1);
    }

    #[test]
    fn summarize_verdicts_counts() {
        let v = ["done", "continue", "DONE", "wait", "blocked", "skipped", "garbage", ""];
        let summary = summarize_verdicts(&v);
        assert_eq!(summary, vec![("done", 2), ("blocked", 1), ("continue", 3), ("wait", 1), ("skipped", 1)]);
        assert_eq!(
            summarize_verdicts(&[]),
            vec![("done", 0), ("blocked", 0), ("continue", 0), ("wait", 0), ("skipped", 0)]
        );
    }
}
