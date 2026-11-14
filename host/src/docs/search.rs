//! Query over an [`Index`]: every term must appear, a path match ranks high, and each hit points at
//! the file and line a person can open.

use super::index::{Doc, Index};
use std::fmt::Write as _;

const SNIPPET: usize = 200;
const READ_LINES: usize = 400;
const READ_CHARS: usize = 60_000;
const MAX_TERMS: usize = 8;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hit {
    pub path: String,
    /// 1-based.
    pub line: usize,
    pub snippet: String,
    pub score: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Excerpt {
    pub path: String,
    pub start: usize,
    pub end: usize,
    pub total: usize,
    pub text: String,
}

/// The words of a query, lowercased, punctuation dropped, bounded.
pub fn terms(query: &str) -> Vec<String> {
    let mut terms: Vec<String> = query
        .split(|c: char| !c.is_alphanumeric())
        .filter(|term| !term.is_empty())
        .map(str::to_lowercase)
        .take(MAX_TERMS)
        .collect();
    terms.dedup();
    terms
}

/// Whether a path is documentation: its extension, or a place under `docs/`.
pub fn is_docs(path: &str) -> bool {
    let lower = path.to_lowercase();
    let extension = lower.rsplit_once('.').map(|(_, extension)| extension);
    matches!(extension, Some("md" | "mdx" | "markdown" | "txt" | "rst" | "adoc"))
        || lower.starts_with("docs/")
        || lower.contains("/docs/")
}

/// The best hits for `terms`, all of which must appear in a document or its path.
pub fn search(index: &Index, terms: &[String], docs_only: bool, limit: usize) -> Vec<Hit> {
    if terms.is_empty() || limit == 0 {
        return Vec::new();
    }
    let mut hits = Vec::new();
    for doc in &index.docs {
        if docs_only && !is_docs(&doc.path) {
            continue;
        }
        let Some(hit) = hit(doc, terms) else { continue };
        hits.push(hit);
    }
    hits.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.path.cmp(&b.path)).then_with(|| a.line.cmp(&b.line)));
    hits.truncate(limit.min(20));
    hits
}

fn hit(doc: &Doc, terms: &[String]) -> Option<Hit> {
    let path = doc.path.to_lowercase();
    let mut score = 0u32;
    let mut best = 0u32;
    let mut best_line = 0usize;
    for (index, line) in doc.lines.iter().enumerate() {
        let lower = line.to_lowercase();
        let count: u32 = terms.iter().map(|term| lower.matches(term).count() as u32).sum();
        if count > best {
            best = count;
            best_line = index;
        }
    }
    for term in terms {
        let in_path = path.contains(term);
        let count: u32 = doc.lines.iter().map(|line| line.to_lowercase().matches(term).count() as u32).sum();
        if !in_path && count == 0 {
            return None;
        }
        if in_path {
            score += 20;
        }
        score += count.min(10) * 2;
    }
    // A heading is more likely to be the doc's subject than a line in its middle.
    if doc.lines.get(best_line).is_some_and(|line| line.trim_start().starts_with('#')) {
        score += 5;
    }
    if best == 0 {
        best_line = 0;
    }
    let snippet = doc.lines.get(best_line).map_or_else(|| doc.path.clone(), |line| snippet(line));
    Some(Hit { path: doc.path.clone(), line: best_line + 1, snippet, score })
}

fn snippet(line: &str) -> String {
    let trimmed = line.trim();
    if trimmed.chars().count() <= SNIPPET {
        return trimmed.to_owned();
    }
    let mut text: String = trimmed.chars().take(SNIPPET).collect();
    text.push('…');
    text
}

/// Lines `start..=end` of `path`, with their numbers, bounded in both lines and characters.
pub fn read(index: &Index, path: &str, start: Option<usize>, end: Option<usize>) -> Option<Excerpt> {
    let doc = index.docs.iter().find(|doc| doc.path == path)?;
    let total = doc.lines.len();
    let start = start.unwrap_or(1).clamp(1, total.max(1));
    let end = end.unwrap_or(start + READ_LINES - 1).clamp(start, total.max(start)).min(start + READ_LINES - 1);
    let mut text = String::new();
    let mut actual = start;
    for (offset, line) in doc.lines.iter().skip(start - 1).take(end - start + 1).enumerate() {
        actual = start + offset;
        let _ = writeln!(text, "{actual}: {line}");
        if text.len() >= READ_CHARS {
            break;
        }
    }
    Some(Excerpt { path: doc.path.clone(), start, end: actual, total, text })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index() -> Index {
        Index {
            root: "/repo".into(),
            built_at: 0,
            truncated: false,
            docs: vec![
                Doc {
                    path: "docs/authentication.md".into(),
                    lines: vec![
                        "# Authentication".into(),
                        "Every request needs a token.".into(),
                        "The token is never logged.".into(),
                    ],
                },
                Doc { path: "src/auth.rs".into(), lines: vec!["pub fn token() {}".into()] },
                Doc { path: "README.md".into(), lines: vec!["Run the token command.".into()] },
            ],
        }
    }

    #[test]
    fn every_term_must_appear_and_a_path_match_ranks_first() {
        let hits = search(&index(), &terms("authentication token"), true, 5);
        assert_eq!(hits[0].path, "docs/authentication.md");
        assert_eq!(hits[0].line, 2);
        assert!(hits.iter().all(|hit| is_docs(&hit.path)));
        assert!(search(&index(), &terms("token missing"), false, 5).is_empty());
    }

    #[test]
    fn a_code_search_reads_source_and_a_read_is_numbered() {
        let hits = search(&index(), &terms("token"), false, 5);
        assert!(hits.iter().any(|hit| hit.path == "src/auth.rs"));
        let excerpt = read(&index(), "docs/authentication.md", Some(2), Some(3)).expect("an excerpt");
        assert_eq!(excerpt.text, "2: Every request needs a token.\n3: The token is never logged.\n");
        assert_eq!((excerpt.start, excerpt.end, excerpt.total), (2, 3, 3));
        assert_eq!(read(&index(), "docs/authentication.md", None, None).expect("all").start, 1);
        assert!(read(&index(), "../secret", None, None).is_none());
    }

    #[test]
    fn a_query_is_words_and_punctuation_is_not_one() {
        assert_eq!(terms("Token, token: auth-token!"), ["token", "auth", "token"]);
        assert_eq!(terms("   "), Vec::<String>::new());
    }
}
