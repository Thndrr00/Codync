# Web client

The web app is Codync's fifth client: `web/app/app/`, a route area of the existing Next.js site. It renders host state and sends intents over the host's ordinary HTTP API, the same one the desktop app uses; it keeps no bots, chats or settings of its own.

## Why it is a bridge, not a hosted client

The host's browser boundary is deliberate:

- `POST /api/<method>` and `GET /events` require `Authorization: Bearer <host token>` **and** a loopback peer (`host/src/api/http.rs`).
- Browser requests may carry only `Origin: http://localhost:<host port>`, `http://127.0.0.1:<host port>` or `http://[::1]:<host port>` (`host/src/api/origin.rs`).
- The DNS-rebinding guard requires loopback hosts to name a loopback host (`host/src/api/rebinding.rs`).

A page served by codync.dev is none of those, and the phone channel is an end-to-end encrypted WebSocket handshake (`GET /channel?v=1`) that a browser client cannot join without reimplementing device pairing.

`web/proxy/host-proxy.mjs` is therefore the supported browser transport. It:

1. serves the static export in `web/out` (or forwards to `next dev` with `--next`);
2. proxies only `POST /api/*` and `GET /events` — not `/hooks/*`, `/channel` or `/oauth/callback`;
3. reads the token from `CODYNC_HOME` or `~/.codync/token` (`~/.codync-dev/token` with `--dev`) and adds it as a header;
4. removes `Origin` and `Referer` so the loopback proxy is the host's only origin;
5. binds `127.0.0.1`.

The browser gets no token. There is no token query parameter, fragment or cookie path anywhere in the client.

### Other deployments

A reverse proxy with the same rules works: serve `web/out`, proxy `/api/*` and `/events` to `127.0.0.1:19222`, add the bearer token, strip `Origin`/`Referer`, listen only on loopback. Exposing that proxy beyond the computer needs TLS and authentication in front; the host token is a machine credential, not a user login. Hosted `/app` on codync.dev has no way to reach a host and says so.

For example, with nginx on the same computer (the token is read by hand and stays in a root-owned config):

```nginx
server {
  listen 127.0.0.1:8443 ssl;
  root /path/to/Codync/web/out;

  location / {
    try_files $uri $uri.html $uri/index.html =404;
  }
  location /api/ {
    proxy_pass http://127.0.0.1:19222;
    proxy_set_header Host 127.0.0.1:19222;
    proxy_set_header Authorization "Bearer <contents of ~/.codync/token>";
    proxy_set_header Origin "";
    proxy_set_header Referer "";
  }
  location = /events {
    proxy_pass http://127.0.0.1:19222;
    proxy_set_header Host 127.0.0.1:19222;
    proxy_set_header Authorization "Bearer <contents of ~/.codync/token>";
    proxy_set_header Origin "";
    proxy_set_header Referer "";
    proxy_buffering off;
    proxy_read_timeout 1h;
  }
}
```

The browser still sends `X-Codync-Web: 1`; a proxy that strips it will make every call fail with 403. That header is the bridge's anti-CSRF check, not authentication.

## Files

| File | Responsibility |
|---|---|
| `web/lib/host-types.ts` | Wire types: bot, entry, artifact, routine, memory and event shapes. |
| `web/lib/host-protocol.ts` | Pure helpers: SSE decoder, event parsing, chat visibility, entry merging, retry delay, copy, relative time. |
| `web/lib/host-client.ts` | `HostClient`: typed `call`, streaming `events`, retries, artifact and Library file reads. |
| `web/lib/host-protocol.test.mts` | `node --test` coverage for the pure helpers. |
| `web/lib/host-client.test.mts` | `node --test` coverage for calls, retries, refusals and the event stream. |
| `web/app/app/host-provider.tsx` | One connection per tab: `hello`, the event stream, the roster, reconnect and a subscriber set. |
| `web/app/app/app-shell.tsx` | Desktop sidebar, mobile bottom bar, connection badge, theme toggle. |
| `web/app/app/connection.tsx` | The bridge setup screen and `HostGate`. |
| `web/app/app/page.tsx` | Roster: search, pinned, hidden, status and unread. |
| `web/app/app/chat/` | History, live entries, approvals, composer, stop and new session. |
| `web/app/app/routines/page.tsx` | Per-bot routine lists, run/pause, last run. |
| `web/app/app/memory/page.tsx` | Read-only About you / This bot / room layers. |
| `web/app/app/library/` | Server Library, artifact viewer sandbox, file download. |
| `web/app/app/settings/page.tsx` | Theme, host details, analytics, connection and desktop-only notes. |

## Data flow

- `HostProvider` calls `hello` once, then opens `/events?since=0&client=web`.
- The host's catch-up replays `bot` and `entry` events; the provider upserts bots by id/rev, and screens subscribe for entries, artifacts and routine-flow changes.
- The client tracks the highest delivered `rev`; a dropped stream reconnects with backoff from that rev. A `resync` event restarts the next connection from zero.
- Chat history is paged with `history {botId, beforeSeq}`; live entries are merged by id and newer rev.
- Approvals come as `permission` entries and answer with `respondPermission {entryId, optionId}`. The host keeps the first answer, so a repeated tap on another device is safe.
- Routines are fetched per bot (`routines {botId}`), because that is the host's shape; `runRoutine` and `setRoutineEnabled` are the only mutations the web client offers.
- Memory is read-only (`memory {botId}`); a bot's `memoryRev` drives refetch.
- The Library uses `serverLibrary {filter}`, `readArtifact` chunks with a SHA-256 check, `readFile` for shared files, and `readUpload` for attachments.

## Parity

| Desktop area | Web |
|---|---|
| Roster, statuses, unread, pins, sections | Yes |
| Chat history, streaming, approvals, stop, new session | Yes |
| Routine list, last run, run now, pause/resume | Yes |
| Routine editor, flow canvas, webhooks | Desktop/phone only |
| Memory layers, usage, agent's own notes | Read-only |
| Artifacts and Library | Yes, sandboxed; diagrams open in desktop |
| File download | Yes, up to 64 MB |
| File upload, attachments in a sent message | Not yet |
| Voice, side threads, replies, reactions, edits | Not yet |
| Remote screen, computer use, island, SSH, terminals, updates | Desktop only |
| Bot creation and full settings | Desktop only |

## Security notes

- Artifact HTML is shown in an iframe with `sandbox="allow-scripts allow-popups allow-forms allow-modals allow-downloads"` and no `allow-same-origin`; scripts cannot read the app's origin, storage or token.
- The host's own CORS/origin refusal stays the last line of defense; the bridge's header stripping is not a replacement for it.
- Every host call from the client carries `X-Codync-Web: 1`; the bridge refuses a request without it. Another origin can't set that header without a CORS preflight the bridge never grants, so a form, an image or a sandboxed artifact can't borrow the host token.
- Error text is the host's `{error}` when present, but 401/403/404/5xx fall back to copy that tells the reader what to start.
- The web client is `noindex`; only the manual's `/docs/web-app` page is in the sitemap.

## Tests

```sh
cd web
npm test          # node --test lib/*.test.mts
npm run check:docs
```

The helper tests cover SSE framing across chunk boundaries and CRLF, event normalization, chat visibility, upsert by rev, backoff bounds, error copy, bot defaults and relative times.

There is no browser test harness in `web/` yet. End-to-end checks use the bridge plus the desktop host: roster, open a chat, send, approve, stop, run a routine, read memory, open an artifact, download a file, toggle theme, and confirm the app is unusable when the bridge is stopped.

## Open work

- Uploads and message attachments.
- Side threads, replies and reactions.
- A routine editor by reusing the host's `saveRoutine` contract.
- A transport for the phone channel's end-to-end encrypted protocol, if a browser is ever to pair directly.

Version compatibility follows the other clients: additive methods need nothing; a renamed or removed method raises `minApp` in the host and the web build ships with that release.
