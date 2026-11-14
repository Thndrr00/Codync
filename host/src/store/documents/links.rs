//! `[[links]]` between a bot's documents. The links are read from the body on every save and
//! resolved by title when they are read, so renaming a page never leaves a broken id behind.
//!
//! Idea from agentes.chat's wiki links and backlinks (`packages/engine`, AGPL-3.0-only): the
//! behavior, not the code.

use super::types::DocumentSummary;
use crate::LockExt;
use crate::store::Store;
use anyhow::Result;
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use serde_json::{Value, json};

/// The most links one document may have; the rest of a very long body is ignored.
const MAX_LINKS: usize = 200;
/// The longest title a link may name.
const MAX_TARGET: usize = 200;

/// One link a document makes: the title its body names, and the page that title is.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Link {
    pub target: String,
    /// The document in the same bot with that title, when there is one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_id: Option<String>,
}

/// The `[[titles]]` a Markdown body links to, in order and without repeats. `[[Title|alias]]` and
/// `[[Title#heading]]` name `Title`.
pub fn parse(body: &str) -> Vec<String> {
    let mut targets: Vec<String> = Vec::new();
    let mut rest = body;
    while let Some(start) = rest.find("[[") {
        let after = &rest[start + 2..];
        let Some(end) = after.find("]]") else { break };
        let target = after[..end].split(['|', '#']).next().unwrap_or_default().trim();
        let fits = !target.is_empty() && target.chars().count() <= MAX_TARGET;
        if fits && !targets.iter().any(|seen| seen == target) {
            targets.push(target.to_owned());
            if targets.len() == MAX_LINKS {
                break;
            }
        }
        rest = &after[end + 2..];
    }
    targets
}

/// Replaces what document `id` links to with what `body` now names.
pub(super) fn reindex(db: &Connection, bot: &str, id: &str, body: &str) -> Result<()> {
    db.execute("DELETE FROM document_links WHERE from_id = ?1", [id])?;
    let mut statement =
        db.prepare("INSERT OR IGNORE INTO document_links(bot_id, from_id, target) VALUES(?1, ?2, ?3)")?;
    for target in parse(body) {
        statement.execute(params![bot, id, target])?;
    }
    Ok(())
}

/// Forgets what a deleted document linked to.
pub(super) fn forget(db: &Connection, id: &str) -> Result<()> {
    db.execute("DELETE FROM document_links WHERE from_id = ?1", [id])?;
    Ok(())
}

impl Store {
    /// What document `id` links to, each with the page it resolves to in the same bot.
    pub fn document_links(&self, id: &str) -> Result<Vec<Link>> {
        let db = self.db.locked();
        let Some(doc) = super::get(&db, id)? else { return Ok(Vec::new()) };
        let mut statement = db.prepare("SELECT target FROM document_links WHERE from_id = ?1 ORDER BY target")?;
        let targets: Vec<String> = statement.query_map([id], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?;
        drop(statement);
        let mut links = Vec::with_capacity(targets.len());
        for target in targets {
            let document_id = db
                .query_row(
                    "SELECT id FROM documents
                     WHERE bot_id = ?1 AND deleted = 0 AND codync_lower(title) = codync_lower(?2)
                     LIMIT 1",
                    params![doc.bot_id, target],
                    |r| r.get(0),
                )
                .optional()?;
            links.push(Link { target, document_id });
        }
        Ok(links)
    }

    /// The documents of the same bot that link to `id`'s title, by title.
    pub fn document_backlinks(&self, id: &str) -> Result<Vec<DocumentSummary>> {
        let db = self.db.locked();
        let Some(doc) = super::get(&db, id)? else { return Ok(Vec::new()) };
        let mut statement = db.prepare(
            "SELECT d.id, d.bot_id, d.parent_id, d.title, d.revision, d.rev, d.created_at, d.updated_at
             FROM document_links l JOIN documents d ON d.id = l.from_id
             WHERE l.bot_id = ?1 AND d.deleted = 0 AND d.id != ?2
               AND codync_lower(l.target) = codync_lower(?3)
             ORDER BY d.title",
        )?;
        let backlinks = statement
            .query_map(params![doc.bot_id, doc.id, doc.title], |r| {
                Ok(DocumentSummary {
                    id: r.get(0)?,
                    bot_id: r.get(1)?,
                    parent_id: r.get(2)?,
                    title: r.get(3)?,
                    revision: r.get(4)?,
                    rev: r.get(5)?,
                    created_at: r.get(6)?,
                    updated_at: r.get(7)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(backlinks)
    }

    /// Every page of `bot` and every link between two of them, for a graph view: nodes are pages,
    /// edges are links that resolve to another page of the same bot.
    pub fn document_graph(&self, bot: &str) -> Result<Value> {
        let db = self.db.locked();
        let mut pages =
            db.prepare("SELECT id, title FROM documents WHERE bot_id = ?1 AND deleted = 0 ORDER BY title")?;
        let nodes: Vec<Value> = pages
            .query_map([bot], |r| Ok(json!({"id": r.get::<_, String>(0)?, "title": r.get::<_, String>(1)?})))?
            .collect::<rusqlite::Result<_>>()?;
        drop(pages);
        let mut links = db.prepare(
            "SELECT l.from_id, d.id FROM document_links l
             JOIN documents d ON d.bot_id = l.bot_id AND d.deleted = 0
                              AND codync_lower(d.title) = codync_lower(l.target)
             WHERE l.bot_id = ?1 AND d.id != l.from_id",
        )?;
        let edges: Vec<Value> = links
            .query_map([bot], |r| Ok(json!({"from": r.get::<_, String>(0)?, "to": r.get::<_, String>(1)?})))?
            .collect::<rusqlite::Result<_>>()?;
        Ok(json!({"nodes": nodes, "edges": edges}))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_are_read_in_order_without_repeats() {
        let body = "See [[Roadmap]] and [[Specs|the specs]], then [[Roadmap]] again. [[#heading]]";
        assert_eq!(parse(body), vec!["Roadmap", "Specs"]);
    }

    #[test]
    fn a_broken_bracket_is_not_a_link() {
        assert_eq!(parse("an [[open bracket").len(), 0);
        assert_eq!(parse("[[   ]]").len(), 0);
        assert_eq!(parse("no links here").len(), 0);
    }

    #[test]
    fn a_link_may_carry_a_heading() {
        assert_eq!(parse("[[Design#Layout]]"), vec!["Design"]);
    }

    #[test]
    fn a_very_long_title_is_not_a_link() {
        assert_eq!(parse(&format!("[[{}]]", "x".repeat(MAX_TARGET + 1))).len(), 0);
    }
}
