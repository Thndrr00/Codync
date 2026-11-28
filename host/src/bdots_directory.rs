#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers from bdots' latest bot-cache, email-directory and integrations sources.
//! Composes validators already ported (`bdots_more`, `bdots_text`, `bdots_desk`, `bdots_extra`)
//! instead of duplicating them. Only `botInsights` calls this.

use serde_json::Value;

/// Whether a cached bot can be used for an instant switch (bot-cache.ts cachedBot).
pub fn can_use_cached(unread: bool, status: &str, has_bot: bool) -> bool {
    has_bot && !unread && status == "idle"
}

/// Reason a cached bot is not usable.
pub fn cache_reason(unread: bool, status: &str, has_bot: bool) -> &'static str {
    if !has_bot {
        "Not cached yet"
    } else if unread {
        "Has unread messages"
    } else if status != "idle" {
        "Busy in a turn"
    } else {
        "Ready"
    }
}

/// SF Symbol for a cache state.
pub fn cache_icon(ready: bool) -> &'static str {
    if ready { "bolt.circle.fill" } else { "clock.circle" }
}

/// Semantic tone for a cache state.
pub fn cache_tone(ready: bool) -> &'static str {
    if ready { "good" } else { "secondary" }
}

/// Count of bots ready for an instant switch from summary rows.
pub fn ready_count(summaries: &[Value]) -> usize {
    summaries.iter().filter(|s| s["unread"] == false && s["status"] == "idle").count()
}

/// Count of bots with unread messages.
pub fn unread_count(summaries: &[Value]) -> usize {
    summaries.iter().filter(|s| s["unread"] == true).count()
}

/// Count of bots currently working.
pub fn busy_count(summaries: &[Value]) -> usize {
    summaries.iter().filter(|s| s["status"] != "idle" && !s["status"].is_null()).count()
}

/// Total number of bot summaries.
pub fn total(summaries: &[Value]) -> usize {
    summaries.len()
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Percent of bots ready for an instant switch.
pub fn ready_percent(summaries: &[Value]) -> u32 {
    crate::bdots_activity::percent(ready_count(summaries), summaries.len())
}

/// Bar of filled cells for the ready share.
pub fn ready_bar(summaries: &[Value], width: usize) -> String {
    "▮".repeat(crate::bdots_activity::bar_width(ready_count(summaries), summaries.len(), width))
}

/// Normalise an email directory name (email-directory.ts).
pub fn directory_name(name: &str) -> String {
    let clean = name.trim().to_ascii_lowercase();
    if clean.is_empty() { "directory".to_owned() } else { clean }
}

/// Whether an email address looks valid enough to route.
pub fn is_routable(email: &str) -> bool {
    email.contains('@') && email.split('@').count() == 2 && email.contains('.')
}

/// Domain part of an email address.
pub fn email_domain(email: &str) -> String {
    email.split('@').nth(1).unwrap_or("").to_owned()
}

/// Local part of an email address.
pub fn email_local(email: &str) -> String {
    email.split('@').next().unwrap_or("").to_owned()
}

/// Human label for an email address, via the shared shortener.
pub fn email_label(email: &str) -> String {
    crate::bdots_more::shorten(email, 64)
}

/// SF Symbol for a routable address.
pub fn route_icon(routable: bool) -> &'static str {
    if routable { "envelope.badge" } else { "envelope.badge.questionmark" }
}

/// Semantic tone for a routable address.
pub fn route_tone(routable: bool) -> &'static str {
    if routable { "good" } else { "attention" }
}

/// Integration display name from an id (integrations.ts).
pub fn integration_name(id: &str) -> String {
    let clean = id.trim();
    if clean.is_empty() {
        return "Integration".to_owned();
    }
    clean
        .split(['-', '_'])
        .map(|w| {
            let mut chars = w.chars();
            chars.next().map_or(String::new(), |c| c.to_uppercase().collect::<String>() + chars.as_str())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Whether an integration id names an endpoint (sameEndpoint from choices.ts).
pub fn is_endpoint(value: &str) -> bool {
    value.starts_with("https://") || value.starts_with("http://")
}

/// Header name for an integration key (integrations.ts integrationHeaders).
pub fn integration_header(name: &str) -> String {
    format!("x-{}", name.to_ascii_lowercase().replace([' ', '_'], "-"))
}

/// Whether a header value is a valid key.
pub fn valid_key(key: Option<&str>) -> bool {
    key.is_some_and(|k| !k.trim().is_empty() && k.len() >= 8)
}

/// Plain reason for a failed directory lookup.
pub fn directory_reason(found: bool, routable: bool) -> String {
    if found {
        "Found in the directory".to_owned()
    } else if !routable {
        "Not a valid address".to_owned()
    } else {
        "Not in the directory".to_owned()
    }
}

/// One plain sentence describing the directory.
pub fn directory_headline(summaries: &[Value]) -> String {
    if summaries.is_empty() {
        return "No bots in the directory.".to_owned();
    }
    format!("{} bots · {} ready", total(summaries), ready_count(summaries))
}

/// Semantic tone for the directory card.
pub fn directory_tone(summaries: &[Value]) -> &'static str {
    if summaries.is_empty() { "secondary" } else { "good" }
}

/// Build display lines from bot summaries.
pub fn build_lines(summaries: &[Value]) -> Vec<(&'static str, String)> {
    vec![
        ("total", count_label(total(summaries))),
        ("ready", ready_count(summaries).to_string()),
        ("unread", unread_count(summaries).to_string()),
        ("headline", directory_headline(summaries)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn summaries() -> Vec<Value> {
        vec![
            json!({"id":"a","unread":false,"status":"idle"}),
            json!({"id":"b","unread":true,"status":"idle"}),
            json!({"id":"c","unread":false,"status":"working"}),
        ]
    }

    #[test]
    fn cache_states() {
        assert!(can_use_cached(false, "idle", true));
        assert!(!can_use_cached(true, "idle", true));
        assert_eq!(cache_reason(false, "working", true), "Busy in a turn");
        assert_eq!(cache_reason(false, "idle", false), "Not cached yet");
        assert_eq!(cache_icon(true), "bolt.circle.fill");
        assert_eq!(cache_tone(false), "secondary");
    }

    #[test]
    fn summary_counts() {
        let s = summaries();
        assert_eq!(total(&s), 3);
        assert_eq!(ready_count(&s), 1);
        assert_eq!(unread_count(&s), 1);
        assert_eq!(busy_count(&s), 1);
        assert_eq!(ready_percent(&s), 33);
        assert!(!ready_bar(&s, 8).is_empty());
        assert_eq!(count_label(1500), "1.5K");
    }

    #[test]
    fn email_helpers() {
        assert_eq!(directory_name(" Directory "), "directory");
        assert!(is_routable("ada@example.com"));
        assert!(!is_routable("ada"));
        assert_eq!(email_domain("ada@example.com"), "example.com");
        assert_eq!(email_local("ada@example.com"), "ada");
        assert_eq!(email_label("ada@example.com"), "ada@example.com");
        assert_eq!(route_icon(true), "envelope.badge");
        assert_eq!(route_tone(false), "attention");
    }

    #[test]
    fn integration_helpers() {
        assert_eq!(integration_name("google-drive"), "Google Drive");
        assert_eq!(integration_name(""), "Integration");
        assert!(is_endpoint("https://api.test"));
        assert!(!is_endpoint("api.test"));
        assert_eq!(integration_header("Stripe"), "x-stripe");
        assert!(valid_key(Some("abcdefgh")));
        assert!(!valid_key(Some("abc")));
    }

    #[test]
    fn headlines_and_lines() {
        assert!(directory_headline(&[]).contains("No bots"));
        assert!(directory_headline(&summaries()).contains("ready"));
        assert_eq!(directory_tone(&[]), "secondary");
        assert_eq!(directory_reason(true, true), "Found in the directory");
        assert_eq!(directory_reason(false, false), "Not a valid address");
        assert_eq!(build_lines(&summaries()).len(), 4);
    }
}
