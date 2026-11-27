#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from hermes-agent's `verification_stop.py`: verification policy, changed
//! path classification and bounded nudge formatting. Composes helpers already ported into this
//! host. Only `botInsights` calls this.

/// Maximum changed paths listed in a nudge.
pub const MAX_CHANGED_PATHS: usize = 8;

/// Maximum characters of a status summary.
pub const MAX_STATUS_CHARS: usize = 1200;

/// Maximum characters of an unattributed command excerpt.
pub const MAX_COMMAND_CHARS: usize = 200;

/// Prose/data extensions with nothing to verify.
pub const NON_CODE_EXTENSIONS: [&str; 12] =
    [".md", ".markdown", ".mdx", ".rst", ".txt", ".text", ".adoc", ".asciidoc", ".org", ".log", ".csv", ".tsv"];

/// Extension-less prose filenames with no verifiable behavior.
pub const NON_CODE_FILENAMES: [&str; 7] =
    ["license", "licence", "notice", "authors", "contributors", "changelog", "codeowners"];

/// Falsy tokens for config/env values.
pub const FALSY_TOKENS: [&str; 4] = ["0", "false", "no", "off"];

/// Truthy tokens for config/env values.
pub const TRUTHY_TOKENS: [&str; 4] = ["1", "true", "yes", "on"];

/// File extension in lower case, or empty.
pub fn extension_of(path: &str) -> String {
    let name = path.rsplit('/').next().unwrap_or(path);
    match name.rsplit_once('.') {
        Some((_, ext)) if !ext.is_empty() => format!(".{}", ext.to_ascii_lowercase()),
        _ => String::new(),
    }
}

/// Base name of a path in lower case.
pub fn basename_of(path: &str) -> String {
    path.rsplit('/').next().unwrap_or(path).to_ascii_lowercase()
}

/// Whether a changed path is documentation/prose with nothing to verify (_isNonCodePath).
pub fn is_non_code_path(path: &str) -> bool {
    let ext = extension_of(path);
    !ext.is_empty() && NON_CODE_EXTENSIONS.contains(&ext.as_str())
        || (ext.is_empty() && NON_CODE_FILENAMES.contains(&basename_of(path).as_str()))
}

/// Whether a turn touched only prose, so verification is suppressed.
pub fn all_non_code(paths: &[String]) -> bool {
    !paths.is_empty() && paths.iter().all(|path| is_non_code_path(path))
}

/// Resolve an env/config token to a boolean (verifyOnStopEnabled).
pub fn token_enabled(token: Option<&str>, messaging_surface: bool) -> bool {
    let Some(token) = token else {
        return false;
    };
    let lower = token.trim().to_ascii_lowercase();
    if FALSY_TOKENS.contains(&lower.as_str()) {
        return false;
    }
    if lower == "auto" {
        return !messaging_surface;
    }
    TRUTHY_TOKENS.contains(&lower.as_str()) || !lower.is_empty()
}

/// Distinct parent directories of changed paths, in order (_candidateCwds).
pub fn candidate_dirs(paths: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for path in paths.iter().filter(|p| !p.trim().is_empty()) {
        let dir = match path.rsplit_once('/') {
            Some((parent, _)) if !parent.is_empty() => parent.to_owned(),
            _ => ".".to_owned(),
        };
        if !out.contains(&dir) {
            out.push(dir);
        }
    }
    out
}

/// Format changed paths for a nudge, bounded to MAX_CHANGED_PATHS (_formatChangedPaths).
pub fn format_changed_paths(paths: &[String]) -> String {
    let mut out: Vec<String> = paths
        .iter()
        .take(MAX_CHANGED_PATHS)
        .map(|path| format!("- {}", crate::bdots_more::shorten(path, 120)))
        .collect();
    if paths.len() > MAX_CHANGED_PATHS {
        out.push(format!("- … {} more", paths.len() - MAX_CHANGED_PATHS));
    }
    out.join("\n")
}

/// Bounded status summary for a nudge.
pub fn status_summary(status: &str) -> String {
    crate::bdots_more::shorten(status.trim(), MAX_STATUS_CHARS)
}

/// Bounded command excerpt for an unattributed run warning.
pub fn command_excerpt(command: &str) -> String {
    crate::bdots_more::shorten(command.trim(), MAX_COMMAND_CHARS)
}

/// Warning text for a check whose exit status could not be attributed.
pub fn unattributed_warning(command: &str) -> String {
    if command.trim().is_empty() {
        return String::new();
    }
    format!("Your last check run (`{}`) did not count; run it directly.", command_excerpt(command))
}

/// Whether a status entry counts as passing evidence.
pub fn is_passing(status: &str) -> bool {
    matches!(status.to_ascii_lowercase().as_str(), "passed" | "ok" | "clean")
}

/// Whether a status entry counts as failing evidence.
pub fn is_failing(status: &str) -> bool {
    matches!(status.to_ascii_lowercase().as_str(), "failed" | "error" | "dirty")
}

/// Evidence state for a status string.
pub fn evidence_state(status: &str) -> &'static str {
    if is_passing(status) {
        "passed"
    } else if is_failing(status) {
        "failed"
    } else {
        "unknown"
    }
}

/// SF Symbol for an evidence state.
pub fn evidence_icon(state: &str) -> &'static str {
    match state {
        "passed" => "checkmark.circle.fill",
        "failed" => "xmark.circle.fill",
        _ => "questionmark.circle",
    }
}

/// Semantic tone for an evidence state.
pub fn evidence_tone(state: &str) -> &'static str {
    match state {
        "passed" => "good",
        "failed" => "attention",
        _ => "secondary",
    }
}

/// Whether a nudge should be shown after an edit.
pub fn should_nudge(enabled: bool, prose_only: bool, evidence: &str) -> bool {
    enabled && !prose_only && evidence != "passed"
}

/// Human label of a nudge decision.
pub fn nudge_label(nudge: bool) -> &'static str {
    if nudge { "Verify" } else { "Skip" }
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(paths: usize, dirs: usize, evidence: &str, nudge: bool) -> Vec<(&'static str, String)> {
    vec![
        ("paths", count_label(paths)),
        ("dirs", count_label(dirs)),
        ("evidence", crate::bdots_textfmt::upper_first(evidence)),
        ("nudge", nudge_label(nudge).to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths() {
        assert_eq!(extension_of("docs/README.md"), ".md");
        assert_eq!(extension_of("LICENSE"), "");
        assert_eq!(basename_of("docs/README.md"), "readme.md");
        assert!(is_non_code_path("docs/README.md"));
        assert!(is_non_code_path("LICENSE"));
        assert!(!is_non_code_path("src/main.rs"));
        assert!(all_non_code(&["a.md".to_owned(), "CHANGELOG".to_owned()]));
        assert!(!all_non_code(&["a.md".to_owned(), "x.rs".to_owned()]));
    }

    #[test]
    fn policy() {
        assert!(token_enabled(Some("yes"), false));
        assert!(!token_enabled(Some("off"), false));
        assert!(token_enabled(Some("auto"), false));
        assert!(!token_enabled(Some("auto"), true));
        assert!(!token_enabled(None, false));
    }

    #[test]
    fn dirs_and_format() {
        assert_eq!(candidate_dirs(&["a/x.rs".to_owned(), "a/y.rs".to_owned(), "b/z.rs".to_owned()]), vec!["a", "b"]);
        let paths: Vec<String> = (0..10).map(|i| format!("f{i}.rs")).collect();
        let formatted = format_changed_paths(&paths);
        assert!(formatted.contains("more"));
        assert!(status_summary("  ok  ").contains("ok"));
        assert!(!command_excerpt("cargo test").is_empty());
        assert!(unattributed_warning("cargo test | tail").contains("did not count"));
    }

    #[test]
    fn evidence_and_nudge() {
        assert!(is_passing("passed"));
        assert!(is_failing("failed"));
        assert_eq!(evidence_state("ok"), "passed");
        assert_eq!(evidence_state("unknown"), "unknown");
        assert_eq!(evidence_icon("failed"), "xmark.circle.fill");
        assert_eq!(evidence_tone("passed"), "good");
        assert!(should_nudge(true, false, "unknown"));
        assert!(!should_nudge(true, false, "passed"));
        assert!(!should_nudge(true, true, "unknown"));
        assert_eq!(nudge_label(true), "Verify");
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(4, 2, "passed", false).len(), 4);
    }
}
