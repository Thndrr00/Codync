import test from "node:test";
import assert from "node:assert/strict";
import {
  SseDecoder,
  formatBytes,
  formatRelative,
  hostErrorMessage,
  isChatEntry,
  mergeEntries,
  normalizeBot,
  parseHostEvent,
  readableError,
  retryDelay,
} from "./host-protocol.ts";
import type { Entry } from "./host-types.ts";

const entry = (patch: Partial<Entry> = {}): Entry => ({
  id: "e1",
  seq: 1,
  botId: "b1",
  rev: 1,
  kind: "user",
  turn: 1,
  data: { text: "hello" },
  createdAt: 1_000,
  updatedAt: 1_000,
  ...patch,
});

test("SSE blocks join data lines and ignore keep-alives", () => {
  const decoder = new SseDecoder();
  assert.deepEqual(decoder.push(": keep-alive\n\n"), []);
  const messages = decoder.push("id: 7\nevent: entry\ndata: {\"a\":\ndata: 1}\n\n");
  assert.deepEqual(messages, [{ event: "entry", id: "7", data: "{\"a\":\n1}" }]);
});

test("SSE framing survives chunks split inside CRLF", () => {
  const decoder = new SseDecoder();
  assert.deepEqual(decoder.push("data: one\r"), []);
  assert.deepEqual(decoder.push("\n\r"), []);
  assert.deepEqual(decoder.push("\ndata: two\r\n\r\n"), [
    { event: "message", data: "one" },
    { event: "message", data: "two" },
  ]);
});

test("an SSE block without data is not a message", () => {
  const decoder = new SseDecoder();
  assert.deepEqual(decoder.push("event: ping\n\n"), []);
});

test("host events decode a bot, a deletion and a resync", () => {
  const bot = parseHostEvent({ type: "bot", bot: { id: "b1", name: "Reviewer" } });
  assert.equal(bot?.type, "bot");
  assert.equal(bot?.type === "bot" && bot.bot.name, "Reviewer");
  assert.equal(bot?.type === "bot" && bot.bot.status, "idle");

  assert.deepEqual(parseHostEvent({ type: "bot", bot: { id: "b1", deleted: true, rev: 9 } }), {
    type: "botDeleted",
    id: "b1",
    rev: 9,
  });
  assert.deepEqual(parseHostEvent({ type: "resync" }), { type: "resync" });
  assert.equal(parseHostEvent({ type: "something-new" }), null);
  assert.equal(parseHostEvent("not an event"), null);
});

test("an entry event normalizes missing data and an artifact event keeps its metadata", () => {
  const event = parseHostEvent({ type: "entry", entry: { id: "e1", seq: 4, botId: "b1", rev: 3, kind: "agent" } });
  assert.equal(event?.type, "entry");
  assert.deepEqual(event?.type === "entry" && event.entry.data, {});
  assert.equal(event?.type === "entry" && event.entry.seq, 4);

  const artifact = parseHostEvent({
    type: "artifact",
    rev: 12,
    artifact: { id: "a1", botId: "b1", title: "Report", size: 10, sha256: "abc", updatedAt: 5 },
  });
  assert.equal(artifact?.type, "artifact");
  assert.equal(artifact?.type === "artifact" && artifact.artifact.title, "Report");
  assert.equal(artifact?.type === "artifact" && artifact.rev, 12);
});

test("chat visibility matches the host's rule", () => {
  assert.equal(isChatEntry(entry()), true);
  assert.equal(isChatEntry(entry({ kind: "permission" })), true);
  assert.equal(isChatEntry(entry({ kind: "agent", data: { text: "…", final: false } })), true);
  assert.equal(isChatEntry(entry({ kind: "tool", data: { title: "Bash" } })), false);
  assert.equal(isChatEntry(entry({ kind: "user", data: { internal: true } })), false);
  assert.equal(isChatEntry(entry({ kind: "notice", data: { memory: true } })), false);
  assert.equal(isChatEntry(entry({ kind: "notice", data: { compaction: true } })), false);
  assert.equal(isChatEntry(entry({ kind: "notice", data: { handoff: { status: "kept" } } })), false);
  assert.equal(isChatEntry(entry({ kind: "notice", data: { text: "Routine ran" } })), true);
  assert.equal(isChatEntry(entry({ kind: "subagents", data: { subagents: [] } })), false);
  assert.equal(isChatEntry(entry({ kind: "subagents", data: { subagents: [{ id: "s", task: "t", state: "done", step: "" }] } })), true);
});

test("entries upsert by id, keep the newer rev and sort by seq", () => {
  const first = entry({ id: "a", seq: 2, rev: 1, data: { text: "two" } });
  const second = entry({ id: "b", seq: 1, rev: 1 });
  const stale = entry({ id: "a", seq: 2, rev: 0, data: { text: "stale" } });
  const fresh = entry({ id: "a", seq: 2, rev: 2, data: { text: "fresh" } });

  const merged = mergeEntries(mergeEntries([first, second], [stale]), [fresh]);
  assert.deepEqual(merged.map((e) => e.id), ["b", "a"]);
  assert.equal(merged[1].data.text, "fresh");
});

test("backoff grows, caps and stays inside its jitter band", () => {
  const flat = retryDelay(0, { random: () => 0.5, jitter: 0 });
  assert.equal(flat, 500);
  assert.equal(retryDelay(3, { random: () => 0.5, jitter: 0 }), 4_000);
  assert.equal(retryDelay(30, { random: () => 0.5, jitter: 0 }), 15_000);
  assert.equal(retryDelay(2, { random: () => 0, jitter: 0.2 }), 1_600);
  assert.equal(retryDelay(2, { random: () => 1, jitter: 0.2 }), 2_400);
});

test("host errors prefer the host's message and explain the status otherwise", () => {
  assert.equal(hostErrorMessage(400, { error: "unknown bot" }), "unknown bot");
  assert.match(hostErrorMessage(401, null), /local bridge/);
  assert.match(hostErrorMessage(403, {}), /origin/);
  assert.match(hostErrorMessage(404, ""), /older/);
  assert.match(hostErrorMessage(0, null), /Can't reach/);
  assert.match(hostErrorMessage(503, null), /Try again/);
  assert.equal(readableError(new Error("no host")), "no host");
  assert.equal(readableError("a string", "fallback"), "fallback");
});

test("a roster row from an older host gets safe defaults", () => {
  const bot = normalizeBot({ id: "b1", name: "Builder" });
  assert.equal(bot.kind, "agent");
  assert.equal(bot.status, "idle");
  assert.equal(bot.unread, 0);
  assert.equal(bot.hidden, false);
  assert.equal(bot.lastAt, 0);

  const working = normalizeBot({ id: "b2", status: "working", unread: 3, pinned: true, memoryRev: 7 });
  assert.equal(working.status, "working");
  assert.equal(working.unread, 3);
  assert.equal(working.pinned, true);
  assert.equal(working.memoryRev, 7);
});

test("relative times read naturally in the past and the future", () => {
  const now = 1_700_000_000_000;
  assert.equal(formatRelative(now - 10_000, now), "now");
  assert.equal(formatRelative(now - 5 * 60_000, now), "5m");
  assert.equal(formatRelative(now - 3 * 3_600_000, now), "3h");
  assert.equal(formatRelative(now + 2 * 3_600_000, now), "in 2h");
  assert.equal(formatRelative(0, now), "");
});

test("file sizes stay short", () => {
  assert.equal(formatBytes(900), "900 B");
  assert.equal(formatBytes(12 * 1024), "12 KB");
  assert.equal(formatBytes(2.5 * 1024 * 1024), "2.5 MB");
});
