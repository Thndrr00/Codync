//! Whether a message the user sends to a working agent joins its running turn by itself (`steer`)
//! or waits in the queue for the next one. On unless the user turned it off (Settings > Sending).

use crate::store::Store;
use anyhow::Result;

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
}
