#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from agentes-chat 0.7.5's search code (`packages/ui/src/search.ts`,
//! `packages/adapters/src/search.ts`, `packages/client/src/lib/search-filters.ts`): query
//! normalisation, filters, matching, scoring and search summaries. Everything is deterministic,
//! takes no I/O and adds no crates; it reuses helpers already ported into this host instead of
//! duplicating them. Only `botInsights` calls this. A new module beside `bdots_search` (no edits).

use serde_json::Value;

/// Reciprocal-rank fusion constant: dampens the weight of top ranks (`RRF_K`).
pub const RRF_K: u32 = 60;

/// Search operators the box understands besides words (`SEARCH_OPS`).
pub const SEARCH_OPS: [&str; 9] = ["from", "in", "has", "mentions", "before", "after", "during", "author", "pinned"];

/// `has:` values (`SEARCH_HAS` / `MESSAGE_HAS`).
pub const SEARCH_HAS: [&str; 4] = ["link", "file", "image", "card"];

/// Sort orders (`SEARCH_SORTS`).
pub const SEARCH_SORTS: [&str; 3] = ["newest", "oldest", "relevance"];

/// Up to +10% recency bonus for recent items (`0.1` in agentes-chat).
pub const RECENCY_BONUS: f64 = 0.1;

/// Half-life of the recency bonus, in days (~30 days in agentes-chat).
pub const RECENCY_HALF_LIFE_DAYS: f64 = 30.0;

/// Recent searches kept, at most (`rememberSearch` default `max = 8`).
pub const RECENT_SEARCHES_MAX: usize = 8;

/// Lowercase and strip accents, what a search compares (`fold`, `norm`): NFD + drop combining marks.
pub fn fold(s: &str) -> String {
    let lowered = s.to_lowercase();
    let mut out = String::with_capacity(lowered.len());
    for c in lowered.chars() {
        if is_combining_mark(c) {
            continue;
        }
        out.push(fold_accented(c));
    }
    out
}

/// Whether every word of `query` is in `text`, ignoring case and accents (`matchesWords`).
pub fn matches_words(text: &str, query: &str) -> bool {
    let hay = fold(text);
    fold(query).split_whitespace().all(|word| hay.contains(word))
}

/// The words of a search, unquoted, for highlighting (`searchTerms`).
pub fn search_terms(text: &str) -> Vec<String> {
    text.split_whitespace().map(|w| w.replace('"', "")).filter(|w| !w.is_empty()).collect()
}

/// Full-width colon to ASCII, as CJK IMEs type it (`asciiColon`, `\u{FF1A}`).
pub fn ascii_colon(s: &str) -> String {
    s.replace('\u{FF1A}', ":")
}

/// The canonical search operator `word` names in English, if any (`opOf`, `lookup`).
pub fn search_operator(word: &str) -> Option<&'static str> {
    let v = fold(word);
    SEARCH_OPS.iter().copied().find(|op| fold(op) == v)
}

/// The sort when none was picked: relevance with words, newest first without (`searchSortFor`).
pub fn search_sort_for<'a>(sort: Option<&'a str>, text: &str) -> &'a str {
    match sort {
        Some(s) => s,
        None => {
            if text.trim().is_empty() {
                "newest"
            } else {
                "relevance"
            }
        }
    }
}

/// Whether there is anything to search: words or a chip (`hasSearch`).
pub fn has_search(chips_len: usize, text: &str) -> bool {
    chips_len > 0 || !text.trim().is_empty()
}

/// Recent searches: the newest first, each once, at most `max` (`rememberSearch`).
pub fn remember_search(recent: &[String], search: &str, max: usize) -> Vec<String> {
    let s = search.trim();
    if s.is_empty() {
        return recent.to_vec();
    }
    let mut out = Vec::with_capacity(recent.len() + 1);
    out.push(s.to_owned());
    out.extend(recent.iter().filter(|r| r.as_str() != s).cloned());
    out.truncate(max);
    out
}

/// One reciprocal-rank-fusion contribution for a 0-based rank in a ranking (`1 / (RRF_K + i + 1)`).
pub fn reciprocal_rank(rank_index: usize) -> f64 {
    1.0 / (RRF_K as f64 + rank_index as f64 + 1.0)
}

/// Apply the recency bonus to a fused score: up to +10%, half-life ~30 days (`1 + 0.1 * 0.5^(age/30)`).
pub fn recency_boost(score: f64, age_days: f64) -> f64 {
    score * (1.0 + RECENCY_BONUS * 0.5f64.powf(age_days / RECENCY_HALF_LIFE_DAYS))
}

/// Postgres vector literal `[x,y,…]`, non-finite numbers as `0` (`vectorLiteral`).
pub fn vector_literal(values: &[f64]) -> String {
    let mut out = String::from("[");
    for (i, x) in values.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        let x = if x.is_finite() { if *x == 0.0 { 0.0 } else { *x } } else { 0.0 };
        out.push_str(&format!("{x}"));
    }
    out.push(']');
    out
}

/// A search summary as JSON for the insights card / `botInsights`.
pub fn search_summary(query: &str, total: usize, returned: usize, score: f64) -> Value {
    let terms = search_terms(query).iter().map(|t| fold(t)).collect::<Vec<_>>();
    serde_json::json!({
        "query": crate::bdots_search::normalize_query(query),
        "terms": terms,
        "total": total,
        "returned": returned,
        "hasMore": returned < total,
        "empty": total == 0,
        "score": score,
        "totalLabel": crate::bdots_desk::compact_count(total as u64),
    })
}

/// Whether `c` is a Unicode combining mark stripped by NFD (`[\u{0300}-\u{036F}]`).
fn is_combining_mark(c: char) -> bool {
    ('\u{0300}'..='\u{036F}').contains(&c)
}

/// Fold a common accented Latin letter to its ASCII base (the NFD step of `fold`).
fn fold_accented(c: char) -> char {
    match c {
        'à'..='å' | 'ā'..='ą' => 'a',
        'ç' | 'ć'..='č' => 'c',
        'ď' | 'đ' => 'd',
        'è'..='ë' | 'ē'..='ę' => 'e',
        'ğ' | 'ĝ'..='ģ' => 'g',
        'ĥ' | 'ħ' => 'h',
        'ì'..='ï' | 'ī'..='į' => 'i',
        'ĵ' => 'j',
        'ķ' => 'k',
        'ĺ'..='ļ' => 'l',
        'ñ' | 'ń'..='ň' => 'n',
        'ò'..='ö' | 'ō'..='ő' => 'o',
        'ŕ'..='ř' => 'r',
        'ś'..='ş' => 's',
        'ť' | 'ţ' | 'þ' => 't',
        'ù'..='ü' | 'ū'..='ű' => 'u',
        'ý' | 'ÿ' | 'ŷ' => 'y',
        'ż'..='ź' | 'ž' => 'z',
        _ => c,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn folding_and_normalisation() {
        assert_eq!(fold("São Paulo"), "sao paulo");
        assert_eq!(fold("Café"), "cafe");
        assert_eq!(fold("e\u{0301}"), "e");
        assert_eq!(fold("Ñoño"), "nono");
    }

    #[test]
    fn matching_every_word() {
        assert!(matches_words("GLM 5.3 Flash", "glm flash"));
        assert!(matches_words("São Paulo", "sao paulo"));
        assert!(!matches_words("Hello", "hello world"));
        assert!(matches_words("anything", ""));
    }

    #[test]
    fn words_unquoted() {
        assert_eq!(search_terms("a \"b c\" d"), vec!["a", "b", "c", "d"]);
        assert_eq!(search_terms("  hello   \"world\" "), vec!["hello", "world"]);
        assert!(search_terms("").is_empty());
    }

    #[test]
    fn colons_and_operators() {
        assert_eq!(ascii_colon("from\u{FF1A}ana"), "from:ana");
        assert_eq!(search_operator("from"), Some("from"));
        assert_eq!(search_operator("Pinned"), Some("pinned"));
        assert_eq!(search_operator("FROM"), Some("from"));
        assert_eq!(search_operator("nope"), None);
        assert_eq!(SEARCH_OPS.len(), 9);
        assert!(SEARCH_OPS.contains(&"from"));
        assert!(SEARCH_OPS.contains(&"in"));
        assert!(SEARCH_OPS.contains(&"has"));
        assert!(SEARCH_HAS.contains(&"image"));
        assert!(SEARCH_SORTS.contains(&"relevance"));
    }

    #[test]
    fn sort_recent_and_presence() {
        assert_eq!(search_sort_for(None, "hi"), "relevance");
        assert_eq!(search_sort_for(None, "  "), "newest");
        assert_eq!(search_sort_for(Some("oldest"), "x"), "oldest");
        assert!(has_search(0, "hi"));
        assert!(has_search(1, ""));
        assert!(!has_search(0, "  "));
        let recent = vec!["a".to_owned(), "b".to_owned()];
        assert_eq!(remember_search(&recent, "c", RECENT_SEARCHES_MAX), vec!["c", "a", "b"]);
        assert_eq!(remember_search(&recent, "a", 8), vec!["a", "b"]);
        assert_eq!(remember_search(&recent, "  ", 8), recent);
        assert_eq!(remember_search(&recent, "c", 1), vec!["c"]);
    }

    #[test]
    fn scoring() {
        assert_eq!(RRF_K, 60);
        assert!((reciprocal_rank(0) - 1.0 / 61.0).abs() < 1e-12);
        assert!((reciprocal_rank(1) - 1.0 / 62.0).abs() < 1e-12);
        assert!((recency_boost(1.0, 0.0) - 1.1).abs() < 1e-12);
        assert!((recency_boost(1.0, RECENCY_HALF_LIFE_DAYS) - 1.05).abs() < 1e-12);
        assert_eq!(recency_boost(0.0, 5.0), 0.0);
        assert_eq!(RECENCY_HALF_LIFE_DAYS, 30.0);
        assert_eq!(RECENCY_BONUS, 0.1);
    }

    #[test]
    fn vector_literals() {
        assert_eq!(vector_literal(&[1.0, 2.5, -3.0]), "[1,2.5,-3]");
        assert_eq!(vector_literal(&[]), "[]");
        assert_eq!(vector_literal(&[f64::NAN, 1.0, f64::INFINITY, f64::NEG_INFINITY]), "[0,1,0,0]");
    }

    #[test]
    fn summaries() {
        let v = search_summary("Hello world", 10, 3, 50.0);
        assert_eq!(v["query"], json!("hello world"));
        assert_eq!(v["terms"], json!(["hello", "world"]));
        assert_eq!(v["total"], json!(10));
        assert_eq!(v["returned"], json!(3));
        assert_eq!(v["hasMore"], json!(true));
        assert_eq!(v["empty"], json!(false));
        assert_eq!(v["score"], json!(50.0));
        assert_eq!(v["totalLabel"], json!("10"));
        let empty = search_summary("", 0, 0, 0.0);
        assert_eq!(empty["empty"], json!(true));
        assert_eq!(empty["hasMore"], json!(false));
        assert_eq!(empty["terms"], json!([]));
    }
}
