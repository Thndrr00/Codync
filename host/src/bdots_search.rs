#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from agentes-chat 0.7.5's search helpers (`packages/ui/src/search.ts`,
//! `packages/adapters/src/search.ts`, `packages/client/src/lib/search-filters.ts`): query
//! normalisation, filters, matching and relevance. Composes helpers already ported into this host.
//! Only `botInsights` calls this.

use serde_json::Value;

/// Normalise a query for matching: trimmed, lower-case, whitespace collapsed.
pub fn normalize_query(query: &str) -> String {
    query.split_whitespace().collect::<Vec<_>>().join(" ").to_ascii_lowercase()
}

/// Whether a query has any searchable term.
pub fn is_empty_query(query: &str) -> bool {
    normalize_query(query).is_empty()
}

/// Terms of a query, in order and without repeats.
pub fn query_terms(query: &str) -> Vec<String> {
    let mut out = Vec::new();
    for term in normalize_query(query).split(' ') {
        if !term.is_empty() && !out.iter().any(|t| t == term) {
            out.push(term.to_owned());
        }
    }
    out
}

/// Whether a text matches every query term.
pub fn matches_query(text: &str, terms: &[String]) -> bool {
    let lower = text.to_ascii_lowercase();
    terms.iter().all(|term| lower.contains(term))
}

/// Relevance of a text to a query, as a whole percent from matched terms.
pub fn relevance_score(text: &str, terms: &[String]) -> u32 {
    if terms.is_empty() {
        return 0;
    }
    let lower = text.to_ascii_lowercase();
    let hits = terms.iter().filter(|term| lower.contains(*term)).count();
    crate::bdots_activity::percent(hits, terms.len())
}

/// Rows whose text field matches every term (search).
pub fn filter_rows<'a>(rows: &'a [Value], terms: &[String], key: &str) -> Vec<&'a Value> {
    rows.iter().filter(|row| matches_query(row[key].as_str().unwrap_or(""), terms)).collect()
}

/// Mark the first occurrence of a term with bold Markdown (highlight).
pub fn highlight(text: &str, term: &str) -> String {
    if term.is_empty() {
        return text.to_owned();
    }
    let lower = text.to_ascii_lowercase();
    let needle = term.to_ascii_lowercase();
    let Some(at) = lower.find(&needle) else {
        return text.to_owned();
    };
    format!("{}**{}**{}", &text[..at], &text[at..at + needle.len()], &text[at + needle.len()..])
}

/// Values of one filter key, from `key:value` entries.
pub fn filter_values<'a>(filters: &'a [String], key: &str) -> Vec<&'a str> {
    filters
        .iter()
        .filter_map(|entry| entry.split_once(':'))
        .filter(|(k, _)| k.eq_ignore_ascii_case(key))
        .map(|(_, v)| v.trim())
        .filter(|v| !v.is_empty())
        .collect()
}

/// Rows left after applying a `kind:value` filter.
pub fn apply_kind_filter(rows: &[Value], filters: &[String]) -> Vec<&Value> {
    let kinds = filter_values(filters, "kind");
    if kinds.is_empty() {
        return rows.iter().collect();
    }
    rows.iter()
        .filter(|row| {
            let kind = row["kind"].as_str().unwrap_or("").to_ascii_lowercase();
            kinds.iter().any(|k| kind == k.to_ascii_lowercase())
        })
        .collect()
}

/// Count label for hits, reusing the shared formatter.
pub fn hit_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// SF Symbol for a search result state.
pub fn search_icon(empty: bool) -> &'static str {
    if empty { "magnifyingglass" } else { "checkmark.circle.fill" }
}

/// Semantic tone for a search result state.
pub fn search_tone(empty: bool) -> &'static str {
    if empty { "secondary" } else { "good" }
}

/// Summary rows for the card.
pub fn summary(hits: usize, filtered: usize, score: u32) -> Vec<(&'static str, String)> {
    vec![
        ("hits", hit_label(hits)),
        ("filtered", hit_label(filtered)),
        ("score", format!("{score}%")),
        ("state", if hits == 0 { "empty" } else { "found" }.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn rows() -> Vec<Value> {
        vec![
            json!({"title":"Hello world","kind":"chat"}),
            json!({"title":"Another world","kind":"doc"}),
            json!({"title":"Nothing here","kind":"chat"}),
        ]
    }

    #[test]
    fn queries() {
        assert_eq!(normalize_query("  Hello   WORLD "), "hello world");
        assert!(is_empty_query("   "));
        assert_eq!(query_terms("a b a"), vec!["a", "b"]);
        let terms = query_terms("hello world");
        assert!(matches_query("Hello WORLD", &terms));
        assert!(!matches_query("Hello", &terms));
    }

    #[test]
    fn relevance_and_filtering() {
        let terms = query_terms("hello world");
        assert_eq!(relevance_score("hello world", &terms), 100);
        assert_eq!(relevance_score("hello", &terms), 50);
        assert_eq!(relevance_score("anything", &[]), 0);
        let all = rows();
        assert_eq!(filter_rows(&all, &terms, "title").len(), 1);
        assert_eq!(apply_kind_filter(&all, &["kind:chat".to_owned()]).len(), 2);
        assert_eq!(apply_kind_filter(&all, &[]).len(), 3);
    }

    #[test]
    fn highlights_and_filters() {
        assert_eq!(highlight("Hello world", "world"), "Hello **world**");
        assert_eq!(highlight("Hello", "missing"), "Hello");
        assert_eq!(filter_values(&["kind:chat".to_owned(), "kind:doc".to_owned()], "kind"), vec!["chat", "doc"]);
    }

    #[test]
    fn labels_and_summary() {
        assert_eq!(hit_label(1500), "1.5K");
        assert_eq!(search_icon(true), "magnifyingglass");
        assert_eq!(search_tone(false), "good");
        assert_eq!(summary(2, 1, 50).len(), 4);
    }
}
