//! Node groups of a canvas. A group is a drawing convenience only: it never changes which links a
//! routine runs. It lives in the kv blob (`routineflow.groups.<bot>`), beside the canvas revision,
//! so an older host that ignores the field and an older client that never sends it both keep working.
//!
//! This module owns the group data and its validation. It stamps the canvas revision the same way
//! `store::flows` does (that module's `touch`/`next_rev` are private), in one transaction.

use crate::store::flows::{MAX_COORDINATE, REV_PREFIX};
use crate::store::{FlowChange, Store};
use anyhow::{Result, bail};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// kv prefix of one bot's groups, followed by its id.
pub const GROUPS_PREFIX: &str = "routineflow.groups.";

/// How many groups and members one canvas may hold, and how long a group name may be.
const MAX_GROUPS: usize = 100;
const MAX_MEMBERS: usize = 200;
const MAX_NAME_CHARS: usize = 120;
const MAX_ID_CHARS: usize = 200;

/// Several nodes drawn together, with one port per output kind and a collapsed stack-card form.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowGroup {
    pub id: String,
    pub name: String,
    /// The node keys in the group (`routine:`, `bot:` or `media:`).
    pub members: Vec<String>,
    /// Whether the group is drawn as a stack card instead of its members.
    pub collapsed: bool,
    /// Where the group (or its stack card) sits on the canvas.
    pub x: f64,
    pub y: f64,
}

/// A node key the canvas can group: `routine:<id>`, `bot:<id>` or `media:<id>`.
fn valid_member(key: &str) -> bool {
    key.split_once(':')
        .is_some_and(|(kind, id)| matches!(kind, "routine" | "bot" | "media") && !id.is_empty() && !id.contains(':'))
        && key.chars().count() <= MAX_ID_CHARS
}

fn valid_text(text: &str, max: usize) -> bool {
    text.chars().count() <= max && !text.chars().any(char::is_control)
}

fn coordinate(v: f64) -> bool {
    v.is_finite() && v.abs() <= MAX_COORDINATE
}

/// Checks the structure of a canvas's groups: valid ids and names, live node keys, no node in two
/// groups and no duplicate member. Existence of the nodes is checked by the API against the store.
pub fn validate(groups: &[FlowGroup]) -> Result<()> {
    if groups.len() > MAX_GROUPS {
        bail!("a canvas holds at most {MAX_GROUPS} groups");
    }
    let mut ids = BTreeSet::new();
    let mut members = BTreeSet::new();
    for group in groups {
        if group.id.is_empty() || !valid_text(&group.id, MAX_ID_CHARS) {
            bail!("a group needs an id of at most {MAX_ID_CHARS} characters");
        }
        if !ids.insert(group.id.as_str()) {
            bail!("two groups have the same id");
        }
        if !valid_text(&group.name, MAX_NAME_CHARS) {
            bail!("a group name must be at most {MAX_NAME_CHARS} characters");
        }
        if group.members.is_empty() || group.members.len() > MAX_MEMBERS {
            bail!("a group holds 1 to {MAX_MEMBERS} nodes");
        }
        if !coordinate(group.x) || !coordinate(group.y) {
            bail!("a group's position must be within ±{MAX_COORDINATE}");
        }
        for key in &group.members {
            if !valid_member(key) {
                bail!("a group member must be a routine, bot or media node key");
            }
            if !members.insert(key.as_str()) {
                bail!("a node can be in only one group");
            }
        }
    }
    Ok(())
}

/// The groups saved for `bot`, or none for a canvas without them (including an older host's data).
pub fn load(store: &Store, bot: &str) -> Vec<FlowGroup> {
    let Some(raw) = store.kv_get(&format!("{GROUPS_PREFIX}{bot}")) else { return Vec::new() };
    serde_json::from_str(&raw).unwrap_or_default()
}

/// Replaces `bot`'s groups and stamps a canvas revision so every client reloads. One transaction
/// keeps the groups, the global rev and this bot's canvas rev in step.
pub fn save(store: &Store, bot: &str, groups: &[FlowGroup]) -> Result<FlowChange> {
    validate(groups)?;
    let value = serde_json::to_string(groups)?;
    store.with_connection(|c| {
        let tx = c.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO kv(k, v) VALUES(?1, ?2) ON CONFLICT(k) DO UPDATE SET v = ?2",
            params![format!("{GROUPS_PREFIX}{bot}"), value],
        )?;
        // The same counter and per-bot record as store::flows::next_rev and touch.
        let rev: i64 = tx.query_row(
            "INSERT INTO kv(k, v) VALUES('rev', '1') ON CONFLICT(k) DO UPDATE SET v = CAST(v AS INTEGER) + 1 RETURNING CAST(v AS INTEGER)",
            [],
            |r| r.get(0),
        )?;
        tx.execute(
            "INSERT INTO kv(k, v) VALUES(?1, ?2) ON CONFLICT(k) DO UPDATE SET v = ?2",
            params![format!("{REV_PREFIX}{bot}"), rev.to_string()],
        )?;
        tx.commit()?;
        Ok(FlowChange { rev, bots: vec![bot.to_owned()] })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn group(members: &[&str]) -> FlowGroup {
        FlowGroup {
            id: "g1".into(),
            name: "Pictures".into(),
            members: members.iter().map(|s| (*s).to_owned()).collect(),
            collapsed: false,
            x: 10.0,
            y: 20.0,
        }
    }

    #[test]
    fn members_are_node_keys() {
        assert!(validate(&[group(&["bot:a", "media:1"])]).is_ok());
        assert!(validate(&[group(&["nope"])]).is_err());
        assert!(validate(&[group(&["bot:a:b"])]).is_err());
    }

    #[test]
    fn a_node_belongs_to_one_group() {
        let mut other = group(&["bot:a"]);
        other.id = "g2".into();
        assert!(validate(&[group(&["bot:a"]), other]).is_err());
        assert!(validate(&[group(&["bot:a", "bot:a"])]).is_err());
    }

    #[test]
    fn empty_and_oversized_groups_are_refused() {
        assert!(validate(&[group(&[])]).is_err());
        let mut long = group(&["bot:a"]);
        long.name = "x".repeat(MAX_NAME_CHARS + 1);
        assert!(validate(&[long]).is_err());
    }
}
