use super::Lease;
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};
use std::path::{Component, Path, PathBuf};
use tokio::io::AsyncReadExt;

async fn target(root: &Path, path: &str) -> Result<PathBuf> {
    ensure!(path.len() <= 4096, "Path is too long.");
    let path = Path::new(path);
    ensure!(
        !path.is_absolute() && path.components().all(|part| matches!(part, Component::Normal(_) | Component::CurDir)),
        "Use a relative workspace path without parent traversal."
    );
    let root = tokio::fs::canonicalize(root).await?;
    let file = tokio::fs::canonicalize(root.join(path)).await?;
    ensure!(file.starts_with(&root), "Path leaves the bot workspace.");
    Ok(file)
}

pub(super) async fn list(root: &Path, path: &str, lease: &Lease) -> Result<Value> {
    let target = target(root, path).await?;
    lease.check()?;
    let mut entries = tokio::fs::read_dir(target).await?;
    let mut items = vec![];
    while let Some(entry) = entries.next_entry().await? {
        lease.check()?;
        if items.len() == 500 {
            return Ok(json!({"items":items,"truncated":true}));
        }
        let kind = entry.file_type().await?;
        items.push(
            json!({"name":entry.file_name().to_string_lossy(),"directory":kind.is_dir(),"link":kind.is_symlink()}),
        );
    }
    items.sort_by_key(|item| item["name"].as_str().unwrap_or_default().to_lowercase());
    Ok(json!({"items":items,"truncated":false}))
}

pub(super) async fn read(root: &Path, path: &str, lease: &Lease) -> Result<Value> {
    let target = target(root, path).await?;
    lease.check()?;
    let file = tokio::fs::File::open(target).await?;
    ensure!(file.metadata().await?.is_file(), "Only regular text files can be previewed.");
    let mut bytes = vec![];
    file.take(128 * 1024 + 1).read_to_end(&mut bytes).await?;
    lease.check()?;
    if bytes.len() > 128 * 1024 {
        bail!("File preview is limited to 128 KiB.");
    }
    let text = String::from_utf8(bytes).context("This file is not UTF-8 text.")?;
    ensure!(!text.contains('\0'), "Binary files cannot be previewed as text.");
    Ok(json!({"text":text}))
}
