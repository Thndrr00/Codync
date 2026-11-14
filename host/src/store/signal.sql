CREATE TABLE IF NOT EXISTS signal_inbox(
    message_id TEXT PRIMARY KEY,generation TEXT NOT NULL,bot_id TEXT NOT NULL,
    chat TEXT NOT NULL,entry_id TEXT UNIQUE NOT NULL,
    text TEXT NOT NULL,files TEXT NOT NULL DEFAULT '[]',user_id TEXT NOT NULL,state TEXT NOT NULL DEFAULT 'queued'
    CHECK(state IN ('queued','started','rejected')),created_at INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS signal_threads(
    generation TEXT NOT NULL,chat TEXT NOT NULL,bot_id TEXT NOT NULL,
    root_id TEXT NOT NULL UNIQUE,message_id TEXT NOT NULL,
    PRIMARY KEY(generation,chat));
CREATE TABLE IF NOT EXISTS signal_outbox(
    entry_id TEXT PRIMARY KEY,state TEXT NOT NULL CHECK(state IN ('sending','sent','failed','unknown')),
    message_id TEXT,updated_at INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS signal_cards(
    generation TEXT NOT NULL,chat TEXT NOT NULL,message_id TEXT NOT NULL,
    entry_id TEXT NOT NULL,PRIMARY KEY(generation,chat,message_id));
CREATE INDEX IF NOT EXISTS signal_inbox_state ON signal_inbox(state,created_at);
