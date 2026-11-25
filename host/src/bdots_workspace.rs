#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers for a bot's work environment: its folder, the agent command, a verification
//! command and a browser profile. Reuses `bdots_text`, `bdots_more` and `bdots_desk` instead of
//! duplicating their formatting. Only `botInsights` calls this.

/// One workspace setting reduced to what the card shows.
#[derive(Clone, Debug, PartialEq)]
pub struct WorkspaceLine {
    pub id: &'static str,
    pub label: String,
    pub detail: String,
    pub icon: &'static str,
    pub tone: &'static str,
    pub set: bool,
}

fn line(
    id: &'static str,
    label: &str,
    detail: String,
    icon: &'static str,
    tone: &'static str,
    set: bool,
) -> WorkspaceLine {
    WorkspaceLine { id, label: label.to_owned(), detail, icon, tone, set }
}

/// Last segment of a folder path, without trailing separators.
pub fn folder_name(path: &str) -> String {
    let trimmed = path.trim_end_matches(['/', '\\']);
    let name = trimmed.rsplit(['/', '\\']).next().unwrap_or(trimmed);
    if name.is_empty() { "home".to_owned() } else { name.to_owned() }
}

/// Whether a path looks absolute.
pub fn is_absolute(path: &str) -> bool {
    path.starts_with('/') || (path.len() > 2 && path.as_bytes()[1] == b':')
}

/// Whether a path is the home directory marker.
pub fn is_home(path: &str) -> bool {
    matches!(path.trim(), "" | "~" | "/" | ".")
}

/// Short display path, reusing the shared shortener.
pub fn short_path(path: &str) -> String {
    if is_home(path) {
        return "Home folder".to_owned();
    }
    crate::bdots_more::shorten(path.trim(), 64)
}

/// SF Symbol for a folder state.
pub fn folder_icon(path: &str) -> &'static str {
    if is_home(path) {
        "house.fill"
    } else if is_absolute(path) {
        "folder.fill"
    } else {
        "folder"
    }
}

/// Semantic tone for a folder state.
pub fn folder_tone(path: &str) -> &'static str {
    if is_home(path) { "secondary" } else { "good" }
}

/// First word of a command, for a compact label.
pub fn command_name(command: &str) -> String {
    command.trim().split_whitespace().next().unwrap_or("").to_owned()
}

/// Whether a command is a shell command.
pub fn is_shell(command: &str) -> bool {
    let lower = command.to_ascii_lowercase();
    ["bash", "sh ", "zsh", "npm", "pnpm", "yarn", "make", "cargo", "python"].iter().any(|c| lower.contains(c))
}

/// Short display command, reusing the shared shortener.
pub fn short_command(command: &str) -> String {
    let clean = command.trim().split_whitespace().collect::<Vec<_>>().join(" ");
    crate::bdots_more::shorten(&clean, 64)
}

/// Whether a verify command is set.
pub fn has_verify(verify: Option<&str>) -> bool {
    verify.is_some_and(|v| !v.trim().is_empty())
}

/// SF Symbol for a verify command.
pub fn verify_icon(verify: Option<&str>) -> &'static str {
    if has_verify(verify) { "checkmark.seal.fill" } else { "checkmark.seal" }
}

/// Semantic tone for a verify command.
pub fn verify_tone(verify: Option<&str>) -> &'static str {
    if has_verify(verify) { "good" } else { "secondary" }
}

/// Whether a browser profile is set.
pub fn has_profile(profile: Option<&str>) -> bool {
    profile.is_some_and(|p| !p.trim().is_empty())
}

/// SF Symbol for a browser profile.
pub fn profile_icon(profile: Option<&str>) -> &'static str {
    if has_profile(profile) { "person.crop.circle.badge.checkmark" } else { "person.crop.circle" }
}

/// Semantic tone for a browser profile.
pub fn profile_tone(profile: Option<&str>) -> &'static str {
    if has_profile(profile) { "accent" } else { "secondary" }
}

/// Compact display for a browser profile.
pub fn profile_label(profile: Option<&str>) -> String {
    profile.filter(|p| !p.trim().is_empty()).map_or_else(|| "Default browser".to_owned(), str::to_owned)
}

/// Count of workspace settings that are explicitly set.
pub fn set_count(cwd: &str, command: Option<&str>, verify: Option<&str>, profile: Option<&str>) -> usize {
    usize::from(!is_home(cwd))
        + usize::from(command.is_some())
        + usize::from(has_verify(verify))
        + usize::from(has_profile(profile))
}

/// 0–100 completeness score.
pub fn workspace_score(cwd: &str, command: Option<&str>, verify: Option<&str>, profile: Option<&str>) -> u32 {
    (set_count(cwd, command, verify, profile) as u32) * 25
}

/// Semantic tone for a workspace score.
pub fn workspace_tone(score: u32) -> &'static str {
    if score >= 75 {
        "good"
    } else if score >= 25 {
        "attention"
    } else {
        "secondary"
    }
}

/// Short label for a workspace score.
pub fn workspace_label(score: u32) -> &'static str {
    if score >= 75 {
        "Fully set up"
    } else if score >= 25 {
        "Basic"
    } else {
        "Defaults"
    }
}

/// One plain sentence describing the workspace.
pub fn workspace_headline(cwd: &str, command: Option<&str>, verify: Option<&str>, profile: Option<&str>) -> String {
    let score = workspace_score(cwd, command, verify, profile);
    format!("{} · {}", workspace_label(score), short_path(cwd))
}

/// Build every workspace line for one bot.
pub fn build_lines(
    cwd: &str,
    command: Option<&str>,
    verify: Option<&str>,
    profile: Option<&str>,
) -> Vec<WorkspaceLine> {
    vec![
        line("cwd", "Folder", short_path(cwd), folder_icon(cwd), folder_tone(cwd), !is_home(cwd)),
        line(
            "command",
            "Agent command",
            command.map_or_else(|| "Default command".to_owned(), short_command),
            "terminal",
            "accent",
            command.is_some(),
        ),
        line(
            "verify",
            "Verify command",
            verify.map_or_else(|| "No verify command".to_owned(), short_command),
            verify_icon(verify),
            verify_tone(verify),
            has_verify(verify),
        ),
        line(
            "profile",
            "Browser",
            profile_label(profile),
            profile_icon(profile),
            profile_tone(profile),
            has_profile(profile),
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
    cwd: &str,
    command: Option<&str>,
    verify: Option<&str>,
    profile: Option<&str>,
) -> Vec<(&'static str, String)> {
    vec![
        ("folder", folder_name(cwd)),
        ("agent", command.map_or_else(|| "default".to_owned(), command_name)),
        ("score", workspace_score(cwd, command, verify, profile).to_string()),
        ("headline", workspace_headline(cwd, command, verify, profile)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folder_helpers() {
        assert_eq!(folder_name("/Users/ada/work/"), "work");
        assert_eq!(folder_name("C:\\work\\"), "work");
        assert_eq!(folder_name(""), "home");
        assert!(is_absolute("/Users/ada"));
        assert!(is_absolute("C:\\work"));
        assert!(!is_absolute("relative"));
        assert!(is_home("~"));
        assert_eq!(short_path(""), "Home folder");
        assert_eq!(folder_icon("~"), "house.fill");
        assert_eq!(folder_tone("/x"), "good");
    }

    #[test]
    fn command_helpers() {
        assert_eq!(command_name("  npm test "), "npm");
        assert!(is_shell("npm test"));
        assert!(!is_shell("echo hi"));
        assert_eq!(short_command("npm   test"), "npm test");
        assert_eq!(command_name(""), "");
    }

    #[test]
    fn verify_and_profile() {
        assert!(has_verify(Some("npm test")));
        assert!(!has_verify(None));
        assert_eq!(verify_icon(Some("x")), "checkmark.seal.fill");
        assert_eq!(verify_tone(None), "secondary");
        assert!(has_profile(Some("work")));
        assert_eq!(profile_icon(None), "person.crop.circle");
        assert_eq!(profile_tone(Some("w")), "accent");
        assert_eq!(profile_label(None), "Default browser");
    }

    #[test]
    fn counts_scores_and_labels() {
        let full = ("/work", Some("claude"), Some("npm test"), Some("work"));
        assert_eq!(set_count(full.0, full.1, full.2, full.3), 4);
        assert_eq!(workspace_score(full.0, full.1, full.2, full.3), 100);
        assert_eq!(workspace_tone(100), "good");
        assert_eq!(workspace_label(0), "Defaults");
        assert!(workspace_headline(full.0, full.1, full.2, full.3).contains("work"));
        let empty = ("", None, None, None);
        assert_eq!(workspace_score(empty.0, empty.1, empty.2, empty.3), 0);
    }

    #[test]
    fn lines_and_summary() {
        let lines = build_lines("/work", Some("claude"), None, Some("work"));
        assert_eq!(lines.len(), 4);
        assert_eq!(enabled_count(&lines), 3);
        assert_eq!(state_icon(true), "checkmark.circle.fill");
        assert_eq!(state_tone(false), "secondary");
        let rows = summary("/work", Some("claude"), None, None);
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].1, "work");
    }
}

fn enabled_count(lines: &[WorkspaceLine]) -> usize {
    lines.iter().filter(|l| l.set).count()
}
