#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from coucou's `whats-new/whats-new.ts`: version comparison, release lookup,
//! localized text, icon names, launch decisions and update hints. Only `botInsights` calls this.

use serde_json::{Value, json};

/// Icons an entry may use (ICON_NAMES).
pub const ICON_NAMES: [&str; 20] = [
    "update",
    "languages",
    "terminal",
    "checklist",
    "bell",
    "chat",
    "mic",
    "phone",
    "shield",
    "sparkles",
    "bolt",
    "puzzle",
    "paint",
    "eye",
    "keyboard",
    "heart",
    "clock",
    "download",
    "bug",
    "gear",
];

/// Delay before asking again (LATER_MS).
pub const LATER_MS: u64 = 24 * 3_600 * 1_000;

/// Blue of the waiting Mochi (UPDATE_BLUE).
pub const UPDATE_BLUE: &str = "#3B82F6";

/// Coffee support URL (COFFEE_URL).
pub const COFFEE_URL: &str = "https://buy.stripe.com/dRm6oGbfa7Kqcqs0hxes003";

/// Kinds of stats in a release card (StatKind).
pub const STAT_KINDS: [&str; 7] = ["agentSessions", "agentTime", "linesAdded", "approvals", "words", "saved", "calls"];

/// Compare two numeric versions: -1, 0 or 1 (compareVersions).
pub fn compare_versions(a: &str, b: &str) -> i32 {
    let parts = |text: &str| -> Vec<i64> { text.split('.').map(|part| part.parse::<i64>().unwrap_or(0)).collect() };
    let x = parts(a);
    let y = parts(b);
    for index in 0..x.len().max(y.len()) {
        let left = x.get(index).copied().unwrap_or(0);
        let right = y.get(index).copied().unwrap_or(0);
        if left != right {
            return if left > right { 1 } else { -1 };
        }
    }
    0
}

/// Entry of an exact version, or null (releaseFor).
pub fn release_for(file: Option<&Value>, version: &str) -> Value {
    let Some(releases) = file.and_then(|value| value.get("releases")).and_then(Value::as_array) else {
        return Value::Null;
    };
    releases
        .iter()
        .find(|release| compare_versions(release["version"].as_str().unwrap_or(""), version) == 0)
        .filter(|release| release["items"].as_array().is_some_and(|items| !items.is_empty()))
        .cloned()
        .unwrap_or(Value::Null)
}

/// Text in a language, else English, else empty (localized).
pub fn localized(texts: Option<&Value>, lang: &str) -> String {
    let Some(texts) = texts.and_then(Value::as_object) else {
        return String::new();
    };
    texts.get(lang).and_then(Value::as_str).or_else(|| texts.get("en").and_then(Value::as_str)).unwrap_or("").to_owned()
}

/// Known icon name, falling back to "update" (iconName).
pub fn icon_name(name: &str) -> &'static str {
    ICON_NAMES.iter().find(|candidate| **candidate == name).copied().unwrap_or("update")
}

/// Whether an update card should open on launch (decideOnLaunch).
pub fn decide_on_launch(seen: Option<&str>, current: &str, just_updated: bool, dismissed: bool) -> bool {
    just_updated && !dismissed && seen != Some(current)
}

/// Stats computed so far from local numbers (soFar).
pub fn so_far(turns: usize, seconds: i64, lines_added: i64, approvals: usize, calls: usize) -> Value {
    json!({
        "agentSessions": turns,
        "agentTime": seconds,
        "linesAdded": lines_added,
        "approvals": approvals,
        "calls": calls,
    })
}

/// Whether a "later" decision has expired (usedBefore).
pub fn used_before(later_ms: Option<u64>, now_ms: u64) -> bool {
    later_ms.is_some_and(|then| now_ms < then)
}

/// Next "later" timestamp, a day from now (later_at).
pub fn later_at(now_ms: u64) -> u64 {
    now_ms + LATER_MS
}

/// Whether the update hint is waiting (updateWaiting).
pub fn update_waiting(status: &str, later_ms: Option<u64>, now_ms: u64) -> bool {
    status == "available" && used_before(later_ms, now_ms)
}

/// Whether the blue Mochi shows a hint (showsUpdateHint).
pub fn shows_update_hint(waiting: bool, state: &str, pending: bool) -> bool {
    waiting && state != "dizzy" && !pending
}

/// Release notes page for a platform (releasePage).
pub fn release_page(platform: &str, version: &str) -> String {
    let slug = if platform == "linux" { "linux" } else { "windows" };
    format!("https://coucou.app/whats-new/{slug}/{version}")
}

/// Title of the first item of a release (releaseTitle).
pub fn release_title(release: &Value, lang: &str) -> String {
    release["items"]
        .as_array()
        .and_then(|items| items.first())
        .map(|item| localized(Some(&item["title"]), lang))
        .unwrap_or_default()
}

/// Number of entries in a release (releaseCount).
pub fn release_count(release: &Value) -> usize {
    release["items"].as_array().map_or(0, Vec::len)
}

/// One line describing a release (whatsNewLine).
pub fn whats_new_line(version: &str, lang: &str, items: usize) -> String {
    format!("{version} · {lang} · {items} items")
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(icons: usize, stats: usize, items: usize, pending: bool) -> Vec<(&'static str, String)> {
    vec![
        ("icons", count_label(icons)),
        ("stats", count_label(stats)),
        ("items", count_label(items)),
        ("pending", if pending { "yes" } else { "no" }.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_and_releases() {
        assert_eq!(ICON_NAMES.len(), 20);
        assert_eq!(STAT_KINDS.len(), 7);
        assert_eq!(LATER_MS, 86_400_000);
        assert_eq!(UPDATE_BLUE, "#3B82F6");
        assert_eq!(compare_versions("0.10.0", "0.9.3"), 1);
        assert_eq!(compare_versions("0.5", "0.5.0"), 0);
        assert_eq!(compare_versions("1.2.3", "1.3.0"), -1);
        let file = json!({"releases":[
            {"version":"1.2.0","items":[{"title":{"en":"New","es":"Nuevo"}}]},
            {"version":"1.1.0","items":[]},
        ]});
        let release = release_for(Some(&file), "1.2.0");
        assert_eq!(release_count(&release), 1);
        assert_eq!(release_title(&release, "es"), "Nuevo");
        assert_eq!(release_title(&release, "fr"), "New");
        assert_eq!(release_for(Some(&file), "1.1.0"), Value::Null);
        assert_eq!(release_for(None, "1.2.0"), Value::Null);
    }

    #[test]
    fn text_and_icons() {
        assert_eq!(localized(Some(&json!({"en":"Hi","es":"Hola"})), "es"), "Hola");
        assert_eq!(localized(Some(&json!({"en":"Hi"})), "fr"), "Hi");
        assert_eq!(localized(None, "es"), "");
        assert_eq!(icon_name("sparkles"), "sparkles");
        assert_eq!(icon_name("unknown"), "update");
        assert_eq!(release_page("linux", "1.2.0"), "https://coucou.app/whats-new/linux/1.2.0");
    }

    #[test]
    fn launch_and_hints() {
        assert!(decide_on_launch(Some("1.1.0"), "1.2.0", true, false));
        assert!(!decide_on_launch(Some("1.2.0"), "1.2.0", true, false));
        assert!(!decide_on_launch(None, "1.2.0", false, false));
        assert!(!decide_on_launch(None, "1.2.0", true, true));
        let stats = so_far(3, 120, 45, 2, 8);
        assert_eq!(stats["agentSessions"], 3);
        assert_eq!(stats["agentTime"], 120);
        assert!(used_before(Some(10_000), 5_000));
        assert!(!used_before(Some(10_000), 20_000));
        assert_eq!(later_at(0), LATER_MS);
        assert!(update_waiting("available", Some(later_at(0)), 1_000));
        assert!(!update_waiting("idle", Some(later_at(0)), 1_000));
        assert!(shows_update_hint(true, "working", false));
        assert!(!shows_update_hint(true, "dizzy", false));
        assert!(!shows_update_hint(true, "working", true));
    }

    #[test]
    fn presentation() {
        assert!(whats_new_line("1.2.0", "es", 3).contains("1.2.0"));
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(20, 7, 3, false).len(), 4);
    }
}
