CREATE TABLE IF NOT EXISTS conversation_links(
    bot_id TEXT NOT NULL,
    source TEXT NOT NULL,
    channel TEXT NOT NULL,
    thread TEXT NOT NULL DEFAULT '',
    root_id TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY(bot_id, source, channel, thread)
);
CREATE INDEX IF NOT EXISTS conversation_links_root ON conversation_links(bot_id, root_id);
