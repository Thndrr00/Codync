// Marketplace reputation: what other people think of a catalog app. Reading is public (a catalog
// screen needs it before anyone signs in); rating and counting an install need an account.

import { ApiError, body, user, type Ctx } from "./common";

/** The catalog's id for an app; it may carry a namespace (`openbot/<id>`, `skills/<name>`). */
const APP_ID = /^[A-Za-z0-9][A-Za-z0-9._:@/-]{0,119}$/;
function appId(c: Ctx, given?: string): string {
  const id = given ?? c.url.pathname.split("/")[4] ?? "";
  if (!APP_ID.test(id)) throw new ApiError("badRequest", "Invalid app id");
  return id;
}

interface RatingRow {
  app_id: string;
  count: number;
  stars: number;
}

/** Every app with at least one rating or install, so one request draws a whole catalog. */
export async function reputation(c: Ctx) {
  const ratings = await c.env.DB.prepare(
    "SELECT app_id, COUNT(*) AS count, AVG(stars) AS stars FROM app_ratings GROUP BY app_id",
  ).all<RatingRow>();
  const installs = await c.env.DB.prepare("SELECT app_id, installs FROM app_installs").all<{
    app_id: string;
    installs: number;
  }>();
  const apps = new Map<string, { appId: string; stars: number; count: number; installs: number }>();
  for (const row of ratings.results ?? []) {
    apps.set(row.app_id, { appId: row.app_id, stars: row.stars, count: row.count, installs: 0 });
  }
  for (const row of installs.results ?? []) {
    const current = apps.get(row.app_id) ?? { appId: row.app_id, stars: 0, count: 0, installs: 0 };
    current.installs = row.installs;
    apps.set(row.app_id, current);
  }
  return { apps: [...apps.values()].sort((a, b) => b.installs - a.installs || b.count - a.count) };
}

/** This account's rating of an app, 1 to 5; rating it again replaces the old stars. */
export async function rate(c: Ctx, id?: string) {
  const app = appId(c, id);
  const me = await user(c);
  const given = body(c).stars;
  if (typeof given !== "number" || !Number.isInteger(given) || given < 1 || given > 5) {
    throw new ApiError("badRequest", "stars must be 1 to 5");
  }
  const stars: number = given;
  await c.env.DB.prepare(
    `INSERT INTO app_ratings(app_id, user_id, stars, updated_at) VALUES (?1, ?2, ?3, ?4)
     ON CONFLICT(app_id, user_id) DO UPDATE SET stars = excluded.stars, updated_at = excluded.updated_at`,
  )
    .bind(app, me.userId, stars, c.now)
    .run();
  return { appId: app, stars };
}

/** The account added this app to one of its computers: counted once per account and app. */
export async function installed(c: Ctx, id?: string) {
  const app = appId(c, id);
  await user(c);
  await c.env.DB.prepare(
    `INSERT INTO app_installs(app_id, installs) VALUES (?1, 1)
     ON CONFLICT(app_id) DO UPDATE SET installs = app_installs.installs + 1`,
  )
    .bind(app)
    .run();
  return { appId: app };
}

// ---- changelog and verified publishers ----

/** Only the maintainers may write these: a bearer token from the worker's own secret. */
function maintainer(c: Ctx): void {
  const configured = c.env.MARKETPLACE_ADMIN;
  if (!configured) throw new ApiError("notConfigured", "No marketplace editor is configured");
  const given = c.req.headers.get("authorization")?.replace(/^Bearer /, "") ?? "";
  if (given !== configured) throw new ApiError("unauthenticated", "Not a marketplace editor");
}

/** Every app's latest published versions, newest first, so a catalog draws them in one request. */
export async function changelog(c: Ctx) {
  const rows = await c.env.DB.prepare(
    "SELECT app_id, version, notes, published_at FROM app_changelog ORDER BY published_at DESC LIMIT 500",
  ).all<{ app_id: string; version: string; notes: string; published_at: number }>();
  return {
    apps: (rows.results ?? []).map((row) => ({
      appId: row.app_id,
      version: row.version,
      notes: row.notes,
      publishedAt: row.published_at,
    })),
  };
}

/** Publishes one version's notes (the maintainers only); the same version replaces its notes. */
export async function publishChangelog(c: Ctx, id?: string) {
  maintainer(c);
  const app = appId(c, id);
  const body_ = body(c);
  const version = typeof body_.version === "string" ? body_.version.trim() : "";
  const notes = typeof body_.notes === "string" ? body_.notes.trim() : "";
  if (!version || version.length > 40 || !notes || notes.length > 4_000) {
    throw new ApiError("badRequest", "version and notes are required");
  }
  await c.env.DB.prepare(
    `INSERT INTO app_changelog(app_id, version, notes, published_at) VALUES (?1, ?2, ?3, ?4)
     ON CONFLICT(app_id, version) DO UPDATE SET notes = excluded.notes, published_at = excluded.published_at`,
  )
    .bind(app, version, notes, c.now)
    .run();
  return { appId: app, version };
}

/** The creators vouched for by the maintainers. */
export async function publishers(c: Ctx) {
  const rows = await c.env.DB.prepare("SELECT name FROM verified_publishers ORDER BY name").all<{ name: string }>();
  return { publishers: (rows.results ?? []).map((row) => row.name) };
}

/** Marks a creator as verified (the maintainers only). */
export async function verifyPublisher(c: Ctx, name?: string) {
  maintainer(c);
  const given = (name ?? c.url.pathname.split("/")[4] ?? "").trim();
  if (!given || given.length > 120) throw new ApiError("badRequest", "Invalid publisher");
  await c.env.DB.prepare(
    "INSERT INTO verified_publishers(name, verified_at) VALUES (?1, ?2) ON CONFLICT(name) DO NOTHING",
  )
    .bind(given, c.now)
    .run();
  return { name: given };
}
