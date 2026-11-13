# Next wave: what Codync can add, and how

Status legend: **done** ships in this branch; **next** is designed here and ready to pick up; **later**
is bigger than one release.

| Area | Status |
| --- | --- |
| Decisions inbox (approvals, unread threads, failed runs) | done (desktop) |
| Team templates with seeded roles and hierarchy | done |
| SBOM for every release | done |
| Local models (chat, embeddings, voice) | next |
| Workspace search with citations | done (lexical) |
| Team meeting: minutes (`/minutes`) done; voice roundtable next |
| Standup in a team chat (`/standup`) | done |
| Task board that moves itself | next |
| Handoff with context between bots | done |
| Extension SDK (wasm) | next |
| A2UI (with choices and forms) and AG-UI server done |
| Agent evaluations harness | done (CI wiring next) |
| Policies by role and tool | next |
| Retention (preview + delete) | done |
| Approvals from the phone notification | next |
| Desktop global shortcut done; agent dock next |
| Global search across chats and documents | done |
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

## Global search — done

- Host: `searchAll` finds messages (by their text) and documents (title or body) across every bot,
  newest first, capped at 50 each.
- Desktop: the command palette gains an "Everything" mode: two characters and a short pause, results
  with the bot's face, the kind (message or document) and a snippet; picking one opens that bot.

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

## Workspace search — done (lexical)

- Host: `host/src/chat/workspace.rs` adds `search_workspace` and `read_workspace_file`, bounded to 4,000
  files, 512 KB per file and 40 matches, skipping `.git`, `node_modules`, `target` and other build
  folders; each match carries file, line and a snippet, and reading returns numbered lines.
- Next: embeddings (fastembed or llama.cpp) behind `codync-mem` for semantic search, and a citation card
  in the chat that opens the file at the line.

## Team meeting — minutes done; voice next

- `/minutes` (or `/acta`) in a smart-mode team chat: the lead writes the meeting's decisions, owners
  and open items with `review_document`, then replies with a short summary.
- Next: a meeting routine kind that runs the roundtable on a schedule, and voice with several bots.
- Acceptance: a 4-bot meeting produces minutes with owners and actions, without any bot answering in its
  own chat.

## Standup — done; the board is next

- Standup: `/standup` (or `/daily`, `/parte`) in a smart-mode team chat makes every member report what
  it finished, what is next and what is blocked, in the group chat. It runs when asked, not on a schedule.
- Board: tasks attached to the group with state driven by the run (`assigned`, `working`, `waiting`,
  `done`), rendered as columns on the group page and as cards on the canvas.
- Acceptance: assigning "review the PR" moves the card as the bot works and ends in `done` with the result.

## Handoff — done

- `message_bot` accepts a `handoff` object (summary, files, open questions) and prepends it as a note,
  so the recipient starts from the sender's result instead of repeating it.
- Acceptance: a bot hands a failing test to the debugger and the debugger starts with the failing output.

## Extension SDK — next

- `wasmtime` host with a manifest (`id`, `name`, `version`, `permissions`, `tools`, `panels`) and a
  capability-based API to the host (read state, ask the user, call an MCP tool).
- Extensions install from a `.codync-ext` file and appear in Extensions › Apps; no network by default.
- Acceptance: a sample extension adds one tool and one panel, and cannot read files without permission.

## A2UI and AG-UI — done

- A2UI: an agent writes a fenced ```a2ui block whose body is JSON (text, facts, table, bars, note,
  choices, form); the desktop validates it against a closed schema and draws it with Codync's
  components. Nothing executes and the model never supplies HTML. A choice or a submitted form sends
  one message back to the bot that drew it.
- AG-UI: `POST /agui/{bot}` takes the last user message (or `message`) and streams RUN_STARTED,
  TEXT_MESSAGE_START/CONTENT/END and RUN_FINISHED over SSE once the reply exists; `threadId` and a
  generated `runId` are echoed. It uses the same bearer token as the local API and accepts loopback.
- Next: delta streaming per token instead of one content event.

## Evaluations — harness done; CI next

- `tools/eval-bots.mjs` runs cases from `evals/*.json` against a running host: final reply must contain
  the expected strings and the expected tools must have been called; exits 1 on failure.
- Next: a seeded host in CI and a nightly job with a score per model and a diff between runs.

## Policies, retention, budgets — retention done; policies and the budget gate next

- Policy: per-role and per-tool allow/deny on top of approval rules (which folder, which host, which MCP).
- Retention: done — `retentionPreview` and `pruneRetention` count and delete messages older than N days,
  and Diagnostics › Message retention previews before deleting.
- Budget: reserve before a turn, release on failure, block with a clear state (today routines pause at the
  limit; user turns never do).
- Acceptance: a role cannot run shell outside its folder; retention previews and then deletes only what it
  showed.

## Phone and desktop comfort — shortcut done; the rest next

- Desktop: done — `CommandOrControl+Shift+Space` brings the chat back from anywhere (or hides it).
- Phone approvals from the notification with the same card as the chat.
- Share-to-bot extension on Android and iOS.
- Agent dock with status.
- Offline: encrypted cache of recent chats, queued messages sent on reconnect.
- Acceptance: approve from the lock screen; share a link to a bot from another app; work on a plane and
  send when back.

## Later

- Marketplace reputation: verified publishers, install counts, changelog and updates.
- Skills that improve themselves: proposals from real use, reviewed and versioned.
- Multi-user server: members, roles and channels with one host per team; a different product shape.
