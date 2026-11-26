#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from agentes-chat's `packages/i18n` (`locale.ts`, `plurals.ts`): the app's
//! locale registry, tag resolution, week start, scripts and plural gaps. Composes helpers already
//! ported into this host. Only `botInsights` calls this.

/// Source and default locale (DEFAULT_LOCALE).
pub const DEFAULT_LOCALE: &str = "en";

/// Every supported locale code, in registry order (LOCALE_CODES).
pub const LOCALE_CODES: [&str; 14] =
    ["en", "pt-BR", "es", "fr", "de", "it", "nl", "ja", "ko", "zh-Hans", "id", "tr", "pl", "ru"];

/// Whether a code is in the locale registry (isAppLocale).
pub fn is_app_locale(code: &str) -> bool {
    LOCALE_CODES.contains(&code)
}

/// Native name of a locale (LOCALES[].native).
pub fn locale_native(code: &str) -> &'static str {
    match code {
        "pt-BR" => "Português (Brasil)",
        "es" => "Español",
        "fr" => "Français",
        "de" => "Deutsch",
        "it" => "Italiano",
        "nl" => "Nederlands",
        "ja" => "日本語",
        "ko" => "한국어",
        "zh-Hans" => "简体中文",
        "id" => "Bahasa Indonesia",
        "tr" => "Türkçe",
        "pl" => "Polski",
        "ru" => "Русский",
        _ => "English",
    }
}

/// English name of a locale (LOCALES[].english).
pub fn locale_english(code: &str) -> &'static str {
    match code {
        "pt-BR" => "Portuguese (Brazil)",
        "es" => "Spanish",
        "fr" => "French",
        "de" => "German",
        "it" => "Italian",
        "nl" => "Dutch",
        "ja" => "Japanese",
        "ko" => "Korean",
        "zh-Hans" => "Chinese (Simplified)",
        "id" => "Indonesian",
        "tr" => "Turkish",
        "pl" => "Polish",
        "ru" => "Russian",
        _ => "English",
    }
}

/// Intl tag used to format dates and numbers for a locale (LOCALES[].intl).
pub fn locale_intl(code: &str) -> &'static str {
    match code {
        "pt-BR" => "pt-BR",
        "fr" => "fr-FR",
        "de" => "de-DE",
        "it" => "it-IT",
        "nl" => "nl-NL",
        "ja" => "ja-JP",
        "ko" => "ko-KR",
        "zh-Hans" => "zh-CN",
        "id" => "id-ID",
        "tr" => "tr-TR",
        "pl" => "pl-PL",
        "ru" => "ru-RU",
        _ => "en-US",
    }
}

/// First day of the week: 0 Sunday, 1 Monday (LOCALES[].weekStart).
pub fn week_start(code: &str) -> u8 {
    if matches!(code, "es" | "fr" | "de" | "it" | "nl" | "tr" | "pl" | "ru") { 1 } else { 0 }
}

/// Human label for a locale's week start.
pub fn week_start_label(code: &str) -> &'static str {
    if week_start(code) == 1 { "Monday" } else { "Sunday" }
}

/// Script used for a locale's typography rules (LOCALES[].script).
pub fn script_of(code: &str) -> &'static str {
    match code {
        "ja" | "ko" | "zh-Hans" => "cjk",
        "ru" => "cyrillic",
        _ => "latin",
    }
}

/// Human label for a script.
pub fn script_label(script: &str) -> &'static str {
    match script {
        "cjk" => "CJK",
        "cyrillic" => "Cyrillic",
        "arabic" | "hebrew" => script,
        _ => "Latin",
    }
}

/// Whether a locale uses a CJK script.
pub fn is_cjk(code: &str) -> bool {
    script_of(code) == "cjk"
}

/// Whether a locale is selectable in the app (all registry entries are ready).
pub fn is_ready(code: &str) -> bool {
    is_app_locale(code)
}

/// Canonical form of a BCP-47 tag, replacing `_` with `-` and lowering case.
pub fn canonical_tag(tag: &str) -> String {
    tag.trim().replace('_', "-").to_ascii_lowercase()
}

/// The app locale a browser tag maps to (tagLocale): pt-PT -> pt-BR, zh-* -> zh-Hans, in -> id.
pub fn tag_locale(tag: &str) -> Option<&'static str> {
    let base = canonical_tag(tag).split('-').next().unwrap_or("").to_owned();
    match base.as_str() {
        "pt" => Some("pt-BR"),
        "zh" => Some("zh-Hans"),
        "in" => Some("id"),
        other if is_app_locale(other) => LOCALE_CODES.iter().copied().find(|code| *code == other),
        _ => None,
    }
}

/// First preferred tag that maps to an app locale (appLocale).
pub fn app_locale(tags: &[&str]) -> Option<&'static str> {
    tags.iter().find_map(|tag| tag_locale(tag))
}

/// Intl tag to format with, preferring a browser tag of the same language (intlLocale).
pub fn intl_locale(code: &str, browser_tags: &[&str]) -> String {
    browser_tags
        .iter()
        .find(|tag| tag_locale(tag) == Some(code))
        .map(|tag| canonical_tag(tag))
        .unwrap_or_else(|| locale_intl(code).to_owned())
}

/// Text direction of a script (latin/cjk/cyrillic are LTR).
pub fn script_direction(script: &str) -> &'static str {
    if matches!(script.to_ascii_lowercase().as_str(), "arabic" | "hebrew") { "rtl" } else { "ltr" }
}

/// Whether a locale writes right-to-left.
pub fn is_rtl(code: &str) -> bool {
    script_direction(script_of(code)) == "rtl"
}

/// Whether CLDR's `many` reads the same as `other` here (MANY_IS_OTHER).
pub fn many_is_other(code: &str) -> bool {
    matches!(code, "es" | "fr" | "it" | "pt-BR")
}

/// Whether this locale fills a `zero` form from `_other` (pt-BR).
pub fn has_plural_zero(code: &str) -> bool {
    code == "pt-BR"
}

/// Plural categories a locale actually distinguishes.
pub fn plural_categories(code: &str) -> &'static [&'static str] {
    match code {
        "pt-BR" => &["zero", "one", "other"],
        "ru" | "pl" => &["one", "few", "many", "other"],
        "ja" | "ko" | "zh-Hans" => &["other"],
        _ => &["one", "other"],
    }
}

/// CLDR-style plural category for a count in a locale.
pub fn plural_key(code: &str, count: u64) -> &'static str {
    match code {
        "pt-BR" if count == 0 => "zero",
        "fr" if count < 2 => "one",
        "ja" | "ko" | "zh-Hans" => "other",
        "ru" => {
            let last = count % 10;
            let pair = count % 100;
            if last == 1 && pair != 11 {
                "one"
            } else if (2..=4).contains(&last) && !(12..=14).contains(&pair) {
                "few"
            } else {
                "many"
            }
        }
        "pl" => {
            let last = count % 10;
            let pair = count % 100;
            if count == 1 {
                "one"
            } else if (2..=4).contains(&last) && !(12..=14).contains(&pair) {
                "few"
            } else {
                "many"
            }
        }
        _ if count == 1 => "one",
        _ => "other",
    }
}

/// Fill a missing plural form from `_other`, so translators may leave gaps (fillPluralGaps).
pub fn plural_form(code: &str, category: &str, other: &str) -> String {
    if plural_categories(code).contains(&category) { category.to_owned() } else { other.to_owned() }
}

/// A label combining the native and English names of a locale.
pub fn locale_label(code: &str) -> String {
    format!("{} · {}", locale_native(code), locale_english(code))
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(code: &str, tag: &str, plural: &str) -> Vec<(&'static str, String)> {
    vec![
        ("locale", locale_native(code).to_owned()),
        ("intl", locale_intl(code).to_owned()),
        ("week", week_start_label(code).to_owned()),
        ("plural", crate::bdots_textfmt::upper_first(plural)),
        ("from", tag.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry() {
        assert!(is_app_locale("pt-BR"));
        assert!(!is_app_locale("pt-PT"));
        assert_eq!(LOCALE_CODES.len(), 14);
        assert_eq!(locale_native("ja"), "日本語");
        assert_eq!(locale_english("es"), "Spanish");
        assert_eq!(locale_intl("de"), "de-DE");
        assert_eq!(week_start("es"), 1);
        assert_eq!(week_start_label("en"), "Sunday");
        assert_eq!(script_of("ru"), "cyrillic");
        assert_eq!(script_label("cjk"), "CJK");
        assert!(is_cjk("zh-Hans"));
        assert!(is_ready("fr"));
    }

    #[test]
    fn tags() {
        assert_eq!(canonical_tag("pt_PT"), "pt-pt");
        assert_eq!(tag_locale("pt-PT"), Some("pt-BR"));
        assert_eq!(tag_locale("zh-TW"), Some("zh-Hans"));
        assert_eq!(tag_locale("in"), Some("id"));
        assert_eq!(tag_locale("xx"), None);
        assert_eq!(app_locale(&["xx", "es-MX", "fr"]), Some("es"));
        assert_eq!(intl_locale("en", &["en-GB"]), "en-gb");
        assert_eq!(intl_locale("pt-BR", &["en-US"]), "pt-BR");
    }

    #[test]
    fn scripts_and_direction() {
        assert_eq!(script_direction("latin"), "ltr");
        assert_eq!(script_direction("Arabic"), "rtl");
        assert!(!is_rtl("ru"));
        assert!(many_is_other("pt-BR"));
        assert!(!many_is_other("ru"));
        assert!(has_plural_zero("pt-BR"));
        assert!(!has_plural_zero("en"));
    }

    #[test]
    fn plurals() {
        assert_eq!(plural_key("en", 1), "one");
        assert_eq!(plural_key("en", 2), "other");
        assert_eq!(plural_key("pt-BR", 0), "zero");
        assert_eq!(plural_key("fr", 0), "one");
        assert_eq!(plural_key("ja", 5), "other");
        assert_eq!(plural_key("ru", 3), "few");
        assert_eq!(plural_key("ru", 11), "many");
        assert_eq!(plural_key("pl", 1), "one");
        assert_eq!(plural_categories("ru").len(), 4);
        assert_eq!(plural_form("pt-BR", "many", "other"), "other");
        assert_eq!(plural_form("ru", "few", "other"), "few");
    }

    #[test]
    fn labels_and_summary() {
        assert!(locale_label("fr").contains("Français"));
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary("fr", "fr-CA", "one").len(), 5);
    }
}
