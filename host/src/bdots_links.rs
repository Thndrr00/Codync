#![allow(dead_code)]

//! Bot-mention links ported from bdots' `lib/bot-links.ts`, using the same shape
//! (`[@Name](codync://bot/<id>)`) so stored messages stay readable by both.

use serde_json::Value;

pub const BOT_LINK_PREFIX: &str = "codync://bot/";

pub fn bot_link(bot_id: &str) -> String {
    format!("{BOT_LINK_PREFIX}{bot_id}")
}

fn markdown_label(name: &str) -> String {
    name.chars()
        .filter(|c| !matches!(c, '[' | ']' | '\\'))
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn bot_mention(name: &str, bot_id: &str) -> String {
    format!("[@{}]({})", markdown_label(name), bot_link(bot_id))
}

/// The bot id in a link, or `None` for anything else. Ids are short lowercase alphanumerics.
pub fn bot_id_from_link(value: Option<&str>) -> Option<String> {
    let trimmed = value.unwrap_or("").trim();
    let rest = trimmed.strip_prefix(BOT_LINK_PREFIX)?;
    let id = rest.trim_end_matches('/');
    if (1..=64).contains(&id.len()) && id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
        Some(id.to_owned())
    } else {
        None
    }
}

fn is_boundary(c: char) -> bool {
    !(c.is_alphanumeric() || c == '_' || c == '@' || c == '[')
}

/// Turn `@Name` into a bot link for each named bot. Only one `@` per mention, emails stay as-is,
/// longest names win, and duplicate names keep the first bot in the list (`linkMentions`).
pub fn link_mentions(text: &str, bots: &[(String, String)]) -> String {
    let mut seen = Vec::new();
    let mut named: Vec<&(String, String)> = bots
        .iter()
        .filter(|(id, name)| {
            let key = name.trim().to_lowercase();
            if key.is_empty() || seen.contains(&key) || !bot_id_from_link(Some(&bot_link(id))).is_some_and(|b| b == *id)
            {
                return false;
            }
            seen.push(key);
            true
        })
        .collect();
    named.sort_by_key(|(_, name)| std::cmp::Reverse(name.trim().len()));
    let mut out = text.to_owned();
    for (id, name) in named {
        let name = name.trim();
        let mut result = String::new();
        let chars: Vec<char> = out.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if chars[i] == '@'
                && (i == 0 || is_boundary(chars[i - 1]))
                && chars[i + 1..].iter().take(name.chars().count()).copied().collect::<String>() == name
            {
                let after = i + 1 + name.chars().count();
                if after >= chars.len() || !chars[after].is_alphanumeric() {
                    result.push_str(&bot_mention(name, id));
                    i = after;
                    continue;
                }
            }
            result.push(chars[i]);
            i += 1;
        }
        out = result;
    }
    out
}

/// Bot links as "@Name" for one-line previews (`plainMentions`).
pub fn plain_mentions(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    loop {
        let Some(open) = rest.find("[@") else {
            out.push_str(rest);
            break;
        };
        out.push_str(&rest[..open]);
        let after = &rest[open + 2..];
        let Some(close) = after.find("](") else {
            out.push_str(&rest[open..]);
            break;
        };
        let label = &after[..close];
        let tail = &after[close + 2..];
        let Some(end) = tail.find(')') else {
            out.push_str(&rest[open..]);
            break;
        };
        let url = &tail[..end];
        if bot_id_from_link(Some(url)).is_some() {
            out.push('@');
            out.push_str(label);
        } else {
            out.push_str(&rest[open..open + 2 + close + 2 + end + 1]);
        }
        rest = &tail[end + 1..];
    }
    out
}

/// The `@` being typed at the caret, for the mention picker (`mentionQuery`).
pub fn mention_query(text: &str, caret: usize) -> Option<(usize, String)> {
    let before = if caret > text.len() { text } else { &text[..caret] };
    let at = before.rfind('@')?;
    if at > 0 && !is_boundary(before[..at].chars().last()?) {
        return None;
    }
    let query = &before[at + 1..];
    if query.len() > 40 || query.contains(['\n', '@', ']']) || query.starts_with(' ') {
        return None;
    }
    Some((at, query.to_owned()))
}

/// Extract labels from a mention card's options for a preview row.
pub fn option_labels(options: &Value) -> Vec<String> {
    options
        .as_array()
        .map(|a| a.iter().filter_map(|o| o["name"].as_str().map(str::to_owned)).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn links_and_mentions_round_trip() {
        let link = bot_mention("Sales Lead", "a1b2c3");
        assert_eq!(bot_id_from_link(Some(&link[8..link.len() - 1])), Some("a1b2c3".into()));
        assert_eq!(bot_id_from_link(Some("https://example.com")), None);
        assert_eq!(plain_mentions(&format!("Ask {link} now")), "Ask @Sales Lead now");
    }

    #[test]
    fn at_mentions_skip_emails_and_prefer_long_names() {
        let bots = vec![("sales".into(), "Sales".into()), ("lead".into(), "Sales Lead".into())];
        let out = link_mentions("Ask @Sales Lead and me@home.com", &bots);
        assert!(out.contains(bot_mention("Sales Lead", "lead").as_str()));
        assert!(out.contains("me@home.com"));
    }

    #[test]
    fn a_query_only_counts_at_a_word_boundary() {
        assert_eq!(mention_query("hi @sal", 7), Some((3, "sal".into())));
        assert_eq!(mention_query("me@mail", 7), None);
        assert_eq!(mention_query("hi @sal", 4), None); // starts with a space
        assert_eq!(option_labels(&json!([{"name":"Yes"},{"name":"No"}])), vec!["Yes", "No"]);
    }
}
