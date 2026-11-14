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
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};
use std::fmt::Write as _;

/// The most hits one search returns.
const MAX_HITS: u32 = 50;
/// Characters kept on each side of a match.
const SNIPPET_TOKENS: i64 = 12;
/// The `kv` key holding the last entry sequence the backfill covered.
const BACKFILLED: &str = "search.backfilled.v1";
/// Entries indexed per backfill pass.
const BACKFILL_ROWS: i64 = 2_000;
/// How many candidates the rank returns per hit asked for, before recency reorders them.
const RANKED_FACTOR: i64 = 4;
/// Rank points a hit loses per day of age. BM25 differences between two decent matches are around one, so a
/// few days apart settles a tie and a month buries a match that is otherwise as good.
const AGE_PER_DAY: f64 = 0.12;
/// The most the age can cost, so an old exact match still beats a vague recent one.
const AGE_CAP: f64 = 3.0;

pub(super) fn open(c: &Connection) -> Result<()> {
    c.execute_batch(
        "CREATE VIRTUAL TABLE IF NOT EXISTS search_fts USING fts5(
            body, kind UNINDEXED, source_id UNINDEXED, bot_id UNINDEXED, at UNINDEXED,
            tokenize = 'unicode61 remove_diacritics 2');",
    )?;
    Ok(())
}

/// The words of a message worth finding: the user's or the agent's text, and a tool's title. A key found in it
/// is masked, so a token pasted into the chat doesn't sit in a second place in the clear.
pub fn message_body(data: &Value) -> String {
    ["text", "title", "detail"]
        .iter()
        .filter_map(|key| data[*key].as_str())
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(scrub)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Token prefixes that say "this is a secret" (`sk-`, a GitHub token, an AWS key, a Slack token, `Bearer`).
const SECRET_PREFIXES: [&str; 6] = ["sk-", "ghp_", "gho_", "ghs_", "AKIA", "xox"];
const MASK: &str = "…";

/// Replaces each token that looks like a key with its first characters and a mask. The prefix still finds the
/// entry, and the index never holds the token twice.
fn scrub(text: &str) -> String {
    text.split(' ')
        .map(|word| {
            let secret = word.len() >= 12
                && (SECRET_PREFIXES.iter().any(|prefix| word.starts_with(prefix)) || word.starts_with("Bearer"));
            if !secret {
                return word.to_owned();
            }
            format!("{}{MASK}", word.chars().take(6).collect::<String>())
        })
        .collect::<Vec<_>>()
        .join(" ")
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

/// Indexes one message as it is written, replacing what it said before (a message still streaming grows, and a
/// backfill pass runs over rows that may already be indexed). Internal entries (a turn's own plumbing, not
/// something the person or the bot said to somebody) stay out, including one that became internal.
pub(super) fn index_message(c: &Connection, id: &str, bot: &str, at: i64, data: &Value) -> Result<()> {
    forget(c, "message", id)?;
    if super::is_internal(data) {
        return Ok(());
    }
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

/// What a hit's age costs it in the ranking: capped, so an old exact match still beats a vague recent one.
fn age_penalty(at: i64, now: i64) -> f64 {
    #[allow(clippy::cast_precision_loss)]
    let days = (now - at).max(0) as f64 / 86_400_000.0;
    (days * AGE_PER_DAY).min(AGE_CAP)
}

/// What a person typed, as an FTS5 query: every word, quoted and as a prefix, all of them required. A stray
/// quote or `AND` becomes a word instead of a syntax error; a word with no letter or digit has nothing to
/// index and is dropped.
fn terms(text: &str) -> String {
    text.split_whitespace()
        .filter(|word| word.chars().any(char::is_alphanumeric))
        .map(|word| format!("\"{}\"*", word.replace('"', "")))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Whose side of a chat a search keeps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Author {
    User,
    Agent,
}

impl Author {
    /// The entry kinds the author writes. A page belongs to nobody, so an author keeps messages only.
    fn kinds(self) -> &'static str {
        match self {
            Self::User => "'user'",
            Self::Agent => "'agent', 'plan'",
        }
    }

    /// The wire value (`author` in a request), if it names a side.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "user" => Some(Self::User),
            "agent" => Some(Self::Agent),
            _ => None,
        }
    }
}

/// How a search narrows: everything worth reading, anywhere in time, by default.
#[derive(Clone, Copy, Debug, Default)]
pub struct Filters {
    /// Tool traces, thoughts, permission prompts and notices too.
    pub tools: bool,
    /// Only what this side said.
    pub author: Option<Author>,
    /// Only hits from this moment on (epoch milliseconds).
    pub since: Option<i64>,
    /// Only hits up to this moment (epoch milliseconds).
    pub until: Option<i64>,
}

impl Store {
    /// What this bot knows about `query`, best match first: the source, its kind and a snippet. Only what was
    /// said is searched: tool traces and notices are the agent's working noise, `search_with_tools` keeps them.
    pub fn search(&self, bot: &str, query: &str, limit: u32) -> Result<Vec<Value>> {
        self.search_filtered(bot, query, limit, &Filters::default())
    }

    /// The same search, tool traces and notices included: a person looking for what a tool printed.
    pub fn search_with_tools(&self, bot: &str, query: &str, limit: u32) -> Result<Vec<Value>> {
        self.search_filtered(bot, query, limit, &Filters { tools: true, ..Filters::default() })
    }

    /// The search narrowed by `filters`, best match first with the newest of equally good matches before the
    /// oldest: last week's answer is usually the one somebody means.
    pub fn search_filtered(&self, bot: &str, query: &str, limit: u32, filters: &Filters) -> Result<Vec<Value>> {
        let terms = terms(query);
        if terms.is_empty() {
            return Ok(Vec::new());
        }
        let limit = i64::from(limit.clamp(1, MAX_HITS));
        self.backfill_search()?;
        let mut args: Vec<Box<dyn rusqlite::ToSql>> =
            vec![Box::new(terms), Box::new(bot.to_owned()), Box::new(SNIPPET_TOKENS)];
        // The limit applies after the kinds are picked, so a page of traces doesn't fill the answer.
        let mut wanted = String::new();
        if !filters.tools {
            wanted.push_str(" AND (f.kind = 'document' OR e.kind IN ('user', 'agent', 'plan'))");
        }
        if let Some(author) = filters.author {
            // The kinds are literal SQL already (`Author::kinds`), not bound values.
            let _ = write!(wanted, " AND f.kind = 'message' AND e.kind IN ({})", author.kinds());
        }
        if let Some(since) = filters.since {
            args.push(Box::new(since));
            let _ = write!(wanted, " AND f.at >= ?{}", args.len());
        }
        if let Some(until) = filters.until {
            args.push(Box::new(until));
            let _ = write!(wanted, " AND f.at <= ?{}", args.len());
        }
        let c = self.db.locked();
        // More candidates than the limit: recency reorders what the rank found, so the cut has to wait.
        args.push(Box::new(limit * RANKED_FACTOR));
        let now = super::now_ms();
        let sql = format!(
            "SELECT f.kind, f.source_id, f.at, snippet(search_fts, 0, '[', ']', '...', ?3),
                    e.kind, e.data, d.title, bm25(search_fts), e.thread_id
             FROM search_fts f
             LEFT JOIN entries e ON f.kind = 'message' AND e.id = f.source_id
             LEFT JOIN documents d ON f.kind = 'document' AND d.id = f.source_id
             WHERE search_fts MATCH ?1 AND f.bot_id = ?2{wanted}
             ORDER BY rank LIMIT ?{}",
            args.len()
        );
        let mut statement = c.prepare(&sql)?;
        let params: Vec<&dyn rusqlite::ToSql> = args.iter().map(AsRef::as_ref).collect();
        let mut ranked = statement
            .query_map(params.as_slice(), |row| {
                let kind: String = row.get(0)?;
                let at: i64 = row.get(2)?;
                let mut hit = json!({
                    "kind": kind,
                    "id": row.get::<_, String>(1)?,
                    "at": at,
                    "snippet": row.get::<_, String>(3)?,
                });
                if kind == "message" {
                    hit["entryKind"] = row.get::<_, Option<String>>(4)?.into();
                    hit["author"] = row
                        .get::<_, Option<String>>(5)?
                        .and_then(|data| serde_json::from_str::<Value>(&data).ok())
                        .and_then(|data| data["author"].as_str().map(str::to_owned))
                        .into();
                    hit["threadId"] = row.get::<_, Option<String>>(8)?.into();
                } else {
                    hit["title"] = row.get::<_, Option<String>>(6)?.into();
                }
                Ok((row.get::<_, f64>(7)? + age_penalty(at, now), hit))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        ranked.sort_by(|(a, _), (b, _)| a.total_cmp(b));
        ranked.truncate(usize::try_from(limit).unwrap_or(usize::MAX));
        Ok(ranked.into_iter().map(|(_, hit)| hit).collect())
    }

    /// Fills the index from what a database already had, resumable: each pass takes the entries past the
    /// last one it covered, so a chat and a library written before search existed are searchable without
    /// a manual step and a large history never blocks one search on all of it.
    pub(super) fn backfill_search(&self) -> Result<()> {
        let marker = BACKFILLED;
        let done: i64 = {
            let c = self.db.locked();
            c.query_row("SELECT CAST(v AS INTEGER) FROM kv WHERE k = ?1", [marker], |r| r.get(0))
                .optional()?
                .unwrap_or(0)
        };
        let rows: Vec<(i64, String, String, i64, String)> = {
            let c = self.db.locked();
            let mut statement = c.prepare(
                "SELECT seq, id, bot_id, created_at, data FROM entries WHERE seq > ?1 ORDER BY seq LIMIT ?2",
            )?;
            statement
                .query_map(params![done, BACKFILL_ROWS], |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?
        };
        if rows.is_empty() {
            return Ok(());
        }
        let documents: Vec<(String, String, i64, String, String)> = if done == 0 {
            let c = self.db.locked();
            let mut statement =
                c.prepare("SELECT id, bot_id, updated_at, title, body FROM documents WHERE deleted = 0")?;
            statement
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)))?
                .collect::<rusqlite::Result<Vec<_>>>()?
        } else {
            Vec::new()
        };
        let mut c = self.db.locked();
        let tx = c.transaction()?;
        let mut last = done;
        for (seq, id, bot, at, data) in &rows {
            if let Ok(data) = serde_json::from_str::<Value>(data) {
                index_message(&tx, id, bot, *at, &data)?;
            }
            last = *seq;
        }
        for (id, bot, at, title, body) in &documents {
            index_document(&tx, id, bot, *at, title, body)?;
        }
        tx.execute(
            "INSERT INTO kv(k, v) VALUES (?1, ?2) ON CONFLICT(k) DO UPDATE SET v = ?2",
            params![marker, last.to_string()],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Rebuilds the whole index: what a repair after a bad migration needs.
    pub fn reindex_search(&self) -> Result<usize> {
        {
            let c = self.db.locked();
            c.execute("DELETE FROM search_fts", [])?;
            c.execute("DELETE FROM kv WHERE k = ?1", [BACKFILLED])?;
        }
        // The whole history, a pass at a time, until a pass has nothing left to cover.
        loop {
            let before = {
                let c = self.db.locked();
                c.query_row("SELECT v FROM kv WHERE k = ?1", [BACKFILLED], |r| r.get::<_, String>(0)).optional()?
            };
            self.backfill_search()?;
            let after = {
                let c = self.db.locked();
                c.query_row("SELECT v FROM kv WHERE k = ?1", [BACKFILLED], |r| r.get::<_, String>(0)).optional()?
            };
            if before == after {
                break;
            }
        }
        let c = self.db.locked();
        let indexed: i64 = c.query_row("SELECT COUNT(*) FROM search_fts", [], |r| r.get(0))?;
        Ok(usize::try_from(indexed).unwrap_or_default())
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

    /// Puts a message at a moment in both its row and the index, so a test needn't race the clock.
    fn at(store: &Store, id: &str, ms: i64) {
        store.backdate_entry(id, ms);
        store.db.locked().execute("UPDATE search_fts SET at = ?1 WHERE source_id = ?2", params![ms, id]).unwrap();
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
        assert_eq!(store.search("b1", "nothing like this", 10).unwrap().len(), 0);
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
        assert_eq!(store.search("b1", "   ", 10).unwrap().len(), 0);
    }

    #[test]
    fn internal_messages_and_their_keys_stay_out() {
        let store = temp_store();
        store.save_bot(&serde_json::from_value(json!({"id": "b1", "name": "B"})).unwrap()).unwrap();
        store
            .insert_entry(&Lane::main("b1"), EntryKind::Notice, 1, &json!({"text": "plumbing", "internal": true}))
            .unwrap();
        said(&store, "b1", "use sk-ant-api03-abcdefghijklmnop for the key");
        assert!(store.search("b1", "plumbing", 10).unwrap().is_empty(), "internal");
        assert_eq!(store.search("b1", "sk-ant", 10).unwrap().len(), 1, "the prefix still finds it");
        let hit = &store.search("b1", "sk-ant", 10).unwrap()[0];
        let snippet = hit["snippet"].as_str().unwrap();
        assert!(snippet.contains("[sk-ant]"), "the match is marked: {snippet}");
        assert!(snippet.contains("[sk-ant]…"), "and masked right after it: {snippet}");
        assert!(!snippet.contains("abcdefghijklmnop"), "{snippet}");
    }

    #[test]
    fn tool_traces_stay_out_unless_asked_for() {
        let store = temp_store();
        store.save_bot(&serde_json::from_value(json!({"id": "b1", "name": "B"})).unwrap()).unwrap();
        store.insert_entry(&Lane::main("b1"), EntryKind::Tool, 1, &json!({"title": "grep deploy"})).unwrap();

        assert_eq!(store.search("b1", "grep", 10).unwrap().len(), 0);
        assert_eq!(store.search_with_tools("b1", "grep", 10).unwrap().len(), 1);
    }

    #[test]
    fn a_newer_hit_of_the_same_words_comes_first() {
        let store = temp_store();
        store.save_bot(&serde_json::from_value(json!({"id": "b1", "name": "B"})).unwrap()).unwrap();
        let old = store
            .insert_entry(&Lane::main("b1"), EntryKind::User, 1, &json!({"text": "ship the deploy on friday"}))
            .unwrap();
        let new = store
            .insert_entry(&Lane::main("b1"), EntryKind::User, 2, &json!({"text": "ship the deploy on friday"}))
            .unwrap();
        // Two months of age, in the row and the index alike, so the backfill agrees with it.
        let now = super::super::now_ms();
        at(&store, &old.id, now - 60 * 86_400_000);
        at(&store, &new.id, now);

        let hits = store.search("b1", "ship deploy", 10).unwrap();
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0]["id"], new.id, "the recent one is first");
    }

    #[test]
    fn an_author_keeps_only_that_side_of_the_chat() {
        let store = temp_store();
        store.save_bot(&serde_json::from_value(json!({"id": "b1", "name": "B"})).unwrap()).unwrap();
        said(&store, "b1", "the deploy is blocked");
        store
            .insert_entry(&Lane::main("b1"), EntryKind::Agent, 2, &json!({"text": "the deploy is unblocked"}))
            .unwrap();
        page(&store, "b1", "Deploy notes", "the deploy is a script");

        let mine = store
            .search_filtered("b1", "deploy", 10, &Filters { author: Some(Author::User), ..Filters::default() })
            .unwrap();
        assert_eq!(mine.len(), 1);
        assert_eq!(mine[0]["entryKind"], "user");
        let theirs = store
            .search_filtered("b1", "deploy", 10, &Filters { author: Some(Author::Agent), ..Filters::default() })
            .unwrap();
        assert_eq!(theirs.len(), 1);
        assert_eq!(theirs[0]["entryKind"], "agent");
    }

    #[test]
    fn a_date_window_keeps_only_what_it_covers() {
        let store = temp_store();
        store.save_bot(&serde_json::from_value(json!({"id": "b1", "name": "B"})).unwrap()).unwrap();
        let old = store.insert_entry(&Lane::main("b1"), EntryKind::User, 1, &json!({"text": "deploy monday"})).unwrap();
        let new =
            store.insert_entry(&Lane::main("b1"), EntryKind::User, 2, &json!({"text": "deploy tuesday"})).unwrap();
        // Pinned, so two inserts can't land in the same millisecond the boundary is taken from.
        let boundary = super::super::now_ms();
        at(&store, &old.id, boundary - 1_000);
        at(&store, &new.id, boundary + 1_000);

        let since = store
            .search_filtered("b1", "deploy", 10, &Filters { since: Some(boundary), ..Filters::default() })
            .unwrap();
        assert_eq!(since.len(), 1, "only the later one");
        assert_eq!(since[0]["id"], new.id);
        let until = store
            .search_filtered("b1", "deploy", 10, &Filters { until: Some(boundary), ..Filters::default() })
            .unwrap();
        assert_eq!(until.len(), 1, "only the earlier one");
        assert_eq!(until[0]["id"], old.id);
    }

    #[test]
    fn the_backfill_resumes_from_where_it_stopped() {
        let store = temp_store();
        store.save_bot(&serde_json::from_value(json!({"id": "b1", "name": "B"})).unwrap()).unwrap();
        for turn in 0..5 {
            said(&store, "b1", &format!("deploy note {turn}"));
        }
        {
            // As if the rows had been written before search existed.
            let c = store.db.locked();
            c.execute("DELETE FROM search_fts", []).unwrap();
            c.execute("DELETE FROM kv WHERE k = ?1", [BACKFILLED]).unwrap();
            c.execute("INSERT INTO kv(k, v) VALUES (?1, '3')", [BACKFILLED]).unwrap();
        }
        let hits = store.search("b1", "deploy", 50).unwrap();
        assert_eq!(hits.len(), 2, "only the entries past the marker: {}", hits.len());
        assert_eq!(store.reindex_search().unwrap(), 5, "a reindex takes all of it");
    }

    #[test]
    fn deleting_a_chat_takes_its_hits_with_it() {
        let store = temp_store();
        store.save_bot(&serde_json::from_value(json!({"id": "b1", "name": "B"})).unwrap()).unwrap();
        said(&store, "b1", "deploy monday");
        page(&store, "b1", "Deploy notes", "the deploy is a script");

        store.delete_bot("b1").unwrap();

        assert_eq!(store.search("b1", "deploy", 10).unwrap(), [] as [Value; 0]);
    }
}
