CREATE TABLE IF NOT EXISTS connector_tool_policies(
    connector_id TEXT NOT NULL, tool_name TEXT NOT NULL,
    mode TEXT NOT NULL CHECK(mode IN ('allow','ask','deny')),
    PRIMARY KEY(connector_id,tool_name));
CREATE TABLE IF NOT EXISTS pending_tool_calls(
    id TEXT PRIMARY KEY, bot_id TEXT NOT NULL, connector_id TEXT NOT NULL,
    tool_name TEXT NOT NULL, message TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('pending','approved','declined','revoked','dispatched')),
    created_at INTEGER NOT NULL, expires_at INTEGER NOT NULL);
CREATE INDEX IF NOT EXISTS pending_tool_calls_bot ON pending_tool_calls(bot_id,status,expires_at);
CREATE TABLE IF NOT EXISTS connector_tool_catalog(
    connector_id TEXT NOT NULL,tool_name TEXT NOT NULL,read_only INTEGER NOT NULL CHECK(read_only IN (0,1)),
    PRIMARY KEY(connector_id,tool_name));
CREATE TABLE IF NOT EXISTS mcp_call_results(
    call_id TEXT PRIMARY KEY,result TEXT NOT NULL,created_at INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS mcp_execution_claims(
    call_id TEXT PRIMARY KEY,created_at INTEGER NOT NULL);
CREATE TRIGGER IF NOT EXISTS mcp_call_cleanup AFTER DELETE ON pending_tool_calls
BEGIN
    DELETE FROM mcp_call_results WHERE call_id=old.id;
    DELETE FROM mcp_execution_claims WHERE call_id=old.id;
END;
