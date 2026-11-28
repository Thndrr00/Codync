#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from mercury-agent's `plugins/store.ts` and `plugins/types.ts`: ids, on-disk
//! layout, compatibility, settings migration and secret masking. Composes helpers already ported
//! into this host. Only `botInsights` calls this.

use serde_json::{Map, Value, json};

/// Manifest version this Mercury reads (MANIFEST_VERSION).
pub const MANIFEST_VERSION: u64 = 1;

/// Plugin API version this Mercury provides (PLUGIN_API_VERSION).
pub const PLUGIN_API_VERSION: &str = "1.2.0";

/// Age after which a plugin lock may be taken over (LOCK_STALE_MS).
pub const LOCK_STALE_MS: u64 = 10 * 60_000;

/// Ids reserved by Mercury (RESERVED_PLUGIN_IDS).
pub const RESERVED_PLUGIN_IDS: [&str; 1] = ["mcp"];

/// Providers a plugin may ask to use (PLUGIN_PROVIDER_IDS).
pub const PLUGIN_PROVIDER_IDS: [&str; 2] = ["openai", "grok"];

/// Platform keys this build supports.
pub const PLATFORM_KEYS: [&str; 4] = ["darwin-arm64", "darwin-x64", "linux-x64", "win32-x64"];

/// Plugin id pattern: letter, then 2–31 lowercase alnum or dashes, ending alnum (PLUGIN_ID_RE).
pub fn is_valid_plugin_id(id: &str) -> bool {
    let chars: Vec<char> = id.chars().collect();
    if !(3..=32).contains(&chars.len())
        || !chars[0].is_ascii_lowercase()
        || !chars[chars.len() - 1].is_ascii_alphanumeric()
    {
        return false;
    }
    chars[1..chars.len() - 1].iter().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-')
}

/// Whether an id is reserved by Mercury itself.
pub fn is_reserved_plugin_id(id: &str) -> bool {
    RESERVED_PLUGIN_IDS.contains(&id)
}

/// Why a plugin id cannot be used (assertId + reserved ids).
pub fn plugin_id_problem(id: &str) -> Option<&'static str> {
    if is_reserved_plugin_id(id) {
        Some("reserved")
    } else if !is_valid_plugin_id(id) {
        Some("invalid")
    } else {
        None
    }
}

/// Tool names a plugin declares (PLUGIN_TOOL_NAME_RE).
pub fn is_valid_tool_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_lowercase()
        && name.len() <= 30
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// A relative path inside a plugin folder: no absolute path, `..` or NUL (relPath).
pub fn is_valid_relative_path(path: &str) -> bool {
    if path.is_empty() || path.len() > 200 || path.starts_with('/') || path.contains('\0') {
        return false;
    }
    let bytes = path.as_bytes();
    let drive = bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic();
    !drive && !path.split(['/', '\\']).any(|part| part == "..")
}

/// Root of the plugin store (pluginsRoot).
pub fn plugins_root(home: &str) -> String {
    format!("{home}/plugins")
}

/// Folder of one plugin (pluginDir).
pub fn plugin_dir(home: &str, id: &str) -> String {
    format!("{}/{}", plugins_root(home), id)
}

/// Folder of one installed plugin version (pluginVersionDir).
pub fn plugin_version_dir(home: &str, id: &str, version: &str) -> String {
    format!("{}/versions/{}", plugin_dir(home, id), version)
}

/// Data folder kept across updates (pluginDataDir).
pub fn plugin_data_dir(home: &str, id: &str) -> String {
    format!("{}/data", plugin_dir(home, id))
}

/// Settings file kept across updates (pluginSettingsPath).
pub fn plugin_settings_path(home: &str, id: &str) -> String {
    format!("{}/settings.json", plugin_dir(home, id))
}

/// Scratch space for installs (stagingRoot).
pub fn staging_root(home: &str) -> String {
    format!("{}/.staging", plugins_root(home))
}

/// Backup of an older settings file (settings.vN.json).
pub fn settings_backup_path(home: &str, id: &str, version: u64) -> String {
    format!("{}/settings.v{version}.json", plugin_dir(home, id))
}

/// Coerce a version string to its numeric core (semver.coerce).
fn version_core(text: &str) -> Option<(u64, u64, u64)> {
    let digits: String =
        text.chars().skip_while(|c| !c.is_ascii_digit()).take_while(|c| c.is_ascii_digit() || *c == '.').collect();
    let parts: Vec<&str> = digits.split('.').filter(|part| !part.is_empty()).collect();
    let value = |index: usize| parts.get(index).and_then(|part| part.parse::<u64>().ok()).unwrap_or(0);
    (!parts.is_empty()).then(|| (value(0), value(1), value(2)))
}

/// Whether a version satisfies a simple semver range (semver.satisfies).
pub fn satisfies_range(version: &str, range: &str) -> bool {
    let Some(actual) = version_core(version) else {
        return false;
    };
    let range = range.trim();
    if range.is_empty() || range == "*" {
        return true;
    }
    if let Some(rest) = range.strip_prefix('^') {
        let Some(required) = version_core(rest) else {
            return false;
        };
        return actual.0 == required.0 && actual >= required;
    }
    if let Some(rest) = range.strip_prefix('~') {
        let Some(required) = version_core(rest) else {
            return false;
        };
        return actual.0 == required.0 && actual.1 == required.1 && actual >= required;
    }
    if let Some(rest) = range.strip_prefix(">=") {
        return version_core(rest).is_some_and(|required| actual >= required);
    }
    version_core(range).is_some_and(|required| actual == required)
}

/// Why a manifest cannot run here, or None (incompatibility).
pub fn incompatibility(manifest: &Value, mercury_version: &str, platform: &str) -> Option<String> {
    if let Some(version) = manifest["manifestVersion"].as_u64() {
        if version != MANIFEST_VERSION {
            return Some(format!(
                "manifest version {version} is not supported (this Mercury reads {MANIFEST_VERSION})"
            ));
        }
    }
    let mercury_range = manifest["engines"]["mercury"].as_str().unwrap_or("*");
    if !satisfies_range(mercury_version, mercury_range) {
        return Some(format!("needs Mercury {mercury_range}, this is {mercury_version}"));
    }
    let api_range = manifest["engines"]["pluginApi"].as_str().unwrap_or("^1.0.0");
    if !satisfies_range(PLUGIN_API_VERSION, api_range) {
        return Some(format!("needs plugin API {api_range}, this Mercury provides {PLUGIN_API_VERSION}"));
    }
    if let Some(platforms) = manifest["platforms"].as_array() {
        let names: Vec<&str> = platforms.iter().filter_map(Value::as_str).collect();
        if !names.contains(&platform) {
            return Some(format!("not available for {platform} (supports {})", names.join(", ")));
        }
    }
    None
}

/// Whether a plugin lock is stale (dead pid or older than 10 minutes).
pub fn is_lock_stale(age_ms: u64, pid_alive: bool, pid: i64) -> bool {
    pid <= 0 || !pid_alive || age_ms > LOCK_STALE_MS
}

/// Settings brought up to a manifest's settings version (migrateSettingValues).
pub fn migrate_setting_values(manifest: &Value, values: &Value, from: u64) -> Value {
    let mut out = values.as_object().cloned().unwrap_or_default();
    let migrations = manifest["settings"]["migrations"].as_array().cloned().unwrap_or_default();
    for step in migrations {
        let to = step["to"].as_u64().unwrap_or(0);
        if to <= from {
            continue;
        }
        if let Some(rename) = step["rename"].as_object() {
            for (old, new) in rename {
                let new_key = new.as_str().unwrap_or("");
                if let Some(value) = out.remove(old) {
                    if !out.contains_key(new_key) {
                        out.insert(new_key.to_owned(), value);
                    }
                }
            }
        }
        for key in step["remove"].as_array().cloned().unwrap_or_default() {
            if let Some(key) = key.as_str() {
                out.remove(key);
            }
        }
        if let Some(mapping) = step["mapValues"].as_object() {
            for (key, map) in mapping {
                if let Some(value) = out.get(key).and_then(Value::as_str) {
                    if let Some(replacement) = map.get(value) {
                        out.insert(key.clone(), replacement.clone());
                    }
                }
            }
        }
    }
    Value::Object(out)
}

/// Highest migration target above the saved version (migrationTarget).
pub fn migration_target(manifest: &Value, from: u64) -> Option<u64> {
    manifest["settings"]["migrations"]
        .as_array()
        .and_then(|steps| steps.iter().filter_map(|step| step["to"].as_u64().filter(|to| *to > from)).max())
}

/// Defaults overlaid by saved values (resolvedSettings).
pub fn resolved_settings(schema: &Value, saved: &Value) -> Value {
    let mut out = Map::new();
    if let Some(properties) = schema.as_object() {
        for (key, definition) in properties {
            let value = saved.get(key).cloned().or_else(|| definition.get("default").cloned()).unwrap_or(Value::Null);
            out.insert(key.clone(), value);
        }
    }
    Value::Object(out)
}

/// Secret value from the plugin key store (secretValue).
pub fn secret_value(name: &str, keys: &Value) -> Option<String> {
    keys.get(name).and_then(Value::as_str).map(str::to_owned)
}

/// Replace secret values with a marker before showing them (maskSecrets).
pub fn mask_secrets(values: &Value, secret_names: &[String]) -> Value {
    let mut out = values.as_object().cloned().unwrap_or_default();
    for name in secret_names {
        if out.contains_key(name) {
            out.insert(name.clone(), Value::String("[set]".to_owned()));
        }
    }
    Value::Object(out)
}

/// One line describing an installed plugin (pluginLine).
pub fn plugin_line(id: &str, version: &str, enabled: bool, problem: Option<&str>) -> String {
    format!("{id} {version} · {} · {}", if enabled { "on" } else { "off" }, problem.unwrap_or("ready"))
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(installed: usize, enabled: usize, problems: usize, secrets: usize) -> Vec<(&'static str, String)> {
    vec![
        ("installed", count_label(installed)),
        ("enabled", count_label(enabled)),
        ("problems", count_label(problems)),
        ("secrets", count_label(secrets)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_and_paths() {
        assert!(is_valid_plugin_id("voice-notes"));
        assert!(!is_valid_plugin_id("x"));
        assert!(!is_valid_plugin_id("Bad"));
        assert!(is_reserved_plugin_id("mcp"));
        assert_eq!(plugin_id_problem("mcp"), Some("reserved"));
        assert!(is_valid_tool_name("search_web"));
        assert!(!is_valid_tool_name("Search"));
        assert!(is_valid_relative_path("data/notes.txt"));
        assert!(!is_valid_relative_path("../etc"));
        assert!(!is_valid_relative_path("C:\\x"));
        assert_eq!(plugins_root("/home/me"), "/home/me/plugins");
        assert_eq!(plugin_version_dir("/home/me", "notes", "1.2.0"), "/home/me/plugins/notes/versions/1.2.0");
        assert_eq!(plugin_data_dir("/home/me", "notes"), "/home/me/plugins/notes/data");
        assert_eq!(plugin_settings_path("/home/me", "notes"), "/home/me/plugins/notes/settings.json");
        assert_eq!(staging_root("/home/me"), "/home/me/plugins/.staging");
        assert_eq!(settings_backup_path("/home/me", "notes", 1), "/home/me/plugins/notes/settings.v1.json");
    }

    #[test]
    fn ranges_and_compatibility() {
        assert!(satisfies_range("2.7.0", "^2.0.0"));
        assert!(!satisfies_range("3.0.0", "^2.0.0"));
        assert!(satisfies_range("1.2.5", "~1.2.0"));
        assert!(satisfies_range("2.7.0", ">=2.0.0"));
        assert_eq!(version_core("v2.7.0-beta").unwrap(), (2, 7, 0));
        let manifest = json!({"engines":{"mercury":"^2.0.0","pluginApi":"^1.0.0"},"platforms":["linux-x64"]});
        assert!(incompatibility(&manifest, "2.7.0", "linux-x64").is_none());
        assert!(incompatibility(&manifest, "3.0.0", "linux-x64").is_some());
        assert!(incompatibility(&manifest, "2.7.0", "darwin-arm64").is_some());
        assert!(incompatibility(&json!({"manifestVersion":2}), "2.7.0", "linux-x64").is_some());
    }

    #[test]
    fn settings_and_secrets() {
        let manifest = json!({"settings":{"migrations":[
            {"to":2,"rename":{"old":"new"},"remove":["gone"],"mapValues":{"mode":{"fast":"quick"}}}
        ]}});
        let values = json!({"old":1,"gone":true,"mode":"fast"});
        let migrated = migrate_setting_values(&manifest, &values, 1);
        assert_eq!(migrated["new"], 1);
        assert!(migrated.get("old").is_none());
        assert_eq!(migrated["mode"], "quick");
        assert_eq!(migration_target(&manifest, 1), Some(2));
        assert_eq!(migration_target(&manifest, 2), None);
        let schema = json!({"city":{"default":"Lisbon"},"mode":{"default":"quick"}});
        let resolved = resolved_settings(&schema, &json!({"city":"Porto"}));
        assert_eq!(resolved["city"], "Porto");
        assert_eq!(resolved["mode"], "quick");
        let keys = json!({"API_TOKEN":"secret"});
        assert_eq!(secret_value("API_TOKEN", &keys).as_deref(), Some("secret"));
        assert_eq!(mask_secrets(&json!({"token":"abc","name":"x"}), &["token".to_owned()])["token"], "[set]");
    }

    #[test]
    fn locks_and_presentation() {
        assert!(is_lock_stale(LOCK_STALE_MS + 1, true, 123));
        assert!(is_lock_stale(10, false, 123));
        assert!(!is_lock_stale(10, true, 123));
        assert!(is_lock_stale(10, true, 0));
        assert_eq!(PLUGIN_API_VERSION, "1.2.0");
        assert_eq!(PLUGIN_PROVIDER_IDS[1], "grok");
        assert!(plugin_line("notes", "1.0.0", true, None).contains("on"));
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(3, 2, 1, 1).len(), 4);
    }
}
