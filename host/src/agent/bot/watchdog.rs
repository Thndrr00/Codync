//! Watches a running turn for two kinds of trouble:
//! - a loop: the same tool with the same input `LOOP_LIMIT` times in a row is a loop, not
//!   progress, and every round costs tokens. The turn is stopped. Any other call in between
//!   resets the count, so retries and polling with changing input go on.
//! - a stall: nothing from the agent for `STALL_AFTER`, with no tool running and no approval
//!   waiting, usually means a hung connection. The chat says so once; the turn keeps running.
//!
//! Ideas from rakazo's `tool-loop.ts` and `stuck-work.ts` (Apache-2.0); no code copied.

use super::Actor;
use crate::store::EntryKind;
use serde_json::{Value, json};
use std::collections::HashSet;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::time::{Duration, Instant};

/// Identical calls in a row that stop the turn.
const LOOP_LIMIT: u32 = 10;
/// Silence that's worth telling the user about.
const STALL_AFTER: Duration = Duration::from_secs(10 * 60);

pub(super) struct Watchdog {
    last_call: Option<u64>,
    streak: u32,
    /// Why the turn must stop, until the actor acts on it.
    tripped: Option<String>,
    last_update: Instant,
    /// Tool calls started and not finished.
    running: HashSet<String>,
    warned: bool,
}

impl Default for Watchdog {
    fn default() -> Self {
        Self {
            last_call: None,
            streak: 0,
            tripped: None,
            last_update: Instant::now(),
            running: HashSet::new(),
            warned: false,
        }
    }
}

impl Watchdog {
    /// A new turn starts with a clean slate.
    pub(super) fn reset(&mut self) {
        *self = Self::default();
    }

    /// Notes one `session/update` of the turn.
    pub(super) fn seen(&mut self, update: &Value) {
        self.last_update = Instant::now();
        let id = update["toolCallId"].as_str().unwrap_or_default().to_owned();
        match update["sessionUpdate"].as_str() {
            Some("tool_call") => {
                self.count_call(update);
                if !matches!(update["status"].as_str(), Some("completed" | "failed")) {
                    self.running.insert(id);
                }
            }
            Some("tool_call_update") if matches!(update["status"].as_str(), Some("completed" | "failed")) => {
                self.running.remove(&id);
            }
            _ => {}
        }
    }

    fn count_call(&mut self, update: &Value) {
        let title = update["title"].as_str().unwrap_or("a tool");
        let mut hasher = DefaultHasher::new();
        (title, update["kind"].as_str(), update["rawInput"].to_string()).hash(&mut hasher);
        let call = hasher.finish();
        self.streak = if self.last_call == Some(call) { self.streak + 1 } else { 1 };
        self.last_call = Some(call);
        if self.streak == LOOP_LIMIT {
            self.tripped = Some(format!(
                "Stopped: the agent called {title} with the same input {LOOP_LIMIT} times in a row. Send a message to continue differently."
            ));
        }
    }

    /// Why the turn must stop for a loop, once.
    pub(super) fn tripped(&mut self) -> Option<String> {
        self.tripped.take()
    }

    /// A note for the user, once per turn, when the agent went silent with nothing to wait for.
    fn stalled(&mut self, now: Instant, waiting_on_user: bool) -> Option<String> {
        if self.warned || waiting_on_user || !self.running.is_empty() || now - self.last_update < STALL_AFTER {
            return None;
        }
        self.warned = true;
        Some(format!(
            "No word from the agent for {} minutes. It may be stuck; stop it to send the message again, or keep waiting.",
            STALL_AFTER.as_secs() / 60
        ))
    }
}

impl Actor {
    /// Tells the chat once when the running turn has gone silent (called on the flush tick).
    pub(super) fn check_stall(&mut self) {
        let Some(turn) = self.turn else { return };
        if let Some(note) = self.watchdog.stalled(Instant::now(), !self.perms.is_empty()) {
            tracing::warn!(bot = %self.cfg.id, "a turn has gone silent");
            self.hub.add_entry(&self.lane, EntryKind::Notice, turn, &json!({"text": note, "style": "info"}));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(title: &str, path: &str) -> Value {
        json!({"sessionUpdate":"tool_call","toolCallId":uuid::Uuid::new_v4().to_string(),"title":title,"kind":"read","rawInput":{"path":path},"status":"completed"})
    }

    #[test]
    fn only_an_unbroken_streak_of_the_same_call_trips() {
        let mut dog = Watchdog::default();
        for _ in 0..LOOP_LIMIT - 1 {
            dog.seen(&call("Read config.toml", "config.toml"));
        }
        dog.seen(&call("Read main.rs", "main.rs"));
        assert!(dog.tripped().is_none(), "a different call breaks the streak");
        for _ in 0..LOOP_LIMIT {
            dog.seen(&call("Read config.toml", "config.toml"));
        }
        assert!(dog.tripped().unwrap().contains("Read config.toml"));
        assert!(dog.tripped().is_none(), "it's told once");
    }

    #[test]
    fn silence_is_a_stall_only_with_nothing_to_wait_for() {
        let mut dog = Watchdog::default();
        let later = Instant::now() + STALL_AFTER + Duration::from_secs(1);
        dog.seen(
            &json!({"sessionUpdate":"tool_call","toolCallId":"build","title":"cargo build","status":"in_progress"}),
        );
        assert!(dog.stalled(later, false).is_none(), "a running tool explains the silence");
        dog.seen(&json!({"sessionUpdate":"tool_call_update","toolCallId":"build","status":"completed"}));
        let later = Instant::now() + STALL_AFTER + Duration::from_secs(1);
        assert!(dog.stalled(later, true).is_none(), "an approval explains it too");
        assert!(dog.stalled(Instant::now(), false).is_none());
        assert!(dog.stalled(later, false).is_some());
        assert!(dog.stalled(later, false).is_none(), "it's told once a turn");
    }
}
