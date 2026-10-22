//! Stops a turn whose agent keeps making the same tool call: the same tool with the same input
//! `LIMIT` times in a row is a loop, not progress, and every round costs the user tokens. Any
//! other call in between resets the count, so polling that changes its input, or retrying a
//! few times, goes on.
//!
//! Idea from rakazo's `packages/adapters/src/tool-loop.ts` (Apache-2.0); no code copied.

use serde_json::Value;
use std::hash::{DefaultHasher, Hash, Hasher};

/// Identical calls in a row that stop the turn.
pub(super) const LIMIT: u32 = 10;

#[derive(Default)]
pub(super) struct ToolLoop {
    last: Option<u64>,
    streak: u32,
}

impl ToolLoop {
    /// A new turn starts counting from zero.
    pub(super) fn reset(&mut self) {
        *self = Self::default();
    }

    /// Counts a `tool_call` update; the reason to stop when it makes the streak `LIMIT` long.
    pub(super) fn observe(&mut self, update: &Value) -> Option<String> {
        let title = update["title"].as_str().unwrap_or("a tool");
        let mut hasher = DefaultHasher::new();
        (title, update["kind"].as_str(), update["rawInput"].to_string()).hash(&mut hasher);
        let call = hasher.finish();
        self.streak = if self.last == Some(call) { self.streak + 1 } else { 1 };
        self.last = Some(call);
        (self.streak == LIMIT).then(|| {
            format!("Stopped: the agent called {title} with the same input {LIMIT} times in a row. Send a message to continue differently.")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn only_an_unbroken_streak_of_the_same_call_trips() {
        let mut guard = ToolLoop::default();
        let same = json!({"title":"Read config.toml","kind":"read","rawInput":{"path":"config.toml"}});
        let other = json!({"title":"Read main.rs","kind":"read","rawInput":{"path":"main.rs"}});
        for _ in 0..LIMIT - 1 {
            assert!(guard.observe(&same).is_none());
        }
        assert!(guard.observe(&other).is_none(), "a different call breaks the streak");
        for _ in 0..LIMIT - 1 {
            assert!(guard.observe(&same).is_none());
        }
        assert!(guard.observe(&same).unwrap().contains("Read config.toml"));
        guard.reset();
        assert!(guard.observe(&same).is_none());
    }
}
