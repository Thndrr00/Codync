CREATE TABLE IF NOT EXISTS policies(
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    expression TEXT NOT NULL,
    effect TEXT NOT NULL CHECK(effect IN ('allow','deny')),
    message TEXT NOT NULL DEFAULT '',
    enabled INTEGER NOT NULL CHECK(enabled IN (0,1)),
    position INTEGER NOT NULL,
    version INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL);
CREATE INDEX IF NOT EXISTS policies_position ON policies(position, name);
CREATE TABLE IF NOT EXISTS policy_versions(
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    policy_id TEXT NOT NULL,
    version INTEGER NOT NULL,
    body TEXT NOT NULL,
    saved_at INTEGER NOT NULL,
    UNIQUE(policy_id, version));
CREATE TRIGGER IF NOT EXISTS policy_versions_cleanup AFTER DELETE ON policies
BEGIN
    DELETE FROM policy_versions WHERE policy_id=old.id;
END;
