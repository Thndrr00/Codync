//! The notes on the apps' memory screen: each layer's list, and one note to read with its recent versions.
//! `MEMORY.md` shows as the layer's facts instead.

use super::files;
use crate::hub::Hub;
use crate::store::BotConfig;
use anyhow::{Result, anyhow};
use codync_mem::files::{self as mf, MAIN};
use codync_mem::layers::Scope;
use serde_json::{Value, json};

/// Versions the detail view lists.
const VERSIONS_SHOWN: usize = 20;

/// The scope a memory-screen argument names: `scope`, or the older `shared`.
fn scope(b: &Value) -> Scope {
    if b["scope"].as_str() == Some(Scope::Brain.name()) || b["shared"] == true { Scope::Brain } else { Scope::Personal }
}

/// Both layers' notes for the memory screen. The caller has scanned the folders.
pub fn listing(hub: &Hub, cfg: &BotConfig) -> Result<Value> {
    let layers = super::layers(&hub.store, cfg);
    let layer = |scope: Scope| -> Result<Value> {
        let layer = layers.layer(scope);
        let notes: Vec<mf::File> =
            hub.store.memory_read(|c| mf::layer(c, layer))?.into_iter().filter(|f| f.name != MAIN).collect();
        Ok(json!({"files": notes, "folder": files::folder(layer).display().to_string()}))
    };
    Ok(json!({
        "yours": if layers.sees(Scope::Brain) { layer(Scope::Brain)? } else { Value::Null },
        "bot": layer(Scope::Personal)?,
        "cap": mf::FILE_CAP,
        "folderCap": mf::FOLDER_CAP,
    }))
}

/// A note as it is on disk now, and its recent versions.
pub fn detail(hub: &Hub, cfg: &BotConfig, b: &Value) -> Result<Value> {
    let name = b["file"].as_str().ok_or_else(|| anyhow!("file required"))?;
    let scope = scope(b);
    let layers = super::layers(&hub.store, cfg);
    let layer = layers.layer(scope);
    files::scan(hub, cfg);
    let content = files::read(layer, name)?;
    let versions = hub.store.memory_read(|c| mf::versions(c, layer, name, VERSIONS_SHOWN))?;
    Ok(json!({
        "file": name,
        "scope": scope.name(),
        "shared": scope == Scope::Brain,
        "content": content,
        "hash": content.as_deref().map(mf::hash),
        "path": files::folder(layer).join(name).display().to_string(),
        "versions": versions,
    }))
}
