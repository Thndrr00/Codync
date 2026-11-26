#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from rakazo's `messaging-commands.ts` and `command-palette.tsx`: owner
//! text commands, palette search and result ranking. Composes helpers already ported into this
//! host. Only `botInsights` calls this.

use serde_json::Value;

/// Owner commands accepted only in a verified 1:1 conversation.
pub const MESSAGING_COMMANDS: [&str; 4] = ["approve", "decline", "leave", "stop"];

/// Pages listed before anything is typed in the palette.
pub const RECENT_PAGES: usize = 5;

/// Stop words in the app's languages (subset of the full set, ASCII-folded).
pub const STOP_WORDS: [&str; 12] = [
    "STOP",
    "STOP NOW",
    "STOPP",
    "STOPP JETZT",
    "DETENTE",
    "STOP NU",
    "ZATRZYMAJ",
    "ARRÊTE",
    "PARE",
    "DUR",
    "ARRANCA",
    "ALTO",
];

/// Parse a normalized owner command from a message (parseMessagingCommand).
pub fn parse_messaging_command(text: &str) -> Option<&'static str> {
    let normalized = text.trim().to_ascii_uppercase();
    match normalized.as_str() {
        "YES" => Some("approve"),
        "NO" => Some("decline"),
        "LEAVE" => Some("leave"),
        _ if STOP_WORDS.contains(&normalized.as_str()) => Some("stop"),
        _ => None,
    }
}

/// Whether a message is an owner command at all.
pub fn is_command(text: &str) -> bool {
    parse_messaging_command(text).is_some()
}

/// Collapse whitespace as the palette does before matching.
pub fn needle_of(text: &str) -> String {
    text.trim().split_whitespace().collect::<Vec<_>>().join(" ").to_ascii_lowercase()
}

/// Searchable text for a bot row (botSearchText).
pub fn bot_search_text(name: &str, description: &str, preview: &str) -> String {
    format!("{name} {description} {preview}").to_ascii_lowercase()
}

/// Subtitle shown under a bot in the palette (botSubtitle).
pub fn bot_subtitle<'a>(description: &'a str, preview: &'a str) -> &'a str {
    let description = description.trim();
    if description.is_empty() { preview.trim() } else { description }
}

/// Filter bot rows by a palette needle.
pub fn filter_bots<'a>(rows: &'a [Value], needle: &str) -> Vec<&'a Value> {
    let needle = needle_of(needle);
    rows.iter()
        .filter(|row| {
            let text = bot_search_text(
                row["name"].as_str().unwrap_or(""),
                row["description"].as_str().unwrap_or(""),
                row["preview"].as_str().unwrap_or(""),
            );
            needle.is_empty() || text.contains(&needle)
        })
        .collect()
}

/// Filter action rows by their label.
pub fn filter_actions<'a>(rows: &'a [Value], needle: &str) -> Vec<&'a Value> {
    let needle = needle_of(needle);
    rows.iter()
        .filter(|row| needle.is_empty() || row["label"].as_str().unwrap_or("").to_ascii_lowercase().contains(&needle))
        .collect()
}

/// Rank a result: title prefix matches first, then substring matches.
pub fn palette_rank(title: &str, needle: &str) -> u8 {
    let title = title.to_ascii_lowercase();
    let needle = needle_of(needle);
    if needle.is_empty() {
        return 2;
    }
    if title.starts_with(&needle) {
        0
    } else if title.contains(&needle) {
        1
    } else {
        3
    }
}

/// Sort rows by prefix/substring rank, preserving stable order.
pub fn rank_rows<'a>(rows: &'a [Value], needle: &str, key: &str) -> Vec<&'a Value> {
    let mut ranked: Vec<(u8, &Value)> =
        rows.iter().map(|row| (palette_rank(row[key].as_str().unwrap_or(""), needle), row)).collect();
    ranked.sort_by_key(|(rank, _)| *rank);
    ranked.into_iter().map(|(_, row)| row).collect()
}

/// The first `max` recent pages, most recent first.
pub fn recent_pages<'a>(pages: &'a [Value], max: usize) -> Vec<&'a Value> {
    pages.iter().rev().take(max).collect()
}

/// Whether a palette result should show the page group first.
pub fn pages_lead(needle: &str, page_title: &str) -> bool {
    let needle = needle_of(needle);
    !needle.is_empty() && page_title.to_ascii_lowercase().starts_with(&needle)
}

/// Modifier label for the palette hint, reusing the shortcut helpers.
pub fn palette_modifier(apple: bool) -> &'static str {
    if apple { "⌘" } else { "Ctrl" }
}

/// Human label for a command class.
pub fn command_class_label(command: &str) -> &'static str {
    match command {
        "approve" => "Approve",
        "decline" => "Decline",
        "leave" => "Leave",
        "stop" => "Stop work",
        _ => "Message",
    }
}

/// SF Symbol for a command class.
pub fn command_class_icon(command: &str) -> &'static str {
    match command {
        "approve" => "checkmark.circle.fill",
        "decline" => "xmark.circle.fill",
        "leave" => "rectangle.portrait.and.arrow.right",
        "stop" => "stop.circle.fill",
        _ => "bubble.left",
    }
}

/// Semantic tone for a command class.
pub fn command_class_tone(command: &str) -> &'static str {
    match command {
        "approve" => "good",
        "decline" | "stop" => "attention",
        "leave" => "secondary",
        _ => "accent",
    }
}

/// Display label for a palette section.
pub fn section_label(name: &str) -> String {
    crate::bdots_textfmt::upper_first(name.replace('_', " ").as_str())
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(commands: usize, bots: usize, actions: usize, pages: usize) -> Vec<(&'static str, String)> {
    vec![
        ("commands", count_label(commands)),
        ("bots", count_label(bots)),
        ("actions", count_label(actions)),
        ("pages", count_label(pages)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn messaging_commands() {
        assert_eq!(parse_messaging_command(" YES "), Some("approve"));
        assert_eq!(parse_messaging_command("no"), Some("decline"));
        assert_eq!(parse_messaging_command("LEAVE"), Some("leave"));
        assert_eq!(parse_messaging_command("stop now"), Some("stop"));
        assert_eq!(parse_messaging_command("PARE"), Some("stop"));
        assert!(parse_messaging_command("hello").is_none());
        assert!(is_command(" YES "));
        assert!(!is_command("maybe"));
    }

    #[test]
    fn search_and_filtering() {
        assert_eq!(needle_of("  Hello   World "), "hello world");
        let text = bot_search_text("Writer", "drafts posts", "last draft");
        assert!(text.contains("drafts posts"));
        assert_eq!(bot_subtitle("  ", " preview "), "preview");
        let bots = vec![
            json!({"name":"Writer","description":"drafts","preview":""}),
            json!({"name":"Reviewer","description":"checks","preview":""}),
        ];
        assert_eq!(filter_bots(&bots, "drafts").len(), 1);
        assert_eq!(filter_bots(&bots, "").len(), 2);
        let actions = vec![json!({"label":"New bot"}), json!({"label":"Settings"})];
        assert_eq!(filter_actions(&actions, "new").len(), 1);
    }

    #[test]
    fn ranking_and_pages() {
        assert_eq!(palette_rank("Writer", "wri"), 0);
        assert_eq!(palette_rank("Super Writer", "wri"), 1);
        assert_eq!(palette_rank("Other", "wri"), 3);
        assert_eq!(palette_rank("Writer", ""), 2);
        let rows = vec![json!({"title":"Other"}), json!({"title":"Writer"})];
        let ranked = rank_rows(&rows, "wri", "title");
        assert_eq!(ranked[0]["title"], "Writer");
        let pages = vec![json!({"title":"a"}), json!({"title":"b"})];
        assert_eq!(recent_pages(&pages, 1)[0]["title"], "b");
        assert!(pages_lead("wri", "writer notes"));
        assert!(!pages_lead("", "writer notes"));
    }

    #[test]
    fn presentation() {
        assert_eq!(palette_modifier(true), "⌘");
        assert_eq!(palette_modifier(false), "Ctrl");
        assert_eq!(command_class_label("stop"), "Stop work");
        assert_eq!(command_class_icon("approve"), "checkmark.circle.fill");
        assert_eq!(command_class_tone("decline"), "attention");
        assert_eq!(section_label("recent_pages"), "Recent pages");
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(4, 2, 3, 1).len(), 4);
    }
}
