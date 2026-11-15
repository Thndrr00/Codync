# Where agentes.chat's ADRs differ from us

The plan's block 6 asks for its 40 ADRs to be read and the decisions that change ours to be
written down (`docs/plan-agentes-chat.md`). All of them were read (0001–0040; the repo has 170,
the rest arrive with their own blocks). This is the record: what they decide, what we do, and
whether ours moves. Almost nothing does — their engine is TypeScript over Postgres with accounts,
tenants, org money and cloud runners, and ours is one Rust host on one person's computer over one
SQLite file. Where a piece is transferable without that stack, it is listed under *adopt later*.

agentes.chat is `AGPL-3.0-only` and read-only reference: [the patterns](agentes-chat.md).

## Decide differently, and keep it that way

| ADR | Their decision | Ours (where) | Why ours stands |
|---|---|---|---|
| 0002 editions and modules | One codebase, two editions, a module interface, `org_id` on every tenant table | One edition; a module is a signed folder the host runs in its own process ([modules](../architecture/modules.md)); no `org_id` in `store/schema.rs` | One person and one file: an org column is a constant on every row, and there is no Cloud edition to switch behaviour on for |
| 0004 configuration | Every env var in one schema, `.env.example`, one master secret with derived keys | Env read at call sites (`chat/decider.rs`, `environment.rs`), settings in `kv`, secrets in the vault under an OS-keyring key (`market/vault.rs`) | The app configures a host install, not an operator with a `.env`; the keyring already replaces a derived key |
| 0005 outbox | One table doubles as queue and outbox, `SKIP LOCKED`, lease, backoff, dedupe | Turns queue in the bot actor (`agent/bot/queue.rs`); runs persist their own retry (`routines/mod.rs`, `routines/retry.rs`) | SQLite has no `SKIP LOCKED`, one process holds the queue, and only routines must survive a restart |
| 0006 auth and tenancy | Better Auth, organizations, roles, `can`/`requirePermission` per request | QR pairing, per-device scopes, `LOCAL_ONLY` methods (`api/devices.rs`) | One person: device scopes and pair-time approval are the permission surface |
| 0007 provider credentials | Envelope encryption with a derived KEK, ciphertext bound to org and id, a gateway as the only model caller | XChaCha20-Poly1305 vault, key in the OS keyring, AAD = the `kv` slot (`market/vault.rs`); the harness calls the model | The keyring beats a config-derived key on one machine, and a gateway would front a harness that already owns the call |
| 0008 agent runs | Runs are rows with a compare-and-set state machine and numbered steps | Turns are actor state; the harness owns the transcript ([context and memory](context-and-memory.md)) | The harness holds the conversation; a run row with a transcript would duplicate it |
| 0010 realtime | WebSocket, per-channel `subscribe`, typed events | Two SSE routes with `rev` catch-up, no subscriptions (`api/events.rs`) | SSE reuses the API's auth, and the one person may see everything |
| 0012 budget reservation | Reserve worst-case cost per month, shrink `maxTokens`, settle with real cost in money | Token tally per bot and fleet, warn at 80%, pause routines at 100% ([provider usage](provider-usage.md)) | The harness makes the call, so there are no dollars and no `maxTokens` to shrink |
| 0015 tool gateway | Gateway is the only path, scopes re-read per call, approval bound to a hash of tool + scope + connection + params, 24 h, approve-always by tool kind | ACP permission cards, ordered pattern rules (`agent/approval_rules.rs`), argument-bound MCP policies with a once-only claim ([approvals](approvals.md), [MCP policies](mcp-tool-policies.md)), fingerprint ledger (`store/approvals.rs`) | There is no scope or connection layer to hash, and binding the arguments is what the MCP policies need |
| 0016 connectors | The org connects once, an admin grants an agent a subset of scopes | Connectors install per computer, on for every bot until a bot opts out ([marketplace](marketplace.md)), governed per tool | Capabilities belong to a computer, and with one person there is no admin separate from the user |
| 0017 search and context | Hybrid `tsvector` + `pgvector` index with channel visibility, a memory loop with trust and proposals | FTS5 with BM25 and an age nudge, bot-scoped, no vectors ([search](search.md)) | No embedding provider in the host, and a bot searches its own rows |
| 0018 provider keys | Adapter per provider, key resolution agent → provider → platform | Each harness signs itself in (`agent/account.rs`); `usage/*` reads the plans' own APIs | A resolution order would be a second source of truth beside the CLI's account |
| 0019 storage | `BlobStore` abstraction, per-org buckets, artifacts never allowed network | Files in the bot folder ([library](opendots-documents.md)), no abstraction, artifact frames run with `allow-scripts` and CDN `https:` (`api/apps_runtime.rs`) | Disk next to the bot is the only storage a single machine has, and artifacts are built on CDN libraries |
| 0020 skills | Skills as org rows with blobs and progressive disclosure, scripts stored but never run | `SKILL.md` folders on disk (`market/skills.rs`, `agent/gear/skills.rs`), run by the harness like any command | "No code execution" is moot once the bot already has a terminal |
| 0021 code sandbox | A persistent cloud machine per bot behind a provider interface, Dockerfile, cost per call | The person's computer, tools through the ACP harness ([computer use](computer-use.md)) | No cloud computer and no money to meter |
| 0023 wiki | Addressed, permissioned pages: slugs, aliases, per-page audience, trash | `[[Title]]` pages per bot with backlinks, versions and review ([documents](opendots-documents.md)) | Pages belong to one bot and one person; a title resolves inside one library without a slug |
| 0025 connected computers | Split lifecycle from channel, `pushFile`/`pullFile` so bytes never cross the model | The connected machine is the only computer; a folder per bot | No cloud lifecycle to split — but the byte-move gap is real (`adopt later`) |
| 0026 budget visibility | Every agent has a budget, money only in `view_usage`, cost fields stripped elsewhere | Plan windows as percent used ([provider usage](provider-usage.md)) | The bot spends the person's plan, not our money, so no cap can be reserved |
| 0029 missions | Executor on the machine is the brain; the server shows, stores, asks, delivers | Status machine over ordinary turns, checkpoint and stop reason ([missions](missions.md)) | The ACP harness is the only brain here, so a mission is a checklist over turns |
| 0030 isolation | Per-agent git identity, protected mode (Seatbelt/bubblewrap), containers | Everything runs as the host user; folder and command blocklist are the bounds ([approvals](approvals.md)) | One person's computer, no server-side credential minting to attach a git identity to |
| 0031 computer use | Four screen kinds, per-agent choice, own action set, vision judge, screenshot retention | The owner's desktop through cua-driver, background-first by element, `type_login`, takeover ([computer use](computer-use.md)) | Tools return text, so no screenshot reaches a model, and protected mode could not apply to the owner's own session |
| 0032 brains | One engine, swappable `Brain` (model, Claude Code, Codex), fallback | The ACP adapter is the brain, one harness per bot, no fallback | Codync holds no model key, so there is nothing to fall back to |
| 0036 desktop app | Tauri tray app with a sidecar runner and stdio channel | Electron with the host service, SSH and updates in the main process ([desktop](../architecture/desktop-app.md)) | The main process already supervises a host; a sidecar buys nothing |

## Same ground, ours is narrower on purpose

- 0009 message partitioning: one `entries` table, scanned. Nothing points at a message, so no
  foreign key and no partition.
- 0011 stack: Bun/Hono/Drizzle against Rust/axum. Their generated client follows an OpenAPI
  document; ours follows the hand-written router in `api/mod.rs`.
- 0014 subscription revocation: no channel subscriptions to revoke; revoking a capability kills
  its device lease (`computer/tests.rs`).
- 0033 public GitHub App: their binding exists to tie an installation to an org; ours binds to the
  host ([GitHub App](github-app.md)).
- 0035 mobile app: Expo/React Native against native SwiftUI and Kotlin. Shared TS only pays off
  with a web client, which Codync doesn't have.
- 0038 mailbox: their own IMAP/SMTP as well as Composio; ours is Composio only
  ([mailbox](mailbox.md)).

## Adopt later, without the stack

Ordered by what the plan does next; none is started.

| ADR | What to take | Cost |
|---|---|---|
| 0037 investigate before answering | Reversible internal changes act at once and post an undo receipt (5 min) instead of a card | Stored before/after; the ledger's fingerprint rows are the shape |
| 0035 mobile app | A high-risk approval is never approved from the notification; deletion confirmed with an emailed code | Two client rules, no new host state |
| 0036 desktop app | Device-code pairing shown on the computer and confirmed from a signed-in session | A new pairing path beside the QR code |
| 0025 connected computers | `pushFile`/`pullFile` so a binary moves without passing through the model; a binary document kind | Two methods and a file kind per client |
| 0013 router | Persist each routing decision so the threshold can be tuned from data | One insert and one read-only view |
| 0028 memory judgements | `valid_from`/`valid_until` on facts and outcome weighting, both judge-free | Columns and a join |
| 0034 agents consulting each other | A blocking `ask_agent` for the cases where a bot must have the answer mid-task | A wait path and a cycle guard |
| 0039 Postgres connector | Its parser, `READ ONLY` transactions, hidden columns — the spec if another engine is ever added | Already the first item in [read-only database](read-only-database.md) |
| 0040 agent browser | A `browser_*` server with its own profile so web work stops stealing the foreground | A pinned sidecar, per-OS Automation grants |
| 0024 git proxy | Branch-scoped pushes where a merge needs an approval the prompt cannot skip | A proxy endpoint and token custody |
| 0027 secrets | Keep the value out of the host where a provider forbids holding it (CLI sign-ins) | An ECDH path and `request_secret` cards |
