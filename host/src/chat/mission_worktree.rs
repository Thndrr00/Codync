//! One worktree per mission (agentes.chat ADR 0044).
//!
//! A mission works in its own folder, branched from the project the person chose, so the project
//! itself is never written by a mission and parallel items do not collide in the person's checkout.
//! This module holds the naming rules; the host creates the worktree when the mission starts.

use serde_json::{Value, json};

/// A branch and folder name derived from the mission id and title, safe for git and disks.
pub fn names(id: &str, title: &str) -> Value {
    let short: String = id.chars().filter(|c| c.is_ascii_alphanumeric()).take(8).collect();
    let slug: String = title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
        .chars()
        .take(40)
        .collect();
    let short = if short.is_empty() { "m".to_owned() } else { short };
    let slug = if slug.is_empty() { "mission".to_owned() } else { slug };
    json!({
        "branch": format!("codync/mission/{short}/{slug}"),
        "folder": format!("m-{short}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_stable_safe_and_bounded() {
        let a = names("abc-123-def", "Fix the Billing Page!");
        assert_eq!(a["branch"], "codync/mission/abc123de/fix-the-billing-page");
        assert_eq!(a["folder"], "m-abc123de");
        assert_eq!(a, names("abc-123-def", "Fix the Billing Page!"));
        let empty = names("!!!", "***");
        assert_eq!(empty["branch"], "codync/mission/m/mission");
        assert!(names("x", &"a".repeat(200))["branch"].as_str().unwrap().len() <= 80);
    }
}
