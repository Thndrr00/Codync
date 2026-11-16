//! A bot tool that reads one web page as plain text: the title and the readable body, with scripts,
//! styles and markup removed. The page itself never renders here and nothing is saved.

use crate::hub::Hub;
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::time::Duration;

pub const TOOLS: &[&str] = &["clip_page"];
const TIMEOUT: Duration = Duration::from_secs(20);
const MAX_BYTES: usize = 5 * 1024 * 1024;
const MAX_CHARS: usize = 20_000;

pub fn tools() -> Value {
    json!([{
        "name": "clip_page",
        "description": "Read a web page as plain text: its title and readable body, with scripts, styles and navigation removed. Use it when the user shares a link and you need what it says, instead of guessing from the URL.",
        "inputSchema": {"type": "object", "properties": {
            "url": {"type": "string", "description": "The http(s) address to read."},
            "maxChars": {"type": "integer", "description": "Longest text to return, 500 to 20000. Default 8000."},
        }, "required": ["url"]},
        "annotations": {"readOnlyHint": true, "idempotentHint": true},
    }])
}

pub async fn call(_hub: &Hub, name: &str, args: &Value) -> Result<Value> {
    ensure!(name == "clip_page", "unknown clip tool: {name}");
    let url = args["url"].as_str().unwrap_or_default().trim();
    ensure!(url.starts_with("https://") || url.starts_with("http://"), "clip_page needs an http(s) URL");
    let limit =
        usize::try_from(args["maxChars"].as_u64().unwrap_or(8_000).clamp(500, MAX_CHARS as u64)).unwrap_or(8_000);
    let response = crate::http().get(url).timeout(TIMEOUT).send().await?;
    ensure!(response.status().is_success(), "{url} answered {}", response.status());
    let html = response.text().await?;
    ensure!(html.len() <= MAX_BYTES, "that page is larger than 5 MiB");
    let (title, text) = readable(&html, limit);
    Ok(json!({"url": url, "title": title, "text": text}))
}

/// The page title and its readable text, capped at `limit` characters.
pub fn readable(html: &str, limit: usize) -> (String, String) {
    let title = between(html, "<title", "</title>").map(|t| text_of(&t)).unwrap_or_default();
    let mut body = html.to_owned();
    for (open, close) in [
        ("<script", "</script>"),
        ("<style", "</style>"),
        ("<noscript", "</noscript>"),
        ("<svg", "</svg>"),
        ("<nav", "</nav>"),
        ("<header", "</header>"),
        ("<footer", "</footer>"),
        ("<form", "</form>"),
    ] {
        body = without_blocks(&body, open, close);
    }
    let mut text = text_of(&body);
    if text.len() > limit {
        let cut = text.char_indices().nth(limit).map_or(text.len(), |(at, _)| at);
        text.truncate(cut);
    }
    (title, text)
}

fn between(html: &str, open: &str, close: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let start = lower.find(open)?;
    let start = lower[start..].find('>').map(|at| start + at + 1)?;
    let end = lower[start..].find(close).map(|at| start + at)?;
    Some(html[start..end].to_owned())
}

fn without_blocks(html: &str, open: &str, close: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    let lower_open = open.to_ascii_lowercase();
    let lower_close = close.to_ascii_lowercase();
    loop {
        let lower = rest.to_ascii_lowercase();
        let Some(start) = lower.find(&lower_open) else {
            out.push_str(rest);
            break;
        };
        out.push_str(&rest[..start]);
        let Some(end) = lower[start..].find(&lower_close) else { break };
        rest = &rest[start + end + lower_close.len()..];
    }
    out
}

/// Tags become spaces, entities are decoded, and runs of whitespace collapse.
fn text_of(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => {
                in_tag = false;
                out.push(' ');
            }
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    let decoded = decode_entities(&out);
    decoded.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn decode_entities(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        let tail = &rest[at..];
        let Some(end) = tail.find(';').filter(|end| *end <= 10) else {
            out.push('&');
            rest = &tail[1..];
            continue;
        };
        let name = &tail[1..end];
        let decoded = match name {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" | "#39" => Some('\''),
            "nbsp" => Some(' '),
            _ if name.starts_with("#x") || name.starts_with("#X") => {
                u32::from_str_radix(&name[2..], 16).ok().and_then(char::from_u32)
            }
            _ if name.starts_with('#') => name[1..].parse::<u32>().ok().and_then(char::from_u32),
            _ => None,
        };
        if let Some(c) = decoded {
            out.push(c);
            rest = &tail[end + 1..];
        } else {
            out.push('&');
            rest = &tail[1..];
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_page_becomes_its_title_and_readable_text() {
        let html = r"<html><head><title>Codync &amp; you</title><style>body{color:red}</style></head>
            <body><nav>Home | Docs</nav><h1>Hello</h1><p>One &lt;two&gt; three.</p>
            <script>alert('no')</script><footer>Copyright</footer></body></html>";
        let (title, text) = readable(html, 1_000);
        assert_eq!(title, "Codync & you");
        assert!(text.contains("Hello One <two> three."), "{text}");
        assert!(!text.contains("color") && !text.contains("alert") && !text.contains("Copyright"));
    }

    #[test]
    fn the_text_stops_at_the_character_limit_without_splitting_one() {
        let (_, text) = readable("<p>abcdefghij</p>", 4);
        assert_eq!(text, "abcd");
    }

    #[test]
    fn entities_and_unknown_ones_are_handled() {
        assert_eq!(decode_entities("a &amp; b &#39;c&#x27; &unknown;"), "a & b 'c' &unknown;");
    }
}
