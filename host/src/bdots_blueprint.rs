#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from hermes-agent's `cron/blueprint_catalog.py` and `blueprint_plugins.py`:
//! slot schemas, validation, placeholder filling and catalog presentation. Composes helpers already
//! ported into this host. Only `botInsights` calls this.

use std::collections::BTreeMap;

/// Slot types every blueprint renderer understands (_SLOT_TYPES).
pub const SLOT_TYPES: [&str; 4] = ["time", "enum", "text", "weekdays"];

/// Prefix of a blueprint slash command.
pub const SLASH_PREFIX: &str = "/";

/// Scheme for a docs deep link to a blueprint.
pub const DEEPLINK_SCHEME: &str = "hermes://blueprint/";

/// Whether a slot type is supported.
pub fn is_slot_type(value: &str) -> bool {
    SLOT_TYPES.contains(&value)
}

/// Whether a blueprint key is well formed (_KEY_RE).
pub fn valid_key(key: &str) -> bool {
    let mut chars = key.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_lowercase() || first.is_ascii_digit())
        && key.len() <= 64
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

/// Whether a plugin namespace is well formed (_NAMESPACE_RE).
pub fn valid_namespace(namespace: &str) -> bool {
    let mut chars = namespace.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_alphanumeric()
        && namespace.len() <= 64
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
}

/// Whether a slot name is well formed (_SLOT_NAME_RE).
pub fn valid_slot_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && name.len() <= 64
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Whether a value is a 24-hour `HH:MM` time (_TIME_RE).
pub fn is_time(value: &str) -> bool {
    time_parts(value).is_some()
}

/// Hours and minutes of a `HH:MM` value.
pub fn time_parts(value: &str) -> Option<(u8, u8)> {
    let (hour, minute) = value.trim().split_once(':')?;
    if hour.len() != 2 || minute.len() != 2 {
        return None;
    }
    let hour: u8 = hour.parse().ok()?;
    let minute: u8 = minute.parse().ok()?;
    (hour <= 23 && minute <= 59).then_some((hour, minute))
}

/// A `HH:MM` label from parsed parts.
pub fn format_time_parts(hour: u8, minute: u8) -> Option<String> {
    if hour <= 23 && minute <= 59 { Some(format!("{hour:02}:{minute:02}")) } else { None }
}

/// Placeholders of a template, in order, without repeats (_placeholders).
pub fn placeholders(template: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        let Some(end_rel) = rest[start + 1..].find('}') else {
            break;
        };
        let name = &rest[start + 1..start + 1 + end_rel];
        if !name.is_empty() && !out.iter().any(|seen| seen == name) {
            out.push(name.to_owned());
        }
        rest = &rest[start + 1 + end_rel + 1..];
    }
    out
}

/// Fill a template's placeholders from resolved values, keeping unknown ones.
pub fn fill_template(template: &str, values: &BTreeMap<String, String>) -> String {
    let mut out = template.to_owned();
    for name in placeholders(template) {
        if let Some(value) = values.get(&name) {
            out = out.replace(&format!("{{{name}}}"), value);
        }
    }
    out
}

/// Placeholders still missing from a template.
pub fn unfilled_placeholders(template: &str, values: &BTreeMap<String, String>) -> Vec<String> {
    placeholders(template).into_iter().filter(|name| !values.contains_key(name)).collect()
}

/// Cron day-of-week field for a named weekday preset (WEEKDAY_PRESETS).
pub fn weekday_preset(name: &str) -> Option<&'static str> {
    match name.to_ascii_lowercase().as_str() {
        "everyday" => Some("*"),
        "weekdays" => Some("1-5"),
        "weekends" => Some("0,6"),
        _ => None,
    }
}

/// Human label for a weekday preset.
pub fn weekday_label(name: &str) -> &'static str {
    match name.to_ascii_lowercase().as_str() {
        "weekdays" => "Monday to Friday",
        "weekends" => "Weekends",
        _ => "Every day",
    }
}

/// Whether a cron day-of-week field is simple and well formed.
pub fn valid_weekdays(field: &str) -> bool {
    !field.is_empty()
        && field.split(',').all(|part| part.split('-').all(|n| n.parse::<u8>().is_ok_and(|v| v <= 7) || n == "*"))
}

/// Number of fields in a cron expression.
pub fn cron_fields(expression: &str) -> usize {
    expression.split_whitespace().count()
}

/// A schedule template rendered as a plain sentence (_humanizeSchedule).
pub fn humanize_schedule(template: &str) -> String {
    let mut out = template.to_owned();
    for (name, word) in [("minute", "minutes"), ("hour", "hours"), ("dow", "Weekdays")] {
        out = out.replace(&format!("{{{name}}}"), word);
    }
    for name in placeholders(&out) {
        out = out.replace(&format!("{{{name}}}"), &name);
    }
    crate::bdots_textfmt::upper_first(&out)
}

/// Slash command that opens a blueprint pre-filled (blueprintSlashCommand).
pub fn blueprint_slash_command(key: &str, values: &BTreeMap<String, String>) -> String {
    let pairs: Vec<String> = values.iter().map(|(k, v)| format!("{k}={v}")).collect();
    if pairs.is_empty() { format!("{SLASH_PREFIX}{key}") } else { format!("{SLASH_PREFIX}{key} {}", pairs.join(" ")) }
}

/// Docs deep link for a blueprint (blueprintDeeplink).
pub fn blueprint_deeplink(key: &str) -> String {
    format!("{DEEPLINK_SCHEME}{key}")
}

/// A slot name as a form label.
pub fn slot_label(name: &str) -> String {
    let spaced = name.replace('_', " ");
    crate::bdots_textfmt::upper_first(&spaced)
}

/// Validation message for an unknown slot type, if any.
pub fn slot_type_error(slot: &str, kind: &str) -> String {
    if is_slot_type(kind) { String::new() } else { format!("Unknown slot type '{kind}' in slot '{slot}'.") }
}

/// Validation message for a missing required placeholder.
pub fn missing_value_error(name: &str) -> String {
    format!("Missing value for '{name}'.")
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(slots: usize, fields: usize, missing: usize, valid: bool) -> Vec<(&'static str, String)> {
    vec![
        ("slots", count_label(slots)),
        ("fields", fields.to_string()),
        ("missing", count_label(missing)),
        ("valid", if valid { "yes" } else { "no" }.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers() {
        assert!(is_slot_type("weekdays"));
        assert!(!is_slot_type("date"));
        assert!(valid_key("daily-report"));
        assert!(!valid_key("Daily Report"));
        assert!(valid_namespace("acme.plugins"));
        assert!(!valid_namespace(".bad"));
        assert!(valid_slot_name("_deliver"));
        assert!(!valid_slot_name("2bad"));
    }

    #[test]
    fn times() {
        assert!(is_time("08:00"));
        assert!(!is_time("8:0"));
        assert!(!is_time("24:00"));
        assert_eq!(time_parts("20:05"), Some((20, 5)));
        assert_eq!(format_time_parts(8, 0).as_deref(), Some("08:00"));
        assert_eq!(format_time_parts(24, 0), None);
    }

    #[test]
    fn templates() {
        let template = "{minute} {hour} * * {dow}";
        assert_eq!(placeholders(template), vec!["minute", "hour", "dow"]);
        let mut values = BTreeMap::new();
        values.insert("minute".to_owned(), "0".to_owned());
        values.insert("hour".to_owned(), "8".to_owned());
        values.insert("dow".to_owned(), "1-5".to_owned());
        assert_eq!(fill_template(template, &values), "0 8 * * 1-5");
        assert!(unfilled_placeholders(template, &values).is_empty());
        assert_eq!(humanize_schedule(template), "Minutes hours * * Weekdays");
    }

    #[test]
    fn schedules_and_commands() {
        assert_eq!(weekday_preset("WEEKDAYS"), Some("1-5"));
        assert_eq!(weekday_label("weekends"), "Weekends");
        assert!(valid_weekdays("1-5"));
        assert!(!valid_weekdays("9"));
        assert_eq!(cron_fields("0 8 * * 1-5"), 5);
        let mut values = BTreeMap::new();
        values.insert("time".to_owned(), "08:00".to_owned());
        assert_eq!(blueprint_slash_command("daily", &values), "/daily time=08:00");
        assert_eq!(blueprint_deeplink("daily"), "hermes://blueprint/daily");
    }

    #[test]
    fn labels_and_summary() {
        assert_eq!(slot_label("deliver_to"), "Deliver to");
        assert!(slot_type_error("time", "bad").contains("Unknown slot type"));
        assert!(slot_type_error("time", "time").is_empty());
        assert!(missing_value_error("time").contains("time"));
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(3, 5, 0, true).len(), 4);
    }
}
