#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure shaping helpers for a bot's capability profile: what it can do with the tools and
//! configuration it has. Composes the validators already in `bdots_desk`, `bdots_more`,
//! `bdots_cron` and `bdots_activity`; no second copy of any of them. The only caller is the
//! `botInsights` endpoint, so every helper is live and clients just render the result.

/// One capability row shown in the profile card.
#[derive(Clone, Debug, PartialEq)]
pub struct Capability {
    pub id: &'static str,
    pub title: String,
    pub detail: String,
    pub icon: &'static str,
    pub tone: &'static str,
    pub enabled: bool,
}

fn capability(
    id: &'static str,
    title: &str,
    detail: String,
    icon: &'static str,
    tone: &'static str,
    enabled: bool,
) -> Capability {
    Capability { id, title: title.to_owned(), detail, icon, tone, enabled }
}

/// SF Symbol for an on/off state, shared by the four clients.
pub fn state_icon(enabled: bool) -> &'static str {
    if enabled { "checkmark.circle.fill" } else { "circle" }
}

/// Semantic tone for an on/off state.
pub fn state_tone(enabled: bool) -> &'static str {
    if enabled { "good" } else { "secondary" }
}

/// Capabilities enabled in the profile.
pub fn enabled_count(caps: &[Capability]) -> usize {
    caps.iter().filter(|c| c.enabled).count()
}

/// Capabilities the bot doesn't have yet.
pub fn missing_count(caps: &[Capability]) -> usize {
    caps.len() - enabled_count(caps)
}

/// Whole percent of capabilities enabled.
pub fn coverage(caps: &[Capability]) -> u32 {
    crate::bdots_activity::percent(enabled_count(caps), caps.len())
}

/// Short bar of filled cells for the coverage strip.
pub fn coverage_bar(caps: &[Capability], width: usize) -> String {
    "▮".repeat(crate::bdots_activity::bar_width(enabled_count(caps), caps.len(), width))
}

/// One line summarising the profile.
pub fn profile_headline(caps: &[Capability]) -> String {
    format!("{} of {} capabilities on", enabled_count(caps), caps.len())
}

/// Capabilities sorted with the enabled ones first, then by title.
pub fn sort_caps(caps: &[Capability]) -> Vec<Capability> {
    let mut out = caps.to_vec();
    out.sort_by(|a, b| b.enabled.cmp(&a.enabled).then_with(|| a.title.cmp(&b.title)));
    out
}

/// The first `max` missing capabilities, for a "next to set up" strip.
pub fn next_up(caps: &[Capability], max: usize) -> Vec<Capability> {
    caps.iter().filter(|c| !c.enabled).take(max).cloned().collect()
}

/// Human label for the number of skills, using the shared compact formatter.
pub fn skill_label(skills: &[String]) -> String {
    crate::bdots_desk::compact_count(skills.len() as u64)
}

/// True when the bot can act on the computer (not a group).
pub fn can_use_computer(computer: bool) -> bool {
    computer
}

/// True when at least one connector gives the bot another account.
pub fn can_reach_apps(connectors: &[String]) -> bool {
    !connectors.is_empty()
}

/// True when the bot has any skill attached.
pub fn can_use_skills(skills: &[String]) -> bool {
    !skills.is_empty()
}

/// True when the bot has a scheduled routine (`next_runs` beyond the current time).
pub fn can_run_on_schedule(routines: usize) -> bool {
    routines > 0
}

/// True when the bot can delegate to another bot (`message_bot` capability from its skills).
pub fn can_delegate(skills: &[String]) -> bool {
    skills.iter().any(|s| {
        let lower = s.to_ascii_lowercase();
        lower.contains("bot") || lower.contains("delegate")
    })
}

/// True when the bot can browse the web (a `browser_` skill or a web connector).
pub fn can_browse(skills: &[String], connectors: &[String]) -> bool {
    skills.iter().chain(connectors.iter()).any(|s| {
        let lower = s.to_ascii_lowercase();
        lower.contains("browser") || lower.contains("web") || lower.contains("http")
    })
}

/// True when the bot can read and write email (an email connector).
pub fn can_use_mail(connectors: &[String]) -> bool {
    connectors.iter().any(|c| c.to_ascii_lowercase().contains("mail"))
}

/// True when the bot can remember things across turns (a memory skill).
pub fn can_remember(skills: &[String]) -> bool {
    skills.iter().any(|s| s.to_ascii_lowercase().contains("memor"))
}

/// True when the bot has an explicit agent backend configured.
pub fn has_backend(backend: &str) -> bool {
    !backend.trim().is_empty()
}

/// True when the bot has a model to think with.
pub fn has_model(model: Option<&str>) -> bool {
    model.is_some_and(|m| crate::bdots_desk::is_model_id(m))
}

/// True when the bot can fall back to another agent.
pub fn has_fallbacks(fallbacks: usize, self_fallbacks: bool) -> bool {
    fallbacks > 0 || self_fallbacks
}

/// True when the bot can verify its own work with a command.
pub fn can_verify(verify_command: Option<&str>) -> bool {
    verify_command.is_some_and(|c| !c.trim().is_empty())
}

/// True when the bot can run a background model on a budget.
pub fn has_background_model(background: Option<&str>, saver: bool) -> bool {
    background.is_some() || saver
}

/// True when the bot can use a browser profile for signed-in pages.
pub fn has_browser_profile(profile: Option<&str>) -> bool {
    profile.is_some_and(|p| !p.trim().is_empty())
}

/// True when the bot can request approvals with its own rules.
pub fn has_approval_rules(rules: usize) -> bool {
    rules > 0
}

/// Build every capability row from the bot's configuration.
pub fn build_caps(
    computer: bool,
    connectors: &[String],
    skills: &[String],
    routines: usize,
    backend: &str,
    model: Option<&str>,
    fallbacks: usize,
    self_fallbacks: bool,
    verify_command: Option<&str>,
    background: Option<&str>,
    saver: bool,
    browser_profile: Option<&str>,
    approval_rules: usize,
) -> Vec<Capability> {
    vec![
        capability(
            "computer",
            "Computer",
            if computer { "Files and apps on this computer" } else { "Off — chat only" }.into(),
            "desktopcomputer",
            "accent",
            can_use_computer(computer),
        ),
        capability(
            "apps",
            "Connectors",
            if can_reach_apps(connectors) {
                format!("{} connected", connectors.len())
            } else {
                "No accounts connected".into()
            },
            "plugs",
            "good",
            can_reach_apps(connectors),
        ),
        capability(
            "skills",
            "Skills",
            skill_label(skills),
            "puzzlepiece.extension",
            "secondary",
            can_use_skills(skills),
        ),
        capability(
            "routines",
            "Routines",
            if can_run_on_schedule(routines) { format!("{routines} scheduled") } else { "No scheduled work".into() },
            "calendar.badge.clock",
            "warn",
            can_run_on_schedule(routines),
        ),
        capability(
            "delegate",
            "Delegation",
            if can_delegate(skills) { "Can ask other bots" } else { "Works alone" }.into(),
            "bubble.left.and.bubble.right",
            "accent",
            can_delegate(skills),
        ),
        capability(
            "web",
            "Web",
            if can_browse(skills, connectors) { "Can open pages" } else { "No browser access" }.into(),
            "globe",
            "good",
            can_browse(skills, connectors),
        ),
        capability(
            "mail",
            "Email",
            if can_use_mail(connectors) { "Can read and send mail" } else { "No mail account" }.into(),
            "envelope",
            "secondary",
            can_use_mail(connectors),
        ),
        capability(
            "memory",
            "Memory",
            if can_remember(skills) { "Remembers across turns" } else { "No memory skill" }.into(),
            "brain",
            "warn",
            can_remember(skills),
        ),
        capability(
            "backend",
            "Agent",
            if has_backend(backend) { backend.to_owned() } else { "No agent chosen".into() },
            "terminal",
            "accent",
            has_backend(backend),
        ),
        capability(
            "model",
            "Model",
            if has_model(model) { model.unwrap_or_default().to_owned() } else { "No model chosen".into() },
            "cpu",
            "good",
            has_model(model),
        ),
        capability(
            "fallbacks",
            "Fallbacks",
            if has_fallbacks(fallbacks, self_fallbacks) { "Another agent can take over" } else { "No fallback" }.into(),
            "arrow.triangle.turn.up.right.diamond",
            "secondary",
            has_fallbacks(fallbacks, self_fallbacks),
        ),
        capability(
            "verify",
            "Verification",
            if can_verify(verify_command) { "Has a verify command" } else { "No verify command" }.into(),
            "checkmark.seal",
            "good",
            can_verify(verify_command),
        ),
        capability(
            "background",
            "Background",
            if has_background_model(background, saver) { "Can work on a budget" } else { "Foreground only" }.into(),
            "moon",
            "warn",
            has_background_model(background, saver),
        ),
        capability(
            "browserProfile",
            "Signed-in pages",
            if has_browser_profile(browser_profile) { "Has a browser profile" } else { "No browser profile" }.into(),
            "person.crop.circle.badge.checkmark",
            "secondary",
            has_browser_profile(browser_profile),
        ),
        capability(
            "rules",
            "Approval rules",
            if has_approval_rules(approval_rules) { format!("{approval_rules} rules") } else { "No rules" }.into(),
            "hand.raised",
            "accent",
            has_approval_rules(approval_rules),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_helpers_are_stable_for_all_clients() {
        assert_eq!(state_icon(true), "checkmark.circle.fill");
        assert_eq!(state_icon(false), "circle");
        assert_eq!(state_tone(true), "good");
        assert_eq!(state_tone(false), "secondary");
    }

    #[test]
    fn coverage_counts_and_bars() {
        let caps = vec![
            capability("a", "A", String::new(), "x", "good", true),
            capability("b", "B", String::new(), "x", "good", false),
            capability("c", "C", String::new(), "x", "good", true),
        ];
        assert_eq!(enabled_count(&caps), 2);
        assert_eq!(missing_count(&caps), 1);
        assert_eq!(coverage(&caps), 66);
        assert!(!coverage_bar(&caps, 8).is_empty());
        assert!(profile_headline(&caps).contains("2 of 3"));
        assert_eq!(next_up(&caps, 1)[0].id, "b");
        assert_eq!(sort_caps(&caps)[0].enabled, true);
    }

    #[test]
    fn detectors_use_real_configuration() {
        assert!(can_use_computer(true));
        assert!(can_reach_apps(&["slack".into()]));
        assert!(can_use_skills(&["a".into()]));
        assert!(can_run_on_schedule(1));
        assert!(can_delegate(&["message-bot".into()]));
        assert!(can_browse(&[], &["browser".into()]));
        assert!(can_use_mail(&["gmail".into()]));
        assert!(can_remember(&["memory".into()]));
        assert!(has_backend("claude"));
        assert!(has_model(Some("claude-sonnet-4")));
        assert!(has_fallbacks(0, true));
        assert!(can_verify(Some("npm test")));
        assert!(has_background_model(None, true));
        assert!(has_browser_profile(Some("work")));
        assert!(has_approval_rules(2));
    }

    #[test]
    fn empty_configuration_is_all_off() {
        assert!(!can_use_computer(false));
        assert!(!can_reach_apps(&[]));
        assert!(!can_use_skills(&[]));
        assert!(!can_run_on_schedule(0));
        assert!(!can_delegate(&[]));
        assert!(!can_browse(&[], &[]));
        assert!(!can_use_mail(&[]));
        assert!(!can_remember(&[]));
        assert!(!has_backend(""));
        assert!(!has_model(None));
        assert!(!has_fallbacks(0, false));
        assert!(!can_verify(None));
        assert!(!has_background_model(None, false));
        assert!(!has_browser_profile(None));
        assert!(!has_approval_rules(0));
    }

    #[test]
    fn build_covers_every_area() {
        let caps = build_caps(
            true,
            &["slack".into()],
            &["memory".into()],
            2,
            "claude",
            Some("claude-sonnet-4"),
            1,
            false,
            Some("npm test"),
            None,
            false,
            None,
            3,
        );
        assert_eq!(caps.len(), 15);
        assert!(caps.iter().any(|c| c.id == "computer" && c.enabled));
        assert!(caps.iter().any(|c| c.id == "mail" && !c.enabled));
        assert_eq!(skill_label(&["a".into(), "b".into()]), "2");
    }
}
