CREATE TABLE IF NOT EXISTS matrix_inbox(
    event_id TEXT PRIMARY KEY,generation TEXT NOT NULL,bot_id TEXT NOT NULL,
    room_id TEXT NOT NULL,entry_id TEXT UNIQUE NOT NULL,
    text TEXT NOT NULL,user_id TEXT NOT NULL,state TEXT NOT NULL DEFAULT 'queued'
    CHECK(state IN ('queued','started','rejected')),created_at INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS matrix_threads(
    generation TEXT NOT NULL,room_id TEXT NOT NULL,bot_id TEXT NOT NULL,
    root_id TEXT NOT NULL UNIQUE,event_id TEXT NOT NULL,
    PRIMARY KEY(generation,room_id));
CREATE TABLE IF NOT EXISTS matrix_outbox(
    entry_id TEXT PRIMARY KEY,state TEXT NOT NULL CHECK(state IN ('sending','sent','failed','unknown')),
    event_id TEXT,updated_at INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS matrix_cards(
    generation TEXT NOT NULL,room_id TEXT NOT NULL,event_id TEXT NOT NULL,
    entry_id TEXT NOT NULL,PRIMARY KEY(generation,room_id,event_id));
CREATE INDEX IF NOT EXISTS matrix_inbox_state ON matrix_inbox(state,created_at);
