//! Durable Markdown pages. Ownership, revision checks and hierarchy live in one transaction.

mod reviews;
mod schema;
#[cfg(test)]
mod tests;
mod types;

pub use reviews::{ReviewDecision, ReviewStatus};
pub use types::{Document, DocumentDelete, DocumentDraft, DocumentPage, DocumentSummary, DocumentWrite};

use super::{Store, next_rev, now_ms};
use crate::LockExt;
use anyhow::{Result, bail};
use rusqlite::{Connection, OptionalExtension, params};
use std::collections::HashSet;

const MAX_DOCUMENTS_PER_BOT: i64 = 1_000;
const COLS: &str = "id, bot_id, parent_id, title, body, revision, rev, created_at, updated_at";

pub(super) use schema::open;

fn row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Document> {
    Ok(Document {
        id: row.get(0)?,
        bot_id: row.get(1)?,
        parent_id: row.get(2)?,
        title: row.get(3)?,
        body: row.get(4)?,
        revision: row.get(5)?,
        rev: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

fn get(db: &Connection, id: &str) -> Result<Option<Document>> {
    Ok(db
        .query_row(
            &format!(
                "SELECT {COLS} FROM documents WHERE id = ?1 AND deleted = 0
                  AND EXISTS(SELECT 1 FROM bots WHERE bots.id = documents.bot_id AND bots.deleted = 0)"
            ),
            [id],
            row,
        )
        .optional()?)
}

fn check_bot(db: &Connection, bot: &str) -> Result<()> {
    if !db
        .query_row("SELECT EXISTS(SELECT 1 FROM bots WHERE id = ?1 AND deleted = 0)", [bot], |r| r.get::<_, bool>(0))?
    {
        bail!("unknown bot");
    }
    Ok(())
}

fn validate(mut draft: DocumentDraft) -> Result<DocumentDraft> {
    draft.title = draft.title.trim().to_owned();
    if draft.title.is_empty() || draft.title.len() > types::MAX_TITLE_BYTES {
        bail!("document title must contain 1–512 bytes");
    }
    if draft.body.len() > types::MAX_BODY_BYTES {
        bail!("document body exceeds 128 KiB");
    }
    if draft.if_revision < 0 || (draft.id.is_none() && draft.if_revision != 0) {
        bail!("new documents require ifRevision 0; updates require their current revision");
    }
    if draft.id.is_some() && draft.parent_id.is_some() {
        bail!("use moveDocument to change a document's parent");
    }
    Ok(draft)
}

fn check_parent(db: &Connection, bot: &str, id: Option<&str>, parent: Option<&str>) -> Result<()> {
    let mut next = parent.map(str::to_owned);
    let mut seen = HashSet::new();
    while let Some(parent) = next {
        if id == Some(parent.as_str()) || !seen.insert(parent.clone()) {
            bail!("a document cannot be nested inside itself");
        }
        let parent = get(db, &parent)?
            .filter(|doc| doc.bot_id == bot)
            .ok_or_else(|| anyhow::anyhow!("parent document must belong to the same bot"))?;
        next = parent.parent_id;
    }
    Ok(())
}

fn save(db: &Connection, bot: &str, draft: &DocumentDraft) -> Result<DocumentWrite> {
    check_bot(db, bot)?;
    let now = now_ms();
    if let Some(id) = &draft.id {
        let current =
            get(db, id)?.filter(|doc| doc.bot_id == bot).ok_or_else(|| anyhow::anyhow!("unknown document"))?;
        if current.revision != draft.if_revision {
            return Ok(DocumentWrite::Conflict(current));
        }
        let rev = next_rev(db)?;
        db.execute("UPDATE documents SET title = ?2, body = ?3, revision = revision + 1, rev = ?4, updated_at = ?5 WHERE id = ?1",
            params![id, draft.title, draft.body, rev, now])?;
        return Ok(DocumentWrite::Saved(Document {
            title: draft.title.clone(),
            body: draft.body.clone(),
            revision: current.revision + 1,
            rev,
            updated_at: now,
            ..current
        }));
    }
    let count: i64 =
        db.query_row("SELECT COUNT(*) FROM documents WHERE bot_id = ?1 AND deleted = 0", [bot], |r| r.get(0))?;
    if count >= MAX_DOCUMENTS_PER_BOT {
        bail!("document limit reached for this bot");
    }
    check_parent(db, bot, None, draft.parent_id.as_deref())?;
    let doc = Document {
        id: uuid::Uuid::new_v4().to_string(),
        bot_id: bot.to_owned(),
        parent_id: draft.parent_id.clone(),
        title: draft.title.clone(),
        body: draft.body.clone(),
        revision: 1,
        rev: next_rev(db)?,
        created_at: now,
        updated_at: now,
    };
    db.execute(
        "INSERT INTO documents(id, bot_id, parent_id, title, body, revision, rev, created_at, updated_at)
        VALUES(?1,?2,?3,?4,?5,1,?6,?7,?7)",
        params![doc.id, bot, doc.parent_id, doc.title, doc.body, doc.rev, now],
    )?;
    Ok(DocumentWrite::Saved(doc))
}

impl Store {
    pub fn document(&self, id: &str) -> Result<Option<Document>> {
        get(&self.db.locked(), id)
    }

    pub fn documents(&self, bot: &str, query: &str, after: Option<&str>, limit: u32) -> Result<DocumentPage> {
        let db = self.db.locked();
        check_bot(&db, bot)?;
        let limit = i64::from(limit.clamp(1, 100));
        let mut statement = db.prepare(
            "SELECT id, bot_id, parent_id, title, revision, rev, created_at, updated_at
            FROM documents WHERE bot_id = ?1 AND deleted = 0 AND id > ?2
            AND instr(codync_lower(title), codync_lower(?3)) > 0 ORDER BY id LIMIT ?4",
        )?;
        let mut items = statement
            .query_map(params![bot, after.unwrap_or_default(), query, limit + 1], |r| {
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
        let more = items.len() > usize::try_from(limit).expect("page limit is positive and bounded");
        items.truncate(usize::try_from(limit).expect("page limit is bounded"));
        let next = more.then(|| items.last().expect("a full page has items").id.clone());
        Ok(DocumentPage { items, next })
    }

    pub fn save_document(&self, bot: &str, draft: DocumentDraft) -> Result<DocumentWrite> {
        self.save_document_once(bot, draft, None)
    }

    /// The same creation nonce recovers its original result, even if a later draft was edited.
    pub fn save_document_once(&self, bot: &str, draft: DocumentDraft, nonce: Option<&str>) -> Result<DocumentWrite> {
        let draft = validate(draft)?;
        if nonce.is_some_and(|nonce| nonce.is_empty() || nonce.len() > 128) {
            bail!("clientNonce must contain 1–128 bytes");
        }
        let mut db = self.db.locked();
        let tx = db.transaction()?;
        check_bot(&tx, bot)?;
        let creation_nonce = nonce.filter(|_| draft.id.is_none());
        if let Some(nonce) = creation_nonce {
            let existing: Option<String> = tx
                .query_row(
                    "SELECT document FROM document_creations WHERE bot_id = ?1 AND nonce = ?2",
                    params![bot, nonce],
                    |row| row.get(0),
                )
                .optional()?;
            if let Some(document) = existing {
                return Ok(DocumentWrite::Saved(serde_json::from_str(&document)?));
            }
        }
        let result = save(&tx, bot, &draft)?;
        if let (Some(nonce), DocumentWrite::Saved(document)) = (creation_nonce, &result) {
            tx.execute(
                "INSERT INTO document_creations(bot_id, nonce, document) VALUES(?1,?2,?3)",
                params![bot, nonce, serde_json::to_string(document)?],
            )?;
        }
        tx.commit()?;
        Ok(result)
    }

    pub fn move_document(&self, id: &str, parent: Option<&str>, revision: i64) -> Result<DocumentWrite> {
        let mut db = self.db.locked();
        let tx = db.transaction()?;
        let current = get(&tx, id)?.ok_or_else(|| anyhow::anyhow!("unknown document"))?;
        if current.revision != revision {
            return Ok(DocumentWrite::Conflict(current));
        }
        check_parent(&tx, &current.bot_id, Some(id), parent)?;
        let rev = next_rev(&tx)?;
        let now = now_ms();
        tx.execute(
            "UPDATE documents SET parent_id = ?2, revision = revision + 1, rev = ?3, updated_at = ?4 WHERE id = ?1",
            params![id, parent, rev, now],
        )?;
        tx.commit()?;
        Ok(DocumentWrite::Saved(Document {
            parent_id: parent.map(str::to_owned),
            revision: current.revision + 1,
            rev,
            updated_at: now,
            ..current
        }))
    }

    pub fn delete_document(&self, id: &str, revision: i64) -> Result<DocumentDelete> {
        let mut db = self.db.locked();
        let tx = db.transaction()?;
        let doc = get(&tx, id)?.ok_or_else(|| anyhow::anyhow!("unknown document"))?;
        if doc.revision != revision {
            return Ok(DocumentDelete::Conflict { document: doc });
        }
        let mut statement = tx.prepare(
            "WITH RECURSIVE tree(id) AS (SELECT ?1 UNION SELECT documents.id FROM documents
            JOIN tree ON documents.parent_id = tree.id WHERE documents.bot_id = ?2 AND documents.deleted = 0)
            SELECT id FROM tree",
        )?;
        let ids = statement
            .query_map(params![id, doc.bot_id], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        drop(statement);
        let rev = next_rev(&tx)?;
        for id in &ids {
            tx.execute(
                "UPDATE documents SET deleted = 1, revision = revision + 1, rev = ?2, updated_at = ?3 WHERE id = ?1",
                params![id, rev, now_ms()],
            )?;
        }
        tx.commit()?;
        Ok(DocumentDelete::Deleted { ids, rev })
    }
}
