#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! More pure helpers ported from bdots: memory facts (`agents/memory.ts`), choice options and
//! endpoint comparison (`lib/choices.ts`), and required-token checks (`agents/guards.ts`).
//! Deterministic, dependency-free; reuses Codync's types where one already exists.

use serde_json::Value;

/// Rough ceiling for one memory document, in characters (bdots: 8_000, about 2k tokens).
pub const MAX_MEMORY_CHARS: usize = 8_000;

fn normalize(line: &str) -> String {
    let line = line.trim().trim_start_matches(|c| c == '-' || c == '*');
    let line = line.split_whitespace().collect::<Vec<_>>().join(" ");
    line.trim_end_matches(|c| c == '.' || c == '!').trim().to_lowercase()
}

/// Append one fact as a bullet unless the document already says it (`appendFact`).
#[derive(Debug)]
pub enum AppendFact {
    Content(String),
    Duplicate,
    Error(&'static str),
}

pub fn append_fact(doc: &str, fact: &str) -> AppendFact {
    let clean = fact.split_whitespace().collect::<Vec<_>>().join(" ");
    if clean.is_empty() {
        return AppendFact::Error("The fact is empty.");
    }
    let wanted = normalize(&clean);
    if doc.lines().any(|line| normalize(line) == wanted) {
        return AppendFact::Duplicate;
    }
    let content = format!("{}{}- {}", doc.trim_end(), if doc.trim().is_empty() { "" } else { "\n" }, clean);
    if content.chars().count() > MAX_MEMORY_CHARS {
        return AppendFact::Error("Memory is full; rewrite it shorter before saving this fact.");
    }
    AppendFact::Content(content)
}

/// A tappable choice option; older calls passed plain strings, newer ones `{ label, detail }`
/// (`choiceOptions`). `None` detail is kept as absent.
pub fn choice_options(input: &Value) -> Vec<(String, Option<String>)> {
    let Some(list) = input.as_array() else { return vec![] };
    list.iter()
        .filter_map(|option| match option {
            Value::String(s) if !s.trim().is_empty() => Some((s.trim().to_owned(), None)),
            Value::Object(map) => {
                let label = map.get("label").and_then(Value::as_str).unwrap_or("").trim();
                if label.is_empty() {
                    return None;
                }
                let detail = map.get("detail").and_then(Value::as_str).map(str::trim).filter(|d| !d.is_empty());
                Some((label.to_owned(), detail.map(str::to_owned)))
            }
            _ => None,
        })
        .collect()
}

/// Two MCP endpoints are the same connection, whatever the trailing slash (`sameEndpoint`).
pub fn same_endpoint(a: &str, b: &str) -> bool {
    let norm = |value: &str| -> String { value.trim().trim_end_matches('/').to_owned() };
    norm(a) == norm(b)
}

/// Which required tokens are missing from a piece of text (`guards.ts` `requiredTokens`). A token
/// is a `[name]` placeholder; an optional one ends with `?`.
pub fn missing_tokens(text: &str, have: &[&str]) -> Vec<String> {
    let mut missing = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '['
            && let Some(end) = chars[i + 1..].iter().position(|c| *c == ']')
        {
            let raw: String = chars[i + 1..i + 1 + end].iter().collect();
            let token = raw.trim_end_matches('?');
            let optional = raw.ends_with('?');
            if !token.is_empty() && !optional && !have.contains(&token) && !missing.iter().any(|m| m == token) {
                missing.push(token.to_owned());
            }
            i += end + 2;
            continue;
        }
        i += 1;
    }
    missing
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_fact_is_appended_once() {
        let doc = "- Uses pnpm.";
        match append_fact(doc, "uses   PNPM!") {
            AppendFact::Duplicate => {}
            other => panic!("expected duplicate, got {other:?}"),
        }
        match append_fact(doc, "Lives in Madrid") {
            AppendFact::Content(content) => assert_eq!(content, "- Uses pnpm.\n- Lives in Madrid"),
            other => panic!("expected content, got {other:?}"),
        }
        match append_fact("", "") {
            AppendFact::Error(_) => {}
            other => panic!("expected error, got {other:?}"),
        }
    }

    #[test]
    fn choice_options_read_strings_and_objects() {
        let options = choice_options(&json!(["Yes", {"label": "No", "detail": "stop"}, "", {"detail": "no label"}]));
        assert_eq!(options, vec![("Yes".into(), None), ("No".into(), Some("stop".into()))]);
    }

    #[test]
    fn endpoints_ignore_trailing_slashes() {
        assert!(same_endpoint("https://api.test/v1/", "https://api.test/v1"));
        assert!(!same_endpoint("https://api.test/v1", "https://api.test/v2"));
    }

    #[test]
    fn required_tokens_ignore_optional_and_known() {
        assert_eq!(missing_tokens("Hi [name], your [code] and [maybe?]", &["name"]), vec!["code"]);
        assert!(missing_tokens("all [done]", &["done"]).is_empty());
    }
}
