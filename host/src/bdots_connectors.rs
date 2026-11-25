#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers for a bot's connectors: classify the accounts it can reach and describe the
//! mix. Reuses `bdots_library`, `bdots_more` and `bdots_activity` instead of duplicating them.
//! Only `botInsights` calls this.

/// One connector reduced to what the card shows.
#[derive(Clone, Debug, PartialEq)]
pub struct ConnectorLine {
    pub name: String,
    pub category: &'static str,
    pub icon: &'static str,
    pub tone: &'static str,
    pub detail: String,
}

/// Category for a connector id, from its name.
pub fn category(name: &str) -> &'static str {
    let lower = name.to_ascii_lowercase();
    if ["slack", "telegram", "whatsapp", "discord", "signal", "teams"].iter().any(|c| lower.contains(c)) {
        "chat"
    } else if ["gmail", "mail", "outlook", "email"].iter().any(|c| lower.contains(c)) {
        "mail"
    } else if ["github", "gitlab", "linear", "jira", "bitbucket"].iter().any(|c| lower.contains(c)) {
        "code"
    } else if ["drive", "notion", "dropbox", "docs", "sheets"].iter().any(|c| lower.contains(c)) {
        "files"
    } else if ["stripe", "shopify", "paypal", "square"].iter().any(|c| lower.contains(c)) {
        "money"
    } else if ["browser", "web", "http"].iter().any(|c| lower.contains(c)) {
        "web"
    } else {
        "other"
    }
}

/// SF Symbol for a connector category, shared by the four clients.
pub fn category_icon(category_name: &str) -> &'static str {
    match category_name {
        "chat" => "bubble.left.and.bubble.right",
        "mail" => "envelope",
        "code" => "chevron.left.forwardslash.chevron.right",
        "files" => "folder",
        "money" => "creditcard",
        "web" => "globe",
        _ => "plugs",
    }
}

/// Semantic tone for a connector category.
pub fn category_tone(category_name: &str) -> &'static str {
    match category_name {
        "chat" => "accent",
        "mail" => "good",
        "code" => "warn",
        "money" => "attention",
        "files" | "web" => "secondary",
        _ => "tertiary",
    }
}

/// Human title for a connector id.
pub fn connector_title(name: &str) -> String {
    let cleaned = name.trim().trim_start_matches("connector:").replace(['-', '_'], " ");
    cleaned
        .split_whitespace()
        .map(|w| {
            let mut chars = w.chars();
            chars.next().map_or(String::new(), |c| c.to_uppercase().collect::<String>() + chars.as_str())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Whether a connector is a chat surface.
pub fn is_chat(name: &str) -> bool {
    category(name) == "chat"
}

/// Whether a connector is a mail surface.
pub fn is_mail(name: &str) -> bool {
    category(name) == "mail"
}

/// Whether a connector is a developer surface.
pub fn is_code(name: &str) -> bool {
    category(name) == "code"
}

/// Whether a connector handles money.
pub fn is_money(name: &str) -> bool {
    category(name) == "money"
}

/// Count connectors per category, in first-seen order.
pub fn count_categories(names: &[String]) -> Vec<(&'static str, usize)> {
    let mut out: Vec<(&'static str, usize)> = Vec::new();
    for name in names {
        let cat = category(name);
        if let Some(entry) = out.iter_mut().find(|(c, _)| *c == cat) {
            entry.1 += 1;
        } else {
            out.push((cat, 1));
        }
    }
    out
}

/// Percentage of connectors in a category.
pub fn category_percent(count: usize, total: usize) -> u32 {
    crate::bdots_activity::percent(count, total)
}

/// Bar of filled cells for a category's share.
pub fn category_bar(count: usize, total: usize, width: usize) -> String {
    "▮".repeat(crate::bdots_activity::bar_width(count, total, width))
}

/// Duplicate connector ids, case-insensitively.
pub fn duplicates(names: &[String]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut dupes = Vec::new();
    for name in names {
        let key = name.to_ascii_lowercase();
        if !seen.insert(key.clone()) && !dupes.contains(&connector_title(name)) {
            dupes.push(connector_title(name));
        }
    }
    dupes
}

/// Number of distinct connector ids.
pub fn distinct(names: &[String]) -> usize {
    names.iter().map(|n| n.to_ascii_lowercase()).collect::<std::collections::HashSet<_>>().len()
}

/// Whether the bot has at least one connector.
pub fn has_any(names: &[String]) -> bool {
    !names.is_empty()
}

/// Whether the bot can talk in a chat surface.
pub fn can_chat(names: &[String]) -> bool {
    names.iter().any(|n| is_chat(n))
}

/// Whether the bot can read or send mail.
pub fn can_mail(names: &[String]) -> bool {
    names.iter().any(|n| is_mail(n))
}

/// Whether the bot can act on code or issues.
pub fn can_code(names: &[String]) -> bool {
    names.iter().any(|n| is_code(n))
}

/// Whether the bot can touch money.
pub fn can_money(names: &[String]) -> bool {
    names.iter().any(|n| is_money(n))
}

/// Connector lines sorted by title, for a stable list across clients.
pub fn sorted(names: &[String]) -> Vec<ConnectorLine> {
    let mut lines: Vec<ConnectorLine> = names.iter().map(|n| line(n)).collect();
    lines.sort_by(|a, b| a.name.cmp(&b.name));
    lines
}

/// One plain sentence describing the connector mix.
pub fn connector_headline(names: &[String]) -> String {
    if names.is_empty() {
        return "No accounts connected.".to_owned();
    }
    let cats = count_categories(names);
    let top = cats.first().map_or("other", |(c, _)| *c);
    format!("{} accounts · mostly {top}", names.len())
}

/// Semantic tone for the connector card.
pub fn connector_tone(names: &[String]) -> &'static str {
    if names.is_empty() { "warn" } else { "good" }
}

/// Build a display line for one connector id.
pub fn line(name: &str) -> ConnectorLine {
    let cat = category(name);
    ConnectorLine {
        name: connector_title(name),
        category: cat,
        icon: category_icon(cat),
        tone: category_tone(cat),
        detail: crate::bdots_more::shorten(name, 48),
    }
}

/// Assemble the connector summary.
pub fn summary(names: &[String]) -> Vec<(&'static str, String)> {
    vec![
        ("total", crate::bdots_desk::compact_count(names.len() as u64)),
        ("distinct", distinct(names).to_string()),
        ("headline", connector_headline(names)),
        ("chat", names.iter().filter(|n| is_chat(n)).count().to_string()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names() -> Vec<String> {
        vec!["slack".into(), "github".into(), "gmail".into(), "stripe".into(), "slack".into()]
    }

    #[test]
    fn categories_and_icons() {
        assert_eq!(category("SLACK"), "chat");
        assert_eq!(category("github"), "code");
        assert_eq!(category("gmail"), "mail");
        assert_eq!(category("stripe"), "money");
        assert_eq!(category("google-drive"), "files");
        assert_eq!(category("unknown"), "other");
        assert_eq!(category_icon("chat"), "bubble.left.and.bubble.right");
        assert_eq!(category_tone("money"), "attention");
    }

    #[test]
    fn titles_and_detectors() {
        assert_eq!(connector_title("google-drive"), "Google Drive");
        assert_eq!(connector_title("connector:slack"), "Slack");
        assert!(is_chat("slack"));
        assert!(is_mail("outlook"));
        assert!(is_code("linear"));
        assert!(is_money("shopify"));
        assert!(!is_chat("github"));
    }

    #[test]
    fn counts_percentages_and_bars() {
        let n = names();
        assert_eq!(count_categories(&n).len(), 4);
        assert_eq!(category_percent(1, 4), 25);
        assert!(!category_bar(1, 4, 8).is_empty());
    }

    #[test]
    fn duplicates_and_distinct() {
        let n = names();
        assert_eq!(distinct(&n), 4);
        assert_eq!(duplicates(&n).len(), 1);
    }

    #[test]
    fn detectors_and_ordering() {
        assert!(has_any(&names()));
        assert!(can_chat(&names()));
        assert!(can_mail(&names()));
        assert!(can_code(&names()));
        assert!(can_money(&names()));
        assert!(!has_any(&[]));
        assert_eq!(sorted(&names())[0].name, "Gmail");
    }

    #[test]
    fn headlines_and_lines() {
        assert!(connector_headline(&[]).contains("No accounts"));
        assert!(connector_headline(&names()).contains("accounts"));
        assert_eq!(connector_tone(&[]), "warn");
        assert_eq!(line("slack").category, "chat");
        assert_eq!(summary(&names()).len(), 4);
    }
}
