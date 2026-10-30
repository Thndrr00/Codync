//! Extensions' Bots and Teams tabs: the host's ready-made bots and coordinator teams (`botCatalog`).

use serde_json::{Value, json};

use super::super::app::{App, fuzzy};
use super::super::l10n::tr;
use super::{Reply, Row, Tab, s};

impl App {
    /// The catalog's bots or teams on `tab`, filtered by `q`.
    pub(super) fn catalog_rows<'a>(catalog: &'a Value, tab: Tab, q: &str) -> Vec<Row<'a>> {
        let (key, row): (&str, fn(&'a Value) -> Row<'a>) =
            if tab == Tab::Teams { ("teams", Row::Team) } else { ("bots", Row::Profile) };
        catalog[key]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|v| fuzzy(q, &[s(v, "name").as_str(), s(v, "summary").as_str()]))
            .map(row)
            .collect()
    }

    /// Creates a catalog bot on the host's defaults, or a whole team (`installTeam`).
    pub(super) fn add_from_catalog(&mut self, item: &Value, team: bool) {
        let id = s(item, "id");
        self.flash(&tr("Adding {name}…").replace("{name}", &s(item, "name")));
        if team {
            self.sheet("installTeam", json!({"id": id}), Reply::CatalogAdded(id));
        } else {
            self.sheet("createBot", profile_body(item), Reply::CatalogAdded(id));
        }
    }
}

/// A catalog bot as a new bot: its name, look and instructions; the host picks the agent and turns on every
/// installed connector and skill, as for a bot made in the New bot dialog.
fn profile_body(p: &Value) -> Value {
    json!({
        "name": p["name"],
        "description": p["instructions"],
        "avatarColor": p["avatarColor"],
        "avatarShape": p["avatarShape"],
        "cwd": "",
        "permission": "auto",
        "notify": true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog() -> Value {
        json!({
            "bots": [
                {"id": "writer", "name": "Writer", "summary": "Drafts docs", "instructions": "Write plainly.",
                 "avatarColor": "red", "avatarShape": "hex"},
                {"id": "researcher", "name": "Research analyst", "summary": "Sources", "instructions": "Cite.",
                 "avatarColor": "cyan", "avatarShape": "cloud"},
            ],
            "teams": [{"id": "desk", "name": "Research desk", "summary": "A lead and an analyst", "members": ["researcher"]}],
        })
    }

    #[test]
    fn each_tab_lists_its_part_of_the_catalog_filtered_by_the_query() {
        let c = catalog();
        assert_eq!(App::catalog_rows(&c, Tab::Bots, "").len(), 2);
        assert_eq!(App::catalog_rows(&c, Tab::Bots, "draft").len(), 1);
        assert!(matches!(App::catalog_rows(&c, Tab::Teams, "")[..], [Row::Team(_)]));
        assert!(App::catalog_rows(&json!({}), Tab::Bots, "").is_empty(), "an older host has no catalog");
    }

    #[test]
    fn a_catalog_bot_leaves_the_agent_and_plugins_to_the_host() {
        let body = profile_body(&catalog()["bots"][0]);
        assert_eq!(body["description"], "Write plainly.");
        assert_eq!(body["avatarShape"], "hex");
        assert!(body["backend"].is_null() && body["connectors"].is_null() && body["skills"].is_null());
    }
}
