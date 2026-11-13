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
| Agent evaluations harness (text, tools, `--bots` comparison) | done (CI wiring next) |
| Policies by role and tool | next |
| Retention (preview + delete) | done |
| Approvals from the phone notification | next |
| Desktop global shortcut done; agent dock next |
| Global search across chats and documents | done |
| Workspace export (bots + documents) | done |
| Full workspace export (routines, scheduled messages, recent chats) | done |
| Scheduled messages | done (desktop) |
| Notification rules (kinds and quiet hours) | done (desktop; phone settings next) |
| Tool rate limits and a read-only tool cache | done |
| Prompt library in the composer | done (desktop; phone next) |
| Unified calendar (routines + scheduled messages) | done (desktop) |
| Compare models in Diagnostics | done (desktop) |
| Density preference | done (desktop) |
| Presentation mode for artifacts | done (desktop) |
| Bot activity timeline | done (desktop) |
| Policies by role (group editor) | done (desktop) |
| Share a chat as a read-only HTML file | done (desktop) |
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

## Scheduled messages and notification rules — done (desktop)

- A message can be scheduled from the composer (`+` › Schedule message…). The host keeps the list in
  `kv` (`scheduled-messages.v1`), and a half-minute loop posts each due message as the person's own
  message in its chat and starts the bot's turn (`from: "scheduled"`). A message whose time passed while
  the host was off goes out right after it starts; one that fails to post goes back in the list. The
  list and its cancel buttons are in Diagnostics › Scheduled messages.
- Notifications: `notification-rules.v1` stores which of the four alerts are on (`done`, `needsInput`,
  `failed`, `paused`), quiet hours and the zone they use. `remote::push::notify_with` consults it before
  asking the relay. An approval ignores quiet hours: a bot waiting for an answer is stuck until the
  person reads it. Diagnostics › Notifications edits the rules.
- Next: the same rules on the phone's own notification settings, and an "only mentions me" rule for groups.

## Prompt library, calculator and compare — done (desktop)

- The composer's `+` menu opens a prompt library: twelve starting points (summarize, plan, review, risks,
  notes into actions, email, compare, debug, explain, extract, ask first, check writing). Picking one
  fills the message box; nothing runs until it is sent.
- The routines calendar now marks scheduled messages on the day they go out, beside routine runs and
  upcoming occurrences, and lists them in the selected day.
- Diagnostics › Compare models sends one prompt through two or three bots at once, each in a throwaway
  session (`chat::oneshot`): the answers, model and time sit side by side. Nothing is posted in their
  chats and nothing is remembered.
- Settings › General › Density offers Comfortable or Compact, which tightens chat rows, roster rows and
  settings rows without changing the text size.
- An artifact has a Present button: it hides Codync's chrome and gives the artifact the whole window;
  Esc (or the small close button) brings the chat back round it.

## Tool guards — done

- `chat/tool_limits.rs` keeps a per-bot, per-tool sliding minute: 8 `research_sources`, 40 workspace
  reads/searches, 60 chat messages, 30 artifacts, 180 for the rest. Over the limit the tool returns an
  error telling the bot to wait, instead of spending another turn.
- Identical read-only calls (`list_artifacts`, `instruction_history`, `search_workspace`,
  `read_workspace_file`) are answered from memory for 20 seconds, capped at 64 KiB and 256 entries.
  Nothing that writes is cached. Both live in memory and vanish with the host.

## Activity and role rules — done (desktop)

- The inspector (the panel beside a chat) gains an **Activity** tab: `botActivity` returns the main chat's
  messages, cards and notices plus the bot's routine runs, newest first, without the tool steps (the
  trace has those). It refreshes every 15 seconds.
- A group's editor gains **Permissions by role**: Files, Shell, Browser and Apps, each Allow / Ask / Block,
  for every role in its team structure. Saving writes the chosen role's rules into each member with that
  role (`updateBot` with `actionRules` and `actionRulesOn`); only roles whose pickers were touched are
  written, so opening and saving a group changes nothing by itself. Unset classes show Ask.
- The same decision is tested per bot in the rule editor (`testApprovalRule`).
- **Share transcript** in the inspector writes the loaded chat as one HTML file (native Save dialog) or
  opens it in the browser: messages, cards and notices, styled plainly, with no scripts and no link
  back to Codync. Only the window already loaded on this device is included; tool steps stay out.

## Policies, retention, budgets — retention and the user-turn gate done; policies and reservations next

- Retention: done — `retentionPreview` and `pruneRetention` count and delete messages older than N days,
  and Diagnostics › Message retention previews before deleting.
- Budget: done for user turns — `chat::budget_gate` posts the message, does not start the turn, and leaves one
  notice that day ("today's token limit is reached"); raising the limit lets the next message through.
  Routines already pause at the limit with the routine's `limitPolicy`. Reservations before the turn (so a
  half-finished turn is counted too) are still next.
- Policy: per-role and per-tool allow/deny exists as approval rules plus the four action classes
  (`action_rules`), tested in the rule editor (`testApprovalRule`); a role preset seeds them. A per-role
  matrix shown as one screen is next.
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
