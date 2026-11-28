#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers for a bot's agent profile: its backend account, mode and fast switch. Reuses
//! `bdots_identity`, `bdots_tuning`, `bdots_desk` and `bdots_text` instead of duplicating them.
//! Only `botInsights` calls this.

/// One agent setting reduced to what the card shows.
#[derive(Clone, Debug, PartialEq)]
pub struct AgentLine {
    pub id: &'static str,
    pub label: String,
    pub detail: String,
    pub icon: &'static str,
    pub tone: &'static str,
    pub set: bool,
}

fn line(id: &'static str, label: &str, detail: String, icon: &'static str, tone: &'static str, set: bool) -> AgentLine {
    AgentLine { id, label: label.to_owned(), detail, icon, tone, set }
}

/// Human label for an account, reusing the shared shortener.
pub fn account_label(account: Option<&str>) -> String {
    account
        .filter(|a| !a.trim().is_empty())
        .map(|a| crate::bdots_more::shorten(a, 48))
        .unwrap_or_else(|| "Default account".to_owned())
}

/// SF Symbol for an account.
pub fn account_icon(account: Option<&str>) -> &'static str {
    if account.is_some_and(|a| !a.trim().is_empty()) {
        "person.crop.circle.fill.badge.checkmark"
    } else {
        "person.crop.circle"
    }
}

/// Semantic tone for an account.
pub fn account_tone(account: Option<&str>) -> &'static str {
    if account.is_some_and(|a| !a.trim().is_empty()) { "good" } else { "secondary" }
}

/// Whether an account is an email address.
pub fn is_email_account(account: &str) -> bool {
    account.contains('@') && account.contains('.')
}

/// Backend name from an account id (before any colon).
pub fn account_backend(account: &str) -> String {
    account.split(':').next().unwrap_or(account).to_owned()
}

/// Human label for a mode, reusing the identity helper.
pub fn mode_label(mode: Option<&str>) -> String {
    crate::bdots_identity::mode_label(mode)
}

/// SF Symbol for a mode.
pub fn mode_icon(mode: Option<&str>) -> &'static str {
    match mode.unwrap_or("").to_ascii_lowercase().as_str() {
        "plan" => "list.bullet.rectangle",
        "auto" => "bolt.circle",
        _ => "gearshape",
    }
}

/// Semantic tone for a mode.
pub fn mode_tone(mode: Option<&str>) -> &'static str {
    match mode.unwrap_or("").to_ascii_lowercase().as_str() {
        "plan" => "accent",
        "auto" => "good",
        _ => "secondary",
    }
}

/// Whether fast mode is on, reusing the tuning helper.
pub fn fast_on(fast: bool) -> bool {
    fast
}

/// SF Symbol for fast mode.
pub fn fast_icon(fast: bool) -> &'static str {
    if fast { "bolt.circle.fill" } else { "bolt.circle" }
}

/// Semantic tone for fast mode.
pub fn fast_tone(fast: bool) -> &'static str {
    if fast { "good" } else { "secondary" }
}

/// Whether the bot came from the marketplace catalogue.
pub fn from_catalog(catalog: Option<&str>) -> bool {
    crate::bdots_identity::from_catalog(catalog)
}

/// Display label for a catalogue entry.
pub fn catalog_label(catalog: Option<&str>) -> String {
    crate::bdots_identity::catalog_label(catalog)
}

/// SF Symbol for a catalogue entry.
pub fn catalog_icon(catalog: Option<&str>) -> &'static str {
    if from_catalog(catalog) { "bag.fill" } else { "hammer" }
}

/// Semantic tone for a catalogue entry.
pub fn catalog_tone(catalog: Option<&str>) -> &'static str {
    if from_catalog(catalog) { "good" } else { "secondary" }
}

/// Count of explicitly set agent settings.
pub fn set_count(account: Option<&str>, mode: Option<&str>, fast: bool, catalog: Option<&str>) -> usize {
    usize::from(account.is_some_and(|a| !a.trim().is_empty()))
        + usize::from(mode.is_some_and(|m| !m.trim().is_empty()))
        + usize::from(fast)
        + usize::from(from_catalog(catalog))
}

/// 0–100 profile score.
pub fn agent_score(account: Option<&str>, mode: Option<&str>, fast: bool, catalog: Option<&str>) -> u32 {
    (set_count(account, mode, fast, catalog) as u32) * 25
}

/// Semantic tone for the profile score.
pub fn agent_tone(score: u32) -> &'static str {
    if score >= 75 {
        "good"
    } else if score >= 25 {
        "attention"
    } else {
        "secondary"
    }
}

/// Short label for the profile score.
pub fn agent_label(score: u32) -> &'static str {
    if score >= 75 {
        "Configured"
    } else if score >= 25 {
        "Basic"
    } else {
        "Defaults"
    }
}

/// One plain sentence describing the agent profile.
pub fn agent_headline(account: Option<&str>, mode: Option<&str>, fast: bool, catalog: Option<&str>) -> String {
    let score = agent_score(account, mode, fast, catalog);
    format!("{} · {}", agent_label(score), mode_label(mode))
}

/// Build every agent line for one bot.
pub fn build_lines(account: Option<&str>, mode: Option<&str>, fast: bool, catalog: Option<&str>) -> Vec<AgentLine> {
    vec![
        line(
            "account",
            "Account",
            account_label(account),
            account_icon(account),
            account_tone(account),
            account.is_some_and(|a| !a.trim().is_empty()),
        ),
        line(
            "mode",
            "Mode",
            mode_label(mode),
            mode_icon(mode),
            mode_tone(mode),
            mode.is_some_and(|m| !m.trim().is_empty()),
        ),
        line("fast", "Fast mode", if fast { "On" } else { "Off" }.into(), fast_icon(fast), fast_tone(fast), fast),
        line(
            "catalog",
            "Source",
            catalog_label(catalog),
            catalog_icon(catalog),
            catalog_tone(catalog),
            from_catalog(catalog),
        ),
    ]
}

/// SF Symbol for a set/unset state.
pub fn state_icon(set: bool) -> &'static str {
    if set { "checkmark.circle.fill" } else { "circle" }
}

/// Semantic tone for a set/unset state.
pub fn state_tone(set: bool) -> &'static str {
    if set { "good" } else { "secondary" }
}

/// Summary rows for the card.
pub fn summary(
    account: Option<&str>,
    mode: Option<&str>,
    fast: bool,
    catalog: Option<&str>,
) -> Vec<(&'static str, String)> {
    vec![
        ("backend", account.map_or_else(|| "default".to_owned(), account_backend)),
        ("mode", mode_label(mode)),
        ("score", agent_score(account, mode, fast, catalog).to_string()),
        ("headline", agent_headline(account, mode, fast, catalog)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_helpers() {
        assert_eq!(account_label(None), "Default account");
        assert_eq!(account_label(Some("ada@example.com")), "ada@example.com");
        assert_eq!(account_icon(None), "person.crop.circle");
        assert_eq!(account_tone(Some("a")), "good");
        assert!(is_email_account("ada@example.com"));
        assert!(!is_email_account("ada"));
        assert_eq!(account_backend("claude:ada"), "claude");
    }

    #[test]
    fn mode_helpers() {
        assert_eq!(mode_label(None), "Default");
        assert_eq!(mode_icon(Some("plan")), "list.bullet.rectangle");
        assert_eq!(mode_icon(Some("auto")), "bolt.circle");
        assert_eq!(mode_tone(Some("plan")), "accent");
        assert_eq!(mode_tone(None), "secondary");
    }

    #[test]
    fn fast_catalog_and_scores() {
        assert!(fast_on(true));
        assert_eq!(fast_icon(true), "bolt.circle.fill");
        assert_eq!(fast_tone(false), "secondary");
        assert!(from_catalog(Some("bot")));
        assert_eq!(catalog_icon(Some("bot")), "bag.fill");
        assert_eq!(catalog_tone(None), "secondary");
        let full = (Some("claude:ada"), Some("auto"), true, Some("bot"));
        assert_eq!(set_count(full.0, full.1, full.2, full.3), 4);
        assert_eq!(agent_score(full.0, full.1, full.2, full.3), 100);
        assert_eq!(agent_tone(100), "good");
        assert_eq!(agent_label(0), "Defaults");
        assert!(agent_headline(full.0, full.1, full.2, full.3).contains("Default"));
    }

    #[test]
    fn lines_and_summary() {
        let l = build_lines(Some("claude:ada"), None, true, None);
        assert_eq!(l.len(), 4);
        assert_eq!(enabled_count(&l), 2);
        assert_eq!(state_icon(true), "checkmark.circle.fill");
        assert_eq!(state_tone(false), "secondary");
        let rows = summary(Some("claude:ada"), None, true, None);
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].1, "claude");
    }
}

fn enabled_count(lines: &[AgentLine]) -> usize {
    lines.iter().filter(|l| l.set).count()
}
