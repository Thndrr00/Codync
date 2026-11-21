//! Tool-call idempotency inside one turn, mirroring the `clientNonce` pattern the host already
//! uses for messages: the first result of a call is kept and a retry with the same tool name and
//! arguments gets that result back instead of running again. Different arguments run normally.
//! One instance lives for one turn, in memory only; a host restart starts over, as with a nonce.

use serde_json::Value;
use std::collections::{VecDeque, hash_map::DefaultHasher};
use std::hash::{Hash, Hasher};

/// How many distinct calls one turn remembers before the oldest is dropped.
pub(super) const CAP: usize = 64;

#[derive(Debug)]
pub(super) struct ToolDedupe {
    /// `(key hash, exact identity, first result)`, oldest first. Bounded by `cap`.
    entries: VecDeque<(u64, String, Value)>,
    cap: usize,
}

impl ToolDedupe {
    pub(super) fn new() -> Self {
        Self::with_cap(CAP)
    }

    pub(super) fn with_cap(cap: usize) -> Self {
        Self { entries: VecDeque::new(), cap: cap.max(1) }
    }

    /// The key is the tool name and a canonical serialization of its arguments, hashed for a fast
    /// lookup; the exact identity is kept too, so a hash collision can never return a foreign result.
    pub(super) fn fingerprint(name: &str, args: &Value) -> (u64, String) {
        let canonical = serde_json::to_string(&canonical(args)).unwrap_or_default();
        let identity = format!("{name}\u{0}{canonical}");
        let mut hasher = DefaultHasher::new();
        identity.hash(&mut hasher);
        (hasher.finish(), identity)
    }

    /// The first result of this call in the turn, when there is one.
    pub(super) fn cached(&self, name: &str, args: &Value) -> Option<&Value> {
        let (hash, identity) = Self::fingerprint(name, args);
        self.lookup(hash, &identity)
    }

    /// Keeps a first result. A repeated call is not stored again and does not move the oldest.
    pub(super) fn remember(&mut self, name: &str, args: &Value, result: Value) {
        let (hash, identity) = Self::fingerprint(name, args);
        self.remember_key(hash, identity, result);
    }

    /// Runs `call` only when the turn has no result for this call yet.
    pub(super) fn run(&mut self, name: &str, args: &Value, call: impl FnOnce() -> Value) -> Value {
        if let Some(first) = self.cached(name, args) {
            return first.clone();
        }
        let result = call();
        self.remember(name, args, result.clone());
        result
    }

    fn lookup(&self, hash: u64, identity: &str) -> Option<&Value> {
        self.entries.iter().find(|(h, id, _)| *h == hash && id == identity).map(|(_, _, value)| value)
    }

    fn remember_key(&mut self, hash: u64, identity: String, result: Value) {
        if self.lookup(hash, &identity).is_some() {
            return;
        }
        if self.entries.len() == self.cap {
            self.entries.pop_front();
        }
        self.entries.push_back((hash, identity, result));
    }
}

/// Object key order is not part of a call's identity. `serde_json` is built with `preserve_order`,
/// so two agents' otherwise equal arguments can arrive in a different order; sort recursively
/// before serializing so both get the same key.
fn canonical(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut pairs: Vec<_> = map.iter().collect();
            pairs.sort_by(|a, b| a.0.cmp(b.0));
            Value::Object(pairs.into_iter().map(|(key, value)| (key.clone(), canonical(value))).collect())
        }
        Value::Array(list) => Value::Array(list.iter().map(canonical).collect()),
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::cell::Cell;

    #[test]
    fn a_repeat_gets_the_first_result_and_different_arguments_run() {
        let mut dedupe = ToolDedupe::new();
        let runs = Cell::new(0);
        let call = |n: i64| {
            runs.set(runs.get() + 1);
            json!({"n": n, "run": runs.get()})
        };
        let args = json!({"path": "a.txt"});
        assert_eq!(dedupe.run("edit", &args, || call(1)), json!({"n": 1, "run": 1}));
        assert_eq!(dedupe.run("edit", &args, || call(2)), json!({"n": 1, "run": 1}), "the first result wins");
        assert_eq!(runs.get(), 1, "the second call never ran");
        assert_eq!(
            dedupe.run("edit", &json!({"path": "b.txt"}), || call(3)),
            json!({"n": 3, "run": 2}),
            "different arguments run"
        );
        assert_eq!(dedupe.run("read", &args, || call(4)), json!({"n": 4, "run": 3}), "a different tool runs too");
        assert_eq!(runs.get(), 3);
    }

    #[test]
    fn argument_key_order_does_not_change_the_call() {
        let mut dedupe = ToolDedupe::new();
        let first = json!({"path": "a.txt", "lines": [1, 2], "opts": {"b": true, "a": 1}});
        let reordered = json!({"opts": {"a": 1, "b": true}, "lines": [1, 2], "path": "a.txt"});
        assert_eq!(ToolDedupe::fingerprint("edit", &first), ToolDedupe::fingerprint("edit", &reordered));
        dedupe.remember("edit", &first, json!("first"));
        assert_eq!(dedupe.cached("edit", &reordered), Some(&json!("first")));
    }

    #[test]
    fn a_hash_collision_never_returns_another_calls_result() {
        let mut dedupe = ToolDedupe::new();
        // Force two different identities to one hash, as a broken hash would.
        dedupe.remember_key(7, "edit\0a".to_owned(), json!("first"));
        assert_eq!(dedupe.lookup(7, "edit\0a"), Some(&json!("first")));
        assert_eq!(dedupe.lookup(7, "edit\0b"), None, "the same hash is not the same call");
        dedupe.remember_key(7, "edit\0b".to_owned(), json!("second"));
        assert_eq!(dedupe.entries.len(), 2, "both calls are kept under their exact identities");
        assert_eq!(dedupe.lookup(7, "edit\0a"), Some(&json!("first")));
        assert_eq!(dedupe.lookup(7, "edit\0b"), Some(&json!("second")));
    }

    #[test]
    fn the_oldest_call_falls_out_past_the_cap() {
        let mut dedupe = ToolDedupe::with_cap(2);
        dedupe.remember("edit", &json!({"n": 1}), json!("one"));
        dedupe.remember("edit", &json!({"n": 2}), json!("two"));
        assert_eq!(dedupe.cached("edit", &json!({"n": 1})), Some(&json!("one")));
        dedupe.remember("edit", &json!({"n": 3}), json!("three"));
        assert_eq!(dedupe.entries.len(), 2);
        assert_eq!(dedupe.cached("edit", &json!({"n": 1})), None, "the oldest was dropped");
        assert_eq!(dedupe.cached("edit", &json!({"n": 2})), Some(&json!("two")));
        assert_eq!(dedupe.cached("edit", &json!({"n": 3})), Some(&json!("three")));
    }
}
