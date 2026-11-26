#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Extra pure helpers ported from hermes-agent's evals
//! (`evals/codebase_navigability/static_metrics.py` for static code metrics, thresholds and
//! aggregations; `evals/postmortem/live_ab/goal_judge_wait.py` plus `hermes_cli/goals.py`
//! `_parse_judge_response` / `_extract_json_object` / `_truncate` for judge verdicts and wait
//! directives). Deterministic, no I/O, no new crates. Nothing here duplicates helpers already
//! ported into this host — score clamping/normalisation, verdicts, grades and aggregation stay in
//! `crate::bdots_eval`, `crate::bdots_activity::percent`, etc. Only `botInsights` calls this.
//! A new module beside `bdots_eval` (no edits to other files).

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

/// Top-level directories skipped by `static_metrics.py` (`SKIP`).
pub const STATIC_METRICS_SKIP_DIRS: [&str; 15] = [
    ".git",
    "node_modules",
    "apps",
    "website",
    "build",
    ".venv",
    "venv",
    "MagicMock",
    "__pycache__",
    ".worktrees",
    "dist",
    "evals",
    "skills",
    "optional-skills",
    "docs",
];

/// File-length thresholds reported by `static_metrics.py`
/// (`files_gt_1000` / `files_gt_2000` / `files_gt_5000`).
pub const FILE_LINES_GT_THRESHOLDS: [u64; 3] = [1000, 2000, 5000];

/// Function-length thresholds reported by `static_metrics.py` (`funcs_gt_100` / `funcs_gt_300`).
pub const FUNC_LEN_GT_THRESHOLDS: [u64; 2] = [100, 300];

/// `elif` chain lengths counted as `elif_chains_ge4`.
pub const ELIF_CHAIN_GE4: usize = 4;

/// `elif` chain lengths counted as `elif_chains_ge8`.
pub const ELIF_CHAIN_GE8: usize = 8;

/// Nesting depth counted as `nesting_ge5`.
pub const NESTING_GE5: usize = 5;

/// Judge verdicts (`goal_judge_wait.py` / `goals.py` `_parse_judge_response`); `skipped` is
/// added by `judge_goal`, the parser only emits `done` / `blocked` / `continue` / `wait`.
pub const JUDGE_VERDICTS: [&str; 5] = ["done", "blocked", "continue", "wait", "skipped"];

/// Cap on judge verdicts collected per `goal_judge_wait.py` run (`n`, default `3`).
pub const GOAL_JUDGE_WAIT_N: usize = 3;

/// `active_delegations` passed to `judge_goal` by `goal_judge_wait.py` (`4`).
pub const GOAL_JUDGE_ACTIVE_DELEGATIONS: usize = 4;

/// Snippet cap for the agent's last response sent to the judge (`_JUDGE_RESPONSE_SNIPPET_CHARS`).
pub const JUDGE_SNIPPET_CHARS: usize = 4000;

/// Snippet cap for the goal text sent to the judge (`_truncate(goal, 2000)`).
pub const JUDGE_GOAL_SNIPPET_CHARS: usize = 2000;

/// Marker appended when `truncate` cuts text (`_truncate`, `"… [truncated]"`).
pub const JUDGE_TRUNCATE_MARKER: &str = "\u{2026} [truncated]";

/// A parsed judge verdict (`_parse_judge_response` return, minus the LLM call).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JudgeResponse {
    /// `done` / `blocked` / `continue` / `wait`.
    pub verdict: &'static str,
    /// Human-readable reason (`"no reason provided"` when absent).
    pub reason: String,
    /// Judge output was unusable (empty / non-JSON); fail-open `continue`.
    pub parse_failed: bool,
    /// Wait target for a `wait` verdict; a `wait` with no target downgrades to `continue`.
    pub wait_directive: Option<WaitDirective>,
}

/// What a `wait` verdict waits on; prefer session, then pid, then seconds (`_parse_judge_response`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WaitDirective {
    /// `wait_on_session` / `session_id` / `wait_session` (releases on the process's own trigger).
    Session(String),
    /// `wait_on_pid` / `pid` / `wait_pid`, positive integer (releases on exit).
    Pid(u64),
    /// `wait_for_seconds` / `seconds` / `wait_seconds`, positive integer.
    Seconds(u64),
}

/// Cycle statistics over a first-party import graph (`import_cycles` / `largest_cycle` /
/// `modules_in_cycles`, Tarjan SCC).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ImportCycleStats {
    /// Number of strongly connected components with more than one module.
    pub import_cycles: usize,
    /// Size of the largest cycle (`0` when acyclic).
    pub largest_cycle: usize,
    /// Total modules inside any cycle.
    pub modules_in_cycles: usize,
}

/// Total line count of a source file (`lines`: `count('\n') + (0 if endswith('\n') else 1)`).
pub fn source_line_count(src: &str) -> u64 {
    let newlines = src.matches('\n').count() as u64;
    if src.ends_with('\n') { newlines } else { newlines + 1 }
}

/// Simplified non-blank, non-`#`-comment line count (`code_lines` fallback branch; the full
/// `tokenize`/AST docstring-aware counting is not portable without a Python parser).
pub fn code_line_count(src: &str) -> u64 {
    src.lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with('#')
        })
        .count() as u64
}

/// `pct`: `a[int(len(a) * q) - 1] if a else 0`, `int` truncates toward zero; Python negative
/// indexing for `int(len(a) * q) < 1`; `sorted_ascending` must be sorted; out-of-range -> `0`.
pub fn percentile_pick(sorted_ascending: &[u64], q: f64) -> u64 {
    if sorted_ascending.is_empty() {
        return 0;
    }
    let len = sorted_ascending.len() as i64;
    let scaled = if q.is_finite() { (sorted_ascending.len() as f64 * q).trunc() } else { 0.0 };
    let idx = scaled as i64 - 1;
    let resolved = if idx < 0 { len + idx } else { idx };
    if resolved < 0 || resolved >= len {
        return 0;
    }
    sorted_ascending[resolved as usize]
}

/// `median_file_lines`: `sorted(lines)[len // 2] if lines else 0` (upper-middle for even counts,
/// not the averaged median in `crate::bdots_eval::median`).
pub fn median_file_lines(lines: &[u64]) -> u64 {
    if lines.is_empty() {
        return 0;
    }
    let mut sorted = lines.to_vec();
    sorted.sort_unstable();
    sorted[sorted.len() / 2]
}

/// Count entries strictly greater than `threshold` (`funcs_gt_*`, `files_gt_*`, `cc_gt_*`, …).
pub fn count_gt(values: &[u64], threshold: u64) -> usize {
    values.iter().filter(|v| **v > threshold).count()
}

/// Count entries greater than or equal to `threshold` (`elif_chains_ge*`, `nesting_ge5`, …).
pub fn count_ge(values: &[u64], threshold: u64) -> usize {
    values.iter().filter(|v| **v >= threshold).count()
}

/// Whether `top` (a top-level directory name relative to the tree root) is skipped (`SKIP`).
pub fn skip_top_dir(top: &str) -> bool {
    STATIC_METRICS_SKIP_DIRS.contains(&top)
}

/// Dotted module name for a `.py` path relative to the tree (`rel[:-3].replace(sep, '.')` then
/// `removesuffix('.__init__')`); accepts `/` and `\` separators.
pub fn module_name_for_path(rel_py_path: &str) -> String {
    let without_ext = rel_py_path.strip_suffix(".py").unwrap_or(rel_py_path);
    let dotted = without_ext.replace(['/', '\\'], ".");
    dotted.strip_suffix(".__init__").unwrap_or(&dotted).to_owned()
}

/// Resolve a dotted import name to the longest first-party module prefix (`resolve`):
/// `while name and name not in fp: name = name.rpartition('.')[0]`.
pub fn resolve_module(name: &str, first_party: &[String]) -> String {
    let mut name = name.to_owned();
    while !name.is_empty() && !first_party.iter().any(|m| m == &name) {
        match name.rfind('.') {
            Some(dot) => name.truncate(dot),
            None => name.clear(),
        }
    }
    name
}

/// Tarjan SCC over a first-party import graph; `edges[a]` = modules `a` imports; targets not in
/// `edges` are treated as edge-less singletons (`strong`, `import_cycles`).
pub fn import_cycle_stats(edges: &BTreeMap<String, BTreeSet<String>>) -> ImportCycleStats {
    let mut index: BTreeMap<String, usize> = BTreeMap::new();
    let mut low: BTreeMap<String, usize> = BTreeMap::new();
    let mut onstack: BTreeSet<String> = BTreeSet::new();
    let mut stack: Vec<String> = Vec::new();
    let mut sccs: Vec<Vec<String>> = Vec::new();
    let mut counter = 0usize;
    for v in edges.keys() {
        if !index.contains_key(v) {
            strong(v, edges, &mut index, &mut low, &mut onstack, &mut stack, &mut sccs, &mut counter);
        }
    }
    let cycles: Vec<&Vec<String>> = sccs.iter().filter(|c| c.len() > 1).collect();
    ImportCycleStats {
        import_cycles: cycles.len(),
        largest_cycle: cycles.iter().map(|c| c.len()).max().unwrap_or(0),
        modules_in_cycles: cycles.iter().map(|c| c.len()).sum(),
    }
}

/// Best-effort JSON object extraction (`_extract_json_object`): strip ```` ``` ```` fences, parse
/// the blob, else pull the first `{...}` (`\{.*?\}` DOTALL); `None` unless the result is an object.
pub fn extract_json_object(raw: &str) -> Option<Value> {
    if raw.is_empty() {
        return None;
    }
    let mut text = raw.trim().to_owned();
    if text.starts_with("```") {
        text = text.trim_matches('`').to_owned();
        if let Some(nl) = text.find('\n') {
            text = text[nl + 1..].to_owned();
        }
    }
    if let Ok(data) = serde_json::from_str::<Value>(&text) {
        return if data.is_object() { Some(data) } else { None };
    }
    let span = first_brace_span(&text)?;
    match serde_json::from_str::<Value>(span) {
        Ok(data) if data.is_object() => Some(data),
        _ => None,
    }
}

/// Fail-open judge parsing (`_parse_judge_response`): `(verdict, reason, parse_failed,
/// wait_directive)`; verdict `done` / `blocked` / `continue` / `wait`, unknown -> `continue`;
/// accepts `{"verdict": ...}` and legacy `{"done": <bool|"true"|"yes"|"1"|"done">}`.
pub fn parse_judge_response(raw: &str) -> JudgeResponse {
    if raw.is_empty() {
        return JudgeResponse {
            verdict: "continue",
            reason: "judge returned empty response".to_owned(),
            parse_failed: true,
            wait_directive: None,
        };
    }
    let Some(data) = extract_json_object(raw) else {
        return JudgeResponse {
            verdict: "continue",
            reason: format!("judge reply was not JSON: {:?}", truncate(raw, 200)),
            parse_failed: true,
            wait_directive: None,
        };
    };
    let obj = data.as_object().cloned().unwrap_or_default();
    let reason = match json_reason(obj.get("reason")) {
        r if !r.is_empty() => r,
        _ => "no reason provided".to_owned(),
    };
    let verdict = match obj.get("verdict") {
        Some(Value::String(s)) => known_verdict(s),
        _ => {
            let done = match obj.get("done") {
                Some(Value::String(s)) => matches!(s.trim().to_lowercase().as_str(), "true" | "yes" | "1" | "done"),
                Some(v) => json_truthy(v),
                None => false,
            };
            if done { "done" } else { "continue" }
        }
    };
    if verdict != "wait" {
        return JudgeResponse { verdict, reason, parse_failed: false, wait_directive: None };
    }
    // Prefer session (releases on the process's own trigger), then pid (exit only), then seconds.
    if let Some(Value::String(sess)) = or_chain(&obj, &["wait_on_session", "session_id", "wait_session"]) {
        let trimmed = sess.trim();
        if !trimmed.is_empty() {
            return JudgeResponse {
                verdict: "wait",
                reason,
                parse_failed: false,
                wait_directive: Some(WaitDirective::Session(trimmed.to_owned())),
            };
        }
    }
    if let Some(pid) = json_first_int(&obj, &["wait_on_pid", "pid", "wait_pid"]) {
        return JudgeResponse {
            verdict: "wait",
            reason,
            parse_failed: false,
            wait_directive: Some(WaitDirective::Pid(pid)),
        };
    }
    if let Some(seconds) = json_first_int(&obj, &["wait_for_seconds", "seconds", "wait_seconds"]) {
        return JudgeResponse {
            verdict: "wait",
            reason,
            parse_failed: false,
            wait_directive: Some(WaitDirective::Seconds(seconds)),
        };
    }
    JudgeResponse {
        verdict: "continue",
        reason: format!("{reason} (wait verdict had no target \u{2014} continuing)"),
        parse_failed: false,
        wait_directive: None,
    }
}

/// `_truncate`: `""` stays empty; `text if len <= limit else text[:limit] + "… [truncated]"`
/// (`len`/`[:limit]` count Unicode code points).
fn truncate(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_owned();
    }
    let kept: String = text.chars().take(limit).collect();
    format!("{kept}{JUDGE_TRUNCATE_MARKER}")
}

/// `_JSON_OBJECT_RE` `\{.*?\}` DOTALL first match, as byte-safe slicing.
fn first_brace_span(text: &str) -> Option<&str> {
    let start = text.find('{')?;
    let rest = &text[start..];
    let end = rest.find('}')?;
    Some(&rest[..end + 1])
}

/// Verdict normalisation (`verdict.strip().lower()`; anything outside the set -> `continue`).
fn known_verdict(raw: &str) -> &'static str {
    match raw.trim().to_lowercase().as_str() {
        "done" => "done",
        "blocked" => "blocked",
        "wait" => "wait",
        "continue" => "continue",
        _ => "continue",
    }
}

/// Python truthiness for a JSON value.
fn json_truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(true),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

/// Python `or` chain over keys: first truthy value (`a or b or c`).
fn or_chain<'a>(obj: &'a serde_json::Map<String, Value>, keys: &[&str]) -> Option<&'a Value> {
    keys.iter().find_map(|k| obj.get(*k).filter(|v| json_truthy(v)))
}

/// `reason` extraction: `str(data.get("reason") or "").strip()`.
fn json_reason(v: Option<&Value>) -> String {
    let Some(v) = v else { return String::new() };
    if !json_truthy(v) {
        return String::new();
    }
    let s = match v {
        Value::String(s) => s.clone(),
        Value::Bool(b) => {
            if *b {
                "True".to_owned()
            } else {
                "False".to_owned()
            }
        }
        Value::Number(n) => n.to_string(),
        Value::Null => return String::new(),
        Value::Array(_) | Value::Object(_) => v.to_string(),
    };
    s.trim().to_owned()
}

/// `int(...)` coercion for a JSON scalar; `None` mirrors `TypeError`/`ValueError` -> `continue`.
fn json_as_int(v: &Value) -> Option<i64> {
    match v {
        Value::Null => Some(0),
        Value::Bool(b) => Some(if *b { 1 } else { 0 }),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Some(i)
            } else {
                n.as_f64().map(|f| f.trunc() as i64)
            }
        }
        Value::String(s) => s.trim().parse::<i64>().ok(),
        _ => None,
    }
}

/// `_first_int(*keys)`: first key with `int(value) > 0`; missing/`None` -> `0`; bad value ->
/// `continue`.
fn json_first_int(obj: &serde_json::Map<String, Value>, keys: &[&str]) -> Option<u64> {
    for k in keys {
        let iv = match obj.get(*k) {
            None | Some(Value::Null) => 0,
            Some(v) => match json_as_int(v) {
                Some(x) => x,
                None => continue,
            },
        };
        if iv > 0 {
            return Some(iv as u64);
        }
    }
    None
}

/// Tarjan `strong`.
#[allow(clippy::too_many_arguments)]
fn strong(
    v: &str,
    edges: &BTreeMap<String, BTreeSet<String>>,
    index: &mut BTreeMap<String, usize>,
    low: &mut BTreeMap<String, usize>,
    onstack: &mut BTreeSet<String>,
    stack: &mut Vec<String>,
    sccs: &mut Vec<Vec<String>>,
    counter: &mut usize,
) {
    index.insert(v.to_owned(), *counter);
    low.insert(v.to_owned(), *counter);
    *counter += 1;
    stack.push(v.to_owned());
    onstack.insert(v.to_owned());
    let neighbors: Vec<String> = edges.get(v).map(|s| s.iter().cloned().collect()).unwrap_or_default();
    for w in &neighbors {
        if !index.contains_key(w) {
            strong(w, edges, index, low, onstack, stack, sccs, counter);
            let lw = low.get(w).copied().unwrap_or(0);
            let lv = low.get(v).copied().unwrap_or(0);
            low.insert(v.to_owned(), lv.min(lw));
        } else if onstack.contains(w) {
            let iw = index.get(w).copied().unwrap_or(0);
            let lv = low.get(v).copied().unwrap_or(0);
            low.insert(v.to_owned(), lv.min(iw));
        }
    }
    if low.get(v).copied() == index.get(v).copied() {
        let mut comp: Vec<String> = Vec::new();
        loop {
            let Some(w) = stack.pop() else { break };
            onstack.remove(&w);
            let last = w == v;
            comp.push(w);
            if last {
                break;
            }
        }
        sccs.push(comp);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &str) -> String {
        v.to_owned()
    }

    #[test]
    fn line_counts_match_python() {
        assert_eq!(source_line_count("a\nb\n"), 2);
        assert_eq!(source_line_count("a\nb"), 2);
        assert_eq!(source_line_count("a\n\nb\n"), 3);
        assert_eq!(source_line_count(""), 1);
        assert_eq!(source_line_count("a\n\n"), 2);
    }

    #[test]
    fn code_line_count_skips_blank_and_hash_comments() {
        let src = "x = 1\n# comment\n\n   \ny = 2  # inline\n   # indented\n";
        assert_eq!(code_line_count(src), 2);
        assert_eq!(code_line_count(""), 0);
        assert_eq!(code_line_count("# only a comment\n"), 0);
    }

    #[test]
    fn percentile_pct_matches_python() {
        let a = [1, 2, 3, 4];
        assert_eq!(percentile_pick(&a, 0.5), 2);
        assert_eq!(percentile_pick(&a, 0.95), 3);
        assert_eq!(percentile_pick(&a, 0.0), 4); // int(0)-1 == -1 -> last
        assert_eq!(percentile_pick(&[], 0.5), 0);
        assert_eq!(percentile_pick(&[7], 0.5), 7);
        assert_eq!(percentile_pick(&[1, 2, 3], 0.5), 1);
    }

    #[test]
    fn median_file_lines_upper_middle() {
        assert_eq!(median_file_lines(&[]), 0);
        assert_eq!(median_file_lines(&[5]), 5);
        assert_eq!(median_file_lines(&[1, 2, 3, 4]), 3);
        assert_eq!(median_file_lines(&[4, 1, 3, 2]), 3);
        assert_eq!(median_file_lines(&[1, 2, 3]), 2);
    }

    #[test]
    fn counts_gt_and_ge() {
        let vals = [1, 2, 3, 4, 4];
        assert_eq!(count_gt(&vals, 3), 2);
        assert_eq!(count_gt(&vals, 4), 0);
        assert_eq!(count_ge(&vals, 4), 2);
        assert_eq!(count_ge(&vals, 2), 4);
        assert_eq!(count_gt(&[], 0), 0);
        assert_eq!(count_ge(&[], 0), 0);
        // hermes thresholds
        assert_eq!(FILE_LINES_GT_THRESHOLDS, [1000, 2000, 5000]);
        assert_eq!(FUNC_LEN_GT_THRESHOLDS, [100, 300]);
        assert_eq!(count_ge(&[3, 4, 8], ELIF_CHAIN_GE4), 2);
        assert_eq!(count_ge(&[4, 8], ELIF_CHAIN_GE8), 1);
        assert_eq!(count_ge(&[4, 5, 9], NESTING_GE5), 2);
    }

    #[test]
    fn skip_top_dir_matches_skip_set() {
        assert!(skip_top_dir("node_modules"));
        assert!(skip_top_dir(".git"));
        assert!(skip_top_dir("__pycache__"));
        assert!(!skip_top_dir("src"));
        assert!(!skip_top_dir("tests")); // handled separately by py_files
        assert!(!skip_top_dir(""));
    }

    #[test]
    fn module_names_and_resolution() {
        assert_eq!(module_name_for_path("pkg/mod.py"), "pkg.mod");
        assert_eq!(module_name_for_path("pkg\\mod.py"), "pkg.mod");
        assert_eq!(module_name_for_path("pkg/__init__.py"), "pkg");
        assert_eq!(module_name_for_path("x.py"), "x");
        assert_eq!(module_name_for_path("pkg/sub/mod.py"), "pkg.sub.mod");
        let fp = vec![s("pkg.mod"), s("pkg"), s("other")];
        assert_eq!(resolve_module("pkg.mod.fn", &fp), "pkg.mod");
        assert_eq!(resolve_module("pkg.mod", &fp), "pkg.mod");
        assert_eq!(resolve_module("pkg.x.y", &fp), "pkg");
        assert_eq!(resolve_module("unknown.x", &fp), "");
        assert_eq!(resolve_module("", &fp), "");
    }

    #[test]
    fn import_cycles_tarjan() {
        let mut edges: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        edges.insert(s("a"), [s("b")].into_iter().collect());
        edges.insert(s("b"), [s("a"), s("c")].into_iter().collect());
        edges.insert(s("c"), [s("d")].into_iter().collect());
        edges.insert(s("d"), BTreeSet::new());
        let stats = import_cycle_stats(&edges);
        assert_eq!(stats.import_cycles, 1);
        assert_eq!(stats.largest_cycle, 2);
        assert_eq!(stats.modules_in_cycles, 2);

        let mut acyclic: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        acyclic.insert(s("a"), [s("b")].into_iter().collect());
        acyclic.insert(s("b"), BTreeSet::new());
        assert_eq!(import_cycle_stats(&acyclic), ImportCycleStats::default());
        assert_eq!(import_cycle_stats(&BTreeMap::new()), ImportCycleStats::default());
    }

    #[test]
    fn extract_json_object_best_effort() {
        let v = extract_json_object("```json\n{\"verdict\":\"done\"}\n```").expect("fenced object");
        assert_eq!(v["verdict"], "done");
        let v = extract_json_object("noise {\"a\":1} trailing").expect("span object");
        assert_eq!(v["a"], 1);
        let v = extract_json_object("{\"a\":1}").expect("plain object");
        assert_eq!(v["a"], 1);
        assert!(extract_json_object("not json").is_none());
        assert!(extract_json_object("").is_none());
        assert!(extract_json_object("[]").is_none()); // not an object
        assert!(extract_json_object("{\"a\":").is_none());
    }

    #[test]
    fn parse_judge_done_and_continue() {
        let r = parse_judge_response("{\"verdict\":\"done\",\"reason\":\"ok\"}");
        assert_eq!(r.verdict, "done");
        assert_eq!(r.reason, "ok");
        assert!(!r.parse_failed);
        assert_eq!(r.wait_directive, None);

        let r = parse_judge_response("{\"done\":true}");
        assert_eq!(r.verdict, "done");
        let r = parse_judge_response("{\"done\":\"yes\"}");
        assert_eq!(r.verdict, "done");
        let r = parse_judge_response("{\"done\":\"1\"}");
        assert_eq!(r.verdict, "done");
        let r = parse_judge_response("{\"done\":false}");
        assert_eq!(r.verdict, "continue");
        let r = parse_judge_response("{\"verdict\":\"bogus\",\"reason\":\"x\"}");
        assert_eq!(r.verdict, "continue");
        let r = parse_judge_response("{\"verdict\":\"WAIT\"}");
        assert_eq!(r.verdict, "continue"); // wait without a target downgrades

        // missing reason -> "no reason provided"
        let r = parse_judge_response("{\"verdict\":\"continue\"}");
        assert_eq!(r.reason, "no reason provided");
    }

    #[test]
    fn parse_judge_wait_directive_priority() {
        let r = parse_judge_response(
            "{\"verdict\":\"wait\",\"wait_on_session\":\"sess_1\",\"wait_on_pid\":9,\"wait_for_seconds\":30,\"reason\":\"w\"}",
        );
        assert_eq!(r.verdict, "wait");
        assert_eq!(r.wait_directive, Some(WaitDirective::Session(s("sess_1"))));

        let r = parse_judge_response("{\"verdict\":\"wait\",\"wait_on_pid\":123}");
        assert_eq!(r.wait_directive, Some(WaitDirective::Pid(123)));

        let r = parse_judge_response("{\"verdict\":\"wait\",\"pid\":\"456\"}");
        assert_eq!(r.wait_directive, Some(WaitDirective::Pid(456)));

        let r = parse_judge_response("{\"verdict\":\"wait\",\"wait_for_seconds\":30}");
        assert_eq!(r.wait_directive, Some(WaitDirective::Seconds(30)));

        let r = parse_judge_response("{\"verdict\":\"wait\",\"seconds\":\"15\"}");
        assert_eq!(r.wait_directive, Some(WaitDirective::Seconds(15)));

        // session key alias + whitespace trim
        let r = parse_judge_response("{\"verdict\":\"wait\",\"session_id\":\"  x  \"}");
        assert_eq!(r.wait_directive, Some(WaitDirective::Session(s("x"))));

        // non-positive / missing target downgrades to continue
        let r = parse_judge_response("{\"verdict\":\"wait\",\"wait_for_seconds\":0}");
        assert_eq!(r.verdict, "continue");
        assert_eq!(r.wait_directive, None);
        assert!(r.reason.contains("no target"));
    }

    #[test]
    fn parse_judge_fail_open_on_bad_output() {
        let r = parse_judge_response("");
        assert_eq!(r.verdict, "continue");
        assert!(r.parse_failed);
        assert!(!r.reason.is_empty());

        let r = parse_judge_response("definitely not json");
        assert_eq!(r.verdict, "continue");
        assert!(r.parse_failed);
        assert!(r.reason.contains("not JSON"));

        // whitespace-only is not `not raw`, but still no JSON
        let r = parse_judge_response("   ");
        assert_eq!(r.verdict, "continue");
        assert!(r.parse_failed);

        assert_eq!(JUDGE_VERDICTS.len(), 5);
        assert_eq!(GOAL_JUDGE_WAIT_N, 3);
        assert_eq!(GOAL_JUDGE_ACTIVE_DELEGATIONS, 4);
        assert_eq!(JUDGE_SNIPPET_CHARS, 4000);
        assert_eq!(JUDGE_GOAL_SNIPPET_CHARS, 2000);
    }

    #[test]
    fn truncate_marks_cut() {
        assert_eq!(truncate("", 5), "");
        assert_eq!(truncate("abc", 5), "abc");
        assert_eq!(truncate("abcdef", 3), format!("abc{JUDGE_TRUNCATE_MARKER}"));
        assert_eq!(truncate("abcdef", 3), format!("abc\u{2026} [truncated]"));
        assert_eq!(truncate("héllo", 2), format!("h\u{E9}{JUDGE_TRUNCATE_MARKER}"));
    }
}
