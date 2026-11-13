//! Guard rails for the tools a bot calls in its own chat.
//!
//! Two small protections, both in memory and both gone with the host:
//!
//! - A per-bot, per-tool rate limit. A loop in a model (the same search ten times in a row) uses
//!   the person's tokens and the computer's time; the tool answers with an error telling the bot to
//!   wait instead of running again. Limits are generous: they catch loops, not normal work.
//! - A brief cache for read-only tools. Asking the same question twice within a few seconds — the
//!   usual retry after a garbled line — is answered from memory instead of walking the workspace
//!   again. Nothing that writes is cached.

use anyhow::{Result, bail};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

const WINDOW: Duration = Duration::from_secs(60);
const DEFAULT_PER_MINUTE: u32 = 180;
const CACHE_TTL: Duration = Duration::from_secs(20);
const CACHE_LIMIT: usize = 256;
const CACHE_MAX_BYTES: usize = 64 * 1024;

#[derive(Clone, Copy)]
struct Window {
    started: Instant,
    count: u32,
}

static CALLS: LazyLock<Mutex<HashMap<(String, String), Window>>> = LazyLock::new(Mutex::default);
static CACHE: LazyLock<Mutex<HashMap<String, (Instant, Value)>>> = LazyLock::new(Mutex::default);

/// Calls a tool may receive from one bot in a minute. A tool missing from here gets the default.
fn per_minute(name: &str) -> u32 {
    match name {
        // These reach the network or run an agent of their own.
        "research_sources" => 8,
        // Walking a workspace can be expensive on a big folder.
        "search_workspace" | "read_workspace_file" => 40,
        // Every one of these shows up in the chat; a model sending more is looping.
        "send_message" | "send_file" | "react" => 60,
        "publish_artifact" => 30,
        _ => DEFAULT_PER_MINUTE,
    }
}

/// Tools whose answer depends only on their arguments and the files at this instant.
fn read_only(name: &str) -> bool {
    matches!(name, "list_artifacts" | "instruction_history" | "search_workspace" | "read_workspace_file")
}

/// Refuses the call once this bot has used up its minute; the tool never runs.
pub fn check(bot: &str, name: &str) -> Result<()> {
    let limit = per_minute(name);
    let now = Instant::now();
    let mut calls = CALLS.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    // Drop windows that already reset, so one busy day does not leave a growing map behind.
    calls.retain(|_, window| now.duration_since(window.started) < WINDOW);
    let window = calls.entry((bot.to_owned(), name.to_owned())).or_insert(Window { started: now, count: 0 });
    if now.duration_since(window.started) >= WINDOW {
        *window = Window { started: now, count: 0 };
    }
    if window.count >= limit {
        bail!(
            "too many calls to {name} in a minute (limit {limit}); wait a moment before calling it again, or narrow what you ask for"
        );
    }
    window.count += 1;
    Ok(())
}

/// The answer to this exact read-only call, when it was made less than a few seconds ago.
pub fn cached(bot: &str, name: &str, arguments: &Value) -> Option<Value> {
    if !read_only(name) {
        return None;
    }
    let key = cache_key(bot, name, arguments);
    let mut cache = CACHE.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let (at, answer) = cache.get(&key)?;
    if at.elapsed() > CACHE_TTL {
        cache.remove(&key);
        return None;
    }
    Some(answer.clone())
}

/// Keeps a read-only answer for the next identical call, when it is small enough to hold.
pub fn remember(bot: &str, name: &str, arguments: &Value, answer: &Value) {
    if !read_only(name) || serde_json::to_string(answer).is_ok_and(|text| text.len() > CACHE_MAX_BYTES) {
        return;
    }
    let key = cache_key(bot, name, arguments);
    let mut cache = CACHE.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if cache.len() >= CACHE_LIMIT {
        cache.clear();
    }
    cache.insert(key, (Instant::now(), answer.clone()));
}

fn cache_key(bot: &str, name: &str, arguments: &Value) -> String {
    // NUL cannot appear in a JSON document, so the parts never run into each other.
    format!("{bot}\0{name}\0{arguments}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_loop_is_stopped_and_another_bot_or_tool_is_not() {
        for _ in 0..3 {
            check("loop-bot", "research_sources").expect("under the limit");
        }
        let mut refused = 0;
        for _ in 0..20 {
            if check("loop-bot", "research_sources").is_err() {
                refused += 1;
            }
        }
        assert_eq!(refused, 15, "8 a minute");
        assert!(check("other-bot", "research_sources").is_ok(), "limits are per bot");
        assert!(check("loop-bot", "send_message").is_ok(), "limits are per tool");
    }

    #[test]
    fn only_read_only_answers_are_cached_and_they_match_their_arguments() {
        let args = json!({"query": "release"});
        assert!(cached("b1", "send_message", &args).is_none());
        remember("b1", "send_message", &args, &json!({"sent": true}));
        assert!(cached("b1", "send_message", &args).is_none(), "writing is never cached");

        remember("b1", "search_workspace", &args, &json!({"files": ["a.md"]}));
        assert_eq!(cached("b1", "search_workspace", &args), Some(json!({"files": ["a.md"]})));
        assert!(cached("b1", "search_workspace", &json!({"query": "other"})).is_none());
        assert!(cached("b2", "search_workspace", &args).is_none(), "caches are per bot");
    }
}
