//! Search over what a bot knows: what was said in its chat and what its documents hold, found by
//! words, with the source of each hit. One FTS5 index holds both kinds, filled as they are written
//! and backfilled once for a database from before this existed.
//!
//! Idea from agentes.chat's search (`packages/engine/src/search.ts`, AGPL-3.0-only): an index of
//! several kinds of source, each with its own builder. The behavior, not the code; written against
//! Codync's own `entries` and `documents`.

use super::Store;
use crate::LockExt;
use anyhow::Result;
use rusqlite::{Connection, params};
use serde_json::{Value, json};

/// The most hits one search returns.
const MAX_HITS: u32 = 50;
/// Characters kept on each side of a match.
const SNIPPET_TOKENS: i64 = 12;

pub(super) fn open(c: &Connection) -> Result<()> {
    c.execute_batch(
        "CREATE VIRTUAL TABLE IF NOT EXISTS search_fts USING fts5(
            body, kind UNINDEXED, source_id UNINDEXED, bot_id UNINDEXED, at UNINDEXED,
            tokenize = 'unicode61 remove_diacritics 2');",
    )?;
    Ok(())
}

/// The words of a message worth finding: the user's or the agent's text, and a tool's title.
pub fn message_body(data: &Value) -> String {
    ["text", "title", "detail"]
        .iter()
        .filter_map(|key| data[*key].as_str())
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// The words of a document: its title, then its body.
pub fn document_body(title: &str, body: &str) -> String {
    format!("{}\n{}", title.trim(), body.trim())
}

fn insert(c: &Connection, body: &str, kind: &str, source: &str, bot: &str, at: i64) -> Result<()> {
    if body.trim().is_empty() {
        return Ok(());
    }
    c.execute(
        "INSERT INTO search_fts(body, kind, source_id, bot_id, at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![body, kind, source, bot, at],
    )?;
    Ok(())
}

/// Indexes one message as it is written.
pub(super) fn index_message(c: &Connection, id: &str, bot: &str, at: i64, data: &Value) -> Result<()> {
    insert(c, &message_body(data), "message", id, bot, at)
}

/// Indexes one document as it is saved, replacing what that page said before.
pub(super) fn index_document(c: &Connection, id: &str, bot: &str, at: i64, title: &str, body: &str) -> Result<()> {
    forget(c, "document", id)?;
    insert(c, &document_body(title, body), "document", id, bot, at)
}

/// Drops a source from the index (a page that was deleted or replaced).
pub(super) fn forget(c: &Connection, kind: &str, source: &str) -> Result<()> {
    c.execute("DELETE FROM search_fts WHERE kind = ?1 AND source_id = ?2", params![kind, source])?;
    Ok(())
}

/// What a person typed, as an FTS5 query: every word, quoted, all of them required. A stray quote or
/// `AND` becomes a word instead of a syntax error.
fn terms(text: &str) -> String {
    text.split_whitespace()
        .map(|word| format!("\"{}\"", word.replace('"', "")))
        .collect::<Vec<_>>()
        .join(" ")
}

impl Store {
    /// What this bot knows about `query`, best match first: the source, its kind and a snippet.
    pub fn search(&self, bot: &str, query: &str, limit: u32) -> Result<Vec<Value>> {
        let terms = terms(query);
        if terms.is_empty() {
            return Ok(Vec::new());
        }
        let limit = i64::from(limit.clamp(1, MAX_HITS));
        self.backfill_search()?;
        let c = self.db.locked();
        let mut statement = c.prepare(
            "SELECT kind, source_id, at, snippet(search_fts, 0, '[', ']', '...', ?3)
             FROM search_fts WHERE search_fts MATCH ?1 AND bot_id = ?2
             ORDER BY rank LIMIT ?4",
        )?;
        let hits = statement
            .query_map(params![terms, bot, SNIPPET_TOKENS, limit], |row| {
                Ok(json!({
                    "kind": row.get::<_, String>(0)?,
                    "id": row.get::<_, String>(1)?,
                    "at": row.get::<_, i64>(2)?,
                    "snippet": row.get::<_, String>(3)?,
                }))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(hits)
    }

    /// Fills the index from what a database already had, once: a chat and a library written before
    /// search existed are searchable on the first search, not after a manual step.
    fn backfill_search(&self) -> Result<()> {
        {
            let c = self.db.locked();
            let indexed: i64 = c.query_row("SELECT COUNT(*) FROM search_fts", [], |r| r.get(0))?;
            if indexed > 0 {
                return Ok(());
            }
        }
        let entries: Vec<(String, String, i64, String)> = {
            let c = self.db.locked();
            let mut statement = c.prepare("SELECT id, bot_id, created_at, data FROM entries")?;
            statement
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))?
                .collect::<rusqlite::Result<Vec<_>>>()?
        };
        let documents: Vec<(String, String, i64, String, String)> = {
            let c = self.db.locked();
            let mut statement =
                c.prepare("SELECT id, bot_id, updated_at, title, body FROM documents WHERE deleted = 0")?;
            statement
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)))?
                .collect::<rusqlite::Result<Vec<_>>>()?
        };
        let c = self.db.locked();
        for (id, bot, at, data) in entries {
            if let Ok(data) = serde_json::from_str::<Value>(&data) {
                index_message(&c, &id, &bot, at, &data)?;
            }
        }
        for (id, bot, at, title, body) in documents {
            index_document(&c, &id, &bot, at, &title, &body)?;
        }
        Ok(())
    }

    /// Rebuilds the whole index: what a repair after a bad migration needs.
    pub fn reindex_search(&self) -> Result<usize> {
        {
            let c = self.db.locked();
            c.execute("DELETE FROM search_fts", [])?;
        }
        self.backfill_search()?;
        let c = self.db.locked();
        Ok(c.query_row("SELECT COUNT(*) FROM search_fts", [], |r| r.get::<_, i64>(0))? as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{DocumentDraft, DocumentWrite, EntryKind, Lane, tests::temp_store};
    use serde_json::json;

    fn said(store: &Store, bot: &str, text: &str) {
        store.insert_entry(&Lane::main(bot), EntryKind::User, 1, &json!({"text": text})).expect("an entry");
    }

    fn page(store: &Store, bot: &str, title: &str, body: &str) -> String {
        let draft = DocumentDraft { id: None, parent_id: None, title: title.into(), body: body.into(), if_revision: 0 };
        match store.save_document(bot, draft).expect("a page") {
            DocumentWrite::Saved(document) => document.id,
            DocumentWrite::Conflict(_) => panic!("unexpected conflict"),
        }
    }

    #[test]
    fn a_search_finds_the_message_that_said_it() {
        let store = temp_store();
        store.save_bot(&serde_json::from_value(json!({"id": "b1", "name": "B"})).unwrap()).unwrap();
        said(&store, "b1", "the deploy needs a rollback plan");
        said(&store, "b1", "lunch at one");

        let hits = store.search("b1", "rollback", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0]["kind"], "message");
        assert!(hits[0]["snippet"].as_str().is_some_and(|s| s.contains("[rollback]")));
        assert!(store.search("b1", "nothing like this", 10).unwrap().is_empty());
    }

    #[test]
    fn a_page_is_found_by_what_it_holds() {
        let store = temp_store();
        store.save_bot(&serde_json::from_value(json!({"id": "b1", "name": "B"})).unwrap()).unwrap();
        page(&store, "b1", "Runbook", "when the deploy fails, roll it back");
        said(&store, "b1", "the deploy needs a rollback plan");

        let hits = store.search("b1", "deploy", 10).unwrap();
        let kinds: Vec<&str> = hits.iter().filter_map(|hit| hit["kind"].as_str()).collect();
        assert!(kinds.contains(&"document"), "{kinds:?}");
        assert!(kinds.contains(&"message"), "{kinds:?}");
    }

    #[test]
    fn a_page_that_changed_is_indexed_once() {
        let store = temp_store();
        store.save_bot(&serde_json::from_value(json!({"id": "b1", "name": "B"})).unwrap()).unwrap();
        let id = page(&store, "b1", "Plan", "ship on monday");
        let draft = DocumentDraft {
            id: Some(id.clone()),
            parent_id: None,
            title: "Plan".into(),
            body: "ship on tuesday".into(),
            if_revision: 1,
        };
        store.save_document("b1", draft).unwrap();
        assert!(store.search("b1", "monday", 10).unwrap().is_empty(), "the old body is gone");
        assert_eq!(store.search("b1", "tuesday", 10).unwrap().len(), 1);
    }

    #[test]
    fn every_word_is_required_and_another_bot_is_another_chat() {
        let store = temp_store();
        for id in ["b1", "b2"] {
            store.save_bot(&serde_json::from_value(json!({"id": id, "name": id})).unwrap()).unwrap();
        }
        said(&store, "b1", "deploy the api");
        said(&store, "b2", "deploy the web");
        assert_eq!(store.search("b1", "deploy api", 10).unwrap().len(), 1);
        assert!(store.search("b1", "deploy web", 10).unwrap().is_empty(), "another chat");
    }

    #[test]
    fn a_stray_quote_is_a_word_not_a_syntax_error() {
        let store = temp_store();
        store.save_bot(&serde_json::from_value(json!({"id": "b1", "name": "B"})).unwrap()).unwrap();
        said(&store, "b1", "he said \"ship it\" twice");
        assert!(store.search("b1", "\"ship", 10).is_ok());
        assert_eq!(store.search("b1", "AND", 10).unwrap().len(), 0);
    }

    #[test]
    fn an_empty_query_finds_nothing() {
        let store = temp_store();
        assert!(store.search("b1", "   ", 10).unwrap().is_empty());
    }
}
