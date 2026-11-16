// The web client's pure wire helpers: SSE framing, host events, chat visibility, retries and copy.
// Kept free of browser APIs so `node --test` can exercise them without a DOM.

import {
  type Artifact,
  type Bot,
  type BotStatus,
  type Entry,
  type HostEvent,
} from "./host-types.ts";

function nullableString(value: unknown): string | null {
  return typeof value === "string" ? value : null;
}

/** A host a little newer or older than this client still yields a usable roster row. */
export function normalizeBot(raw: Partial<Bot> & { id: string }): Bot {
  const createdAt = typeof raw.createdAt === "number" && Number.isFinite(raw.createdAt) ? raw.createdAt : 0;
  const status: BotStatus = raw.status === "working" || raw.status === "needsInput" || raw.status === "error" ? raw.status : "idle";
  return {
    id: raw.id,
    kind: typeof raw.kind === "string" ? raw.kind : "agent",
    members: Array.isArray(raw.members) ? raw.members.filter((m): m is string => typeof m === "string") : [],
    name: typeof raw.name === "string" ? raw.name : "Bot",
    description: typeof raw.description === "string" ? raw.description : "",
    avatarColor: typeof raw.avatarColor === "string" ? raw.avatarColor : "blue",
    avatarShape: typeof raw.avatarShape === "string" ? raw.avatarShape : "blob",
    avatarLook: nullableString(raw.avatarLook),
    avatarPhoto: nullableString(raw.avatarPhoto),
    backend: typeof raw.backend === "string" ? raw.backend : "custom",
    model: nullableString(raw.model),
    cwd: typeof raw.cwd === "string" ? raw.cwd : "",
    managedWorkspace: raw.managedWorkspace === true,
    permission: typeof raw.permission === "string" ? raw.permission : "auto",
    section: nullableString(raw.section),
    pinned: raw.pinned === true,
    lead: raw.lead === true,
    hidden: raw.hidden === true,
    archived: raw.archived === true,
    status,
    activity: typeof raw.activity === "string" ? raw.activity : "",
    unread: typeof raw.unread === "number" && Number.isFinite(raw.unread) ? raw.unread : 0,
    lastMessage: nullableString(raw.lastMessage),
    lastAt: typeof raw.lastAt === "number" && Number.isFinite(raw.lastAt) ? raw.lastAt : createdAt,
    createdAt,
    rev: typeof raw.rev === "number" && Number.isFinite(raw.rev) ? raw.rev : 0,
    person: nullableString(raw.person),
    memoryRev: typeof raw.memoryRev === "number" && Number.isFinite(raw.memoryRev) ? raw.memoryRev : 0,
  };
}

export interface SseMessage {
  event: string;
  data: string;
  id?: string;
}

/**
 * Server-Sent Events framing (WHATWG): blocks end at a blank line, `data:` lines join with `\n`,
 * comments (`: keep-alive`) are ignored, and a block without data is not a message. Chunks may split
 * anywhere, including between the `\r` and `\n` of a CRLF pair.
 */
export class SseDecoder {
  private buffer = "";

  push(chunk: string): SseMessage[] {
    this.buffer += chunk;
    const messages: SseMessage[] = [];
    for (;;) {
      const match = /\r?\n\r?\n/.exec(this.buffer);
      if (!match) break;
      const block = this.buffer.slice(0, match.index);
      this.buffer = this.buffer.slice(match.index + match[0].length);
      const message = parseSseBlock(block);
      if (message) messages.push(message);
    }
    return messages;
  }

  reset() {
    this.buffer = "";
  }
}

export function parseSseBlock(block: string): SseMessage | null {
  let event = "message";
  let id: string | undefined;
  const data: string[] = [];
  for (const line of block.split(/\r?\n/)) {
    if (!line || line.startsWith(":")) continue;
    const colon = line.indexOf(":");
    const field = colon < 0 ? line : line.slice(0, colon);
    let value = colon < 0 ? "" : line.slice(colon + 1);
    if (value.startsWith(" ")) value = value.slice(1);
    if (field === "event") event = value;
    else if (field === "id") id = value;
    else if (field === "data") data.push(value);
  }
  return data.length ? { event, data: data.join("\n"), id } : null;
}

function object(value: unknown): Record<string, unknown> | null {
  return typeof value === "object" && value !== null && !Array.isArray(value) ? (value as Record<string, unknown>) : null;
}

function num(value: unknown, fallback = 0): number {
  return typeof value === "number" && Number.isFinite(value) ? value : fallback;
}

function str(value: unknown, fallback = ""): string {
  return typeof value === "string" ? value : fallback;
}

/** One event's JSON, or null for a type this client doesn't render (it ignores those on purpose). */
export function parseHostEvent(raw: unknown): HostEvent | null {
  const value = object(raw);
  if (!value) return null;
  switch (value.type) {
    case "hello":
      return {
        type: "hello",
        hostId: str(value.hostId),
        computerId: typeof value.computerId === "string" ? value.computerId : undefined,
        rev: num(value.rev),
        catchUpRev: num(value.catchUpRev),
        catchUp: num(value.catchUp),
        usage: object(value.usage) ? (value.usage as { providers: unknown[] }) : undefined,
        screen: value.screen,
      };
    case "icons":
      return { type: "icons", iconsRevision: num(value.iconsRevision) };
    case "bot": {
      const bot = object(value.bot);
      if (!bot || typeof bot.id !== "string") return null;
      if (bot.deleted === true) return { type: "botDeleted", id: bot.id, rev: num(bot.rev) };
      return { type: "bot", bot: normalizeBot(bot as Partial<Bot> & { id: string }) };
    }
    case "botDeleted":
      return typeof value.id === "string" ? { type: "botDeleted", id: value.id, rev: num(value.rev) } : null;
    case "entry": {
      const entry = object(value.entry);
      if (!entry || typeof entry.id !== "string" || typeof entry.botId !== "string") return null;
      return {
        type: "entry",
        entry: {
          id: entry.id,
          seq: num(entry.seq),
          botId: entry.botId,
          rev: num(entry.rev),
          kind: str(entry.kind),
          turn: num(entry.turn),
          createdAt: num(entry.createdAt),
          updatedAt: num(entry.updatedAt),
          threadId: typeof entry.threadId === "string" ? entry.threadId : null,
          data: object(entry.data) ? (entry.data as Entry["data"]) : {},
        },
      };
    }
    case "entryDeleted":
      return typeof value.id === "string" ? { type: "entryDeleted", id: value.id, rev: num(value.rev) } : null;
    case "historyCleared":
      return typeof value.botId === "string"
        ? { type: "historyCleared", botId: value.botId, rev: num(value.rev), deleted: num(value.deleted) }
        : null;
    case "artifact": {
      const artifact = object(value.artifact);
      if (!artifact || typeof artifact.id !== "string" || typeof artifact.botId !== "string") return null;
      return {
        type: "artifact",
        rev: typeof value.rev === "number" ? value.rev : undefined,
        artifact: artifact as unknown as Artifact,
      };
    }
    case "routineFlow": {
      const botIds = Array.isArray(value.botIds) ? value.botIds.filter((id): id is string => typeof id === "string") : [];
      return { type: "routineFlow", rev: num(value.rev), botIds };
    }
    case "usage":
      return object(value.usage) ? { type: "usage", usage: value.usage as { providers: unknown[] }, rev: num(value.rev) } : null;
    case "resync":
      return { type: "resync" };
    default:
      return null;
  }
}

/** The rev a client reconnects from: every replayable event carries one. */
export function eventRevision(event: HostEvent): number {
  switch (event.type) {
    case "bot":
      return event.bot.rev;
    case "botDeleted":
    case "entryDeleted":
    case "historyCleared":
    case "artifact":
    case "routineFlow":
    case "usage":
      return "rev" in event ? (event.rev ?? 0) : 0;
    case "entry":
      return event.entry.rev;
    default:
      return 0;
  }
}

/** Chat-visible lines, matching the desktop and phone apps (`models.isChat`). */
export function isChatEntry(entry: Entry): boolean {
  if (entry.data.internal) return false;
  switch (entry.kind) {
    case "user":
    case "permission":
      return true;
    case "subagents":
      return (entry.data.subagents ?? []).length > 0;
    case "notice":
      return !entry.data.memory && !entry.data.memoryFile && !entry.data.compaction && entry.data.handoff?.status !== "kept";
    case "agent":
      // Streaming text and final replies both show; tool/thought lines stay out of the chat.
      return true;
    default:
      return false;
  }
}

/** Upsert by id, keep the newer rev, and order by `seq` (then creation time). */
export function mergeEntries(list: Entry[], incoming: Entry[]): Entry[] {
  const byId = new Map(list.map((entry) => [entry.id, entry]));
  for (const entry of incoming) {
    const existing = byId.get(entry.id);
    if (!existing || entry.rev >= existing.rev) byId.set(entry.id, entry);
  }
  return [...byId.values()].sort((a, b) => a.seq - b.seq || a.createdAt - b.createdAt);
}

export interface RetryOptions {
  baseMs?: number;
  capMs?: number;
  /** Fraction of the delay that is randomized either way (0.2 = ±20%). */
  jitter?: number;
  random?: () => number;
}

/** Exponential backoff with jitter; attempt 0 is the first retry. */
export function retryDelay(attempt: number, options: RetryOptions = {}): number {
  const base = options.baseMs ?? 500;
  const cap = options.capMs ?? 15_000;
  const jitter = Math.min(Math.max(options.jitter ?? 0.2, 0), 1);
  const random = options.random ?? Math.random;
  const exponential = Math.min(cap, base * 2 ** Math.min(Math.max(attempt, 0), 10));
  return Math.round(exponential * (1 - jitter + random() * jitter * 2));
}

/** The host's `{error}` message, or copy that says what to do about the status. */
export function hostErrorMessage(status: number, payload: unknown): string {
  const body = object(payload);
  const message = typeof body?.error === "string" ? body.error.trim() : "";
  if (message) return message;
  switch (status) {
    case 0:
      return "Can't reach your computer. Is the Codync host running and is this page served by the local bridge?";
    case 401:
      return "The host didn't accept this browser. Start the local bridge (web/proxy/host-proxy.mjs) and open the address it prints.";
    case 403:
      return "The host refused this page's origin. Open the web app from the host's own origin, never from another site.";
    case 404:
      return "This host doesn't offer that yet; it may be older than the web client.";
    case 429:
      return "Too many requests. Wait a moment and try again.";
    default:
      return status >= 500 ? "The host ran into a problem. Try again in a moment." : `The host answered ${status}.`;
  }
}

/** The message of any failure, with a line that fits a button's result when the value isn't an Error. */
export function readableError(error: unknown, fallback = "That didn't work. Try again."): string {
  return error instanceof Error && error.message ? error.message : fallback;
}

/** "now", "4m", "3h", "2d", a short date, or the future's "in 4m". */
export function formatRelative(at: number, now: number): string {
  if (!Number.isFinite(at) || at <= 0) return "";
  const delta = now - at;
  const future = delta < 0;
  const seconds = Math.abs(delta) / 1000;
  const phrase =
    seconds < 45
      ? "now"
      : seconds < 3600
        ? `${Math.max(1, Math.round(seconds / 60))}m`
        : seconds < 86_400
          ? `${Math.max(1, Math.round(seconds / 3600))}h`
          : seconds < 7 * 86_400
            ? `${Math.max(1, Math.round(seconds / 86_400))}d`
            : new Date(at).toLocaleDateString(undefined, { month: "short", day: "numeric" });
  if (phrase === "now") return phrase;
  return future ? `in ${phrase}` : phrase;
}

export function formatTime(at: number): string {
  return new Date(at).toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" });
}

export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0) return "";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(bytes < 10 * 1024 ? 1 : 0)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

/** What the roster says under a bot's name. */
export function botStatusLine(bot: Bot): string {
  switch (bot.status) {
    case "working":
      return bot.activity || "Working";
    case "needsInput":
      return "Waiting for you";
    case "error":
      return bot.activity || "Needs attention";
    default:
      return bot.activity || bot.lastMessage || bot.description || "Idle";
  }
}

/** Routine statuses the host writes, as a person reads them. */
export function runStatusLabel(status: string): string {
  switch (status) {
    case "pending":
      return "Queued";
    case "starting":
      return "Starting";
    case "running":
      return "Running";
    case "recovering":
      return "Recovering";
    case "succeeded":
      return "Succeeded";
    case "failed":
      return "Failed";
    case "interrupted":
      return "Interrupted";
    case "cancelled":
      return "Cancelled";
    default:
      return status;
  }
}
