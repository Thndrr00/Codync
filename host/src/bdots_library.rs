#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers for a bot's skill library: parse refs, categorise resources and describe the
//! collection. Composes `bdots_desk`, `bdots_text` and `bdots_more` rather than duplicating them.
//! Only the `botInsights` endpoint calls this, so every helper is live.

/// A skill reduced to what the library card shows.
#[derive(Clone, Debug, PartialEq)]
pub struct SkillLine {
    pub name: String,
    pub kind: &'static str,
    pub icon: &'static str,
    pub tone: &'static str,
    pub detail: String,
}

/// Symbol for a resource kind, shared by the four clients.
pub fn kind_icon(kind: &str) -> &'static str {
    match kind {
        "script" => "terminal",
        "reference" => "book",
        "asset" => "photo",
        "file" => "doc",
        _ => "puzzlepiece.extension",
    }
}

/// Semantic tone for a resource kind.
pub fn kind_tone(kind: &str) -> &'static str {
    match kind {
        "script" => "accent",
        "reference" => "good",
        "asset" => "warn",
        "file" => "secondary",
        _ => "secondary",
    }
}

/// Short human title for a skill path: last segment, without extension.
pub fn skill_title(path: &str) -> String {
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    name.rsplit_once('.').map_or(name, |(stem, _)| stem).to_owned()
}

/// Whether a skill path looks like a script (by kind or extension).
pub fn is_script(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.ends_with(".sh") || lower.ends_with(".py") || lower.ends_with(".js") || lower.contains("/scripts/")
}

/// Whether a skill path looks like a reference document.
pub fn is_reference(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.ends_with(".md") || lower.contains("/references/")
}

/// Whether a skill path looks like an asset (image, font, data).
pub fn is_asset(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    [".png", ".jpg", ".jpeg", ".webp", ".svg", ".woff", ".json"].iter().any(|ext| lower.ends_with(ext))
}

/// Percentage of skills of a given kind, clamped.
pub fn kind_percent(count: usize, total: usize) -> u32 {
    crate::bdots_activity::percent(count, total)
}

/// Bar of filled cells for a kind's share.
pub fn kind_bar(count: usize, total: usize, width: usize) -> String {
    "▮".repeat(crate::bdots_activity::bar_width(count, total, width))
}

/// Count each resource kind from a list of skill paths.
pub fn count_kinds(skills: &[String]) -> Vec<(&'static str, usize)> {
    let mut out = Vec::new();
    let mut add = |kind: &'static str| {
        if let Some(entry) = out.iter_mut().find(|(k, _)| *k == kind) {
            entry.1 += 1;
        } else {
            out.push((kind, 1));
        }
    };
    for skill in skills {
        if is_script(skill) {
            add("script");
        } else if is_reference(skill) {
            add("reference");
        } else if is_asset(skill) {
            add("asset");
        } else {
            add("file");
        }
    }
    out
}

/// Duplicate skill paths, case-insensitively, as titles.
pub fn duplicates(skills: &[String]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut dupes = Vec::new();
    for skill in skills {
        let key = skill.to_ascii_lowercase();
        if !seen.insert(key.clone()) && !dupes.contains(&skill_title(&key)) {
            dupes.push(skill_title(skill));
        }
    }
    dupes
}

/// Total number of skills.
pub fn total_skills(skills: &[String]) -> usize {
    skills.len()
}

/// Number of distinct skill titles.
pub fn distinct_skills(skills: &[String]) -> usize {
    skills.iter().map(|s| skill_title(s).to_ascii_lowercase()).collect::<std::collections::HashSet<_>>().len()
}

/// Whether the library has any skill.
pub fn has_any(skills: &[String]) -> bool {
    !skills.is_empty()
}

/// Whether the library has at least one script.
pub fn has_script(skills: &[String]) -> bool {
    skills.iter().any(|s| is_script(s))
}

/// Whether the library has at least one reference document.
pub fn has_reference(skills: &[String]) -> bool {
    skills.iter().any(|s| is_reference(s))
}

/// Whether the library has at least one asset.
pub fn has_asset(skills: &[String]) -> bool {
    skills.iter().any(|s| is_asset(s))
}

/// Skills sorted by title, for a stable list across clients.
pub fn sorted(skills: &[String]) -> Vec<&String> {
    let mut out: Vec<&String> = skills.iter().collect();
    out.sort_by(|a, b| skill_title(a).cmp(&skill_title(b)));
    out
}

/// Up to `max` skills, scripts first, for a "ready to use" strip.
pub fn most_useful(skills: &[String], max: usize) -> Vec<&String> {
    let mut out: Vec<&String> = skills.iter().collect();
    out.sort_by(|a, b| is_script(b).cmp(&is_script(a)).then_with(|| skill_title(a).cmp(&skill_title(b))));
    out.into_iter().take(max).collect()
}

/// One plain sentence describing the library.
pub fn library_headline(skills: &[String]) -> String {
    if skills.is_empty() {
        return "No skills installed yet.".to_owned();
    }
    let kinds = count_kinds(skills);
    let top = kinds.first().map_or("files", |(k, _)| *k);
    format!("{} skills · mostly {top}", skills.len())
}

/// Semantic tone for the library card.
pub fn library_tone(skills: &[String]) -> &'static str {
    if skills.is_empty() { "warn" } else { "good" }
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(skills: &[String]) -> String {
    crate::bdots_desk::compact_count(skills.len() as u64)
}

/// Build a display line from a skill path.
pub fn skill_line(path: &str) -> SkillLine {
    let kind = if is_script(path) {
        "script"
    } else if is_reference(path) {
        "reference"
    } else if is_asset(path) {
        "asset"
    } else {
        "file"
    };
    SkillLine {
        name: skill_title(path),
        kind,
        icon: kind_icon(kind),
        tone: kind_tone(kind),
        detail: crate::bdots_more::shorten(path, 64),
    }
}

/// Assemble the whole library summary.
pub fn summary(skills: &[String]) -> Vec<(&'static str, String)> {
    vec![
        ("total", count_label(skills)),
        ("distinct", distinct_skills(skills).to_string()),
        ("headline", library_headline(skills)),
        ("scripts", count_kinds(skills).iter().find(|(k, _)| *k == "script").map_or(0, |(_, n)| *n).to_string()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skills() -> Vec<String> {
        vec![
            "skills/deploy/scripts/deploy.sh".into(),
            "skills/deploy/references/setup.md".into(),
            "skills/deploy/assets/logo.png".into(),
            "skills/notes.md".into(),
        ]
    }

    #[test]
    fn icons_and_tones_are_stable() {
        assert_eq!(kind_icon("script"), "terminal");
        assert_eq!(kind_icon("reference"), "book");
        assert_eq!(kind_tone("asset"), "warn");
        assert_eq!(kind_tone("file"), "secondary");
    }

    #[test]
    fn titles_and_detectors() {
        assert_eq!(skill_title("skills/deploy/scripts/deploy.sh"), "deploy");
        assert_eq!(skill_title("skills/notes.md"), "notes");
        assert!(is_script("skills/x.py"));
        assert!(is_reference("skills/x.md"));
        assert!(is_asset("skills/x.png"));
        assert!(!is_script("skills/x.md"));
    }

    #[test]
    fn kinds_bars_and_percentages() {
        let s = skills();
        assert_eq!(count_kinds(&s).len(), 4);
        assert_eq!(kind_percent(1, 4), 25);
        assert!(!kind_bar(1, 4, 8).is_empty());
    }

    #[test]
    fn duplicates_and_distinct() {
        let mut s = skills();
        s.push("skills/deploy/scripts/deploy.sh".into());
        assert_eq!(distinct_skills(&s), 4);
        assert_eq!(duplicates(&s).len(), 1);
    }

    #[test]
    fn detectors_and_ordering() {
        assert!(has_any(&skills()));
        assert!(has_script(&skills()));
        assert!(has_reference(&skills()));
        assert!(has_asset(&skills()));
        assert!(!has_any(&[]));
        assert_eq!(sorted(&skills())[0].as_str(), "skills/deploy/assets/logo.png");
        assert!(is_script(most_useful(&skills(), 1)[0]));
    }

    #[test]
    fn headlines_and_lines() {
        assert!(library_headline(&[]).contains("No skills"));
        assert!(library_headline(&skills()).contains("skills"));
        assert_eq!(library_tone(&[]), "warn");
        assert_eq!(library_tone(&skills()), "good");
        assert_eq!(skill_line("skills/x.py").kind, "script");
        assert_eq!(summary(&skills()).len(), 4);
        assert_eq!(count_label(&skills()), "4");
    }
}
