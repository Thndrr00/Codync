CREATE TABLE IF NOT EXISTS teams_inbox(
    activity_id TEXT PRIMARY KEY,generation TEXT NOT NULL,bot_id TEXT NOT NULL,
    conversation_id TEXT NOT NULL,entry_id TEXT UNIQUE NOT NULL,
    text TEXT NOT NULL,user_id TEXT NOT NULL,service_url TEXT NOT NULL,
    state TEXT NOT NULL DEFAULT 'queued' CHECK(state IN ('queued','started','rejected')),
    created_at INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS teams_threads(
    generation TEXT NOT NULL,conversation_id TEXT NOT NULL,bot_id TEXT NOT NULL,
    root_id TEXT NOT NULL UNIQUE,activity_id TEXT NOT NULL,
    PRIMARY KEY(generation,conversation_id));
CREATE TABLE IF NOT EXISTS teams_refs(
    generation TEXT NOT NULL,conversation_id TEXT NOT NULL,
    service_url TEXT NOT NULL,tenant_id TEXT NOT NULL DEFAULT '',
    user_id TEXT NOT NULL DEFAULT '',user_name TEXT NOT NULL DEFAULT '',
    updated_at INTEGER NOT NULL,PRIMARY KEY(generation,conversation_id));
CREATE TABLE IF NOT EXISTS teams_outbox(
    entry_id TEXT PRIMARY KEY,state TEXT NOT NULL CHECK(state IN ('sending','sent','failed','unknown')),
    activity_id TEXT,updated_at INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS teams_cards(
    generation TEXT NOT NULL,conversation_id TEXT NOT NULL,activity_id TEXT NOT NULL,
    entry_id TEXT NOT NULL,PRIMARY KEY(generation,conversation_id,activity_id));
CREATE INDEX IF NOT EXISTS teams_inbox_state ON teams_inbox(state,created_at);
