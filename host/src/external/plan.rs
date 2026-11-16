//! How long a plan card of a terminal agent waits before it approves the plan by itself.
//!
//! One choice belongs to the computer and covers every agent on it: 10 s (the default), 30 s,
//! 1 min, 5 min, a time the person types, or "when I press Start", which never approves on its own.
//! The card says when it will start (`autoApproveAt`), so every client can count down.
//!
//! Ported from `CopilotKit/OpenBot`'s `agent-defaults-preference-store.ts` and `PlanStartRow.tsx`
//! (Apache-2.0): the presets, the 5-second floor and the 10-second default are its Settings > Agents >
//! "Start the work".

use crate::store::Store;

/// The presets, in milliseconds.
pub const PRESETS: [i64; 4] = [10_000, 30_000, 60_000, 300_000];
/// The time until the person changes it.
pub const DEFAULT: Option<i64> = Some(PRESETS[0]);
/// A typed time shorter than this is refused.
pub const MIN: i64 = 5_000;
/// The longest time a person may type. The hook that holds the plan open lasts [`HOOK_SECONDS`], so a
/// longer wait could never approve anything.
pub const MAX: i64 = 840_000;
/// Seconds Claude Code lets the waiting hook run: the longest plan wait plus room to answer.
pub const HOOK_SECONDS: u64 = 900;
const KV: &str = "agents.planStartAfterMs";
/// Saved when the plan waits for the person instead of a timer.
const WAIT: &str = "wait";
/// How long the host holds the card open after it starts the work by itself.
const SLACK: i64 = 15_000;

/// The saved time, or `None` when the plan waits for the person. Anything unusable falls back to the default.
pub fn saved(store: &Store) -> Option<i64> {
    store
        .kv_read(KV)
        .ok()
        .flatten()
        .map(
            |raw| {
                if raw == WAIT { Ok(None) } else { raw.parse::<i64>().ok().filter(|ms| valid(*ms)).map(Some).ok_or(()) }
            },
        )
        .and_then(Result::ok)
        .unwrap_or(DEFAULT)
}

/// Saves a new time (`None` = when the person presses Start) and returns it.
pub fn set(store: &Store, after: Option<i64>) -> anyhow::Result<Option<i64>> {
    let raw = match after {
        None => WAIT.to_owned(),
        Some(ms) => {
            anyhow::ensure!(
                valid(ms),
                "The plan starts after at least {} seconds, or never by itself; {ms} isn't a time like that.",
                MIN / 1_000
            );
            ms.to_string()
        }
    };
    store.kv_set(KV, &raw)?;
    Ok(after)
}

/// A whole number of seconds from [`MIN`] to [`MAX`].
fn valid(ms: i64) -> bool {
    ms >= MIN && ms <= MAX && ms % 1_000 == 0
}

/// How long the host holds a plan card open: until it starts the work by itself, or the usual wait for a
/// person. The card is answered before the hook gives up.
pub fn wait(after: Option<i64>) -> std::time::Duration {
    after.map_or(super::DECISION_WAIT, |ms| {
        std::time::Duration::from_millis(u64::try_from(ms + SLACK).unwrap_or_default())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> Store {
        Store::open(std::path::Path::new(":memory:")).expect("store")
    }

    #[test]
    fn the_time_defaults_to_ten_seconds_and_survives_a_save() {
        let store = store();
        assert_eq!(saved(&store), DEFAULT);
        assert_eq!(set(&store, Some(300_000)).unwrap(), Some(300_000));
        assert_eq!(saved(&store), Some(300_000));
        assert_eq!(set(&store, None).unwrap(), None);
        assert_eq!(saved(&store), None, "the plan waits for the person");
    }

    #[test]
    fn a_time_outside_the_choices_or_a_saved_value_nobody_wrote_falls_back() {
        let store = store();
        assert!(set(&store, Some(4_000)).is_err(), "under five seconds");
        assert!(set(&store, Some(90_500)).is_err(), "not a whole second");
        assert!(set(&store, Some(MAX + 1_000)).is_err());
        store.kv_set(KV, "later").unwrap();
        assert_eq!(saved(&store), DEFAULT);
    }
}
