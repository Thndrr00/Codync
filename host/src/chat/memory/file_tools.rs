//! The memory tools every harness gets for its files: read one or list them, and change one. Agents that reach the
//! folders with their own file tools may use those instead; both go through the same checks.

use super::files::{self, Writer};
use crate::hub::Hub;
use crate::store::{BotConfig, Store};
use anyhow::{Result, anyhow, bail};
use codync_mem::files::{self as mf, Author, Cause, MAIN};
use codync_mem::layers::Scope;
use serde_json::{Value, json};
use std::sync::Arc;

pub fn definitions() -> [Value; 2] {
    let file =
        json!({"type": "string", "description": "MEMORY.md, or a note's name like people.md or health/allergies.md."});
    let shared = json!({"type": "boolean", "description": "True for the brain's memory, shared with the other bots working in it (the Fleet brain: with all the user's bots)."});
    let scope = json!({"type": "string", "enum": ["personal", "brain"], "description": "personal: your own memory; brain: your brain's, shared with the other bots in it."});
    [
        json!({
            "name": "read_memory",
            "description": "Read one of your memory files, or without a file, list them with their descriptions and sizes.",
            "inputSchema": {"type": "object", "properties": {"file": file, "shared": shared, "scope": scope}},
            "annotations": {"readOnlyHint": true},
        }),
        json!({
            "name": "edit_memory",
            "description": "Change one of your memory files. MEMORY.md holds one short fact about the user per line and is in every session in full; dreaming keeps it within its cap by moving detail into notes. A note holds one topic's detail, in a topic folder when one fits (health/allergies.md); it starts with frontmatter whose description: line says when to read it, and holds at most 2,000 tokens. Give append to add a line, old and new to replace text that appears once, content to write the whole file, or delete. Changing or taking out lines works only in a turn the user started in your own chat; a line the user had you forget comes back only when they ask for it there.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file": file,
                    "shared": shared,
                    "scope": scope,
                    "append": {"type": "string", "description": "A line to add at the end, like \"- Has two cats\"."},
                    "old": {"type": "string", "description": "Text the file has exactly once."},
                    "new": {"type": "string", "description": "What replaces old."},
                    "content": {"type": "string", "description": "The whole file (a note: frontmatter first)."},
                    "delete": {"type": "boolean"},
                },
                "required": ["file"],
            },
        }),
    ]
}

pub async fn call(hub: &Arc<Hub>, cfg: &BotConfig, name: &str, args: &Value) -> Result<Value> {
    let (hub, cfg, name, args) = (hub.clone(), cfg.clone(), name.to_owned(), args.clone());
    tokio::task::spawn_blocking(move || match name.as_str() {
        "read_memory" => read(&hub, &cfg, &args),
        "edit_memory" => edit(&hub, &cfg, &args),
        _ => bail!("unknown memory tool: {name}"),
    })
    .await?
}

/// The layer an argument names: its own, or its brain's (`None` is the Fleet brain's shared folder).
fn layer(store: &Store, cfg: &BotConfig, args: &Value) -> Result<Option<String>> {
    let scope = if args["scope"].as_str() == Some(Scope::Brain.name()) || args["shared"] == true {
        Scope::Brain
    } else {
        Scope::Personal
    };
    let layers = super::layers(store, cfg);
    if !layers.sees(scope) {
        bail!("This bot works in no brain, so it has no shared memory.");
    }
    Ok(layers.layer(scope).map(str::to_owned))
}

fn read(hub: &Hub, cfg: &BotConfig, args: &Value) -> Result<Value> {
    files::scan(hub, cfg);
    let Some(name) = args["file"].as_str() else {
        let mut files = Vec::new();
        let layers = files::layers(&hub.store, cfg);
        for (scope, layer) in layers.keys() {
            let live = hub.store.memory_read(|c| mf::layer(c, layer))?.into_iter().filter(|f| !f.deleted);
            files.extend(live.map(|f| {
                json!({"file": f.name, "scope": scope.name(), "shared": scope == Scope::Brain,
                    "description": f.description, "tokens": f.tokens})
            }));
        }
        return Ok(json!({"files": files}));
    };
    let layer = layer(&hub.store, cfg, args)?;
    let content = files::read(layer.as_deref(), name)?;
    let content = match content {
        Some(content) => content,
        None if name == MAIN => String::new(),
        None => bail!("There's no memory file {name}."),
    };
    Ok(json!({"file": name, "content": content}))
}

fn edit(hub: &Hub, cfg: &BotConfig, args: &Value) -> Result<Value> {
    let name = args["file"].as_str().ok_or_else(|| anyhow!("file required"))?;
    let layer = layer(&hub.store, cfg, args)?;
    files::scan(hub, cfg);
    let before = files::read(layer.as_deref(), name)?;
    let after = if args["delete"] == true {
        if before.is_none() {
            bail!("There's no memory file {name}.");
        }
        None
    } else if let Some(content) = args["content"].as_str() {
        Some(content.to_owned())
    } else if let Some(line) = args["append"].as_str() {
        let line = line.trim();
        if line.is_empty() {
            bail!("append needs a line.");
        }
        let start = match before.as_deref().map(str::trim_end) {
            Some(text) if !text.is_empty() => format!("{text}\n"),
            _ if name == MAIN => String::new(),
            _ => bail!("There's no note {name}; give content to create it, frontmatter first."),
        };
        Some(format!("{start}{line}\n"))
    } else {
        let (old, new) = (args["old"].as_str().unwrap_or_default(), args["new"].as_str().unwrap_or_default());
        let current =
            before.as_deref().ok_or_else(|| anyhow!("There's no memory file {name}; give content to create it."))?;
        match current.matches(old).count() {
            _ if old.is_empty() => bail!("Give append, old and new, content, or delete."),
            1 => Some(current.replacen(old, new, 1)),
            0 => bail!("old isn't in {name}. Read it again."),
            _ => bail!("old appears more than once in {name}. Give more of the text around it."),
        }
    };
    let expected = before.as_deref().map(mf::hash);
    let cause = if after.is_some() { Cause::Edit } else { Cause::Delete };
    let message = hub.runtime(&cfg.id).message;
    let writer = Writer { author: Author::Bot, bot: Some(cfg), cause, message: message.as_deref() };
    files::write(hub, layer.as_deref(), name, after.as_deref(), expected.as_deref(), writer)?;
    Ok(json!({"status": "saved", "file": name}))
}
