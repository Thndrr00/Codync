//! The ceiling on local script requests: a script that loops can't spend the person's quota.
//!
//! A delivery made on this computer (`POST /hooks/routines/:id`) takes a slot of its bot's window
//! before it queues a run. The window is one hour, the setting belongs to the computer (every bot
//! shares the choice), and the window's counts are memory-only: a restart forgets them, as a
//! ceiling on runaway scripts needs.
//!
//! Ported from `CopilotKit/OpenBot`'s `automation-server.ts` (Apache-2.0): the choices, the
//! default and the per-agent hour come from its Settings > Agents > Local scripts.

use super::hooks::Refused;
use crate::LockExt;
use crate::store::Store;
use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

/// What the person may set.
pub const CHOICES: [u32; 5] = [30, 60, 120, 300, 1000];
/// The ceiling until the person changes it.
pub const DEFAULT: u32 = CHOICES[0];
const KV: &str = "routines.scriptsPerHour";
const HOUR_MS: i64 = 3_600_000;

/// The saved ceiling; anything but a choice falls back to the default.
pub fn saved(store: &Store) -> u32 {
    store
        .kv_read(KV)
        .ok()
        .flatten()
        .and_then(|raw| raw.parse::<u32>().ok())
        .filter(|per_hour| CHOICES.contains(per_hour))
        .unwrap_or(DEFAULT)
}

/// Saves a new ceiling and returns it.
pub fn set(store: &Store, per_hour: u32) -> anyhow::Result<u32> {
    anyhow::ensure!(CHOICES.contains(&per_hour), "Use {CHOICES:?} requests an hour; {per_hour} isn't one of them.");
    store.kv_set(KV, &per_hour.to_string())?;
    Ok(per_hour)
}

/// How many local requests each bot made in the last hour.
#[derive(Default)]
pub struct Slots(Mutex<HashMap<String, VecDeque<i64>>>);

impl Slots {
    /// Takes a slot of `bot`'s window, or refuses once the hour is spent.
    pub fn take(&self, store: &Store, bot: &str, now: i64) -> Result<(), Refused> {
        let limit = saved(store);
        let mut all = self.0.locked();
        let recent = all.entry(bot.to_owned()).or_default();
        while recent.front().is_some_and(|time| now - time >= HOUR_MS) {
            recent.pop_front();
        }
        if recent.len() >= limit as usize {
            return Err(Refused::RateLimited(limit));
        }
        recent.push_back(now);
        Ok(())
    }

    /// Forgets every window (tests, and nothing else so far).
    #[cfg(test)]
    pub fn clear(&self) {
        self.0.locked().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> Store {
        Store::open(std::path::Path::new(":memory:")).expect("store")
    }

    #[test]
    fn a_bot_spends_its_hour_then_is_refused_until_the_window_moves() {
        let store = store();
        let slots = Slots::default();
        let start = 1_000_000;
        for _ in 0..DEFAULT {
            assert_eq!(slots.take(&store, "bot", start), Ok(()));
        }
        assert_eq!(slots.take(&store, "bot", start), Err(Refused::RateLimited(DEFAULT)));
        assert_eq!(slots.take(&store, "other", start), Ok(()), "another bot has its own window");
        assert_eq!(slots.take(&store, "bot", start + HOUR_MS), Ok(()), "an hour later a slot is free");
    }

    #[test]
    fn the_ceiling_is_one_of_the_choices_and_defaults_when_u_saved() {
        let store = store();
        assert_eq!(saved(&store), DEFAULT);
        assert_eq!(set(&store, 300).unwrap(), 300);
        assert_eq!(saved(&store), 300);
        assert!(set(&store, 45).is_err());
        store.kv_set(KV, "45").unwrap();
        assert_eq!(saved(&store), DEFAULT, "a saved value outside the choices is ignored");
    }
}
