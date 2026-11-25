#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers for a bot's memory settings: the shared/harness switches and any memory skills.
//! Reuses `bdots_library`, `bdots_profile` and `bdots_text` instead of duplicating their shape.
//! Only `botInsights` calls this.

/// One memory setting reduced to what the card shows.
#[derive(Clone, Debug, PartialEq)]
pub struct MemoryLine {
    pub id: &'static str,
    pub title: String,
    pub detail: String,
    pub icon: &'static str,
    pub tone: &'static str,
    pub on: bool,
}

fn line(id: &'static str, title: &str, detail: String, icon: &'static str, tone: &'static str, on: bool) -> MemoryLine {
    MemoryLine { id, title: title.to_owned(), detail, icon, tone, on }
}

/// Human label for the shared-memory switch.
pub fn shared_label(shared: Option<bool>) -> &'static str {
    match shared {
        Some(true) => "Shared folder",
        Some(false) => "Private folder",
        None => "Host default",
    }
}

/// SF Symbol for the shared-memory switch.
pub fn shared_icon(shared: Option<bool>) -> &'static str {
    match shared {
        Some(true) => "folder.shared",
        Some(false) => "folder.fill",
        None => "questionmark.folder",
    }
}

/// Semantic tone for the shared-memory switch.
pub fn shared_tone(shared: Option<bool>) -> &'static str {
    match shared {
        Some(true) => "good",
        Some(false) => "accent",
        None => "secondary",
    }
}

/// One plain sentence about the shared-memory switch.
pub fn shared_detail(shared: Option<bool>) -> &'static str {
    match shared {
        Some(true) => "Notes are shared with other bots in this folder",
        Some(false) => "Notes stay with this bot alone",
        None => "Uses the folder's own setting",
    }
}

/// Whether harness memory is on.
pub fn harness_on(harness: bool) -> bool {
    harness
}

/// SF Symbol for harness memory.
pub fn harness_icon(harness: bool) -> &'static str {
    if harness { "brain.fill" } else { "brain" }
}

/// Semantic tone for harness memory.
pub fn harness_tone(harness: bool) -> &'static str {
    if harness { "good" } else { "secondary" }
}

/// Whether a skill path names memory.
pub fn is_memory_skill(path: &str) -> bool {
    path.to_ascii_lowercase().contains("memor")
}

/// Memory skills from a skill list.
pub fn memory_skills(skills: &[String]) -> Vec<&String> {
    skills.iter().filter(|s| is_memory_skill(s)).collect()
}

/// Number of memory skills.
pub fn memory_skill_count(skills: &[String]) -> usize {
    memory_skills(skills).len()
}

/// Human title for a memory skill, reusing the library helper.
pub fn skill_title(path: &str) -> String {
    crate::bdots_library::skill_title(path)
}

/// Whether any memory capability is configured.
pub fn has_any(shared: Option<bool>, harness: bool, skills: &[String]) -> bool {
    shared.is_some() || harness || memory_skill_count(skills) > 0
}

/// Whether memory is fully off.
pub fn is_off(shared: Option<bool>, harness: bool, skills: &[String]) -> bool {
    !has_any(shared, harness, skills)
}

/// Count of on settings among the three surfaces.
pub fn on_count(shared: Option<bool>, harness: bool, skills: &[String]) -> usize {
    usize::from(shared == Some(true)) + usize::from(harness) + usize::from(memory_skill_count(skills) > 0)
}

/// 0–100 memory score.
pub fn memory_score(shared: Option<bool>, harness: bool, skills: &[String]) -> u32 {
    let mut score = 0;
    if shared.is_some() {
        score += 30;
    }
    if harness {
        score += 30;
    }
    score += (memory_skill_count(skills).min(2) as u32) * 20;
    score.min(100)
}

/// Semantic tone for a memory score.
pub fn memory_tone(score: u32) -> &'static str {
    if score >= 70 {
        "good"
    } else if score >= 30 {
        "attention"
    } else {
        "secondary"
    }
}

/// Short label for a memory score.
pub fn memory_label(score: u32) -> &'static str {
    if score >= 70 {
        "Full memory"
    } else if score >= 30 {
        "Partial memory"
    } else {
        "No memory"
    }
}

/// One plain sentence describing memory.
pub fn memory_headline(shared: Option<bool>, harness: bool, skills: &[String]) -> String {
    let score = memory_score(shared, harness, skills);
    if is_off(shared, harness, skills) {
        return "Remembers nothing between turns.".to_owned();
    }
    format!("{} · {} skills", memory_label(score), memory_skill_count(skills))
}

/// Build every memory line for one bot.
pub fn build_lines(shared: Option<bool>, harness: bool, skills: &[String]) -> Vec<MemoryLine> {
    let mut out = vec![
        line(
            "shared",
            "Shared notes",
            shared_detail(shared).to_owned(),
            shared_icon(shared),
            shared_tone(shared),
            shared == Some(true),
        ),
        line(
            "harness",
            "Run memory",
            if harness { "Remembers across runs" } else { "One run at a time" }.into(),
            harness_icon(harness),
            harness_tone(harness),
            harness,
        ),
    ];
    for skill in memory_skills(skills) {
        out.push(line(
            "skill",
            skill_title(skill),
            crate::bdots_more::shorten(skill, 48),
            "book.closed",
            "accent",
            true,
        ));
    }
    out
}

/// SF Symbol for an on/off state.
pub fn state_icon(on: bool) -> &'static str {
    if on { "checkmark.circle.fill" } else { "circle" }
}

/// Semantic tone for an on/off state.
pub fn state_tone(on: bool) -> &'static str {
    if on { "good" } else { "secondary" }
}

/// Count lines that are on.
pub fn enabled_count(lines: &[MemoryLine]) -> usize {
    lines.iter().filter(|l| l.on).count()
}

/// Summary rows for the card.
pub fn summary(shared: Option<bool>, harness: bool, skills: &[String]) -> Vec<(&'static str, String)> {
    vec![
        ("shared", shared_label(shared).to_owned()),
        ("harness", if harness { "On" } else { "Off" }.to_owned()),
        ("skills", memory_skill_count(skills).to_string()),
        ("headline", memory_headline(shared, harness, skills)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skills() -> Vec<String> {
        vec!["skills/memory/notes.md".into(), "skills/deploy/scripts/deploy.sh".into()]
    }

    #[test]
    fn shared_switch_labels_and_icons() {
        assert_eq!(shared_label(Some(true)), "Shared folder");
        assert_eq!(shared_label(Some(false)), "Private folder");
        assert_eq!(shared_label(None), "Host default");
        assert_eq!(shared_icon(Some(true)), "folder.shared");
        assert_eq!(shared_tone(None), "secondary");
        assert!(shared_detail(Some(false)).contains("alone"));
    }

    #[test]
    fn harness_and_skill_detectors() {
        assert!(harness_on(true));
        assert_eq!(harness_icon(true), "brain.fill");
        assert_eq!(harness_tone(false), "secondary");
        assert!(is_memory_skill("skills/memory/notes.md"));
        assert!(!is_memory_skill("skills/deploy/deploy.sh"));
        assert_eq!(memory_skills(&skills()).len(), 1);
        assert_eq!(memory_skill_count(&skills()), 1);
        assert_eq!(skill_title("skills/memory/notes.md"), "notes");
    }

    #[test]
    fn presence_and_scores() {
        assert!(has_any(None, true, &[]));
        assert!(!has_any(None, false, &[]));
        assert!(is_off(None, false, &[]));
        assert_eq!(on_count(Some(true), true, &skills()), 3);
        assert_eq!(memory_score(Some(true), true, &skills()), 100);
        assert_eq!(memory_score(None, false, &[]), 0);
        assert_eq!(memory_tone(80), "good");
        assert_eq!(memory_label(50), "Partial memory");
        assert!(memory_headline(None, false, &[]).contains("nothing"));
    }

    #[test]
    fn lines_and_state() {
        let l = build_lines(Some(true), true, &skills());
        assert_eq!(l.len(), 3);
        assert_eq!(enabled_count(&l), 3);
        assert!(l.iter().any(|x| x.id == "skill"));
        assert_eq!(state_icon(true), "checkmark.circle.fill");
        assert_eq!(state_tone(false), "secondary");
    }

    #[test]
    fn summary_rows() {
        let rows = summary(None, false, &[]);
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[2].1, "0");
    }
}
