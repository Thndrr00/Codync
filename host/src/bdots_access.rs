#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers from bdots' app-access, app-store, apps-runtime and email-routing sources.
//! Composes validators already ported (`bdots_extra`, `bdots_more`, `bdots_desk`) instead of
//! duplicating them. Only `botInsights` calls this.

use serde_json::Value;

/// A public link's mode: edit, view or submit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkMode {
    Edit,
    View,
    Submit,
}

/// Parse a link mode, defaulting to view.
pub fn parse_mode(value: &str) -> LinkMode {
    match value.to_ascii_lowercase().as_str() {
        "edit" => LinkMode::Edit,
        "submit" => LinkMode::Submit,
        _ => LinkMode::View,
    }
}

/// Human label for a link mode.
pub fn mode_label(mode: LinkMode) -> &'static str {
    match mode {
        LinkMode::Edit => "Can edit",
        LinkMode::View => "Read only",
        LinkMode::Submit => "Public form",
    }
}

/// SF Symbol for a link mode.
pub fn mode_icon(mode: LinkMode) -> &'static str {
    match mode {
        LinkMode::Edit => "pencil.circle.fill",
        LinkMode::View => "eye.circle.fill",
        LinkMode::Submit => "doc.text.fill",
    }
}

/// Semantic tone for a link mode.
pub fn mode_tone(mode: LinkMode) -> &'static str {
    match mode {
        LinkMode::Edit => "attention",
        LinkMode::View => "secondary",
        LinkMode::Submit => "good",
    }
}

/// Whether a mode can write.
pub fn can_write(mode: LinkMode) -> bool {
    mode == LinkMode::Edit
}

/// Whether a mode is a public form.
pub fn is_form(mode: LinkMode) -> bool {
    mode == LinkMode::Submit
}

/// Collections visible in a link mode: submit links only show their read collections.
pub fn visible_collections<'a>(mode: LinkMode, read: &[String], all: &'a [String]) -> Vec<&'a String> {
    if mode != LinkMode::Submit {
        return all.iter().collect();
    }
    all.iter().filter(|c| read.iter().any(|r| r == *c)).collect()
}

/// Whether a collection name is valid (app-store.ts assertCollection).
pub fn valid_collection(name: &str) -> bool {
    crate::bdots_more::is_app_slug(name)
}

/// Names of invalid collections in a list.
pub fn invalid_collections(names: &[String]) -> Vec<String> {
    names.iter().filter(|n| !valid_collection(n)).cloned().collect()
}

/// Clean a list of names: trim, drop empties and duplicates, in order.
pub fn clean_names(names: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for name in names {
        let trimmed = name.trim();
        if !trimmed.is_empty() && !out.iter().any(|n| n == trimmed) {
            out.push(trimmed.to_owned());
        }
    }
    out
}

/// Error line for a form with no submit collections.
pub fn form_error(submit: usize) -> String {
    if submit == 0 { "A public form needs at least one submit collection".to_owned() } else { String::new() }
}

/// Collections named in both read and submit.
pub fn overlapping(read: &[String], submit: &[String]) -> Vec<String> {
    submit.iter().filter(|s| read.iter().any(|r| r == *s)).cloned().collect()
}

/// Access line from a parsed access blob.
pub fn access_line(value: &Value) -> String {
    let read = value["read"].as_array().map_or(0, Vec::len);
    let submit = value["submit"].as_array().map_or(0, Vec::len);
    format!("{} read · {} submit", read, submit)
}

/// Whether an access blob grants anything.
pub fn has_access(value: &Value) -> bool {
    value["read"].as_array().is_some_and(|a| !a.is_empty()) || value["submit"].as_array().is_some_and(|a| !a.is_empty())
}

/// Caller label from an app caller (apps-runtime.ts publicCaller).
pub fn caller_label(value: &Value) -> String {
    value["label"].as_str().map_or_else(|| "Unknown caller".to_owned(), |s| crate::bdots_more::shorten(s, 48))
}

/// Caller mode from an app caller.
pub fn caller_mode(value: &Value) -> LinkMode {
    parse_mode(value["mode"].as_str().unwrap_or(""))
}

/// Live app name from user, bot and slug (apps-runtime.ts appLiveName).
pub fn live_name(user: &str, bot: &str, slug: &str) -> String {
    format!("{user}:{bot}:{slug}")
}

/// Whether email routing is configured (email-routing.ts routingManaged).
pub fn routing_managed(zone: Option<&str>, token: Option<&str>) -> bool {
    zone.is_some_and(|z| !z.trim().is_empty()) && token.is_some_and(|t| !t.trim().is_empty())
}

/// Human label for routing state.
pub fn routing_label(managed: bool) -> &'static str {
    if managed { "Managed routing" } else { "Local routing" }
}

/// SF Symbol for routing state.
pub fn routing_icon(managed: bool) -> &'static str {
    if managed { "cloud.fill" } else { "house.fill" }
}

/// Semantic tone for routing state.
pub fn routing_tone(managed: bool) -> &'static str {
    if managed { "good" } else { "secondary" }
}

/// Count of public links from the config's access blobs.
pub fn link_count(values: &[Value]) -> usize {
    values.iter().filter(|v| has_access(v)).count()
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(mode: LinkMode, read: usize, submit: usize, values: &[Value]) -> Vec<(&'static str, String)> {
    vec![
        ("mode", mode_label(mode).to_owned()),
        ("links", count_label(link_count(values))),
        ("read", read.to_string()),
        ("submit", submit.to_string()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn modes_and_detectors() {
        assert_eq!(parse_mode("EDIT"), LinkMode::Edit);
        assert_eq!(parse_mode("submit"), LinkMode::Submit);
        assert_eq!(parse_mode("x"), LinkMode::View);
        assert_eq!(mode_label(LinkMode::Edit), "Can edit");
        assert_eq!(mode_icon(LinkMode::View), "eye.circle.fill");
        assert_eq!(mode_tone(LinkMode::Submit), "good");
        assert!(can_write(LinkMode::Edit));
        assert!(!can_write(LinkMode::View));
        assert!(is_form(LinkMode::Submit));
    }

    #[test]
    fn collections_and_validation() {
        let all = vec!["responses".into(), "private".into(), "public".into()];
        let read = vec!["public".into()];
        assert_eq!(visible_collections(LinkMode::Submit, &read, &all).len(), 1);
        assert_eq!(visible_collections(LinkMode::View, &read, &all).len(), 3);
        assert!(valid_collection("responses"));
        assert!(!valid_collection("Bad Name"));
        assert_eq!(invalid_collections(&all).len(), 0);
        assert_eq!(clean_names(&["a".into(), " a ".into(), "".into()]).len(), 1);
        assert!(form_error(0).contains("at least one"));
        assert!(form_error(1).is_empty());
        assert_eq!(overlapping(&read, &all).len(), 1);
    }

    #[test]
    fn access_and_callers() {
        let value = json!({"read":["a"],"submit":["b"]});
        assert!(has_access(&value));
        assert!(!has_access(&json!({})));
        assert_eq!(access_line(&value), "1 read · 1 submit");
        assert_eq!(caller_label(&json!({"label":"Phone"})), "Phone");
        assert_eq!(caller_mode(&json!({"mode":"edit"})), LinkMode::Edit);
        assert_eq!(live_name("u", "b", "s"), "u:b:s");
    }

    #[test]
    fn routing_and_counts() {
        assert!(routing_managed(Some("zone"), Some("token")));
        assert!(!routing_managed(None, Some("token")));
        assert_eq!(routing_label(true), "Managed routing");
        assert_eq!(routing_icon(false), "house.fill");
        assert_eq!(routing_tone(true), "good");
        assert_eq!(link_count(&[json!({"read":["a"]}), json!({})]), 1);
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(LinkMode::View, 1, 1, &[]).len(), 4);
    }
}
