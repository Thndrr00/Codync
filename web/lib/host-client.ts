// A small typed client for the Codync host HTTP API (`host/src/api/`): `POST /api/<method>` and the
// `/events` SSE stream. It never puts the token in a URL: when a token is supplied it travels only in
// the Authorization header, and the supported browser setup is a same-origin bridge that injects it.

import {
  eventRevision,
  hostErrorMessage,
  mergeEntries,
  parseHostEvent,
  retryDelay,
  SseDecoder,
} from "./host-protocol.ts";
import type {
  Artifact,
  ArtifactChunk,
  Bot,
  BotTasks,
  Entry,
  Hello,
  HostEvent,
  MemoryView,
  RoutinesAnswer,
} from "./host-types.ts";

export class HostError extends Error {
  readonly status: number;
  readonly retryAfterMs?: number;

  constructor(status: number, message: string, retryAfterMs?: number) {
    super(message);
    this.name = "HostError";
    this.status = status;
    this.retryAfterMs = retryAfterMs;
  }

  /** The link failed or the host is briefly unavailable: the same call can work again. */
  get isTransient() {
    return this.status === 0 || this.status === 408 || this.status === 429 || this.status >= 500;
  }
}

export interface CallOptions {
  timeoutMs?: number;
  /** Retries for calls that are safe to repeat; mutating calls default to none. */
  retries?: number;
}

export interface HostClientOptions {
  /** Origin of the same-origin bridge. Empty (the default) means this page's origin. */
  baseUrl?: string;
  /** Only for a page the host itself serves; the bridge injects the token instead. */
  token?: string;
  fetchImpl?: typeof fetch;
  /** Bytes a Library file may take; bigger files open in the desktop app. */
  maxFileBytes?: number;
}

export type StreamStatus = "connecting" | "online" | "offline";

export interface StreamHandlers {
  onEvent: (event: HostEvent) => void;
  onStatus?: (status: StreamStatus, error?: HostError) => void;
  /** The client name the host sees in `/events`. */
  client?: string;
}

/** Bigger files stay on the computer; the web client doesn't fill a tab with them. */
const DEFAULT_MAX_FILE_BYTES = 64 * 1024 * 1024;

function normalizeBaseUrl(value: string | undefined): string {
  if (!value) return "";
  if (value.startsWith("/")) return value.replace(/\/+$/, "");
  const url = new URL(value);
  if (url.protocol !== "http:" && url.protocol !== "https:") throw new Error("The host address must be http or https.");
  if (url.username || url.password) throw new Error("The host address can't carry a username or password.");
  if (url.search || url.hash) throw new Error("The host address can't carry a query or fragment.");
  return `${url.origin}${url.pathname.replace(/\/+$/, "")}`;
}

function base64Bytes(data: string): Uint8Array {
  const binary = atob(data);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
  return bytes;
}

function bytesHex(bytes: Uint8Array): string {
  return [...bytes].map((byte) => byte.toString(16).padStart(2, "0")).join("");
}

function mimeOf(name: string): string {
  const extension = name.split(".").pop()?.toLowerCase() ?? "";
  const known: Record<string, string> = {
    html: "text/html",
    htm: "text/html",
    md: "text/markdown",
    txt: "text/plain",
    json: "application/json",
    csv: "text/csv",
    pdf: "application/pdf",
    png: "image/png",
    jpg: "image/jpeg",
    jpeg: "image/jpeg",
    gif: "image/gif",
    webp: "image/webp",
    svg: "image/svg+xml",
    mp4: "video/mp4",
    mov: "video/quicktime",
    mp3: "audio/mpeg",
    wav: "audio/wav",
  };
  return known[extension] ?? "application/octet-stream";
}

export class HostClient {
  private readonly baseUrl: string;
  private readonly token: string;
  private readonly request: typeof fetch;
  private readonly maxFileBytes: number;

  constructor(options: HostClientOptions = {}) {
    this.baseUrl = normalizeBaseUrl(options.baseUrl);
    this.token = options.token?.trim() ?? "";
    this.request = options.fetchImpl ?? ((input, init) => fetch(input, init));
    this.maxFileBytes = options.maxFileBytes ?? DEFAULT_MAX_FILE_BYTES;
  }

  private url(path: string): string {
    return `${this.baseUrl}${path}`;
  }

  private headers(json = true): Headers {
    const headers = new Headers({ Accept: "application/json", "X-Codync-Web": "1" });
    if (json) headers.set("Content-Type", "application/json");
    // Never a query parameter: a URL lands in history, logs and Referer.
    if (this.token) headers.set("Authorization", `Bearer ${this.token}`);
    return headers;
  }

  /** One host method with a bounded wait and retries for idempotent reads. */
  async call<T>(method: string, body: unknown = {}, options: CallOptions = {}): Promise<T> {
    return this.callPath<T>(`/api/${encodeURIComponent(method)}`, body, options);
  }

  private async callPath<T>(path: string, body: unknown, options: CallOptions): Promise<T> {
    const timeoutMs = options.timeoutMs ?? 20_000;
    const retries = Math.max(0, options.retries ?? 0);
    let lastError: HostError | null = null;
    for (let attempt = 0; attempt <= retries; attempt++) {
      const controller = new AbortController();
      const timer = setTimeout(() => controller.abort(), timeoutMs);
      try {
        const response = await this.request(this.url(path), {
          method: "POST",
          headers: this.headers(),
          body: JSON.stringify(body ?? {}),
          signal: controller.signal,
          credentials: "same-origin",
        });
        const text = await response.text();
        let payload: unknown = null;
        if (text) {
          try {
            payload = JSON.parse(text);
          } catch {
            throw new HostError(response.status, "The host sent a response the web client couldn't read.");
          }
        }
        if (!response.ok) {
          const header = response.headers.get("retry-after");
          const retryAfter = header === null ? Number.NaN : Number(header);
          throw new HostError(
            response.status,
            hostErrorMessage(response.status, payload),
            Number.isFinite(retryAfter) ? retryAfter * 1000 : undefined,
          );
        }
        return payload as T;
      } catch (error) {
        const failure =
          error instanceof HostError
            ? error
            : new HostError(0, error instanceof DOMException && error.name === "AbortError" ? "The host took too long to answer." : hostErrorMessage(0, null));
        lastError = failure;
        if (attempt >= retries || !failure.isTransient) throw failure;
        await new Promise((resolve) => setTimeout(resolve, Math.min(failure.retryAfterMs ?? retryDelay(attempt), 60_000)));
      } finally {
        clearTimeout(timer);
      }
    }
    throw lastError ?? new HostError(0, hostErrorMessage(0, null));
  }

  /**
   * Live events, reconnecting with backoff and resuming from the last rev. A full resync starts the
   * next connection from zero, because the host dropped events this client never saw.
   */
  events(handlers: StreamHandlers): () => void {
    let stopped = false;
    let since = 0;
    let attempt = 0;
    let controller: AbortController | null = null;
    let timer: ReturnType<typeof setTimeout> | null = null;
    const { onEvent, onStatus } = handlers;

    const run = async () => {
      if (stopped) return;
      controller = new AbortController();
      onStatus?.("connecting");
      try {
        const query = new URLSearchParams({ since: String(since), client: handlers.client ?? "web" });
        const response = await this.request(this.url(`/events?${query}`), {
          headers: {
            Accept: "text/event-stream",
            "X-Codync-Web": "1",
            ...(this.token ? { Authorization: `Bearer ${this.token}` } : {}),
          },
          signal: controller.signal,
          credentials: "same-origin",
          cache: "no-store",
        });
        if (!response.ok) {
          const text = await response.text();
          let payload: unknown = null;
          try {
            payload = JSON.parse(text);
          } catch {
            // The bridge may answer with plain text.
          }
          throw new HostError(response.status, hostErrorMessage(response.status, payload));
        }
        if (!response.body) throw new HostError(0, "The host closed the live connection.");
        onStatus?.("online");
        attempt = 0;
        const stream = response.body;
        const reader = stream.getReader();
        const decoder = new TextDecoder();
        const sse = new SseDecoder();
        for (;;) {
          const { done, value } = await reader.read();
          if (done) throw new HostError(0, "The host closed the live connection.");
          for (const message of sse.push(decoder.decode(value, { stream: true }))) {
            let raw: unknown;
            try {
              raw = JSON.parse(message.data);
            } catch {
              continue;
            }
            const event = parseHostEvent(raw);
            if (!event) continue;
            if (event.type === "resync") {
              since = 0;
              throw new HostError(0, "The host asked for a fresh sync.");
            }
            const revision = eventRevision(event);
            if (revision > since) since = revision;
            onEvent(event);
          }
        }
      } catch (error) {
        if (stopped) return;
        const failure = error instanceof HostError ? error : new HostError(0, hostErrorMessage(0, null));
        onStatus?.("offline", failure);
        // A rejected origin or token won't fix itself; the connection screen's Try again starts fresh.
        if (failure.status >= 400 && failure.status < 500) return;
        await new Promise<void>((resolve) => {
          timer = setTimeout(resolve, retryDelay(attempt++));
        });
        if (!stopped) void run();
      }
    };

    void run();
    return () => {
      stopped = true;
      controller?.abort();
      if (timer) clearTimeout(timer);
    };
  }

  // ---- Computer and roster -------------------------------------------------

  hello() {
    return this.call<Hello>("hello", {}, { retries: 2 });
  }

  history(botId: string, beforeSeq = Number.MAX_SAFE_INTEGER, limit = 100) {
    return this.call<{ entries: Entry[] }>("history", { botId, beforeSeq, limit }, { retries: 2 });
  }

  markRead(botId: string) {
    return this.call<Record<string, never>>("markRead", { botId });
  }

  botTasks(botId: string) {
    return this.call<BotTasks>("botTasks", { botId }, { retries: 2 });
  }

  // ---- Chat ----------------------------------------------------------------

  send(botId: string, text: string, clientNonce: string, replyTo: string | null = null) {
    return this.call<{ entry: Entry }>("send", {
      botId,
      text,
      clientNonce,
      attachments: null,
      ...(replyTo ? { replyTo } : {}),
    });
  }

  stop(botId: string) {
    return this.call<Record<string, never>>("stop", { botId });
  }

  newSession(botId: string) {
    return this.call<Record<string, never>>("newSession", { botId });
  }

  respondPermission(entryId: string, optionId: string) {
    return this.call<{ entry?: Entry }>("respondPermission", { entryId, optionId });
  }

  // ---- Routines ------------------------------------------------------------

  routines(botId: string) {
    return this.call<RoutinesAnswer>("routines", { botId }, { retries: 2 });
  }

  runRoutine(botId: string, id: string) {
    return this.call<Record<string, unknown>>("runRoutine", { botId, id });
  }

  setRoutineEnabled(botId: string, id: string, enabled: boolean) {
    return this.call<Record<string, unknown>>("setRoutineEnabled", { botId, id, enabled });
  }

  // ---- Memory --------------------------------------------------------------

  memory(botId: string) {
    return this.call<MemoryView>("memory", { botId }, { retries: 2 });
  }

  // ---- Library and artifacts ----------------------------------------------

  library(botId: string) {
    return this.call<{ items: LibraryItem[] }>("library", { botId }, { retries: 2 });
  }

  serverLibrary(filter: "all" | "images" | "files" = "all") {
    return this.call<{ items: ServerLibraryItem[] }>("serverLibrary", filter === "all" ? {} : { filter }, { retries: 2 });
  }

  private async readChunks(method: "readArtifact" | "readFile" | "readUpload", body: (offset: number) => Record<string, unknown>) {
    const parts: Uint8Array[] = [];
    let size = 0;
    for (;;) {
      const answer = await this.call<{ data: string; size: number }>(method, body(size), { timeoutMs: 60_000 });
      const chunk = base64Bytes(answer.data);
      parts.push(chunk);
      size += chunk.length;
      if (size > this.maxFileBytes) throw new HostError(0, "That file is too large for the web client; open it in the desktop app.");
      if (!chunk.length || size >= answer.size) break;
    }
    const bytes = new Uint8Array(size);
    let at = 0;
    for (const part of parts) {
      bytes.set(part, at);
      at += part.length;
    }
    return bytes;
  }

  /**
   * An artifact's whole page, read in chunks and checked against the host's SHA-256. A republish
   * mid-read restarts from the new version rather than mixing two pages.
   */
  async readArtifactHtml(botId: string, artifactId: string): Promise<{ artifact: Artifact; html: string }> {
    let artifact: Artifact | null = null;
    let version = "";
    let bytes = new Uint8Array(0);
    let offset = 0;
    for (;;) {
      const chunk = await this.call<ArtifactChunk>("readArtifact", { botId, artifactId, offset }, { timeoutMs: 60_000 });
      if (chunk.artifact.sha256 !== version) {
        const restart = offset > 0;
        artifact = chunk.artifact;
        version = chunk.artifact.sha256;
        if (artifact.size > this.maxFileBytes) throw new HostError(0, "That artifact is too large for the web client.");
        bytes = new Uint8Array(chunk.artifact.size);
        offset = 0;
        if (restart) continue;
      }
      artifact = chunk.artifact;
      if (artifact.size > this.maxFileBytes) throw new HostError(0, "That artifact is too large for the web client.");
      const part = base64Bytes(chunk.data);
      if (!part.length && offset < artifact.size) throw new HostError(0, "The artifact ended before its full size.");
      bytes.set(part, offset);
      offset += part.length;
      if (offset >= artifact.size) break;
    }
    if (!artifact) throw new HostError(0, "The artifact couldn't be read.");
    if (globalThis.crypto?.subtle) {
      const digest = bytesHex(new Uint8Array(await globalThis.crypto.subtle.digest("SHA-256", bytes)));
      if (digest !== version) throw new HostError(0, "The artifact failed its integrity check; try again.");
    }
    return { artifact, html: new TextDecoder().decode(bytes) };
  }

  /** A Library file's bytes, however the host stores it: a bot's shared file or the user's attachment. */
  async downloadLibraryFile(item: {
    kind: "file" | "attachment";
    botId: string;
    entryId: string;
    file: { id: string; name: string; size: number };
  }): Promise<{ name: string; blob: Blob }> {
    const size = item.file.size;
    if (size > this.maxFileBytes) throw new HostError(0, "That file is too large for the web client; open it in the desktop app.");
    const method = item.kind === "file" ? "readFile" : "readUpload";
    const bytes = await this.readChunks(method, (offset) => ({
      ...(item.kind === "file" ? { entryId: item.entryId, fileId: item.file.id } : { botId: item.botId, uploadId: item.file.id }),
      offset,
    }));
    return { name: item.file.name, blob: new Blob([bytes.buffer as ArrayBuffer], { type: mimeOf(item.file.name) }) };
  }
}

export { mergeEntries };
export type { Bot, Entry, HostEvent };
