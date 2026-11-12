//! Bounded read-only Git context. Navigation selects existing visible bots, never changes a checkout.
use super::str_arg;
use crate::chat::calls::project::git_output;
use crate::hub::Hub;
use anyhow::{Context, Result, ensure};
use serde::Serialize;
use serde_json::{Value, json};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const MAX_CONFLICTS: usize = 100;
const MAX_WORKTREES: usize = 20;
const MAX_BOTS: usize = 1000;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Worktree {
    path: String,
    branch: Option<String>,
    head: Option<String>,
    detached: bool,
    locked: bool,
    prunable: bool,
    bot_ids: Vec<String>,
    #[serde(skip)]
    bare: bool,
    #[serde(skip)]
    canonical: Option<PathBuf>,
}

impl Worktree {
    fn new(path: &str) -> Self {
        Self {
            path: path.to_owned(),
            branch: None,
            head: None,
            detached: false,
            locked: false,
            prunable: false,
            bot_ids: Vec::new(),
            bare: false,
            canonical: None,
        }
    }
}

pub(super) async fn read(hub: &Arc<Hub>, body: &Value) -> Result<Value> {
    let bot = hub
        .store
        .bot(str_arg(body, "botId")?)?
        .filter(|bot| !bot.deleted && !bot.config.hidden && !bot.config.is_group())
        .context("This bot is not available in the roster")?;
    ensure!(!bot.config.cwd.is_empty(), "This bot has no project folder");
    let cwd = bot.config.cwd;
    let dir = Path::new(&cwd);
    let (root, conflicts, worktrees) = tokio::join!(
        git_output(dir, &["rev-parse", "--show-toplevel"]),
        git_output(
            dir,
            &["diff", "--no-ext-diff", "--no-textconv", "--name-only", "--diff-filter=U", "--no-relative", "-z"]
        ),
        git_output(dir, &["worktree", "list", "--porcelain", "-z"]),
    );
    let mut errors = Vec::new();
    let root = match root.and_then(|bytes| root_path(&bytes)) {
        Ok(root) => Some(root),
        Err(error) => {
            errors.push(format!("Current worktree: {error}"));
            None
        }
    };
    let mut conflicts = match conflicts.and_then(|bytes| conflict_paths(&bytes)) {
        Ok(paths) => paths,
        Err(error) => {
            errors.push(format!("Conflicts: {error}"));
            Vec::new()
        }
    };
    if conflicts.len() > MAX_CONFLICTS {
        errors.push(format!("Showing the first {MAX_CONFLICTS} conflict files."));
        conflicts.truncate(MAX_CONFLICTS);
    }
    let mut worktrees = match worktrees.and_then(|bytes| parse_worktrees(&bytes)) {
        Ok(trees) => trees.into_iter().filter(|tree| !tree.bare).collect::<Vec<_>>(),
        Err(error) => {
            errors.push(format!("Worktrees: {error}"));
            Vec::new()
        }
    };
    // Keep the selected worktree visible even when a large repository exceeds the display limit.
    if let Some(root) = &root {
        worktrees.sort_by_key(|tree| !same_path(Path::new(&tree.path), Path::new(root)));
    }
    let truncated = worktrees.len() > MAX_WORKTREES;
    if truncated {
        errors.push(format!("Showing the first {MAX_WORKTREES} worktrees; only exact-root bot matches are available."));
        worktrees.truncate(MAX_WORKTREES);
    }
    let mut bots = hub
        .store
        .bots()?
        .into_iter()
        .filter(|bot| !bot.deleted && !bot.config.hidden && !bot.config.is_group() && !bot.config.cwd.is_empty())
        .map(|bot| (bot.config.id, bot.config.cwd))
        .collect::<Vec<_>>();
    bots.sort_by(|left, right| left.0.cmp(&right.0));
    if bots.len() > MAX_BOTS {
        errors.push(format!("Only the first {MAX_BOTS} roster bots were matched."));
        bots.truncate(MAX_BOTS);
    }
    // Canonicalization may touch mounted filesystems. Keep it off the async executor.
    let worktrees = tokio::task::spawn_blocking(move || {
        for tree in &mut worktrees {
            tree.canonical = std::fs::canonicalize(&tree.path).ok();
        }
        for (id, cwd) in bots {
            let Ok(cwd) = std::fs::canonicalize(cwd) else { continue };
            if let Some(index) = matching_worktree(&worktrees, &cwd, !truncated) {
                worktrees[index].bot_ids.push(id);
            }
        }
        worktrees
    })
    .await
    .context("Could not match worktree folders")?;
    Ok(
        json!({"root": root, "conflicts": conflicts, "worktrees": worktrees, "error": (!errors.is_empty()).then(|| errors.join(" · "))}),
    )
}

fn root_path(bytes: &[u8]) -> Result<String> {
    let path = std::str::from_utf8(bytes).context("Git returned a non-UTF-8 project folder")?;
    // Remove only the command's line ending, preserving spaces and newline characters in the path itself.
    let path = path.strip_suffix('\n').unwrap_or(path);
    #[cfg(windows)]
    let path = path.strip_suffix('\r').unwrap_or(path);
    ensure!(!path.is_empty() && !path.contains('\0'), "Git returned an invalid project folder");
    Ok(path.to_owned())
}

fn nul_fields(bytes: &[u8]) -> Result<Vec<&str>> {
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    ensure!(bytes.last() == Some(&0), "Git output was incomplete");
    let text = std::str::from_utf8(bytes).context("Git returned a non-UTF-8 path")?;
    Ok(text.strip_suffix('\0').unwrap_or(text).split('\0').collect())
}

fn conflict_paths(bytes: &[u8]) -> Result<Vec<String>> {
    nul_fields(bytes)?
        .into_iter()
        .map(|path| {
            ensure!(!path.is_empty(), "Git returned an empty conflict path");
            Ok(path.to_owned())
        })
        .collect()
}

fn parse_worktrees(bytes: &[u8]) -> Result<Vec<Worktree>> {
    let mut trees = Vec::new();
    let mut current: Option<Worktree> = None;
    for field in nul_fields(bytes)? {
        if field.is_empty() {
            if let Some(tree) = current.take() {
                trees.push(tree);
            }
            continue;
        }
        if let Some(path) = field.strip_prefix("worktree ") {
            ensure!(current.is_none() && !path.is_empty(), "Git returned an invalid worktree record");
            current = Some(Worktree::new(path));
            continue;
        }
        let tree = current.as_mut().context("Git returned worktree metadata without a path")?;
        if let Some(head) = field.strip_prefix("HEAD ") {
            ensure!(
                matches!(head.len(), 40 | 64)
                    && head.bytes().all(|byte| byte.is_ascii_hexdigit())
                    && tree.head.is_none(),
                "Git returned an invalid worktree commit"
            );
            tree.head = Some(head.to_owned());
        } else if let Some(branch) = field.strip_prefix("branch ") {
            ensure!(!branch.is_empty() && tree.branch.is_none(), "Git returned an invalid worktree branch");
            tree.branch = Some(branch.strip_prefix("refs/heads/").unwrap_or(branch).to_owned());
        } else if field == "detached" {
            tree.detached = true;
        } else if field == "bare" {
            tree.bare = true;
        } else if field == "locked" || field.starts_with("locked ") {
            tree.locked = true;
        } else if field == "prunable" || field.starts_with("prunable ") {
            tree.prunable = true;
        }
        // Future read-only fields do not become arbitrary UI metadata.
    }
    ensure!(current.is_none(), "Git returned an unterminated worktree record");
    Ok(trees)
}

fn path_parts(path: &Path) -> Vec<OsString> {
    path.components()
        .map(|part| {
            let value = part.as_os_str();
            if cfg!(windows) {
                value.to_str().map(|value| OsString::from(value.to_lowercase())).unwrap_or_else(|| value.to_owned())
            } else {
                value.to_owned()
            }
        })
        .collect()
}

fn same_path(left: &Path, right: &Path) -> bool {
    path_parts(left) == path_parts(right)
}

fn matching_worktree(trees: &[Worktree], cwd: &Path, descendants: bool) -> Option<usize> {
    let cwd = path_parts(cwd);
    trees
        .iter()
        .enumerate()
        .filter_map(|(index, tree)| {
            let root = path_parts(tree.canonical.as_deref().unwrap_or_else(|| Path::new(&tree.path)));
            let matches = if descendants { cwd.starts_with(&root) } else { cwd == root };
            matches.then_some((index, root.len()))
        })
        .max_by_key(|(_, depth)| *depth)
        .filter(|(index, _)| !trees[*index].prunable && trees[*index].canonical.is_some())
        .map(|(index, _)| index)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(path: &str) -> Worktree {
        let mut tree = Worktree::new(path);
        tree.canonical = Some(PathBuf::from(path));
        tree
    }

    #[test]
    fn nul_records_preserve_spaces_newlines_and_unicode_paths() {
        let output = format!(
            "worktree /repo with spaces\n中文\0HEAD {}\0branch refs/heads/feature\0locked private reason\0\0worktree /other\0HEAD {}\0detached\0prunable secret reason\0\0",
            "a".repeat(40),
            "b".repeat(40)
        );
        let trees = parse_worktrees(output.as_bytes()).unwrap();
        assert_eq!(trees.len(), 2);
        assert_eq!(trees[0].path, "/repo with spaces\n中文");
        assert_eq!(trees[0].branch.as_deref(), Some("feature"));
        assert!(trees[0].locked);
        assert!(trees[1].detached && trees[1].prunable);
        let serialized = serde_json::to_string(&trees).unwrap();
        assert!(
            !serialized.contains("private reason")
                && !serialized.contains("secret reason")
                && !serialized.contains("canonical")
        );
        assert_eq!(conflict_paths("a b\0line\nbreak\0中文\0".as_bytes()).unwrap(), ["a b", "line\nbreak", "中文"]);
    }

    #[test]
    fn malformed_and_truncated_git_outputs_are_unavailable() {
        assert!(parse_worktrees(b"worktree /repo\0HEAD broken\0\0").is_err());
        assert!(parse_worktrees(b"worktree /repo\0").is_err());
        assert!(parse_worktrees(b"worktree /one\0worktree /two\0\0").is_err());
        assert!(parse_worktrees(b"HEAD a\0\0").is_err());
        assert!(conflict_paths(b"a\0b").is_err());
        assert!(conflict_paths(b"\xff\0").is_err());
        assert!(conflict_paths(b"\0").is_err());
        assert!(parse_worktrees(b"").unwrap().is_empty());
        assert!(conflict_paths(b"").unwrap().is_empty());
    }

    #[test]
    fn bot_mapping_uses_the_closest_canonical_worktree_and_component_boundaries() {
        let mut trees = vec![tree("/repo"), tree("/repo/nested"), tree("/elsewhere")];
        assert_eq!(matching_worktree(&trees, Path::new("/repo/nested/package"), true), Some(1));
        assert_eq!(matching_worktree(&trees, Path::new("/repo-other"), true), None);
        assert_eq!(matching_worktree(&trees, Path::new("/repo/package"), false), None);
        assert_eq!(matching_worktree(&trees, Path::new("/repo"), false), Some(0));
        trees[1].prunable = true;
        // A prunable nested worktree must not claim a visible bot.
        assert_ne!(matching_worktree(&trees, Path::new("/repo/nested"), false), Some(1));
        assert_eq!(matching_worktree(&trees, Path::new("/repo/nested/package"), true), None);
        trees[0].canonical = None;
        assert_eq!(matching_worktree(&trees, Path::new("/repo"), true), None);
    }

    #[test]
    fn root_path_removes_only_one_command_line_ending() {
        assert_eq!(root_path(b"/repo with spaces \n").unwrap(), "/repo with spaces ");
        assert_eq!(root_path(b"/repo\n\n").unwrap(), "/repo\n");
        assert!(root_path(b"\0\n").is_err());
        assert!(root_path(b"\n").is_err());
    }
}
