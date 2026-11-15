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
