#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from coucou's `windows/src/core/pills.ts`: pill catalog, host support,
//! declared integrations and pill ordering. Only `botInsights` calls this.

use serde_json::{Value, json};

/// Categories of a pill (PillCategory).
pub const PILL_CATEGORIES: [&str; 4] = ["workspace", "agent", "ai", "service"];

/// Support values of a pill (PillSupport).
pub const PILL_SUPPORT: [&str; 5] = ["yes", "windows", "linux", "soon", "no"];

/// Default main pill (DEFAULT_MAIN_PILL).
pub const DEFAULT_MAIN_PILL: &str = "integration_claude";

/// Most declared integrations (MAX_DECLARED).
pub const MAX_DECLARED: usize = 4;

/// Catalog of pills by id, category and support.
pub const PILL_CATALOG: [(&str, &str, &str); 12] = [
    ("integration_claude", "workspace", "yes"),
    ("agent_codex", "workspace", "yes"),
    ("agent_cursor", "workspace", "yes"),
    ("agent_hermes", "agent", "yes"),
    ("agent_claude-desktop", "agent", "yes"),
    ("integration_whatsapp", "service", "windows"),
    ("integration_mail", "service", "windows"),
    ("integration_github", "service", "yes"),
    ("integration_notion", "ai", "soon"),
    ("integration_slack", "ai", "linux"),
    ("integration_telegram", "service", "no"),
    ("agent_gemini", "agent", "yes"),
];

/// Subtitles of live session pills (sessionSubtitle).
pub const SUBTITLES: [(&str, &str); 5] = [
    ("integration_claude", "Claude Code"),
    ("agent_cursor", "Cursor"),
    ("agent_codex", "Codex"),
    ("agent_hermes", "Hermes"),
    ("agent_claude-desktop", "Claude Desktop"),
];

/// Definition of a pill by id (pillDefinition).
pub fn pill_definition(id: &str) -> Value {
    PILL_CATALOG
        .iter()
        .find(|(candidate, _, _)| *candidate == id)
        .map_or(Value::Null, |(id, category, support)| json!({"id": id, "category": category, "support": support}))
}

/// Whether a pill is offered on this host (offeredOn).
pub fn offered_on(support: &str, os: &str) -> bool {
    support == "yes" || support == "soon" || support == os
}

/// Pills offered on a host, in catalog order (availablePills).
pub fn available_pills(os: &str) -> Vec<Value> {
    PILL_CATALOG
        .iter()
        .filter(|(_, _, support)| offered_on(support, os))
        .map(|(id, category, support)| json!({"id": id, "category": category, "support": support}))
        .collect()
}

/// Whether a pill is coming soon (isComingSoon).
pub fn is_coming_soon(id: &str) -> bool {
    pill_definition(id)["support"].as_str() == Some("soon")
}

/// Whether a pill's sessions arrive as hook events (isHookPill).
pub fn is_hook_pill(id: &str) -> bool {
    id.starts_with("agent_") || id == DEFAULT_MAIN_PILL
}

/// Workspace pills that can be the main one (mainPillChoices).
pub fn main_pill_choices(os: &str) -> Vec<Value> {
    available_pills(os)
        .into_iter()
        .filter(|pill| pill["category"].as_str() == Some("workspace") && pill["support"].as_str() != Some("soon"))
        .collect()
}

/// Subtitle of a live session pill (sessionSubtitle).
pub fn session_subtitle(id: &str) -> &'static str {
    SUBTITLES.iter().find(|(candidate, _)| *candidate == id).map_or("Agent", |(_, subtitle)| *subtitle)
}

/// Drop unknown, duplicated and main pills from declared integrations (sanitizeDeclared).
pub fn sanitize_declared(declared: &Value, os: &str) -> Value {
    let mains = main_pill_choices(os);
    let main = declared["mainPill"].as_str().unwrap_or(DEFAULT_MAIN_PILL);
    let main_pill = if mains.iter().any(|pill| pill["id"].as_str() == Some(main)) { main } else { DEFAULT_MAIN_PILL };
    let offered = available_pills(os);
    let mut active: Vec<String> = Vec::new();
    for value in declared["activeIntegrations"].as_array().cloned().unwrap_or_default() {
        if let Some(id) = value.as_str() {
            if id != main_pill
                && offered.iter().any(|pill| pill["id"].as_str() == Some(id))
                && !active.iter().any(|value| value == id)
            {
                active.push(id.to_owned());
            }
        }
    }
    json!({"mainPill": main_pill, "activeIntegrations": active})
}

/// Toggle a declared integration; null when it can't be toggled (toggleDeclared).
pub fn toggle_declared(declared: &Value, id: &str, os: &str) -> Value {
    if id == declared["mainPill"].as_str().unwrap_or("")
        || !available_pills(os).iter().any(|pill| pill["id"].as_str() == Some(id))
    {
        return Value::Null;
    }
    let active: Vec<String> = declared["activeIntegrations"]
        .as_array()
        .map(|values| values.iter().filter_map(Value::as_str).map(str::to_owned).collect())
        .unwrap_or_default();
    if let Some(index) = active.iter().position(|value| value == id) {
        let mut next = active;
        next.remove(index);
        return json!(next);
    }
    if active.len() >= MAX_DECLARED {
        return Value::Null;
    }
    let mut next = active;
    next.push(id.to_owned());
    json!(next)
}

/// Choose a new main pill; null when it isn't a choice (chooseMainPill).
pub fn choose_main_pill(declared: &Value, id: &str, os: &str) -> Value {
    if !main_pill_choices(os).iter().any(|pill| pill["id"].as_str() == Some(id)) {
        return Value::Null;
    }
    let active: Vec<String> = declared["activeIntegrations"]
        .as_array()
        .map(|values| values.iter().filter_map(Value::as_str).filter(|value| *value != id).map(str::to_owned).collect())
        .unwrap_or_default();
    json!({"mainPill": id, "activeIntegrations": active})
}

/// Rank of a pill in the catalog, with the main pill first (pillRank).
pub fn pill_rank(id: &str, main_pill: &str) -> i64 {
    if id == main_pill {
        -2
    } else {
        PILL_CATALOG.iter().position(|(candidate, _, _)| *candidate == id).map_or(-1, |index| index as i64)
    }
}

/// Order pills: main first, then unknown, then catalog order (orderPills).
pub fn order_pills(pills: &[Value], main_pill: &str) -> Vec<Value> {
    let mut indexed: Vec<(usize, &Value)> = pills.iter().enumerate().collect();
    indexed.sort_by_key(|(index, pill)| {
        let id = pill["id"].as_str().unwrap_or("");
        (pill_rank(id, main_pill), *index)
    });
    indexed.into_iter().map(|(_, pill)| pill.clone()).collect()
}

/// One line describing a pill (pillLine).
pub fn pill_line(id: &str, support: &str, coming: bool) -> String {
    format!("{id} · {support} · {}", if coming { "soon" } else { "now" })
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(pills: usize, available: usize, declared: usize, main: bool) -> Vec<(&'static str, String)> {
    vec![
        ("pills", count_label(pills)),
        ("available", count_label(available)),
        ("declared", count_label(declared)),
        ("main", if main { "yes" } else { "no" }.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn catalog_and_support() {
        assert_eq!(DEFAULT_MAIN_PILL, "integration_claude");
        assert_eq!(MAX_DECLARED, 4);
        assert_eq!(PILL_CATEGORIES.len(), 4);
        assert_eq!(PILL_SUPPORT.len(), 5);
        assert_eq!(pill_definition(DEFAULT_MAIN_PILL)["category"], "workspace");
        assert_eq!(pill_definition("unknown"), Value::Null);
        assert!(offered_on("yes", "linux"));
        assert!(offered_on("windows", "windows"));
        assert!(!offered_on("linux", "windows"));
        assert_eq!(available_pills("windows").len(), 10);
        assert!(is_coming_soon("integration_notion"));
        assert!(is_hook_pill("agent_codex"));
        assert_eq!(session_subtitle("agent_cursor"), "Cursor");
        assert_eq!(session_subtitle("other"), "Agent");
    }

    #[test]
    fn declared_state() {
        let declared = json!({"mainPill":"integration_claude","activeIntegrations":["agent_codex","unknown","integration_claude","agent_codex"]});
        let clean = sanitize_declared(&declared, "windows");
        assert_eq!(clean["activeIntegrations"].as_array().map_or(0, Vec::len), 1);
        let toggled = toggle_declared(&declared, "agent_codex", "windows");
        assert_eq!(toggled.as_array().map_or(0, Vec::len), 0);
        assert_eq!(toggle_declared(&declared, "integration_claude", "windows"), Value::Null);
        let chosen = choose_main_pill(&declared, "agent_cursor", "windows");
        assert_eq!(chosen["mainPill"], "agent_cursor");
        assert!(main_pill_choices("windows").iter().all(|pill| pill["support"].as_str() != Some("soon")));
    }

    #[test]
    fn ordering_and_presentation() {
        let pills = vec![json!({"id":"agent_codex"}), json!({"id":"other"}), json!({"id":"integration_claude"})];
        let ordered = order_pills(&pills, DEFAULT_MAIN_PILL);
        assert_eq!(ordered[0]["id"], DEFAULT_MAIN_PILL);
        assert_eq!(ordered[1]["id"], "other");
        assert_eq!(pill_rank(DEFAULT_MAIN_PILL, DEFAULT_MAIN_PILL), -2);
        assert!(pill_line("x", "yes", false).contains("now"));
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(12, 10, 2, true).len(), 4);
    }
}
