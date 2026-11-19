//! One JSON blob per chat in kv (`library.folders.<bot>`), stamped with the next sync rev on every change. A blob this
//! build cannot read reads as empty: an old or damaged library stays usable, without its folders.

use super::folders::FolderState;
use crate::store::Store;
use anyhow::Result;

const PREFIX: &str = "library.folders.";

fn key(bot: &str) -> String {
    format!("{PREFIX}{bot}")
}

pub(super) fn load(store: &Store, bot: &str) -> Result<FolderState> {
    match store.kv_read(&key(bot))? {
        Some(raw) => Ok(serde_json::from_str(&raw).unwrap_or_default()),
        None => Ok(FolderState::default()),
    }
}

/// Stamps one global rev for the change and saves the blob with it, in one transaction.
pub(super) fn save(store: &Store, bot: &str, state: &mut FolderState) -> Result<()> {
    store.kv_set_stamped(&key(bot), |rev| -> Result<String> {
        state.rev = rev;
        Ok(serde_json::to_string(state)?)
    })
}

/// The wire key of a library item, the same one the desktop uses when it drops an item on a folder.
pub(super) fn item_key(item: &serde_json::Value) -> Option<String> {
    match item["kind"].as_str()? {
        "artifact" => Some(format!("artifact:{}", item["artifact"]["id"].as_str()?)),
        "file" | "attachment" => {
            Some(format!("{}:{}:{}", item["kind"].as_str()?, item["entryId"].as_str()?, item["file"]["id"].as_str()?))
        }
        _ => None,
    }
}
