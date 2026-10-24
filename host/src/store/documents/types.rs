use serde::{Deserialize, Serialize};

pub const MAX_BODY_BYTES: usize = 128 * 1024;
pub const MAX_TITLE_BYTES: usize = 512;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Document {
    pub id: String,
    pub bot_id: String,
    pub parent_id: Option<String>,
    pub title: String,
    pub body: String,
    pub revision: i64,
    pub rev: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSummary {
    pub id: String,
    pub bot_id: String,
    pub parent_id: Option<String>,
    pub title: String,
    pub revision: i64,
    pub rev: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize)]
pub struct DocumentPage {
    pub items: Vec<DocumentSummary>,
    pub next: Option<String>,
}

/// An update always carries the revision whose text the caller actually edited.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DocumentDraft {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub parent_id: Option<String>,
    pub title: String,
    pub body: String,
    pub if_revision: i64,
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", content = "document", rename_all = "camelCase")]
pub enum DocumentWrite {
    Saved(Document),
    Conflict(Document),
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum DocumentDelete {
    Deleted { ids: Vec<String>, rev: i64 },
    Conflict { document: Document },
}
