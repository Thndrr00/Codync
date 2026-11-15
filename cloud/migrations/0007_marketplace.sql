-- Marketplace reputation: a star rating per person and app, and an install tally per app. The app id
-- is the catalog's (`openbot/<id>`, a skill's source, an MCP server name), so no catalog lives here.

CREATE TABLE IF NOT EXISTS app_ratings(
  app_id TEXT NOT NULL,
  user_id TEXT NOT NULL,
  stars INTEGER NOT NULL CHECK (stars BETWEEN 1 AND 5),
  updated_at INTEGER NOT NULL,
  PRIMARY KEY (app_id, user_id)
);

CREATE TABLE IF NOT EXISTS app_installs(
  app_id TEXT NOT NULL PRIMARY KEY,
  installs INTEGER NOT NULL DEFAULT 0
);
