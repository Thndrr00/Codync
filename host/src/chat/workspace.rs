//! Workspace search for a bot: a lexical pass over its folder, with file, line and a snippet, so
//! answers can cite where something lives. No embeddings and no index: it reads text files on
//! demand, bounded by size, depth and time, and skips build and dependency folders.

use crate::hub::Hub;
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

pub const TOOLS: &[&str] = &["search_workspace", "read_workspace_file"];
/// Search that reranks the lexical matches with Ollama embeddings when Ollama is running.
pub const SEMANTIC_TOOL: &str = "semantic_search";
const MAX_FILES: usize = 4_000;
const MAX_BYTES: u64 = 512 * 1024;
const MAX_MATCHES: usize = 40;
const SNIPPET_CHARS: usize = 160;
const READ_BYTES: usize = 64 * 1024;
const SKIP_DIRS: &[&str] =
    &[".git", "node_modules", "target", "dist", "build", ".next", ".venv", "venv", "__pycache__", ".idea", ".vscode"];

pub fn tools() -> Value {
    json!([
        {
            "name": "search_workspace",
            "description": "Search the text files of this bot's folder for words and return up to 40 matches with the file, the line number and a snippet. Use it to find where something lives before answering, and cite file:line. Skips .git, node_modules, target and other build folders; binary and very large files are ignored.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {"type": "string", "minLength": 1, "maxLength": 200, "description": "Words that must all appear on the same line, in any order."},
                    "path": {"type": "string", "description": "A subfolder or file, relative to the bot's folder (optional)."},
                    "caseSensitive": {"type": "boolean", "description": "Match case exactly (default false)."}
                },
                "required": ["query"]
            },
            "annotations": {"readOnlyHint": true}
        },
        {
            "name": "read_workspace_file",
            "description": "Read up to 64 KB of a text file in this bot's folder, with line numbers. Use it after search_workspace to read the surrounding code. Binary files are refused.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "minLength": 1, "description": "A file, relative to the bot's folder."},
                    "offset": {"type": "integer", "minimum": 1, "description": "First line (1-based, default 1)."},
                    "limit": {"type": "integer", "minimum": 1, "maximum": 1000, "description": "At most this many lines (default 200)."}
                },
                "required": ["path"]
            },
            "annotations": {"readOnlyHint": true}
        },
        {
            "name": "semantic_search",
            "description": "Like search_workspace, but reranks the matches by meaning with a local Ollama embedding model when Ollama is running; otherwise it returns the lexical order. Use it when the words in the question may not match the words in the files. Cite file:line from the result.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {"type": "string", "minLength": 1, "maxLength": 200, "description": "What you are looking for, in a sentence if the exact words are unknown."},
                    "path": {"type": "string", "description": "A subfolder or file, relative to the bot's folder (optional)."},
                    "caseSensitive": {"type": "boolean", "description": "Match case exactly for the lexical pass (default false)."}
                },
                "required": ["query"]
            },
            "annotations": {"readOnlyHint": true}
        }
    ])
}

pub fn call(hub: &Hub, bot: &str, name: &str, arguments: &Value) -> Result<Value> {
    let row = hub.store.bot(bot)?.filter(|b| !b.deleted).context("unknown bot")?;
    let root = PathBuf::from(&row.config.cwd);
    if root.as_os_str().is_empty() || !root.is_dir() {
        bail!("this bot has no folder to search");
    }
    match name {
        "search_workspace" => search(&root, arguments),
        "read_workspace_file" => read(&root, arguments),
        other => bail!("unknown workspace tool {other}"),
    }
}

/// Lexical candidates reranked by meaning with a local Ollama embedding model. Without Ollama it is
/// the lexical search with `semantic: false`, so the tool always answers.
pub async fn semantic_search(hub: &Hub, bot: &str, arguments: &Value) -> Result<Value> {
    let row = hub.store.bot(bot)?.filter(|b| !b.deleted).context("unknown bot")?;
    let root = PathBuf::from(&row.config.cwd);
    if root.as_os_str().is_empty() || !root.is_dir() {
        bail!("this bot has no folder to search");
    }
    let mut value = search(&root, arguments)?;
    let matches = value["matches"].as_array().cloned().unwrap_or_default();
    let Some(query) = arguments["query"].as_str().map(str::trim).filter(|q| !q.is_empty()) else {
        bail!("query is required");
    };
    let Some(query_vector) = crate::chat::embeddings::embed(query).await else {
        value["semantic"] = false.into();
        return Ok(value);
    };
    let vectors = futures::future::join_all(
        matches.iter().map(|item| crate::chat::embeddings::embed(item["snippet"].as_str().unwrap_or_default())),
    )
    .await;
    let mut scored: Vec<(f32, Value)> = matches
        .into_iter()
        .zip(vectors)
        .map(|(item, vector)| {
            let score = vector.as_deref().map_or(0.0, |vector| crate::chat::embeddings::cosine(&query_vector, vector));
            (score, item)
        })
        .collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let matches: Vec<Value> = scored
        .into_iter()
        .map(|(score, mut item)| {
            item["score"] = json!(score);
            item
        })
        .collect();
    Ok(json!({"matches": matches, "semantic": true}))
}

fn inside(root: &Path, relative: &str) -> Result<PathBuf> {
    let root = root.canonicalize().context("the bot's folder is gone")?;
    let candidate = root.join(relative);
    let resolved = candidate.canonicalize().with_context(|| format!("no such path: {relative}"))?;
    if !resolved.starts_with(&root) {
        bail!("that path is outside this bot's folder");
    }
    Ok(resolved)
}

fn search(root: &Path, arguments: &Value) -> Result<Value> {
    let query = arguments["query"].as_str().context("query is required")?.trim().to_owned();
    if query.is_empty() {
        bail!("query is required");
    }
    let case = arguments["caseSensitive"].as_bool().unwrap_or(false);
    let needles: Vec<String> = if case {
        query.split_whitespace().map(str::to_owned).collect()
    } else {
        query.split_whitespace().map(str::to_lowercase).collect()
    };
    let start = match arguments["path"].as_str().map(str::trim).filter(|p| !p.is_empty()) {
        Some(path) => inside(root, path)?,
        None => root.canonicalize()?,
    };
    let mut files = 0;
    let mut matches: Vec<Value> = vec![];
    let mut stack = vec![start];
    while let Some(path) = stack.pop() {
        if files >= MAX_FILES || matches.len() >= MAX_MATCHES {
            break;
        }
        let meta = match std::fs::metadata(&path) {
            Ok(meta) => meta,
            Err(_) => continue,
        };
        if meta.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
            if SKIP_DIRS.contains(&name) {
                continue;
            }
            if let Ok(entries) = std::fs::read_dir(&path) {
                for entry in entries.flatten() {
                    stack.push(entry.path());
                }
            }
            continue;
        }
        if !meta.is_file() || meta.len() > MAX_BYTES {
            continue;
        }
        let Some(text) = read_text(&path) else { continue };
        files += 1;
        for (index, line) in text.lines().enumerate() {
            if matches.len() >= MAX_MATCHES {
                break;
            }
            let haystack = if case { line.to_owned() } else { line.to_lowercase() };
            if needles.iter().all(|needle| haystack.contains(needle)) {
                let relative = path.strip_prefix(root).unwrap_or(&path).display().to_string();
                matches.push(json!({
                    "path": relative,
                    "line": index + 1,
                    "snippet": snippet(line),
                }));
            }
        }
    }
    Ok(json!({"matches": matches, "filesScanned": files}))
}

fn read(root: &Path, arguments: &Value) -> Result<Value> {
    let relative = arguments["path"].as_str().context("path is required")?;
    let path = inside(root, relative)?;
    let meta = std::fs::metadata(&path)?;
    if !meta.is_file() {
        bail!("that is not a file");
    }
    if meta.len() > READ_BYTES as u64 {
        bail!("that file is too large to read here (over 64 KB)");
    }
    let text = read_text(&path).context("that file is not text")?;
    let offset = arguments["offset"].as_u64().unwrap_or(1).max(1) as usize;
    let limit = arguments["limit"].as_u64().unwrap_or(200).clamp(1, 1000) as usize;
    let numbered: Vec<String> = text
        .lines()
        .enumerate()
        .skip(offset - 1)
        .take(limit)
        .map(|(index, line)| format!("{:>6}  {line}", index + 1))
        .collect();
    let root = root.canonicalize()?;
    let relative = path.strip_prefix(&root).unwrap_or(&path).display().to_string();
    Ok(json!({"path": relative, "offset": offset, "text": numbered.join("\n")}))
}

fn read_text(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    // A NUL in the first block means binary.
    if bytes.iter().take(1024).any(|b| *b == 0) {
        return None;
    }
    String::from_utf8(bytes).ok()
}

fn snippet(line: &str) -> String {
    let trimmed = line.trim();
    if trimmed.chars().count() <= SNIPPET_CHARS {
        return trimmed.to_owned();
    }
    let cut: String = trimmed.chars().take(SNIPPET_CHARS).collect();
    format!("{cut}…")
}
