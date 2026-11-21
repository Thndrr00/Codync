//! Stall gate: stops a turn that keeps failing the same way. When the same tool call fails with the same
//! error [`SAME_ERROR_STRIKES`] times in a row, with no successful tool call in between, the agent is looping
//! and only spending the user's quota; the host stops the turn and says why.
//!
//! Call arguments are compared as canonical JSON with sorted keys (`stable_json`), so two attempts that
//! differ only in key order are still the same blind retry (Idea 31 from bdots). Codync keeps its own
//! three-strike threshold instead of bdots' five: a stuck agent is stopped sooner.

use serde_json::Value;

/// Identical failures in a row that stop the turn.
pub const SAME_ERROR_STRIKES: u8 = 3;
/// Error text kept in a signature and shown in the notice.
const ERROR_TEXT_MAX: usize = 200;

/// What a tool call ended with, as far as the gate cares.
pub enum ToolOutcome<'a> {
    Succeeded,
    Failed { title: &'a str, output: &'a str, args: Option<&'a str> },
}

#[derive(Default)]
pub struct LoopGuard {
    last: Option<String>,
    strikes: u8,
    /// Set once the gate trips: the reason shown to the user.
    tripped: Option<String>,
}

impl LoopGuard {
    /// Records a finished tool call; returns the reason the first time the gate trips.
    pub fn observe(&mut self, outcome: &ToolOutcome<'_>) -> Option<String> {
        let ToolOutcome::Failed { title, output, args } = outcome else {
            self.last = None;
            self.strikes = 0;
            return None;
        };
        let signature = format!("{title}\n{}\n{}", stable_args(*args), normalize(output));
        if self.last.as_deref() == Some(signature.as_str()) {
            self.strikes += 1;
        } else {
            self.last = Some(signature);
            self.strikes = 1;
        }
        if self.strikes < SAME_ERROR_STRIKES || self.tripped.is_some() {
            return None;
        }
        let error = normalize(output);
        let what = if error.is_empty() { (*title).to_owned() } else { format!("{title}: {error}") };
        let reason = format!("Stopped after {SAME_ERROR_STRIKES} identical failed tool calls in a row ({what}).");
        self.tripped = Some(reason.clone());
        Some(reason)
    }

    /// Why the gate stopped this turn, if it did.
    pub fn tripped(&self) -> Option<&str> {
        self.tripped.as_deref()
    }
}

/// Error text with runs of digits (durations, line numbers, ids) collapsed, so the same failure matches
/// itself; whitespace folded and clipped to [`ERROR_TEXT_MAX`] characters.
fn normalize(text: &str) -> String {
    let mut out = String::new();
    let mut in_digits = false;
    for word in text.split_whitespace() {
        if !out.is_empty() {
            out.push(' ');
        }
        for c in word.chars() {
            if c.is_ascii_digit() {
                if !in_digits {
                    out.push_str("<n>");
                }
                in_digits = true;
            } else {
                out.push(c);
                in_digits = false;
            }
        }
        in_digits = false;
    }
    match out.char_indices().nth(ERROR_TEXT_MAX) {
        Some((cut, _)) => format!("{}…", &out[..cut]),
        None => out,
    }
}

/// Canonical JSON: object keys sorted alphabetically at every level, so the same call with its
/// properties in a different order has one signature (Idea 31's `stableJson()`).
pub fn stable_json(value: &Value) -> String {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let body = keys
                .iter()
                .map(|key| format!("{}:{}", quote(key.as_str()), stable_json(&map[key.as_str()])))
                .collect::<Vec<_>>()
                .join(",");
            format!("{{{body}}}")
        }
        Value::Array(items) => {
            let body = items.iter().map(stable_json).collect::<Vec<_>>().join(",");
            format!("[{body}]")
        }
        other => other.to_string(),
    }
}

fn quote(text: &str) -> String {
    serde_json::to_string(text).unwrap_or_else(|_| format!("\"{text}\""))
}

/// Canonical form of a tool call's raw arguments; unparseable text is used as-is.
fn stable_args(args: Option<&str>) -> String {
    let Some(args) = args else { return String::new() };
    match serde_json::from_str::<Value>(args) {
        Ok(value) => stable_json(&value),
        Err(_) => args.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn failed<'a>(title: &'a str, output: &'a str) -> ToolOutcome<'a> {
        ToolOutcome::Failed { title, output, args: None }
    }

    #[test]
    fn three_identical_failures_in_a_row_stop_the_turn_once() {
        let mut guard = LoopGuard::default();
        assert!(guard.observe(&failed("cargo test", "error: 2 tests failed in 0.41s")).is_none());
        assert!(guard.observe(&failed("cargo test", "error: 2 tests failed in 0.57s")).is_none());
        let reason = guard.observe(&failed("cargo test", "error: 2 tests failed in 1.02s")).expect("trips");
        assert!(reason.contains("cargo test: error: <n> tests failed"), "{reason}");
        assert_eq!(guard.tripped(), Some(reason.as_str()));
        assert!(guard.observe(&failed("cargo test", "error: 2 tests failed")).is_none(), "reported once");
    }

    #[test]
    fn a_successful_call_or_a_different_failure_resets_the_count() {
        let mut guard = LoopGuard::default();
        guard.observe(&failed("npm test", "boom"));
        guard.observe(&failed("npm test", "boom"));
        guard.observe(&ToolOutcome::Succeeded);
        guard.observe(&failed("npm test", "boom"));
        guard.observe(&failed("npm test", "boom"));
        assert!(guard.tripped().is_none(), "an edit in between is progress");
        guard.observe(&failed("npm test", "another error"));
        assert!(guard.observe(&failed("npm test", "boom")).is_none());
        assert!(guard.tripped().is_none());
    }

    #[test]
    fn reading_different_missing_files_is_not_a_loop() {
        let mut guard = LoopGuard::default();
        for title in ["Read src/a.rs", "Read src/b.rs", "Read src/c.rs"] {
            guard.observe(&failed(title, "File does not exist."));
        }
        assert!(guard.tripped().is_none());
    }

    #[test]
    fn argument_key_order_does_not_hide_a_blind_retry() {
        let mut guard = LoopGuard::default();
        let a = r#"{"path":"src/a.rs","mode":"write","nested":{"z":1,"a":2}}"#;
        let b = r#"{"nested":{"a":2,"z":1},"mode":"write","path":"src/a.rs"}"#;
        let call = |args| ToolOutcome::Failed { title: "Write", output: "read-only", args: Some(args) };
        assert!(guard.observe(&call(a)).is_none());
        assert!(guard.observe(&call(b)).is_none());
        assert!(guard.observe(&call(a)).is_some(), "same call in a different key order still trips");
    }

    #[test]
    fn different_arguments_are_not_the_same_call() {
        let mut guard = LoopGuard::default();
        let call = |args| ToolOutcome::Failed { title: "Read", output: "missing", args: Some(args) };
        guard.observe(&call(r#"{"path":"a.rs"}"#));
        guard.observe(&call(r#"{"path":"b.rs"}"#));
        assert!(guard.tripped().is_none());
    }

    #[test]
    fn stable_json_sorts_keys_at_every_level() {
        let value: Value = serde_json::from_str(r#"{"b":{"d":1,"c":2},"a":[{"z":0,"y":1}]}"#).unwrap();
        assert_eq!(stable_json(&value), r#"{"a":[{"y":1,"z":0}],"b":{"c":2,"d":1}}"#);
    }

    #[test]
    fn long_errors_are_clipped() {
        let long = "x".repeat(500);
        assert_eq!(normalize(&long).chars().count(), ERROR_TEXT_MAX + 1);
        assert_eq!(normalize("  line 12\n\tcol 7 "), "line <n> col <n>");
    }
}
