//! How long an agent rests after it couldn't answer. Usage belongs to the agent's account, not to one bot, so a
//! rest is the computer's: once one bot finds Claude Code out of usage, every bot skips it until it resets. A usage
//! limit rests until the reset time the agent reported (`usage`); other failures rest longer each time in a row.
//! Kept in `kv` (`rest.<account>`, see `account`), so a host restart remembers them.

use super::Reason;
use crate::store::Store;
use crate::usage::Usage;
use serde::{Deserialize, Serialize};

const MINUTE: i64 = 60_000;
/// Transient failures in a row rest 1, 5, 25, then 60 minutes.
const BACKOFF: [i64; 4] = [MINUTE, 5 * MINUTE, 25 * MINUTE, 60 * MINUTE];
/// A usage limit whose reset time isn't known: try again in an hour.
const UNKNOWN_RESET: i64 = 60 * MINUTE;
/// No credits: the user has to add some, so don't look again for a while.
const OUT_OF_CREDITS: i64 = 6 * 60 * MINUTE;
/// Signed out of an agent Codync signs in to: signing in ends it sooner (`fallback::signed_in`), so look again only
/// now and then (a sign-in outside Codync).
const SIGNED_OUT: i64 = 60 * MINUTE;
/// Signed out of one it can't sign in to (not in `backends::HARNESSES`): only a sign-in outside Codync helps, so look
/// again soon.
const SIGNED_OUT_ELSEWHERE: i64 = 10 * MINUTE;
const NOT_STARTING: i64 = 5 * MINUTE;

/// How long an ACP agent may stay silent while Codync opens its session before the start is given up on.
/// OpenBot 0.75.0 (#12871): `session/new` had no bound, so an agent that never answered left the work at
/// "starting" forever and kept a channel member busy, so its later tasks were never assigned. After this much
/// silence the process is ended with "did not answer in time", the start fails, and the queued work is freed.
/// Agents that run one process per session (Mistral, Antigravity) keep their current behavior: the watchdog only
/// applies to a process shared by sessions, as for session restore.
pub const SESSION_START_STALL: i64 = NOT_STARTING;

#[allow(
    dead_code,
    reason = "ported start-timeout decision; the ACP session-open loop is in agent/acp.rs, outside this change"
)]
pub fn silent_start_timed_out(silent_ms: i64, per_session_process: bool) -> bool {
    !per_session_process && silent_ms >= SESSION_START_STALL
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Rest {
    pub until: i64,
    pub reason: Reason,
    /// Failures in a row, for the backoff.
    pub failures: u32,
    /// The reset time of the last usage read that had data. OpenBot 0.75.0 (#13018): after a restart a read
    /// can return no data (idle process stopped, read waited, failed or timed out). A later read with no data
    /// keeps this time, so a backup model isn't sent back to its main account before the limit resets.
    #[serde(default)]
    pub reset_at: Option<i64>,
}

/// Which reset time a usage-limit rest holds to. A fresh reading with data wins; a read with no data keeps the
/// reset time of the last real reading, so the agent stays on its backup until the window actually resets. A first
/// read with no data keeps the old unknown-reset wait. OpenBot 0.75.0 (#13018).
pub fn held_reset(fresh: Option<i64>, held: Option<i64>, now: i64) -> Option<i64> {
    let fresh = fresh.filter(|t| *t > now);
    fresh.or_else(|| held.filter(|t| *t > now))
}

fn key(account: &str) -> String {
    format!("rest.{account}")
}

/// Whose usage a rest is about: the agent's account, or a custom command's own (two custom commands are two agents).
pub fn account(cfg: &crate::store::BotConfig) -> String {
    account_of(&cfg.backend, cfg.command.as_deref())
}

/// `account` of agent `backend` run by `command`, if a custom one.
pub fn account_of(backend: &str, command: Option<&str>) -> String {
    use sha2::{Digest as _, Sha256};
    use std::fmt::Write as _;
    let Some(command) = command.map(str::trim).filter(|c| !c.is_empty()) else {
        return backend.to_owned();
    };
    let hash = Sha256::digest(command);
    hash[..6].iter().fold(format!("{backend}#"), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

/// The agent `account` runs.
fn backend_of(account: &str) -> &str {
    account.split_once('#').map_or(account, |(backend, _)| backend)
}

/// `account`'s last rest, over or not.
fn saved(store: &Store, account: &str) -> Option<Rest> {
    serde_json::from_str(&store.kv_get(&key(account))?).ok()
}

/// `account`'s rest at `now`, while it lasts.
pub fn get(store: &Store, account: &str, now: i64) -> Option<Rest> {
    saved(store, account).filter(|rest| rest.until > now)
}

/// Records that `account` couldn't answer for `reason` at `now`. `resets_at`: when its usage limit resets, if it
/// said. A model the agent lacks is the bot's setup, not the agent's, so it doesn't rest the agent.
pub fn record(store: &Store, account: &str, reason: Reason, now: i64, resets_at: Option<i64>) -> Option<Rest> {
    let before = saved(store, account);
    // An hour after its last rest ended, a failure starts the backoff over.
    let failures = before.filter(|r| now - r.until < BACKOFF[3]).map_or(0, |r| r.failures) + 1;
    let held = before.filter(|r| r.reason == Reason::UsageLimit).and_then(|r| r.reset_at);
    let (wait, reset_at) = match reason {
        Reason::ModelMissing => return None,
        Reason::UsageLimit => {
            let reset = held_reset(resets_at, held, now);
            (reset.map_or(UNKNOWN_RESET, |t| t - now), reset)
        }
        Reason::OutOfCredits => (OUT_OF_CREDITS, None),
        Reason::SignedOut if crate::agent::backends::harness(backend_of(account)).is_some() => (SIGNED_OUT, None),
        Reason::SignedOut => (SIGNED_OUT_ELSEWHERE, None),
        Reason::NotStarting => (NOT_STARTING, None),
        Reason::RateLimited | Reason::Overloaded => (BACKOFF[(failures as usize - 1).min(BACKOFF.len() - 1)], None),
    };
    let rest = Rest { until: now + wait, reason, failures, reset_at };
    let saved = serde_json::to_string(&rest).map_err(anyhow::Error::from).and_then(|v| store.kv_set(&key(account), &v));
    if let Err(error) = saved {
        tracing::warn!(account, error = format!("{error:#}"), "couldn't record the agent's rest");
    }
    Some(rest)
}

/// When `backend`'s usage limit resets, from the windows it last reported (`usage`): the latest reset of a window
/// that is used up. `None` when none is, or none said when.
pub fn reset_time(usage: &Usage, backend: &str, now: i64) -> Option<i64> {
    let provider = usage.providers.iter().find(|p| p.id == backend)?;
    provider.windows.iter().filter(|w| w.percent >= 99.5).filter_map(|w| w.resets_at).filter(|t| *t > now).max()
}

/// Whether a fresh turn or routine may clear a failure latch. OpenBot 0.74.0: a provider that did
/// not start several times in a row is stopped, but the next message or routine must start it once
/// instead of waiting for a provider refresh. In Codync that latch is a `NotStarting` rest; the
/// failing loop still records its rest as usual, only the turn boundary clears it.
pub fn retry_startable(rest: Option<&Rest>) -> bool {
    rest.is_some_and(|rest| rest.reason == Reason::NotStarting && rest.until > 0)
}

/// Clears a start-failure latch so the next run tries the agent once. A usage limit, rate limit or
/// signed-out rest is not a start failure and is left alone. Returns whether anything was cleared.
pub fn retry_start(store: &Store, account: &str) -> bool {
    if !retry_startable(saved(store, account).as_ref()) {
        return false;
    }
    if let Err(error) = store.kv_delete(&key(account)) {
        tracing::warn!(account, error = format!("{error:#}"), "couldn't clear the agent's start-failure latch");
    }
    true
}

/// Agent `backend`'s accounts (also ones a command of its own runs) whose last rest was for being signed out.
pub fn signed_out(store: &Store, backend: &str) -> Vec<String> {
    // The prefix also finds agents whose name starts with `backend`'s.
    store
        .kv_prefix(&key(backend))
        .iter()
        .filter_map(|k| k.strip_prefix("rest."))
        .filter(|a| backend_of(a) == backend && saved(store, a).is_some_and(|r| r.reason == Reason::SignedOut))
        .map(str::to_owned)
        .collect()
}

/// The agent answered (or the user asked for it again): its rest and backoff are over. Whether it was resting.
pub fn clear(store: &Store, account: &str) -> bool {
    if store.kv_get(&key(account)).is_none() {
        return false;
    }
    if let Err(error) = store.kv_delete(&key(account)) {
        tracing::warn!(account, error = format!("{error:#}"), "couldn't clear the agent's rest");
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signing_in_ends_only_a_signed_out_rest() {
        let store = Store::open(std::path::Path::new(":memory:")).expect("a store");
        let wrapped = account_of("claude", Some("claude-wrapper"));
        for account in ["claude", wrapped.as_str(), "claudette"] {
            assert_eq!(record(&store, account, Reason::SignedOut, 0, None).map(|r| r.reason), Some(Reason::SignedOut));
        }
        record(&store, "codex", Reason::UsageLimit, 0, None);
        let mut ended = signed_out(&store, "claude");
        ended.sort();
        assert_eq!(ended, ["claude".to_owned(), wrapped], "a command of its own signs in with the agent");
        assert!(signed_out(&store, "codex").is_empty(), "signing in doesn't give usage back");
        // One Codync can't sign in to looks again sooner.
        assert_eq!(get(&store, "claudette", 0).map(|r| r.until), Some(SIGNED_OUT_ELSEWHERE));
        assert_eq!(record(&store, "claude", Reason::SignedOut, 0, None).map(|r| r.until), Some(SIGNED_OUT));
    }

    #[test]
    fn a_silent_acp_session_start_is_bounded_unless_the_agent_has_one_process_per_session() {
        assert!(!silent_start_timed_out(0, false));
        assert!(!silent_start_timed_out(SESSION_START_STALL - 1, false));
        assert!(silent_start_timed_out(SESSION_START_STALL, false), "five minutes of silence ends the start");
        assert!(
            !silent_start_timed_out(SESSION_START_STALL * 2, true),
            "Mistral and Antigravity run one process per session and keep their behavior"
        );
    }

    #[test]
    fn a_usage_read_without_data_keeps_the_last_real_reset_time() {
        // OpenBot 0.75.0 (#13018): a later read with no data must not send a backup model back early.
        assert_eq!(held_reset(Some(9_000), Some(7_200_000), 1_000), Some(9_000), "a fresh read wins");
        assert_eq!(held_reset(None, Some(7_200_000), 1_000), Some(7_200_000), "no data keeps the held reset");
        assert_eq!(held_reset(None, None, 1_000), None, "a first read with no data works as before");
        assert_eq!(held_reset(None, Some(900), 1_000), None, "a reset that already passed is not kept");

        let store = Store::open(std::path::Path::new(":memory:")).expect("a store");
        let first = record(&store, "claude", Reason::UsageLimit, 1_000, Some(7_200_000)).expect("rests");
        assert_eq!(first.reset_at, Some(7_200_000));
        // A later read with no data still rests until the same reset, not for the unknown-reset hour.
        let later = record(&store, "claude", Reason::UsageLimit, 2_000, None).expect("rests");
        assert_eq!(later.until, 7_200_000);
        assert_eq!(later.reset_at, Some(7_200_000));
        // A fresh read with its own reset replaces the held one.
        let fresh = record(&store, "claude", Reason::UsageLimit, 2_000, Some(8_000_000)).expect("rests");
        assert_eq!(fresh.until, 8_000_000);
    }

    #[test]
    fn a_new_turn_clears_only_a_start_failure_latch() {
        let store = Store::open(std::path::Path::new(":memory:")).expect("a store");
        record(&store, "codex", Reason::NotStarting, 1_000, None);
        assert!(retry_startable(get(&store, "codex", 1_000).as_ref()));
        assert!(retry_start(&store, "codex"), "a new turn retries a provider that wouldn't start");
        assert!(get(&store, "codex", 1_000).is_none(), "the latch is gone, not shortened");

        record(&store, "claude", Reason::UsageLimit, 1_000, Some(9_000));
        assert!(!retry_startable(get(&store, "claude", 1_000).as_ref()));
        assert!(!retry_start(&store, "claude"), "a plan limit is not a start failure");
        assert_eq!(get(&store, "claude", 1_000).map(|rest| rest.reason), Some(Reason::UsageLimit));
        assert!(!retry_startable(None), "no rest is nothing to retry");
    }

    #[test]
    fn a_usage_limit_rests_until_its_reset_and_transient_failures_back_off() {
        let store = Store::open(std::path::Path::new(":memory:")).expect("a store");
        let rest = record(&store, "claude", Reason::UsageLimit, 1_000, Some(7_201_000)).expect("rests");
        assert_eq!(rest.until, 7_201_000);
        assert_eq!(get(&store, "claude", 7_200_999).map(|r| r.reason), Some(Reason::UsageLimit));
        assert_eq!(get(&store, "claude", 7_201_000), None, "over at the reset");
        assert_eq!(record(&store, "codex", Reason::UsageLimit, 0, None).map(|r| r.until), Some(UNKNOWN_RESET));

        let waits: Vec<i64> =
            (0..5).filter_map(|_| record(&store, "gemini", Reason::Overloaded, 0, None)).map(|r| r.until).collect();
        assert_eq!(waits, [MINUTE, 5 * MINUTE, 25 * MINUTE, 60 * MINUTE, 60 * MINUTE]);
        assert!(clear(&store, "gemini"));
        assert_eq!(record(&store, "gemini", Reason::Overloaded, 0, None).map(|r| r.until), Some(MINUTE));
        assert!(!clear(&store, "cursor"));
        assert_eq!(record(&store, "cursor", Reason::ModelMissing, 0, None), None);

        let mut custom: crate::store::BotConfig =
            serde_json::from_value(serde_json::json!({"id": "b", "name": "B", "backend": "custom", "command": "a"}))
                .expect("a bot");
        let a = account(&custom);
        custom.command = Some("b".into());
        assert!(a.starts_with("custom#") && a != account(&custom), "two custom commands are two agents");
        custom.backend = "codex".into();
        custom.command = None;
        assert_eq!(account(&custom), "codex");
    }
}
