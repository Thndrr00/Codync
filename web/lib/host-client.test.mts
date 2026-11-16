import test from "node:test";
import assert from "node:assert/strict";
import { HostClient, HostError } from "./host-client.ts";
import type { HostEvent } from "./host-types.ts";

function json(body: unknown, status = 200, headers: Record<string, string> = {}) {
  return new Response(JSON.stringify(body) ?? "", { status, headers: { "content-type": "application/json", ...headers } });
}

type Seen = { url: string; method: string; headers: Headers; body: string };

function clientWith(handler: (seen: Seen) => Promise<Response>, options: { token?: string; baseUrl?: string } = {}) {
  const seen: Seen[] = [];
  const fetchImpl = (async (input: RequestInfo | URL, init?: RequestInit) => {
    const record = {
      url: String(input),
      method: init?.method ?? "GET",
      headers: new Headers(init?.headers),
      body: typeof init?.body === "string" ? init.body : "",
    };
    seen.push(record);
    return handler(record);
  }) as typeof fetch;
  const client = new HostClient({ baseUrl: options.baseUrl ?? "http://localhost:19222", token: options.token, fetchImpl });
  return { client, seen };
}

test("a call sends the token in the Authorization header, never the URL", async () => {
  const { client, seen } = clientWith(() => Promise.resolve(json({ hostId: "h", name: "Mac" })), { token: "secret-token" });
  const hello = await client.hello();
  assert.equal(hello.name, "Mac");
  assert.equal(seen.length, 1);
  assert.equal(seen[0].url, "http://localhost:19222/api/hello");
  assert.equal(seen[0].method, "POST");
  assert.equal(seen[0].headers.get("authorization"), "Bearer secret-token");
  assert.equal(seen[0].headers.get("x-codync-web"), "1");
  assert.equal(seen[0].body, "{}");
  assert.ok(!seen[0].url.includes("secret-token"), "the token never reaches a URL");
});

test("a read retries a transient answer and succeeds", async () => {
  let calls = 0;
  const { client, seen } = clientWith(() => {
    calls += 1;
    return Promise.resolve(calls === 1 ? json({ error: "busy" }, 503, { "retry-after": "0" }) : json({ entries: [] }));
  });
  const answer = await client.history("b1");
  assert.deepEqual(answer.entries, []);
  assert.equal(seen.length, 2);
});

test("a refused token is not retried and keeps the host's message", async () => {
  const { client, seen } = clientWith(() => Promise.resolve(json({ error: "invalid token" }, 401)));
  await assert.rejects(client.hello(), (error: unknown) => {
    assert.ok(error instanceof HostError);
    assert.equal(error.status, 401);
    assert.equal(error.message, "invalid token");
    return true;
  });
  assert.equal(seen.length, 1);
});

test("a network failure becomes a reachable message, not a raw TypeError", async () => {
  const { client } = clientWith(() => Promise.reject(new TypeError("fetch failed")));
  await assert.rejects(client.call("hello", {}, { retries: 0 }), (error: unknown) => {
    assert.ok(error instanceof HostError);
    assert.equal(error.status, 0);
    assert.match(error.message, /Can't reach/);
    return true;
  });
});

test("a base URL can't smuggle a token through a query or fragment", () => {
  assert.throws(() => new HostClient({ baseUrl: "http://localhost:19222/?token=x" }), /query/);
  assert.throws(() => new HostClient({ baseUrl: "http://user:pass@localhost:19222" }), /username/);
  assert.throws(() => new HostClient({ baseUrl: "file:///tmp/host" }), /http/);
});

test("the event stream decodes a host event and stops when asked", async () => {
  const body = new ReadableStream<Uint8Array>({
    start(controller) {
      controller.enqueue(new TextEncoder().encode('data: {"type":"bot","bot":{"id":"b1","name":"Builder"}}\n\n'));
      controller.close();
    },
  });
  const { client } = clientWith(() =>
    Promise.resolve(new Response(body, { status: 200, headers: { "content-type": "text/event-stream" } })),
  );
  const event = await new Promise<HostEvent>((resolve) => {
    let stop = () => {};
    stop = client.events({
      onEvent: (next) => {
        resolve(next);
        stop();
      },
      onStatus: () => {},
    });
  });
  assert.equal(event.type, "bot");
  assert.equal(event.type === "bot" && event.bot.name, "Builder");
});
