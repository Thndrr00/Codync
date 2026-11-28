#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from coucou's `windows/src/core/shortcuts.ts`: accelerators, recorded key
//! presses, clashes and island navigation. Only `botInsights` calls this.

use serde_json::{Value, json};

/// Modifier tokens accepted in an accelerator.
pub const MODIFIERS: [&str; 8] = ["ctrl", "control", "commandorcontrol", "cmdorctrl", "alt", "shift", "meta", "super"];

/// Keys that are modifiers alone (MODIFIER_KEYS).
pub const MODIFIER_KEYS: [&str; 6] = ["Control", "Alt", "Shift", "Meta", "CapsLock", "Super"];

/// Glyphs used when displaying keys.
pub const KEY_GLYPHS: [(&str, &str); 4] = [("Up", "↑"), ("Down", "↓"), ("Left", "←"), ("Right", "→")];

/// Layouts where Ctrl+Alt types a character (AltGr).
pub const ALTGR_LAYOUTS: [&str; 4] = ["US", "German", "French", "Spanish"];

/// Island shortcut count shown in Settings.
pub const ISLAND_SHORTCUT_COUNT: usize = 6;

/// Parse "Ctrl+Shift+K" into its parts (parseKeys).
pub fn parse_keys(text: &str) -> Value {
    let tokens: Vec<&str> = text.split('+').map(str::trim).collect();
    if tokens.is_empty() || tokens.iter().any(|token| token.is_empty()) {
        return Value::Null;
    }
    let mut combo = json!({"ctrl": false, "alt": false, "shift": false, "meta": false, "key": ""});
    for (index, token) in tokens.iter().enumerate() {
        let last = index + 1 == tokens.len();
        let lower = token.to_ascii_lowercase();
        let field = match lower.as_str() {
            "ctrl" | "control" | "commandorcontrol" | "cmdorctrl" => Some("ctrl"),
            "alt" => Some("alt"),
            "shift" => Some("shift"),
            "meta" | "super" => Some("meta"),
            _ => None,
        };
        if let Some(field) = field {
            if last {
                return Value::Null;
            }
            combo[field] = Value::Bool(true);
        } else if !last || token.is_empty() {
            return Value::Null;
        } else {
            combo["key"] = Value::String((*token).to_owned());
        }
    }
    if combo["key"].as_str().unwrap_or("").is_empty() {
        return Value::Null;
    }
    combo
}

/// Canonical form of an accelerator (formatKeys).
pub fn format_keys(combo: &Value) -> String {
    let mut parts = Vec::new();
    for (field, label) in [("ctrl", "Ctrl"), ("alt", "Alt"), ("shift", "Shift"), ("meta", "Super")] {
        if combo[field] == true {
            parts.push(label);
        }
    }
    parts.push(combo["key"].as_str().unwrap_or(""));
    parts.join("+")
}

/// Normalize an accelerator, or null when it isn't one (normalizeKeys).
pub fn normalize_keys(text: &str) -> Option<String> {
    let combo = parse_keys(text);
    if combo.is_null() { None } else { Some(format_keys(&combo)) }
}

/// What a person reads: glyphs for arrows, Win for meta (displayKeys).
pub fn display_keys(text: &str) -> String {
    let combo = parse_keys(text);
    if combo.is_null() {
        return if text.trim().is_empty() { "-".to_owned() } else { text.trim().to_owned() };
    }
    let mut out = String::new();
    for (field, label) in [("ctrl", "Ctrl"), ("alt", "Alt"), ("shift", "Shift"), ("meta", "Win")] {
        if combo[field] == true {
            out.push_str(label);
            out.push('+');
        }
    }
    let key = combo["key"].as_str().unwrap_or("");
    let glyph = KEY_GLYPHS.iter().find(|(name, _)| *name == key).map_or(key, |(_, glyph)| *glyph);
    out.push_str(glyph);
    out
}

/// Effective binding: stored value or the default (effective).
pub fn effective(def: &Value, stored: Option<&Value>) -> Value {
    let own = stored.and_then(|bindings| bindings.get(def["id"].as_str().unwrap_or("")));
    if let Some(binding) = own.filter(|value| value.is_object()) {
        json!({"keys": binding["keys"].as_str().unwrap_or(""), "enabled": binding["enabled"].as_bool().unwrap_or(false)})
    } else {
        json!({"keys": def["defaultKeys"].as_str().unwrap_or(""), "enabled": def["enabledByDefault"] == true})
    }
}

/// What a key press records (recordPress).
pub fn record_press(press: &Value) -> Value {
    let key = press["key"].as_str().unwrap_or("");
    if MODIFIER_KEYS.contains(&key) {
        return json!({"kind": "pending"});
    }
    let bare =
        press["ctrlKey"] != true && press["altKey"] != true && press["metaKey"] != true && press["shiftKey"] != true;
    if key == "Escape" && bare {
        return json!({"kind": "cancel"});
    }
    if matches!(key, "Backspace" | "Delete") && bare {
        return json!({"kind": "clear"});
    }
    if key.is_empty() {
        return json!({"kind": "unsupported"});
    }
    if press["ctrlKey"] != true && press["altKey"] != true && press["metaKey"] != true {
        return json!({"kind": "needsModifier"});
    }
    let combo = json!({
        "ctrl": press["ctrlKey"] == true,
        "alt": press["altKey"] == true,
        "shift": press["shiftKey"] == true,
        "meta": press["metaKey"] == true,
        "key": key,
    });
    json!({"kind": "combo", "keys": format_keys(&combo)})
}

/// Bindings that share the same normalized accelerator (duplicates).
pub fn duplicates(entries: &[(String, String)]) -> Vec<String> {
    let mut seen: Vec<(String, String)> = Vec::new();
    let mut out = Vec::new();
    for (id, keys) in entries {
        let Some(normalized) = normalize_keys(keys) else {
            continue;
        };
        if let Some((_, other)) = seen.iter().find(|(_, value)| *value == normalized) {
            if !out.contains(id) {
                out.push(id.clone());
            }
            if !out.contains(other) {
                out.push(other.clone());
            }
        } else {
            seen.push((id.clone(), normalized));
        }
    }
    out
}

/// Layouts where Ctrl+Alt types a character (altGrClashes).
pub fn alt_gr_clashes(keys: &str) -> Vec<&'static str> {
    let combo = parse_keys(keys);
    if combo.is_null() || combo["ctrl"] != true || combo["alt"] != true || combo["meta"] == true {
        return Vec::new();
    }
    ALTGR_LAYOUTS.to_vec()
}

/// Next selection of a card list, clamped (navigate).
pub fn navigate(selection: Option<usize>, delta: i64, item_count: usize) -> Option<usize> {
    if item_count == 0 {
        return None;
    }
    match selection {
        Some(current) => {
            let next = current as i64 + delta;
            Some(next.clamp(0, (item_count - 1) as i64) as usize)
        }
        None => Some(if delta > 0 { 0 } else { item_count - 1 }),
    }
}

/// Pill `delta` steps from the current one, wrapping (cyclePill).
pub fn cycle_pill(ids: &[String], current: Option<&str>, delta: i64) -> Option<String> {
    if ids.is_empty() {
        return None;
    }
    let at = current.map_or(0, |value| ids.iter().position(|id| id == value).unwrap_or(0)) as i64;
    let count = ids.len() as i64;
    let next = (((at + delta) % count) + count) % count;
    ids[next as usize].clone().into()
}

/// Pill number `n`, counting from one (pillByNumber).
pub fn pill_by_number(ids: &[String], n: usize) -> Option<String> {
    (n >= 1 && n <= ids.len()).then(|| ids[n - 1].clone())
}

/// Action of a bare Ctrl key in the island (islandKeyAction).
pub fn island_key_action(key: &str, ctrl: bool, alt: bool, shift: bool, meta: bool, in_text_field: bool) -> Value {
    if !ctrl || alt || shift || meta {
        return Value::Null;
    }
    match key {
        "ArrowRight" | "ArrowLeft" if !in_text_field => {
            json!({"kind": "cycle", "delta": if key == "ArrowRight" { 1 } else { -1 }})
        }
        "ArrowDown" | "ArrowUp" if !in_text_field => {
            json!({"kind": "nav", "delta": if key == "ArrowDown" { 1 } else { -1 }})
        }
        "o" => json!({"kind": "open"}),
        "e" => json!({"kind": "diff"}),
        _ => Value::Null,
    }
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(shortcuts: usize, bindings: usize, clashes: usize, enabled: bool) -> Vec<(&'static str, String)> {
    vec![
        ("shortcuts", count_label(shortcuts)),
        ("bindings", count_label(bindings)),
        ("clashes", count_label(clashes)),
        ("enabled", if enabled { "yes" } else { "no" }.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accelerators() {
        assert_eq!(MODIFIERS.len(), 8);
        assert_eq!(MODIFIER_KEYS.len(), 6);
        assert_eq!(KEY_GLYPHS.len(), 4);
        let combo = parse_keys("Ctrl+Shift+K");
        assert_eq!(combo["ctrl"], true);
        assert_eq!(combo["key"], "K");
        assert_eq!(format_keys(&combo), "Ctrl+Shift+K");
        assert_eq!(normalize_keys("ctrl+alt+k"), "Ctrl+Alt+K");
        assert_eq!(normalize_keys("Ctrl+"), None);
        assert_eq!(display_keys("Ctrl+Up"), "Ctrl+↑");
        assert_eq!(display_keys("Meta+Space"), "Win+Space");
        assert_eq!(display_keys(""), "-");
    }

    #[test]
    fn bindings_and_press() {
        let def = json!({"id":"open","defaultKeys":"Ctrl+O","enabledByDefault":true});
        let stored = json!({"open":{"keys":"Ctrl+P","enabled":false}});
        assert_eq!(effective(&def, Some(&stored))["keys"], "Ctrl+P");
        assert_eq!(effective(&def, None)["keys"], "Ctrl+O");
        assert_eq!(record_press(&json!({"key":"Control"}))["kind"], "pending");
        assert_eq!(record_press(&json!({"key":"Escape","shiftKey":false}))["kind"], "cancel");
        assert_eq!(record_press(&json!({"key":"Backspace","shiftKey":false}))["kind"], "clear");
        assert_eq!(record_press(&json!({"key":"k","ctrlKey":true}))["kind"], "combo");
        assert_eq!(record_press(&json!({"key":"k","shiftKey":false}))["kind"], "needsModifier");
    }

    #[test]
    fn clashes_and_navigation() {
        let entries = vec![
            ("a".to_owned(), "Ctrl+K".to_owned()),
            ("b".to_owned(), "ctrl+k".to_owned()),
            ("c".to_owned(), "Alt+K".to_owned()),
        ];
        let dups = duplicates(&entries);
        assert!(dups.contains(&"a".to_owned()) && dups.contains(&"b".to_owned()));
        assert_eq!(alt_gr_clashes("Ctrl+Alt+K").len(), 4);
        assert!(alt_gr_clashes("Ctrl+K").is_empty());
        assert_eq!(navigate(None, 1, 5), Some(0));
        assert_eq!(navigate(None, -1, 5), Some(4));
        assert_eq!(navigate(Some(2), 9, 5), Some(4));
        let ids = vec!["a".to_owned(), "b".to_owned(), "c".to_owned()];
        assert_eq!(cycle_pill(&ids, Some("c"), 1).as_deref(), Some("a"));
        assert_eq!(pill_by_number(&ids, 2).as_deref(), Some("b"));
        assert_eq!(ISLAND_SHORTCUT_COUNT, 6);
        assert_eq!(island_key_action("ArrowRight", true, false, false, false, false)["kind"], "cycle");
        assert_eq!(island_key_action("o", true, false, false, false, false)["kind"], "open");
        assert_eq!(island_key_action("o", false, false, false, false, false), Value::Null);
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(6, 4, 1, true).len(), 4);
    }
}
