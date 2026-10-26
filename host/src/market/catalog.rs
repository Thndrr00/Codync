//! Codync's own catalog of ready-made bots and coordinator teams (`botCatalog`, `installTeam`).
//!
//! A profile is only a starting point: a name, a look and standing instructions. Creating one goes
//! through the same defaults as any new bot, so it never turns on more access than a blank bot gets.

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::LazyLock;

/// A ready-made bot: what it is for and how it starts.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BotProfile {
    pub id: String,
    pub name: String,
    pub summary: String,
    pub instructions: String,
    pub avatar_color: String,
    pub avatar_shape: String,
    /// Offered as a suggestion on the New bot screen.
    #[serde(default)]
    pub featured: bool,
}

/// A group chat led by a coordinator bot, with the catalog bots it hands work to.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TeamProfile {
    pub id: String,
    pub name: String,
    pub summary: String,
    /// The group's description.
    pub purpose: String,
    pub coordinator: BotProfile,
    /// Ids of catalog bots.
    pub members: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Catalog {
    pub bots: Vec<BotProfile>,
    pub teams: Vec<TeamProfile>,
}

static CATALOG: LazyLock<Catalog> = LazyLock::new(|| {
    serde_json::from_str(include_str!("catalog.json")).expect("the bundled catalog is valid (checked by its tests)")
});

pub fn catalog() -> &'static Catalog {
    &CATALOG
}

pub fn team(id: &str) -> Result<&'static TeamProfile> {
    CATALOG.teams.iter().find(|t| t.id == id).ok_or_else(|| anyhow!("Unknown team"))
}

impl TeamProfile {
    /// The coordinator first, then its teammates in catalog order.
    pub fn profiles(&'static self) -> Result<Vec<&'static BotProfile>> {
        let mut list = vec![&self.coordinator];
        for id in &self.members {
            list.push(CATALOG.bots.iter().find(|b| &b.id == id).ok_or_else(|| anyhow!("Unknown team member"))?);
        }
        Ok(list)
    }
}

impl BotProfile {
    /// The `createBot` body for this profile, on `backend`/`model` when the caller picked them.
    pub fn draft(&self, backend: Option<&str>, model: Option<&str>) -> Value {
        json!({
            "name": self.name,
            "description": self.instructions,
            "avatarColor": self.avatar_color,
            "avatarShape": self.avatar_shape,
            "backend": backend.unwrap_or_default(),
            "model": model,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const COLORS: [&str; 11] =
        ["black", "brown", "red", "orange", "yellow", "green", "cyan", "blue", "violet", "magenta", "gray"];
    const SHAPES: [&str; 8] = ["blob", "pebble", "squircle", "tablet", "wedge", "hex", "cloud", "teardrop"];

    fn check(profile: &BotProfile) {
        assert!(!profile.name.trim().is_empty() && !profile.instructions.trim().is_empty(), "{}", profile.id);
        assert!(COLORS.contains(&profile.avatar_color.as_str()), "{} color", profile.id);
        assert!(SHAPES.contains(&profile.avatar_shape.as_str()), "{} shape", profile.id);
    }

    #[test]
    fn every_profile_is_complete_and_unique() {
        let all = catalog();
        let mut ids: Vec<&str> = all.bots.iter().map(|b| b.id.as_str()).collect();
        ids.extend(all.teams.iter().map(|t| t.id.as_str()));
        ids.extend(all.teams.iter().map(|t| t.coordinator.id.as_str()));
        let count = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), count, "ids are unique");
        all.bots.iter().for_each(check);
        assert!(all.bots.iter().any(|b| b.featured));
    }

    #[test]
    fn every_team_resolves_to_a_coordinator_and_its_members() {
        for t in &catalog().teams {
            check(&t.coordinator);
            let profiles = t.profiles().expect("members are catalog bots");
            assert_eq!(profiles.len(), t.members.len() + 1);
            assert_eq!(profiles[0].id, t.coordinator.id);
            assert!(!t.members.is_empty() && !t.purpose.trim().is_empty());
        }
        assert!(team("nope").is_err());
    }

    #[test]
    fn a_draft_leaves_unpicked_choices_to_the_defaults() {
        let bot = &catalog().bots[0];
        let draft = bot.draft(None, None);
        assert_eq!(draft["backend"], "");
        assert!(draft["model"].is_null());
        assert_eq!(draft["description"], bot.instructions.as_str());
        assert!(draft["connectors"].is_null() && draft["skills"].is_null());
        assert_eq!(bot.draft(Some("codex"), Some("gpt-5")).get("model").and_then(Value::as_str), Some("gpt-5"));
    }
}
