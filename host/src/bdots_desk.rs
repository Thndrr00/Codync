#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Card text helpers ported from bdots' `agents/desk.ts` and `lib/shop.ts`, `lib/auth.ts`,
//! `agents/skill-sources.ts`, `lib/models.ts`. Pure formatting/validation for chat cards.

/// The first plain sentence or line of a reply, as a card title (≤110 chars, `titleOf`).
pub fn title_of(markdown: &str) -> String {
    let line = markdown
        .lines()
        .map(str::trim)
        .find(|l| {
            !l.is_empty() && !l.starts_with('#') && !l.starts_with("```") && !l.starts_with('|') && !l.starts_with('-')
        })
        .unwrap_or("");
    let mut clean = String::new();
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '!' && i + 1 < chars.len() && chars[i + 1] == '[' {
            i += 1;
            continue;
        }
        if matches!(chars[i], '*' | '_' | '`' | '[' | ']' | '(' | ')') {
            i += 1;
            continue;
        }
        clean.push(chars[i]);
        i += 1;
    }
    let sentence = clean.split(". ").next().unwrap_or(&clean).trim();
    let mut out: String = sentence.chars().take(110).collect();
    if sentence.chars().count() > 110 {
        out = format!("{}…", out.trim_end());
    }
    out
}

/// Remove the sentence that became the title so the card body isn't repetitive (`withoutTitle`).
pub fn without_title(markdown: &str, title: &str) -> String {
    let body = markdown.strip_prefix(title).unwrap_or(markdown).trim();
    body.trim_start_matches(|c: char| c == '.' || c == '\n' || c == ' ' || c == '\r').to_owned()
}

/// Drop trailing markdown links, shown as chips instead (`withoutTrailingLinks`).
pub fn without_trailing_links(markdown: &str) -> String {
    let mut out = markdown.trim_end().to_owned();
    while let Some(start) = out.rfind('[') {
        let Some(mid) = out[start..].find("](") else { break };
        let close = start + mid + 2;
        let Some(end) = out[close..].find(')') else { break };
        if out[close + end + 1..].trim().is_empty() {
            out = out[..start].trim_end().to_owned();
        } else {
            break;
        }
    }
    out
}

/// First N distinct HTTP(S) markdown links, with clipped labels (`linksIn`).
pub fn links_in(markdown: &str, max: usize) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let mut rest = markdown;
    while let Some(open) = rest.find('[') {
        let after = &rest[open + 1..];
        let Some(mid) = after.find("](") else { break };
        let label = &after[..mid];
        let tail = &after[mid + 2..];
        let Some(end) = tail.find(')') else { break };
        let url = &tail[..end];
        if url.starts_with("http://") || url.starts_with("https://") {
            let entry = (label.chars().take(60).collect::<String>(), url.to_owned());
            if !out.iter().any(|(_, u)| u == url) {
                out.push(entry);
                if out.len() >= max {
                    break;
                }
            }
        }
        rest = &tail[end + 1..];
    }
    out
}

/// Whole percent off a list price, same currency only (`discountPercent`).
pub fn discount_percent(price: f64, list_price: Option<f64>) -> Option<i64> {
    let list = list_price?;
    if list <= 0.0 || price > list {
        return None;
    }
    Some(((1.0 - price / list) * 100.0).round() as i64)
}

/// "12k" for large review counts (`compactCount`).
pub fn compact_count(n: u64) -> String {
    if n < 1000 {
        return n.to_string();
    }
    if n < 1_000_000 {
        let k = n / 1000;
        return format!("{k}k");
    }
    format!("{}m", n / 1_000_000)
}

/// Email → slug user id: lowercase, non-alphanumeric runs become `-` (`userIdForEmail`).
pub fn user_id_for_email(email: &str) -> String {
    let lower = email.to_ascii_lowercase();
    let mut out = String::new();
    let mut dash = false;
    for c in lower.chars() {
        if c.is_ascii_alphanumeric() {
            if dash && !out.is_empty() {
                out.push('-');
            }
            out.push(c);
            dash = false;
        } else {
            dash = true;
        }
    }
    out.trim_matches('-').to_owned()
}

/// A skill resource path → its kind (`resourceKind`).
pub fn resource_kind(path: &str) -> &'static str {
    let lower = path.to_ascii_lowercase();
    if lower.contains("/scripts/") || lower.ends_with(".sh") || lower.ends_with(".py") || lower.ends_with(".js") {
        "script"
    } else if lower.contains("/references/") || lower.ends_with(".md") {
        "reference"
    } else if lower.contains("/assets/") {
        "asset"
    } else {
        "file"
    }
}

/// `@cf/x`, `provider/model`, `dynamic/route`: a model id shape (`isModelId`).
pub fn is_model_id(id: &str) -> bool {
    let parts: Vec<&str> = id.split('/').collect();
    if parts.len() < 2 || parts.len() > 3 {
        return false;
    }
    id.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '-' | '_' | '.' | ':'))
}

/// Provider id from a model id, or `None` (`providerOf`).
pub fn provider_of(id: &str) -> Option<&str> {
    if !is_model_id(id) {
        return None;
    }
    id.split('/').next()
}

/// "12k" reviews, percent off, slugs and resource kinds all stay deterministic.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_titles_and_links() {
        assert_eq!(title_of("# Heading\n**Revenue** grew 12%. More."), "Revenue grew 12%");
        assert_eq!(without_title("Hello world. Rest of it", "Hello world"), "Rest of it");
        assert_eq!(without_trailing_links("See docs [read](https://x.test)"), "See docs");
        assert_eq!(links_in("See [docs](https://x.test) and [api](https://y.test)", 3).len(), 2);
    }

    #[test]
    fn money_counts_and_ids() {
        assert_eq!(discount_percent(75.0, Some(100.0)), Some(25));
        assert_eq!(discount_percent(120.0, Some(100.0)), None);
        assert_eq!(compact_count(12_345), "12k");
        assert_eq!(compact_count(2_500_000), "2m");
        assert_eq!(user_id_for_email("Alice.Smith+tag@Example.com"), "alice-smith-tag-example-com");
    }

    #[test]
    fn skill_paths_and_model_ids() {
        assert_eq!(resource_kind("scripts/run.py"), "script");
        assert_eq!(resource_kind("references/api.md"), "reference");
        assert_eq!(resource_kind("assets/logo.png"), "asset");
        assert!(is_model_id("anthropic/claude-3"));
        assert_eq!(provider_of("anthropic/claude-3"), Some("anthropic"));
        assert!(!is_model_id("not a model!"));
    }
}
