-- Marketplace changelog and verified publishers: what changed in an app, and which creators the
-- maintainers vouched for.

CREATE TABLE IF NOT EXISTS app_changelog(
  app_id TEXT NOT NULL,
  version TEXT NOT NULL,
  notes TEXT NOT NULL,
  published_at INTEGER NOT NULL,
  PRIMARY KEY (app_id, version)
);

CREATE TABLE IF NOT EXISTS verified_publishers(
  name TEXT PRIMARY KEY,
  verified_at INTEGER NOT NULL
);
