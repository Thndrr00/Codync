//! Owner-facing document intents. Approval accepts an ID and decision, never replacement text.

use super::str_arg;
use crate::hub::Hub;
use crate::store::{Document, DocumentDraft, ReviewDecision};
use anyhow::{Context, Result, anyhow, bail};
use serde::Deserialize;
use serde_json::{Value, json};

pub(super) const METHODS: &[&str] = &[
    "listDocuments",
    "getDocument",
    "saveDocument",
    "moveDocument",
    "deleteDocument",
    "listDocumentReviews",
    "getDocumentReview",
    "reviewDocument",
    "resolveAction",
];

fn document(hub: &Hub, id: &str) -> Result<Document> {
    hub.store.document(id)?.ok_or_else(|| anyhow!("unknown document"))
}

fn revision(body: &Value) -> Result<i64> {
    body["ifRevision"].as_i64().filter(|revision| *revision >= 0).context("ifRevision must be a nonnegative integer")
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Resolution {
    id: String,
    decision: ReviewDecision,
}

pub(super) fn call(hub: &Hub, method: &str, body: Value) -> Result<Value> {
    Ok(match method {
        "listDocuments" => {
            let bot = str_arg(&body, "botId")?;
            let limit = match body.get("limit") {
                Some(value) => u32::try_from(value.as_u64().context("limit must be a positive integer")?)
                    .context("limit is too large")?,
                None => 100,
            };
            serde_json::to_value(hub.store.documents(
                bot,
                body["query"].as_str().unwrap_or_default(),
                body["afterId"].as_str(),
                limit,
            )?)?
        }
        "getDocument" => json!({"document": document(hub, str_arg(&body, "id")?)?}),
        "saveDocument" => {
            let bot = str_arg(&body, "botId")?.to_owned();
            let nonce = body
                .get("clientNonce")
                .map(|value| value.as_str().map(str::to_owned).context("clientNonce must be a string"))
                .transpose()?;
            let mut draft = body;
            let fields = draft.as_object_mut().context("document request must be an object")?;
            fields.remove("botId");
            fields.remove("clientNonce");
            let draft: DocumentDraft = serde_json::from_value(draft).context("invalid document draft")?;
            serde_json::to_value(
                hub.change_documents(&bot, |store| store.save_document_once(&bot, draft, nonce.as_deref()))?,
            )?
        }
        "moveDocument" => {
            let id = str_arg(&body, "id")?;
            let doc = document(hub, id)?;
            let parent = match body.get("parentId") {
                Some(Value::Null) => None,
                Some(Value::String(parent)) => Some(parent.as_str()),
                _ => bail!("parentId must be a document ID or null"),
            };
            serde_json::to_value(
                hub.change_documents(&doc.bot_id, |store| store.move_document(id, parent, revision(&body)?))?,
            )?
        }
        "deleteDocument" => {
            let id = str_arg(&body, "id")?;
            let doc = document(hub, id)?;
            serde_json::to_value(
                hub.change_documents(&doc.bot_id, |store| store.delete_document(id, revision(&body)?))?,
            )?
        }
        "listDocumentReviews" => json!({"items": hub.store.document_reviews(str_arg(&body, "botId")?)?}),
        "getDocumentReview" => {
            let review = hub.store.document_review(str_arg(&body, "id")?)?.context("unknown document review")?;
            let current = review.draft.id.as_deref().map(|id| hub.store.document(id)).transpose()?.flatten();
            json!({"review": review, "currentDocument": current})
        }
        "reviewDocument" => {
            let bot = str_arg(&body, "botId")?;
            let draft: DocumentDraft = serde_json::from_value(body["draft"].clone()).context("invalid review draft")?;
            let nonce = body
                .get("clientNonce")
                .map(|value| value.as_str().context("clientNonce must be a string"))
                .transpose()?;
            json!({"review": hub.change_documents(bot, |store| store.review_document(bot, draft, nonce))?})
        }
        "resolveAction" => {
            let resolution: Resolution =
                serde_json::from_value(body).context("approval accepts only id and decision")?;
            let review = hub.store.document_review(&resolution.id)?.context("unknown document review")?;
            json!({"review": hub.change_documents(&review.bot_id, |store| store.resolve_document_review(&resolution.id, resolution.decision))?})
        }
        _ => bail!("unknown document method"),
    })
}
