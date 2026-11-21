//! Whether a message the user sends to a working agent joins its running turn by itself (`steer`)
//! or waits in the queue for the next one. On unless the user turned it off (Settings > Sending).

use crate::chat::group;
use crate::store::Store;
use anyhow::{Result, bail};

const KEY: &str = "chat.steerWhileWorking";

/// True unless the user chose Queue. A missing value is the default, which steers.
pub fn enabled(store: &Store) -> bool {
    store.kv_get(KEY).is_none_or(|value| value != "0")
}

/// Saves the choice and gives it back as the clients read it.
pub fn set(store: &Store, on: bool) -> Result<bool> {
    store.kv_set(KEY, if on { "1" } else { "0" })?;
    Ok(on)
}

/// The hint a steer or queue call gives when it names an id one character off: the same near match a
/// group uses for a mistyped member, so the caller can retry with the right bot instead of guessing
/// (OpenBot 0.74.0). Ported here so the check lives with the sending choice, not in the router.
pub(crate) fn near_bot_hint<'a>(id: &str, bots: impl Iterator<Item = (&'a str, &'a str)>) -> Option<String> {
    group::nearest(id, bots).map(|(near_id, name)| format!("Did you mean {name} (`{near_id}`)?"))
}

/// Checks the bot a queue/steer intent names. A near miss is named in the error, a missing bot isn't.
pub fn ensure_bot(store: &Store, id: &str) -> Result<()> {
    if store.bot(id)?.is_some_and(|row| !row.deleted) {
        return Ok(());
    }
    let bots: Vec<(String, String)> =
        store.bots()?.into_iter().filter(|row| !row.deleted).map(|row| (row.config.id, row.config.name)).collect();
    if let Some(hint) = near_bot_hint(id, bots.iter().map(|(id, name)| (id.as_str(), name.as_str()))) {
        bail!("Unknown bot `{id}`. {hint}");
    }
    bail!("Unknown bot")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steering_is_the_default_and_the_choice_sticks() {
        let store = Store::open(std::path::Path::new(":memory:")).expect("store");
        assert!(enabled(&store), "a missing value steers");
        assert!(!set(&store, false).expect("save"));
        assert!(!enabled(&store));
        assert!(set(&store, true).expect("save"));
        assert!(enabled(&store));
    }

    #[test]
    fn a_typo_in_a_queue_target_names_the_close_bot() {
        let right = "9f8b7c6d-5e4f-3a2b-1c0d-eeddccbbaa99";
        let wrong = "9f8b7c6d-5e4f-3a2b-1c0d-eeddccbbaa98";
        let bots = [(right, "Research"), ("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa", "Other")];
        let hint = |id: &str| near_bot_hint(id, bots.iter().map(|(id, name)| (*id, *name)));
        assert_eq!(hint(wrong).as_deref(), Some("Did you mean Research (`9f8b7c6d-5e4f-3a2b-1c0d-eeddccbbaa99`)?"));
        assert!(hint("short").is_none(), "a short id has no useful near match");
        assert!(hint("completely-different-id").is_none(), "an unrelated id gets no guess");
    }
}
