//! A review's immutable payload and its effect commit together, so retrying cannot save twice.

use super::{Document, DocumentDraft, DocumentWrite, Store, check_bot, save, validate};
use crate::LockExt;
use crate::store::now_ms;
use anyhow::{Result, bail};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};

const REVIEW_TTL_MS: i64 = 60 * 60 * 1_000;
const MAX_REVIEWS_PER_BOT: i64 = 1_000;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ReviewStatus {
    Pending,
    Approved,
    Declined,
    Expired,
    Conflict,
}

impl ReviewStatus {
    fn read(value: &str, expires_at: i64) -> rusqlite::Result<Self> {
        Ok(match value {
            "pending" if expires_at <= now_ms() => Self::Expired,
            "pending" => Self::Pending,
            "approved" => Self::Approved,
            "declined" => Self::Declined,
            "expired" => Self::Expired,
            "conflict" => Self::Conflict,
            _ => return Err(rusqlite::Error::InvalidQuery),
        })
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReviewDecision {
    Approve,
    Decline,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentReview {
    pub id: String,
    pub bot_id: String,
    pub draft: DocumentDraft,
    pub status: ReviewStatus,
    pub document: Option<Document>,
    pub created_at: i64,
    pub expires_at: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentReviewSummary {
    pub id: String,
    pub bot_id: String,
    pub title: String,
    pub document_id: Option<String>,
    pub status: ReviewStatus,
    pub created_at: i64,
    pub expires_at: i64,
}

fn decode<T: DeserializeOwned>(text: &str, index: usize) -> rusqlite::Result<T> {
    serde_json::from_str(text)
        .map_err(|error| rusqlite::Error::FromSqlConversionFailure(index, rusqlite::types::Type::Text, Box::new(error)))
}

fn get(db: &Connection, id: &str) -> Result<Option<DocumentReview>> {
    Ok(db
        .query_row(
            "SELECT id, bot_id, payload, status, document, created_at, expires_at
        FROM document_reviews WHERE id = ?1 AND EXISTS(SELECT 1 FROM bots
        WHERE bots.id = document_reviews.bot_id AND bots.deleted = 0)",
            [id],
            |row| {
                let payload: String = row.get(2)?;
                let status: String = row.get(3)?;
                let document: Option<String> = row.get(4)?;
                let expires_at = row.get(6)?;
                Ok(DocumentReview {
                    id: row.get(0)?,
                    bot_id: row.get(1)?,
                    draft: decode(&payload, 2)?,
                    status: ReviewStatus::read(&status, expires_at)?,
                    document: document.map(|text| decode(&text, 4)).transpose()?,
                    created_at: row.get(5)?,
                    expires_at,
                })
            },
        )
        .optional()?)
}

fn digest(value: impl Serialize) -> Result<String> {
    use std::fmt::Write as _;
    let mut encoded = String::with_capacity(64);
    for byte in Sha256::digest(serde_json::to_vec(&value)?) {
        write!(encoded, "{byte:02x}").expect("writing to a String is infallible");
    }
    Ok(encoded)
}

impl Store {
    pub fn document_review(&self, id: &str) -> Result<Option<DocumentReview>> {
        get(&self.db.locked(), id)
    }

    pub fn document_reviews(&self, bot: &str) -> Result<Vec<DocumentReviewSummary>> {
        let db = self.db.locked();
        check_bot(&db, bot)?;
        let mut statement = db.prepare(
            "SELECT id, bot_id, json_extract(payload, '$.title'),
            json_extract(payload, '$.id'), status, created_at, expires_at FROM document_reviews
            WHERE bot_id = ?1 ORDER BY (status = 'pending' AND expires_at > ?2) DESC,
            created_at DESC, id DESC LIMIT 1000",
        )?;
        Ok(statement
            .query_map(rusqlite::params![bot, now_ms()], |row| {
                let status: String = row.get(4)?;
                let expires_at = row.get(6)?;
                Ok(DocumentReviewSummary {
                    id: row.get(0)?,
                    bot_id: row.get(1)?,
                    title: row.get(2)?,
                    document_id: row.get(3)?,
                    status: ReviewStatus::read(&status, expires_at)?,
                    created_at: row.get(5)?,
                    expires_at,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn review_document(&self, bot: &str, draft: DocumentDraft, nonce: Option<&str>) -> Result<DocumentReview> {
        let draft = validate(draft)?;
        if nonce.is_some_and(|nonce| nonce.is_empty() || nonce.len() > 128) {
            bail!("clientNonce must contain 1–128 bytes");
        }
        let payload_hash = digest(("document-review-v1", bot, &draft))?;
        let effect_key = match nonce {
            Some(nonce) => digest(("document-review-request-v1", bot, nonce))?,
            None => payload_hash.clone(),
        };
        let mut db = self.db.locked();
        let tx = db.transaction()?;
        check_bot(&tx, bot)?;
        let existing: Option<(String, String)> = tx
            .query_row("SELECT id, payload_hash FROM document_reviews WHERE effect_key = ?1", [&effect_key], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .optional()?;
        if let Some((id, hash)) = existing {
            if hash != payload_hash {
                bail!("clientNonce already belongs to a different document draft");
            }
            return get(&tx, &id)?.ok_or_else(|| anyhow::anyhow!("unknown document review"));
        }
        if let Some(id) = &draft.id {
            let document = super::get(&tx, id)?
                .filter(|doc| doc.bot_id == bot)
                .ok_or_else(|| anyhow::anyhow!("unknown document"))?;
            if document.revision != draft.if_revision {
                bail!("document changed; reload it before requesting review");
            }
        } else {
            super::check_parent(&tx, bot, None, draft.parent_id.as_deref())?;
        }
        let count: i64 = tx.query_row(
            "SELECT COUNT(*) FROM document_reviews WHERE bot_id = ?1 AND status = 'pending' AND expires_at > ?2",
            params![bot, now_ms()],
            |r| r.get(0),
        )?;
        if count >= MAX_REVIEWS_PER_BOT {
            bail!("document review limit reached for this bot");
        }
        let id = uuid::Uuid::new_v4().to_string();
        let now = now_ms();
        tx.execute("INSERT INTO document_reviews(id, bot_id, effect_key, payload_hash, payload, status, created_at, expires_at)
            VALUES(?1,?2,?3,?4,?5,'pending',?6,?7)",
            params![id, bot, effect_key, payload_hash, serde_json::to_string(&draft)?, now, now + REVIEW_TTL_MS])?;
        super::next_rev(&tx)?;
        tx.commit()?;
        Ok(DocumentReview {
            id,
            bot_id: bot.to_owned(),
            draft,
            status: ReviewStatus::Pending,
            document: None,
            created_at: now,
            expires_at: now + REVIEW_TTL_MS,
        })
    }

    pub fn resolve_document_review(&self, id: &str, decision: ReviewDecision) -> Result<DocumentReview> {
        let mut db = self.db.locked();
        let tx = db.transaction()?;
        let mut review = get(&tx, id)?.ok_or_else(|| anyhow::anyhow!("unknown document review"))?;
        if review.status != ReviewStatus::Pending {
            return Ok(review);
        }
        let status = match decision {
            ReviewDecision::Decline => {
                review.status = ReviewStatus::Declined;
                "declined"
            }
            ReviewDecision::Approve => match save(&tx, &review.bot_id, &review.draft)? {
                DocumentWrite::Saved(document) => {
                    review.document = Some(document);
                    review.status = ReviewStatus::Approved;
                    "approved"
                }
                DocumentWrite::Conflict(document) => {
                    review.document = Some(document);
                    review.status = ReviewStatus::Conflict;
                    "conflict"
                }
            },
        };
        let document = review.document.as_ref().map(serde_json::to_string).transpose()?;
        tx.execute(
            "UPDATE document_reviews SET status = ?2, document = ?3 WHERE id = ?1",
            params![id, status, document],
        )?;
        super::next_rev(&tx)?;
        tx.commit()?;
        Ok(review)
    }
}
