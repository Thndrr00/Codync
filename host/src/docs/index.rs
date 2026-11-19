//! A bounded index of one folder: its documentation and source files, read as UTF-8 text, with the
//! person's `.gitignore` respected. The index is small enough to rebuild lazily and is kept under the
//! host's data directory, so a restart does not walk a repository again.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub(super) const MAX_FILES: usize = 4_000;
pub(super) const MAX_FILE_BYTES: u64 = 400 * 1024;
pub(super) const MAX_TOTAL_BYTES: usize = 24 * 1024 * 1024;
const MAX_DEPTH: usize = 24;
const MAX_IGNORE_FILES: usize = 100;
/// How long an index is reused before its folder is walked again.
const INDEX_TTL_MS: i64 = 10 * 60 * 1000;
const CACHE_FILE_MAX: usize = 16 * 1024 * 1024;

const SKIPPED_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "build",
    "dist",
    ".next",
    "vendor",
    ".venv",
    "venv",
    "__pycache__",
    ".idea",
    ".gradle",
    ".turbo",
    "coverage",
    "out",
    "Pods",
    "DerivedData",
];

#[derive(Clone, Serialize, Deserialize)]
pub struct Doc {
    pub path: String,
    pub lines: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Index {
    pub root: String,
    pub built_at: i64,
    pub truncated: bool,
    pub docs: Vec<Doc>,
}

impl Index {
    pub fn fresh(&self) -> bool {
        crate::store::now_ms().saturating_sub(self.built_at) < INDEX_TTL_MS
    }

    pub fn bytes(&self) -> usize {
        self.docs.iter().flat_map(|doc| doc.lines.iter()).map(|line| line.len() + 1).sum()
    }
}

#[derive(Clone)]
struct Rule {
    pattern: String,
    negated: bool,
    dir_only: bool,
    anchored: bool,
}

impl Rule {
    fn matches(&self, relative: &str, is_dir: bool) -> bool {
        if self.dir_only && !is_dir {
            return false;
        }
        if self.anchored {
            glob(&self.pattern, relative)
        } else {
            // An unanchored rule names a path segment anywhere below its .gitignore.
            relative.split('/').any(|part| glob(&self.pattern, part))
        }
    }
}

fn parse_gitignore(text: &str) -> Vec<Rule> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| {
            let negated = line.starts_with('!');
            let line = line.trim_start_matches('!');
            let dir_only = line.ends_with('/');
            let line = line.trim_end_matches('/');
            let anchored = line.starts_with('/') || line.contains('/');
            Some(Rule { pattern: line.trim_start_matches('/').to_owned(), negated, dir_only, anchored })
        })
        .collect()
}

/// `*` and `?` never cross a `/`; `**` does.
fn glob(pattern: &str, path: &str) -> bool {
    let pattern: Vec<&str> = pattern.split('/').filter(|part| !part.is_empty()).collect();
    let path: Vec<&str> = path.split('/').filter(|part| !part.is_empty()).collect();
    segments(&pattern, &path)
}

fn segments(pattern: &[&str], path: &[&str]) -> bool {
    match pattern.split_first() {
        None => path.is_empty(),
        Some(("**", rest)) => (0..=path.len()).any(|skip| segments(rest, &path[skip..])),
        Some((part, rest)) => {
            path.split_first().is_some_and(|(name, tail)| segment(part, name) && segments(rest, tail))
        }
    }
}

fn segment(pattern: &str, name: &str) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let name: Vec<char> = name.chars().collect();
    // Iterative with backtracking for `*`.
    let (mut p, mut n, mut star, mut mark) = (0, 0, None, 0);
    while n < name.len() {
        if p < pattern.len() && (pattern[p] == '?' || pattern[p] == name[n]) {
            p += 1;
            n += 1;
        } else if p < pattern.len() && pattern[p] == '*' {
            star = Some(p);
            mark = n;
            p += 1;
        } else if let Some(at) = star {
            p = at + 1;
            mark += 1;
            n = mark;
        } else {
            return false;
        }
    }
    while p < pattern.len() && pattern[p] == '*' {
        p += 1;
    }
    p == pattern.len()
}

struct Walk {
    root: PathBuf,
    docs: Vec<Doc>,
    bytes: usize,
    files: usize,
    truncated: bool,
    seen_paths: HashSet<String>,
    ignores: Vec<(String, Vec<Rule>)>,
    ignore_files: usize,
}

/// Reads `root` into an index, respecting `.gitignore` files and every size limit.
pub(super) fn build(root: &Path) -> Index {
    let mut walk = Walk {
        root: root.to_path_buf(),
        docs: Vec::new(),
        bytes: 0,
        files: 0,
        truncated: false,
        seen_paths: HashSet::new(),
        ignores: Vec::new(),
        ignore_files: 0,
    };
    walk.dir(root, "", 0);
    Index {
        root: root.to_string_lossy().into_owned(),
        built_at: crate::store::now_ms(),
        truncated: walk.truncated,
        docs: walk.docs,
    }
}

impl Walk {
    fn ignored(&self, relative: &str, dir: bool) -> bool {
        let mut ignored = false;
        for (base, rules) in &self.ignores {
            let local = if base.is_empty() {
                relative
            } else if relative == base {
                ""
            } else {
                match relative.strip_prefix(&format!("{base}/")) {
                    Some(local) => local,
                    None => continue,
                }
            };
            if local.is_empty() {
                continue;
            }
            for rule in rules {
                if rule.matches(local, dir) {
                    ignored = !rule.negated;
                }
            }
        }
        ignored
    }

    fn dir(&mut self, path: &Path, relative: &str, depth: usize) {
        if self.files >= MAX_FILES || self.bytes >= MAX_TOTAL_BYTES || depth > MAX_DEPTH {
            self.truncated = true;
            return;
        }
        let Ok(entries) = std::fs::read_dir(path) else { return };
        let mut names: Vec<(String, PathBuf, bool)> = entries
            .flatten()
            .filter_map(|entry| {
                let name = entry.file_name().to_string_lossy().into_owned();
                let path = entry.path();
                let dir = entry.file_type().ok()?.is_dir();
                Some((name, path, dir))
            })
            .collect();
        names.sort_by(|a, b| a.0.cmp(&b.0));
        // Each folder's rules apply below it, and only there.
        let mut pushed = false;
        if self.ignore_files < MAX_IGNORE_FILES
            && let Ok(text) = std::fs::read_to_string(path.join(".gitignore"))
        {
            let rules = parse_gitignore(&text);
            if !rules.is_empty() {
                self.ignores.push((relative.to_owned(), rules));
                self.ignore_files += 1;
                pushed = true;
            }
        }
        for (name, child, dir) in names {
            if name.starts_with('.') || SKIPPED_DIRS.contains(&name.as_str()) {
                continue;
            }
            let child_relative = if relative.is_empty() { name.clone() } else { format!("{relative}/{name}") };
            if self.ignored(&child_relative, dir) {
                continue;
            }
            if dir {
                self.dir(&child, &child_relative, depth + 1);
            } else {
                self.file(&child, &child_relative);
            }
            if self.files >= MAX_FILES || self.bytes >= MAX_TOTAL_BYTES {
                self.truncated = true;
                break;
            }
        }
        if pushed {
            self.ignores.pop();
        }
    }

    fn file(&mut self, path: &Path, relative: &str) {
        let Ok(metadata) = std::fs::metadata(path) else { return };
        if !metadata.is_file() || metadata.len() > MAX_FILE_BYTES {
            return;
        }
        let Ok(bytes) = std::fs::read(path) else { return };
        if bytes.iter().any(|byte| *byte == 0) {
            return;
        }
        let Ok(text) = String::from_utf8(bytes) else { return };
        if !self.seen_paths.insert(relative.to_owned()) {
            return;
        }
        let lines: Vec<String> = text.lines().map(str::to_owned).collect();
        if lines.is_empty() {
            return;
        }
        let size: usize = lines.iter().map(|line| line.len() + 1).sum();
        if self.bytes + size > MAX_TOTAL_BYTES {
            self.truncated = true;
            return;
        }
        self.bytes += size;
        self.files += 1;
        self.docs.push(Doc { path: relative.to_owned(), lines });
    }
}

/// Where the cache for `root` lives: one file per folder, under the host's data directory.
fn cache_path(root: &Path) -> PathBuf {
    let mut digest = Sha256::new();
    digest.update(root.to_string_lossy().as_bytes());
    let name = crate::chat::files::digest_hex(&digest.finalize());
    crate::service::data_dir().join("docs-index").join(format!("{}.json", &name[..16.min(name.len())]))
}

/// The cached index for `root` while it is still fresh, or a fresh walk.
pub(super) fn open(root: &Path) -> Index {
    if let Some(index) = read_cache(root)
        && index.fresh()
    {
        return index;
    }
    let index = build(root);
    write_cache(&index);
    index
}

fn read_cache(root: &Path) -> Option<Index> {
    let bytes = std::fs::read(cache_path(root)).ok()?;
    if bytes.len() > CACHE_FILE_MAX {
        return None;
    }
    let index: Index = serde_json::from_slice(&bytes).ok()?;
    (index.root == root.to_string_lossy().as_ref()).then_some(index)
}

fn write_cache(index: &Index) {
    let path = cache_path(Path::new(&index.root));
    let Ok(bytes) = serde_json::to_vec(index) else { return };
    if bytes.len() > CACHE_FILE_MAX {
        return;
    }
    if let Some(parent) = path.parent()
        && std::fs::create_dir_all(parent).is_err()
    {
        return;
    }
    if let Err(error) = std::fs::write(&path, bytes) {
        tracing::warn!(error = %error, path = %path.display(), "docs index was not cached");
    }
}

/// `root` as an absolute path, when it names a folder.
pub(super) fn folder(value: &str) -> Result<PathBuf> {
    let path = Path::new(value.trim());
    anyhow::ensure!(!value.trim().is_empty(), "Choose the folder to index.");
    let root = path.canonicalize().context("That folder does not exist.")?;
    anyhow::ensure!(root.is_dir(), "That is not a folder.");
    Ok(root)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gitignore_patterns_follow_segments_and_double_stars() {
        assert!(glob("*.log", "debug.log"));
        assert!(glob("*.log", "a/debug.log"));
        assert!(!glob("*.log", "debug.txt"));
        assert!(glob("**/draft.md", "docs/draft.md"));
        assert!(glob("docs/**", "docs/a/b.md"));
        assert!(glob("docs/*.md", "docs/a.md"));
        assert!(!glob("docs/*.md", "docs/a/b.md"));
        assert!(segment("a?c", "abc"));
    }

    #[test]
    fn a_gitignore_rule_is_read_with_its_directory_and_negation() {
        let rules = parse_gitignore("# notes\nbuild/\n!keep.log\n*.log\n/sub/only\n");
        assert_eq!(rules.len(), 4);
        assert!(rules[0].dir_only && !rules[0].negated);
        assert!(rules[1].negated);
        assert!(rules[3].anchored);
        assert!(rules[0].matches("build", true));
        assert!(rules[0].matches("nested/build", true));
        assert!(!rules[0].matches("build", false), "a folder rule is not a file rule");
        assert!(rules[3].matches("sub/only", false));
        assert!(!rules[3].matches("other/sub/only", false));
    }
}
