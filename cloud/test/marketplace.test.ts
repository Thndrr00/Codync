// Marketplace reputation: public totals, one rating per person, installs counted per account.

import { describe, expect, it } from "vitest";
import { call, clerkToken, type Json } from "./helpers";

const app = (name: string) =>
  `openbot/${name}-${Math.random().toString(36).slice(2, 8)}`;

describe("marketplace reputation", () => {
  it("keeps one rating per person and averages it for everyone", async () => {
    const id = app("writer");
    const [ana, ben] = [await clerkToken("user_rate_ana"), await clerkToken("user_rate_ben")];

    const rated = await call("PUT", `/v1/marketplace/apps/${id}/rating`, { token: ana, body: { stars: 5 } });
    expect(rated.status).toBe(200);
    expect((rated.body as Json).stars).toBe(5);

    // Rating again replaces the stars rather than adding a second vote.
    await call("PUT", `/v1/marketplace/apps/${id}/rating`, { token: ana, body: { stars: 3 } });
    await call("PUT", `/v1/marketplace/apps/${id}/rating`, { token: ben, body: { stars: 4 } });

    const all = await call("GET", "/v1/marketplace/reputation");
    expect(all.status).toBe(200);
    const entry = ((all.body as Json).apps as Json[]).find((row) => row.appId === id);
    expect(entry).toMatchObject({ appId: id, count: 2, stars: 3.5, installs: 0 });
  });

  it("counts an install once and keeps totals public", async () => {
    const id = app("helper");
    const token = await clerkToken("user_install_ana");
    expect((await call("POST", `/v1/marketplace/apps/${id}/install`, { token })).status).toBe(200);
    const all = await call("GET", "/v1/marketplace/reputation");
    const entry = ((all.body as Json).apps as Json[]).find((row) => row.appId === id);
    expect(entry).toMatchObject({ appId: id, installs: 1, count: 0, stars: 0 });
  });

  it("refuses a rating without an account, outside one to five, or with a bad id", async () => {
    const id = app("strict");
    expect((await call("PUT", `/v1/marketplace/apps/${id}/rating`, { body: { stars: 5 } })).status).toBe(401);
    const token = await clerkToken("user_rate_strict");
    for (const stars of [0, 6, 2.5, "5"]) {
      expect((await call("PUT", `/v1/marketplace/apps/${id}/rating`, { token, body: { stars } })).status).toBe(400);
    }
    expect((await call("PUT", "/v1/marketplace/apps/bad%20id/rating", { token, body: { stars: 5 } })).status).toBe(400);
  });
});
