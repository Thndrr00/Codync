//! The apps' brain methods: which memory several bots share. A brain is chosen per bot; the Fleet brain is the
//! shared memory every bot used before brains, and a bot joins it by using the shared memory again.

use super::files;
use crate::hub::Hub;
use crate::store::{BotConfig, FLEET};
use anyhow::{Result, anyhow, bail};
use codync_mem::files as mf;
use serde_json::{Value, json};

/// The most a brain's name may hold.
const NAME_MAX: usize = 60;

/// Every brain with the bots working in it, the Fleet one first.
pub fn list(hub: &Hub) -> Result<Value> {
    let bots = bots(hub)?;
    let brains: Vec<Value> = hub
        .store
        .brains()?
        .into_iter()
        .map(|brain| {
            let members: Vec<&str> =
                bots.iter().filter(|cfg| in_brain(cfg, &brain.id)).map(|cfg| cfg.id.as_str()).collect();
            json!({"id": brain.id, "name": brain.name, "shared": brain.layer.is_none(), "members": members})
        })
        .collect();
    Ok(json!({"brains": brains}))
}

/// A new brain.
pub fn create(hub: &Hub, b: &Value) -> Result<Value> {
    let name = name(hub, b, None)?;
    let id = hub.store.save_brain(None, &name)?;
    // Its folder exists before any bot brings the agent here.
    files::ensure(&files::folder(Some(&format!("brain:{id}"))));
    let brains = list(hub)?;
    Ok(json!({"brains": brains["brains"], "brain": brain(hub, &id)?}))
}

/// A brain renamed in place; its bots keep working in it.
pub fn rename(hub: &Hub, b: &Value) -> Result<Value> {
    let id = arg(b, "id")?;
    let name = name(hub, b, Some(id))?;
    hub.store.save_brain(Some(id), &name)?;
    Ok(json!({"brains": list(hub)?["brains"], "brain": brain(hub, id)?}))
}

/// A bot set to work in a brain. The Fleet brain is the shared memory: joining it leaves the project brain.
pub fn join(hub: &Hub, b: &Value) -> Result<Value> {
    let bot = arg(b, "botId")?;
    let id = arg(b, "brainId")?;
    let brain = hub.store.brain(id)?.ok_or_else(|| anyhow!("unknown brain"))?;
    let patch = if brain.layer.is_none() {
        json!({"id": bot, "brain": null, "sharedMemory": true})
    } else {
        json!({"id": bot, "brain": id})
    };
    let bot = hub.update_bot(&patch)?;
    super::changed(hub, Some(id));
    Ok(json!({"bot": bot, "brains": list(hub)?["brains"]}))
}

/// A bot taken out of its brain: it keeps only its own memory (picking a brain again brings the shared one back).
pub fn leave(hub: &Hub, b: &Value) -> Result<Value> {
    let bot = arg(b, "botId")?;
    let bot = hub.update_bot(&json!({"id": bot, "brain": null, "sharedMemory": false}))?;
    Ok(json!({"bot": bot, "brains": list(hub)?["brains"]}))
}

/// A brain deleted: its bots go back to the Fleet brain, which keeps the shared memory they used.
pub fn delete(hub: &Hub, b: &Value) -> Result<Value> {
    let id = arg(b, "id")?;
    if id == FLEET {
        bail!("the Fleet brain stays");
    }
    if hub.store.brain(id)?.is_none() {
        bail!("unknown brain");
    }
    let mut updated = Vec::new();
    for cfg in bots(hub)?.into_iter().filter(|cfg| in_brain(cfg, id)) {
        updated.push(hub.update_bot(&json!({"id": cfg.id, "brain": null, "sharedMemory": true}))?);
        super::changed(hub, Some(&cfg.id));
    }
    hub.store.delete_brain(id)?;
    files::remove_brain(id);
    Ok(json!({"bots": updated, "brains": list(hub)?["brains"]}))
}

/// Whether `cfg` works in the brain `id`; the Fleet brain is the bots that use the shared memory and no project
/// brain.
fn in_brain(cfg: &BotConfig, id: &str) -> bool {
    match cfg.brain.as_deref() {
        Some(brain) => brain == id,
        None => id == FLEET && super::shared(cfg),
    }
}

/// The bots that can have a brain.
fn bots(hub: &Hub) -> Result<Vec<BotConfig>> {
    Ok(hub
        .store
        .bots()?
        .into_iter()
        .filter(|row| !row.deleted && !row.config.is_group())
        .map(|row| row.config)
        .collect())
}

/// The brain `id` as the apps show it, with its bots.
pub fn brain(hub: &Hub, id: &str) -> Result<Value> {
    let brain = hub.store.brain(id)?.ok_or_else(|| anyhow!("unknown brain"))?;
    let members: Vec<String> = bots(hub)?.into_iter().filter(|cfg| in_brain(cfg, id)).map(|cfg| cfg.id).collect();
    Ok(json!({"id": brain.id, "name": brain.name, "shared": brain.layer.is_none(), "members": members}))
}

/// The name a request gives, trimmed, one to [`NAME_MAX`] characters and the only brain with it (`id`: the brain
/// being renamed).
fn name(hub: &Hub, b: &Value, id: Option<&str>) -> Result<String> {
    let name: String = arg(b, "name")?.trim().chars().take(NAME_MAX).collect();
    if name.is_empty() {
        bail!("name required");
    }
    let taken = hub
        .store
        .brains()?
        .into_iter()
        .any(|brain| brain.name.eq_ignore_ascii_case(&name) && Some(brain.id.as_str()) != id);
    if taken {
        bail!("another brain is called that");
    }
    Ok(name)
}

fn arg<'a>(b: &'a Value, k: &str) -> Result<&'a str> {
    b[k].as_str().map(str::trim).filter(|v| !v.is_empty()).ok_or_else(|| anyhow!("{k} required"))
}

/// A brain bundle's format and the most files one holds.
const FORMAT: &str = "codync-brain-bundle";
const VERSION: u32 = 1;
const MAX_FILES: usize = 200;

/// A brain as one JSON file: its name and the exact content of each of its files, `MEMORY.md` included, so another
/// computer can hold the same memory.
pub fn export_file(hub: &Hub, b: &Value) -> Result<Value> {
    let id = arg(b, "id")?;
    let brain = hub.store.brain(id)?.ok_or_else(|| anyhow!("unknown brain"))?;
    if let Some(cfg) = bots(hub)?.into_iter().find(|cfg| in_brain(cfg, id)) {
        files::scan(hub, &cfg);
    }
    let files = hub.store.memory_read(|c| mf::layer(c, brain.layer.as_deref()))?;
    let files: Vec<Value> = files
        .into_iter()
        .filter(|f| !f.deleted)
        .filter_map(|f| {
            let content = files::read(brain.layer.as_deref(), &f.name).ok().flatten()?;
            Some(json!({"name": f.name, "content": content}))
        })
        .collect();
    Ok(json!({"format": FORMAT, "version": VERSION, "brain": {"name": brain.name}, "files": files}))
}

/// A brain made from an exported one. `name` renames it on the way in. Answers the brain and the files the checks
/// turned away.
pub fn import_file(hub: &Hub, b: &Value) -> Result<Value> {
    let bundle = &b["bundle"];
    if bundle["format"] != FORMAT {
        bail!("This isn't a Codync brain bundle");
    }
    if bundle["version"].as_u64().is_none_or(|v| v > u64::from(VERSION)) {
        bail!("This bundle is from a newer Codync; update to import it");
    }
    let files: Vec<(String, String)> = serde_json::from_value(bundle["files"].clone())
        .map_err(|e| anyhow!("The bundle's files are unreadable: {e}"))?;
    if files.len() > MAX_FILES {
        bail!("A brain bundle holds at most {MAX_FILES} files");
    }
    let asked = b["name"].as_str().map(str::trim).filter(|n| !n.is_empty());
    let named: String = match asked {
        Some(name) => name.chars().take(NAME_MAX).collect(),
        None => bundle["brain"]["name"].as_str().unwrap_or("Brain").trim().chars().take(NAME_MAX).collect(),
    };
    let id = hub.store.save_brain(None, &free_name(hub, &named))?;
    let stored = hub.store.brain(id.as_str())?.ok_or_else(|| anyhow!("unknown brain"))?;
    let mut skipped = Vec::new();
    for (name, content) in files {
        let writer = files::Writer { author: mf::Author::Imported, bot: None, cause: mf::Cause::Edit, message: None };
        if let Err(error) = files::write(hub, stored.layer.as_deref(), &name, Some(&content), None, writer) {
            skipped.push(json!({"name": name, "why": format!("{error:#}")}));
        }
    }
    super::changed(hub, stored.layer.as_deref());
    Ok(json!({"brain": brain(hub, &id)?, "brains": list(hub)?["brains"], "skipped": skipped}))
}

/// A brain's name as a file name: letters, digits, dot, dash and underscore stay, the rest becomes `_`.
pub fn file_name(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || matches!(c, '.' | '-' | '_') { c } else { '_' })
        .collect::<String>()
        .trim_matches('.')
        .to_owned()
        .chars()
        .take(60)
        .collect()
}

/// `name`, or the first "name (2)", "name (3)"… no brain has. With `brains` taken, `brains + 1` tries always finds
/// one.
fn free_name(hub: &Hub, name: &str) -> String {
    let brains = hub.store.brains().unwrap_or_default();
    (1..=brains.len() as u64 + 1)
        .map(|n| if n == 1 { name.to_owned() } else { format!("{name} ({n})") })
        .find(|candidate| !brains.iter().any(|brain| brain.name.eq_ignore_ascii_case(candidate)))
        .unwrap_or_else(|| name.to_owned())
}
