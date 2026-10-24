CREATE TABLE IF NOT EXISTS slack_inbox(
    event_id TEXT PRIMARY KEY,generation TEXT NOT NULL,bot_id TEXT NOT NULL,
    channel_id TEXT NOT NULL,thread_ts TEXT NOT NULL,entry_id TEXT UNIQUE NOT NULL,
    text TEXT NOT NULL,user_id TEXT NOT NULL,state TEXT NOT NULL DEFAULT 'queued'
    CHECK(state IN ('queued','started','rejected')),created_at INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS slack_threads(
    generation TEXT NOT NULL,channel_id TEXT NOT NULL,thread_ts TEXT NOT NULL,
    bot_id TEXT NOT NULL,root_id TEXT NOT NULL UNIQUE,
    PRIMARY KEY(generation,channel_id,thread_ts));
CREATE TABLE IF NOT EXISTS slack_outbox(
    entry_id TEXT PRIMARY KEY,state TEXT NOT NULL CHECK(state IN ('sending','sent','failed','unknown')),
    external_ts TEXT,updated_at INTEGER NOT NULL);
CREATE INDEX IF NOT EXISTS slack_inbox_state ON slack_inbox(state,created_at);
