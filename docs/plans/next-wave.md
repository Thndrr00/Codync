# Next wave: what Codync can add, and how

Status legend: **done** ships in this branch; **next** is designed here and ready to pick up; **later**
is bigger than one release.

| Area | Status |
| --- | --- |
| Decisions inbox (approvals, unread threads, failed runs) | done (desktop) |
| Team templates with seeded roles and hierarchy | done |
| SBOM for every release | done |
| Local models (chat, embeddings, voice) | next |
| Workspace RAG with citations | next |
| Team meeting: voice roundtable + minutes | next |
| Standup routines for a team | next |
| Task board that moves itself | next |
| Handoff with context between bots | next |
| Extension SDK (wasm) | next |
| AG-UI server and A2UI in the chat | next |
| Agent evaluations in CI | next |
| Policies by role and tool | next |
| Retention and redaction | next |
| Approvals from the phone notification | next |
| Desktop agent dock and global shortcut | next |
| Share-to-bot on Android and iOS | next |
| Real offline mode | next |
| Marketplace reputation and updates | later |
| Skills that improve themselves | later |
| Multi-user server | later |

## Decisions inbox — done

`InboxView` (desktop) asks every online computer for `botTasks` of each agent bot every eight seconds and
groups what waits: open approvals, unread threads and failed or recovering routine runs. The rail gains an
Inbox item; a row opens the bot. No host change: `botTasks` already returns exactly those three lists.

Next: a count badge on the rail, actions in place (approve/deny without opening the chat), and the same
list on the phone through one `inbox` host method that merges every bot (today the phone would need one
call per bot).

## Team templates — done

The bundled catalog now carries `roles` per team (coordinator included) and six teams: software, research,
product, personal office, quality squad and design squad. Installing a team seeds its `team` settings in
smart mode: the coordinator is the lead, every member gets the role from the template, and the cards land
in two rows so the canvas opens usable.

Next: let a person edit the template before installing (roles and members), and show the resulting
hierarchy in the install sheet.

## SBOM — done

`tools/sbom.mjs` writes a CycloneDX 1.5 inventory of the Rust crates (via `cargo metadata`) and the
desktop npm tree (production dependencies only), deduplicated by purl. Attach it to the release job next
to the signed installers.

## Local models — next

Goal: a bot can run without an account.

- Host: a new backend kind `local` that speaks the OpenAI chat-completions API against
  `http://127.0.0.1:11434` (Ollama) or a configurable base URL, in `host/src/agent/local.rs`. It reuses
  the existing turn loop and tool calls; no ACP.
- Settings: provider row "Local model" with base URL, model and context size; detected models listed from
  `/api/tags`.
- Memory and search: `fastembed` (or llama.cpp embeddings) behind `codync-mem`, with vectors in
  `sqlite-vec`.
- Voice: `whisper.cpp` for dictation and `piper` for reading, run as tools of the runner.
- Acceptance: a bot answers and uses a tool with no network, and memory search returns results without a
  provider key.

## Workspace RAG — next

- Host: `host/src/rag/` indexes the bot's folder (code, Markdown, PDF through PDFium) into chunks with
  embeddings; a `search_workspace` tool returns passages with file and line.
- Desktop: a citation card in the chat that opens the file at the line.
- Acceptance: "where is the login handled?" answers with three citations that resolve.

## Team meeting — next

- A routine kind `meeting` runs the group's smart roundtable: every member speaks once, the lead closes
  with decisions and owners, and the transcript becomes a document linked to the group.
- Voice: reuse the existing call pipeline with several bots; each speaks its turn, the user can interrupt.
- Acceptance: a 4-bot meeting produces minutes with owners and actions, without any bot answering in its
  own chat.

## Standup and the board — next

- Standup: a group routine with the roles as the agenda; each member reports, the lead summarises.
- Board: tasks attached to the group with state driven by the run (`assigned`, `working`, `waiting`,
  `done`), rendered as columns on the group page and as cards on the canvas.
- Acceptance: assigning "review the PR" moves the card as the bot works and ends in `done` with the result.

## Handoff — next

- Extend `message_bot` with `handoff: true`: the recipient receives the summary of what was done, the
  files touched and the open questions; the sender's chat shows a single line instead of the full history.
- Acceptance: a bot hands a failing test to the debugger and the debugger starts with the failing output.

## Extension SDK — next

- `wasmtime` host with a manifest (`id`, `name`, `version`, `permissions`, `tools`, `panels`) and a
  capability-based API to the host (read state, ask the user, call an MCP tool).
- Extensions install from a `.codync-ext` file and appear in Extensions › Apps; no network by default.
- Acceptance: a sample extension adds one tool and one panel, and cannot read files without permission.

## AG-UI and A2UI — next

- Host: `host/src/agui/` exposes `POST /agui/{bot}` streaming AG-UI events from the existing bus.
- Desktop: renders A2UI messages from the agent with Codync's components; the catalog is explicit, no
  generated code.
- Acceptance: the AG-UI Dojo talks to a bot, and a bot draws a form that returns values to the host.

## Evaluations — next

- A suite per bot: prompts, expected tools, forbidden actions; run with `aimock` so no key is needed.
- CI job nightly and on demand; a score per model and a diff between runs.
- Acceptance: changing a bot's prompt shows pass/fail per case before merging.

## Policies, retention, budgets — next

- Policy: per-role and per-tool allow/deny on top of approval rules (which folder, which host, which MCP).
- Retention: days per space for messages, attachments and runs; a prune job with a dry-run preview.
- Budget: reserve before a turn, release on failure, block with a clear state (today routines pause at the
  limit; user turns never do).
- Acceptance: a role cannot run shell outside its folder; retention previews and then deletes only what it
  showed.

## Phone and desktop comfort — next

- Phone approvals from the notification with the same card as the chat.
- Share-to-bot extension on Android and iOS.
- Desktop: menu-bar agent dock with status and a global shortcut, as OpenBot's.
- Offline: encrypted cache of recent chats, queued messages sent on reconnect.
- Acceptance: approve from the lock screen; share a link to a bot from another app; work on a plane and
  send when back.

## Later

- Marketplace reputation: verified publishers, install counts, changelog and updates.
- Skills that improve themselves: proposals from real use, reviewed and versioned.
- Multi-user server: members, roles and channels with one host per team; a different product shape.
