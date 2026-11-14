//! Search over a chat: what was said, found by words, with the message that said it. An FTS5 index
//! holds the text of each entry; the host fills it as messages arrive and backfills once on the first
//! search of an older database.
//!
//! Idea from agentes.chat's search and context (AGPL-3.0-only): the behavior, not the code. Nothing
//! here is copied: it is written against Codync's own entries table.

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
        "CREATE VIRTUAL TABLE IF NOT EXISTS entries_fts USING fts5(
            body, entry_id UNINDEXED, bot_id UNINDEXED, created_at UNINDEXED,
            tokenize = 'unicode61 remove_diacritics 2');",
    )?;
    Ok(())
}

/// The words of an entry worth finding: the user's or the agent's text, and a tool's title.
pub fn body(data: &Value) -> String {
    ["text", "title", "detail"]
        .iter()
        .filter_map(|key| data[*key].as_str())
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Indexes one entry, if it has words. Called as each entry is written.
pub(super) fn index(c: &Connection, id: &str, bot: &str, created_at: i64, data: &Value) -> Result<()> {
    let body = body(data);
    if body.is_empty() {
        return Ok(());
    }
    c.execute(
        "INSERT INTO entries_fts(body, entry_id, bot_id, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![body, id, bot, created_at],
    )?;
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
    /// What this bot's chat says about `query`, best match first, with the message and a snippet.
    pub fn search(&self, bot: &str, query: &str, limit: u32) -> Result<Vec<Value>> {
        let terms = terms(query);
        if terms.is_empty() {
            return Ok(Vec::new());
        }
        let limit = i64::from(limit.clamp(1, MAX_HITS));
        self.backfill_search()?;
        let c = self.db.locked();
        let mut statement = c.prepare(
            "SELECT entry_id, created_at, snippet(entries_fts, 0, '[', ']', '...', ?3)
             FROM entries_fts WHERE entries_fts MATCH ?1 AND bot_id = ?2
             ORDER BY rank LIMIT ?4",
        )?;
        let hits = statement
            .query_map(params![terms, bot, SNIPPET_TOKENS, limit], |row| {
                Ok(json!({
                    "entryId": row.get::<_, String>(0)?,
                    "at": row.get::<_, i64>(1)?,
                    "snippet": row.get::<_, String>(2)?,
                }))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(hits)
    }

    /// Fills the index from the messages a database already had, once: a chat written before search
    /// existed is searchable on its first search, not after a manual step.
    fn backfill_search(&self) -> Result<()> {
        {
            let c = self.db.locked();
            let indexed: i64 = c.query_row("SELECT COUNT(*) FROM entries_fts", [], |r| r.get(0))?;
            if indexed > 0 {
                return Ok(());
            }
        }
        let rows: Vec<(String, String, i64, String)> = {
            let c = self.db.locked();
            let mut statement = c.prepare("SELECT id, bot_id, created_at, data FROM entries")?;
            statement
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))?
                .collect::<rusqlite::Result<Vec<_>>>()?
        };
        let c = self.db.locked();
        for (id, bot, created_at, data) in rows {
            if let Ok(data) = serde_json::from_str::<Value>(&data) {
                index(&c, &id, &bot, created_at, &data)?;
            }
        }
        Ok(())
    }

    /// Rebuilds the whole index: what a repair after a bad migration needs.
    pub fn reindex_search(&self) -> Result<usize> {
        {
            let c = self.db.locked();
            c.execute("DELETE FROM entries_fts", [])?;
        }
        self.backfill_search()?;
        let c = self.db.locked();
        Ok(c.query_row("SELECT COUNT(*) FROM entries_fts", [], |r| r.get::<_, i64>(0))? as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{EntryKind, Lane, tests::temp_store};
    use serde_json::json;

    fn said(store: &Store, bot: &str, text: &str) {
        store
            .insert_entry(&Lane::main(bot), EntryKind::User, 1, &json!({"text": text}))
            .expect("an entry");
    }

    #[test]
    fn a_search_finds_the_message_that_said_it() {
        let store = temp_store();
        store.save_bot(&serde_json::from_value(json!({"id": "b1", "name": "B"})).unwrap()).unwrap();
        said(&store, "b1", "the deploy needs a rollback plan");
        said(&store, "b1", "lunch at one");

        let hits = store.search("b1", "rollback", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert!(hits[0]["snippet"].as_str().is_some_and(|s| s.contains("[rollback]")));
        assert!(store.search("b1", "nothing like this", 10).unwrap().is_empty());
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
