//! Codync's own documentation and code index, served to a bot as the `codync-docs` MCP server.
//!
//! Tools: `docs_search` and `code_search` find text in the bot's repository (or a folder its owner
//! named) and answer with a file and line; `docs_read` returns numbered lines of one indexed file.
//! Nothing leaves this computer: the index is built from the working folder, respects `.gitignore`
//! and is bounded in files and bytes.
//!
//! Pathfinder was read as the pattern for these tools but not packaged: it is source-available under
//! the Elastic License 2.0 with an Additional Use Grant, not an OSI license, and it indexes through a
//! PostgreSQL database and embeddings. Codync needs neither, so this index is its own.

mod index;
mod search;
#[cfg(test)]
mod tests;

use crate::LockExt;
use crate::hub::Hub;
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub const METHODS: &[&str] = &["docsStatus", "setDocsFolder", "docsCall"];

pub const INSTRUCTIONS: &str = "Search this computer's documentation and code before asking or guessing: \
docs_search finds documentation, code_search finds source, and docs_read returns numbered lines of one \
indexed file. Every answer carries the file path and line: cite them to the person as path:line. The folder \
indexed is the bot's working folder, or the one its owner named in Settings. The index respects .gitignore.";

/// The one folder most recently indexed. Bot tool calls arrive in short bursts from the same repository,
/// so a small cache saves walking it again; a different root replaces it.
static CACHE: Mutex<Option<(String, Arc<index::Index>)>> = Mutex::new(None);

pub fn tools() -> Value {
    json!([
        {
            "name": "docs_search",
            "description": "Search this computer's documentation (Markdown, text and docs folders). Every result names its file and line, to cite.",
            "inputSchema": {"type": "object", "properties": {
                "query": {"type": "string", "description": "Words to find; all of them must appear."},
                "limit": {"type": "integer", "minimum": 1, "maximum": 20, "description": "How many results at most (default 8)."}
            }, "required": ["query"]}
        },
        {
            "name": "code_search",
            "description": "Search the source files of this computer's repository or indexed folder. Every result names its file and line, to cite.",
            "inputSchema": {"type": "object", "properties": {
                "query": {"type": "string", "description": "Words to find; all of them must appear."},
                "limit": {"type": "integer", "minimum": 1, "maximum": 20, "description": "How many results at most (default 8)."}
            }, "required": ["query"]}
        },
        {
            "name": "docs_read",
            "description": "Read numbered lines of one indexed file. Use the path docs_search or code_search returned.",
            "inputSchema": {"type": "object", "properties": {
                "path": {"type": "string", "description": "A path as a search result gave it."},
                "start": {"type": "integer", "minimum": 1, "description": "First line (default 1)."},
                "end": {"type": "integer", "minimum": 1, "description": "Last line (default 400 lines after start)."}
            }, "required": ["path"]}
        }
    ])
}

pub fn call(hub: &Arc<Hub>, method: &str, body: &Value) -> Result<Value> {
    match method {
        "docsCall" => tool(hub, body),
        "docsStatus" => status(hub, body),
        "setDocsFolder" => set_folder(hub, body),
        other => bail!("Unknown docs method `{other}`."),
    }
}

/// `docs_search`, `code_search` or `docs_read` for `botId`.
fn tool(hub: &Arc<Hub>, body: &Value) -> Result<Value> {
    let bot = body["botId"].as_str().filter(|bot| !bot.is_empty()).context("botId is required.")?;
    let name = body["name"].as_str().context("name is required.")?;
    let arguments = &body["arguments"];
    let index = index_for(hub, bot)?;
    match name {
        "docs_search" => Ok(search_result(&index, arguments, true)),
        "code_search" => Ok(search_result(&index, arguments, false)),
        "docs_read" => {
            let path = arguments["path"].as_str().filter(|path| !path.is_empty()).context("path is required.")?;
            let start = arguments["start"].as_u64().map(|line| usize::try_from(line).unwrap_or(1));
            let end = arguments["end"].as_u64().map(|line| usize::try_from(line).unwrap_or(1));
            let excerpt = search::read(&index, path, start, end)
                .with_context(|| format!("{path} is not in the index. Use a path a search returned."))?;
            Ok(json!({
                "path": excerpt.path,
                "start": excerpt.start,
                "end": excerpt.end,
                "totalLines": excerpt.total,
                "citation": format!("{}:{}", excerpt.path, excerpt.start),
                "text": excerpt.text,
            }))
        }
        other => bail!("Unknown docs tool `{other}`."),
    }
}

fn search_result(index: &index::Index, arguments: &Value, docs_only: bool) -> Value {
    let query = arguments["query"].as_str().unwrap_or_default();
    let limit = arguments["limit"].as_u64().map_or(8, |limit| usize::try_from(limit).unwrap_or(8)).clamp(1, 20);
    let hits = search::search(index, &search::terms(query), docs_only, limit);
    let results: Vec<Value> = hits
        .iter()
        .map(|hit| {
            json!({
                "path": hit.path,
                "line": hit.line,
                "snippet": hit.snippet,
                "score": hit.score,
                "citation": format!("{}:{}", hit.path, hit.line),
            })
        })
        .collect();
    json!({"query": query, "results": results, "truncated": index.truncated})
}

fn status(hub: &Arc<Hub>, body: &Value) -> Result<Value> {
    let bot = body["botId"].as_str().filter(|bot| !bot.is_empty()).context("botId is required.")?;
    let index = index_for(hub, bot)?;
    Ok(json!({
        "folder": index.root,
        "files": index.docs.len(),
        "bytes": index.bytes(),
        "truncated": index.truncated,
        "builtAt": index.built_at,
    }))
}

/// Names the folder a bot's index reads, or clears the choice so its working folder is used.
fn set_folder(hub: &Arc<Hub>, body: &Value) -> Result<Value> {
    let bot = body["botId"].as_str().filter(|bot| !bot.is_empty()).context("botId is required.")?;
    hub.store.bot(bot)?.filter(|row| !row.deleted).context("Unknown bot.")?;
    let path = body["path"].as_str().unwrap_or_default().trim();
    if path.is_empty() {
        hub.store.kv_delete(&format!("docs.folder.{bot}"))?;
    } else {
        let folder = index::folder(path)?;
        hub.store.kv_set(&format!("docs.folder.{bot}"), &folder.to_string_lossy())?;
    }
    *CACHE.locked() = None;
    status(hub, body)
}

fn root_for(hub: &Hub, bot: &str) -> Result<PathBuf> {
    if let Some(folder) = hub.store.kv_read(&format!("docs.folder.{bot}"))?.filter(|folder| !folder.trim().is_empty()) {
        return Ok(PathBuf::from(folder));
    }
    let row = hub.store.bot(bot)?.filter(|row| !row.deleted).context("Unknown bot.")?;
    let cwd = PathBuf::from(&row.config.cwd);
    let cwd = cwd.canonicalize().unwrap_or(cwd);
    anyhow::ensure!(cwd.is_dir(), "The bot's working folder does not exist, so there is nothing to index.");
    Ok(cwd)
}

fn index_for(hub: &Arc<Hub>, bot: &str) -> Result<Arc<index::Index>> {
    let root = root_for(hub, bot)?;
    let key = root.to_string_lossy().into_owned();
    {
        let cache = CACHE.locked();
        if let Some((cached, index)) = cache.as_ref()
            && *cached == key
            && index.fresh()
        {
            return Ok(index.clone());
        }
    }
    let index = Arc::new(index::open(&root));
    *CACHE.locked() = Some((key, index.clone()));
    Ok(index)
}
