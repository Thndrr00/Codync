use anyhow::Result;
use rusqlite::Connection;

pub(in crate::store) fn open(db: &Connection) -> Result<()> {
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS documents(
            id TEXT PRIMARY KEY, bot_id TEXT NOT NULL, parent_id TEXT,
            title TEXT NOT NULL, body TEXT NOT NULL, revision INTEGER NOT NULL,
            rev INTEGER NOT NULL, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL,
            deleted INTEGER NOT NULL DEFAULT 0);
         CREATE INDEX IF NOT EXISTS documents_bot ON documents(bot_id, deleted, id);
         CREATE INDEX IF NOT EXISTS documents_parent ON documents(parent_id, deleted);
         CREATE TABLE IF NOT EXISTS document_creations(
            bot_id TEXT NOT NULL, nonce TEXT NOT NULL, document TEXT NOT NULL,
            PRIMARY KEY(bot_id, nonce));
         CREATE TABLE IF NOT EXISTS document_reviews(
            id TEXT PRIMARY KEY, bot_id TEXT NOT NULL, effect_key TEXT UNIQUE NOT NULL,
            payload_hash TEXT NOT NULL, payload TEXT NOT NULL,
            status TEXT NOT NULL CHECK(status IN ('pending','approved','declined','expired','conflict')),
            document TEXT, created_at INTEGER NOT NULL, expires_at INTEGER NOT NULL);
         CREATE INDEX IF NOT EXISTS document_reviews_bot ON document_reviews(bot_id, created_at);",
    )?;
    Ok(())
}
