//! The ceiling on local mini-apps: the bots on this computer build them in their workspaces and the
//! host serves them from this machine, so the ceiling is a safeguard here, not a plan.
//!
//! The setting belongs to the computer (every bot shares it) and lives in kv. Only a new app takes
//! a slot: an update to an app the bot already registered never counts, so a bot can keep improving
//! what it made after the computer is full. Deleting an app makes room for another.
//!
//! The choices, the default and the shape of the refusal follow `routines/limits.rs`. Card F11.16 of
//! `docs/guides/openbot-gap-census.md` records why Codync has this local limit instead of plans.

use crate::store::Store;
use anyhow::{Result, bail};
use serde_json::{Value, json};

/// What the person may set.
pub const CHOICES: [u32; 5] = [25, 50, 100, 250, 1_000];
/// The ceiling until the person changes it.
pub const DEFAULT: u32 = 100;
const KV: &str = "localApps.limit";

/// The saved ceiling; anything but a choice falls back to the default.
pub fn saved(store: &Store) -> u32 {
    store
        .kv_read(KV)
        .ok()
        .flatten()
        .and_then(|raw| raw.parse::<u32>().ok())
        .filter(|limit| CHOICES.contains(limit))
        .unwrap_or(DEFAULT)
}

/// Saves a new ceiling and returns it.
pub fn set(store: &Store, limit: u32) -> Result<u32> {
    anyhow::ensure!(CHOICES.contains(&limit), "Use {CHOICES:?} local apps; {limit} isn't one of them.");
    store.kv_set(KV, &limit.to_string())?;
    Ok(limit)
}

/// How many local apps this computer holds, its ceiling, the choices and how many bots made them.
pub fn status(store: &Store) -> Result<Value> {
    let totals = store.app_totals()?;
    Ok(json!({
        "count": totals.apps,
        "limit": saved(store),
        "choices": CHOICES,
        "botsWithApps": totals.bots,
    }))
}

/// Refuses a new app once the computer is full. An app the bot already registered is an update, not a slot.
pub fn check_new(store: &Store, bot_id: &str, slug: &str) -> Result<()> {
    if store.has_bot_app(bot_id, slug)? {
        return Ok(());
    }
    let limit = saved(store);
    let count = store.app_totals()?.apps;
    if i64::from(limit) <= count {
        bail!("{}", refusal(count, limit));
    }
    Ok(())
}

/// What `save_app` answers when the computer is full, naming both numbers and either way out.
fn refusal(count: i64, limit: u32) -> String {
    format!(
        "This computer has {count} registered apps and a limit of {limit}. Delete an app you no longer use, or raise the limit in Settings > General > Local apps to make room. It is a local safeguard, not a plan."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> Store {
        Store::open(std::path::Path::new(":memory:")).expect("store")
    }

    #[test]
    fn the_ceiling_is_one_of_the_choices_and_defaults_when_unsaved() {
        let store = store();
        assert_eq!(saved(&store), DEFAULT);
        assert_eq!(set(&store, 250).unwrap(), 250);
        assert_eq!(saved(&store), 250);
        assert!(set(&store, 75).is_err());
        store.kv_set(KV, "75").unwrap();
        assert_eq!(saved(&store), DEFAULT, "a saved value outside the choices is ignored");
    }

    #[test]
    fn the_status_counts_every_app_and_the_bots_that_made_them() {
        let store = store();
        store.save_bot_app("one", "first", "First", None, None).unwrap();
        store.save_bot_app("one", "second", "Second", None, None).unwrap();
        store.save_bot_app("two", "third", "Third", None, None).unwrap();
        let status = status(&store).unwrap();
        assert_eq!(status["count"], 3);
        assert_eq!(status["limit"], DEFAULT);
        assert_eq!(status["botsWithApps"], 2);
        assert_eq!(status["choices"].as_array().unwrap().len(), CHOICES.len());
    }

    #[test]
    fn a_new_app_is_refused_at_the_limit_but_updates_and_deleting_make_room() {
        let store = store();
        set(&store, CHOICES[0]).unwrap();
        for n in 0..CHOICES[0] {
            store.save_bot_app("bot", &format!("app-{n}"), "App", None, None).unwrap();
        }
        assert_eq!(
            check_new(&store, "bot", "extra").unwrap_err().to_string(),
            "This computer has 25 registered apps and a limit of 25. Delete an app you no longer use, or raise the limit in Settings > General > Local apps to make room. It is a local safeguard, not a plan."
        );
        // One app past the ceiling (a save that predates this setting, say) still names both numbers.
        store.save_bot_app("bot", "over", "Over", None, None).unwrap();
        let refusal = check_new(&store, "bot", "extra").unwrap_err().to_string();
        assert_eq!(
            refusal,
            "This computer has 26 registered apps and a limit of 25. Delete an app you no longer use, or raise the limit in Settings > General > Local apps to make room. It is a local safeguard, not a plan."
        );

        // An app the bot already registered is an update, and never takes a new slot.
        check_new(&store, "bot", "app-0").unwrap();
        store.save_bot_app("bot", "app-0", "App again", None, None).unwrap();

        // Deleting an app makes room for a new one.
        assert!(store.delete_bot_app("bot", "app-0").unwrap());
        assert!(store.delete_bot_app("bot", "over").unwrap());
        check_new(&store, "bot", "extra").unwrap();
        store.save_bot_app("bot", "extra", "Extra", None, None).unwrap();
        assert!(check_new(&store, "bot", "another").is_err());

        // Raising the ceiling makes room without deleting anything.
        set(&store, CHOICES[1]).unwrap();
        check_new(&store, "bot", "another").unwrap();
    }
}
