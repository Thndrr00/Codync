//! Document mutations notify clients through the existing owner-bot snapshot event.

use super::Hub;
use crate::LockExt;
use crate::store::Store;
use anyhow::Result;
use serde_json::json;

impl Hub {
    pub(crate) fn change_documents<T>(&self, bot: &str, change: impl FnOnce(&Store) -> Result<T>) -> Result<T> {
        let _guard = self.emit_lock.locked();
        let before = self.store.current_rev();
        let result = change(&self.store)?;
        if self.store.current_rev() > before {
            self.store.touch_bot(bot)?;
            if let Some(value) = self.bot_value(bot) {
                let _ = self.events.send(json!({"type": "bot", "rev": value["rev"], "bot": value}));
            }
        }
        Ok(result)
    }
}
