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
| Team meeting: minutes (`/minutes`) done; text roundtable done, voice next |
| Standup in a team chat (`/standup`) | done |
| Task board that moves itself | done (desktop) |
| Handoff with context between bots | done |
| Extension SDK (wasm) | next |
| A2UI (with choices and forms) and AG-UI server done |
| Agent evaluations harness (text, tools, `--bots` comparison) | done (CI wiring next) |
| Policies by role and tool | done (desktop) |
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
| Task board (four columns, bot per card) | done (desktop) |
| Connector health in the doctor | done |
| A2A server (agent card + `message/send`) | done |
| Notification center | done (desktop) |
| Quick commands `/task` and `/schedule` | done (desktop) |
| Board cards in the workspace export | done |
| Document revision history | done (desktop) |
| Custom prompts in the prompt library | done (desktop) |
| Board from anywhere (`Ctrl+Shift+B`) | done (desktop) |
| Board automation (routine runs, due reminders) | done |
| Inbox approvals in place and a rail count | done (desktop) |
| Document diff (revision vs now) | done (desktop) |
| Board search, bot filter, drag between columns | done (desktop) |
| Activity: approve or deny in place | done (desktop) |
| Read-only links (chat and document) with revocation | done |
| Scheduled message editing | done (desktop) |
| Board repeating cards | done |
| Doctor: stale approvals | done |
| Document presentation and Markdown export | done (desktop) |
| Global search: board cards and scheduled messages | done (desktop) |
| Team templates (save and install a group) | done |
| Board in a group's details | done (desktop) |
| Board: run its routine, CSV export, clear done | done (desktop) |
| Scheduled message "Send now" | done (desktop) |
| Notification bell count | done (desktop) |
| Save a message as a skill | done (desktop) |
| Database backup from Diagnostics | done |
| Approval rules as JSON (copy/apply) | done (desktop) |
| Share-to-bot on Android and iOS | next |
| Real offline mode | text outbox on desktop done; encrypted full cache next |
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

## Standup — done; the board follows routines

- Standup: `/standup` (or `/daily`, `/parte`) in a smart-mode team chat makes every member report what
  it finished, what is next and what is blocked, in the group chat. It runs when asked, not on a schedule.
- Board: the standalone board is done (four columns, one card per task, drag and menu moves, a routine
  per card that moves it on its next run). Attaching a card to a group's room turn, and rendering the
  board on the group page and canvas, is still next.
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

## Reserves, policy, offline and a roundtable — done

- **Budget reservations**: `Budgets::reserve` holds 4,000 tokens for a turn and refuses it when the
  bot or the fleet has no room; the reservation is released when usage is counted and expires after
  ten minutes if a turn never reports. User turns and scheduled messages both go through it.
- **Tool policy**: a bot's `blockedTools` (comma-separated names, `*` for every chat tool) is checked
  before cache and rate limits, so a blocked tool never runs.
- **Offline text**: a message typed while its computer is offline waits in a device outbox
  (`store/outbox.ts`), shows "Waiting for the computer to come online", and sends with the same nonce
  when the connection returns. Files still need the computer online.
- **AG-UI**: the finished reply is emitted as 160-character `TEXT_MESSAGE_CONTENT` deltas instead of
  one content event.
- **Roundtable**: `teamMeeting` asks each group member in turn, in a throwaway session, with the
  earlier turns in its prompt, and returns the turns and draft minutes; the group editor opens it with
  a topic. Voice is still the phone's job.

## Teams, skills and upkeep — done

- **Team templates**: a group can be saved as a template (`saveTeamTemplate`), listing its members,
  roles and hierarchy. The New group sheet offers them; installing creates the bots and the group with
  the same roles remapped to the new ids. Up to 30 templates.
- **Group board**: a group's inspector shows the board filtered to its members.
- **Board**: a card can run its linked routine now, the board exports CSV, and **Clear done** removes
  the finished cards after a confirmation.
- **Scheduled messages**: **Send now** moves a waiting message to the next tick.
- **Notifications**: the bell shows how many notices arrived since it was last opened.
- **Save as a skill** on a message opens a sheet (name, when to use it, instructions); the skill is
  installed and turned on for that bot only (`installSkill` now takes `enableFor`).
- **Backup**: Diagnostics writes a consistent copy of the database with `VACUUM INTO` under
  `backups/` and copies the path.
- **Approval rules**: a sheet copies them as JSON and applies a pasted list back.

## Links, repeats and smaller answers — done

- **Read-only links**: `share::publish` keeps a self-contained HTML page under a 128-bit token for a
  week; `GET /share/{token}` serves it without a sign-in, and `sharedLinks` / `revokeShare` list and
  revoke them (Diagnostics › Read-only links). The chat's share menu and the document's share menu
  create one and copy it; the page has no scripts and no link back to Codync.
- **Scheduled messages** can be edited (text and time) in Diagnostics, not just cancelled.
- **Repeating cards**: a board card can repeat daily or weekly; finishing it creates the next one in To
  do with the due date moved on. `chat::board_auto` reminders then apply to it as well.
- **Doctor**: a warning when an approval has waited more than a day.
- **Documents**: a Present button shows the document alone, full window (Esc returns), and Share saves
  it as Markdown as well as publishing a link.
- **Global search** now includes board cards (which open the board) and scheduled messages, next to
  messages and documents.

## Board that moves itself, inbox answers - done

- A card in **Doing** can name a routine (`chat::board_auto`): when that routine's next run finishes
  after the card was moved, the card goes to **Done** on success or **Waiting** on failure. The board's
  add row offers the owning bot's routines.
- A card due today or tomorrow (or up to a week late) writes one notice a day in its bot's chat, so the
  chat and the notification center carry the reminder.
- The board gains a search box, a filter by owning bot, drag between columns (the card menu still
  works), and the new-card field takes focus when the board opens.
- The inbox answers approvals in place: **Deny** and **Allow** beside each card (`botTasks` now returns
  its options), and the rail shows how many approvals wait.
- Activity's approval rows answer the same way.

## Prompts, documents and comparisons — done

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

## A2A, notifications and quick capture — done (desktop)

- **A2A**: `GET /.well-known/agent.json` publishes one skill per agent bot; `POST /a2a` takes JSON-RPC
  `message/send`, reads the text from `params.message.parts`, names the bot with `params.metadata.botId`
  (its skill id) and answers with a `task` whose artifact carries the reply. It runs through
  `chat::oneshot`, so nothing appears in the person's chats and nothing is remembered. Same bearer token
  as the local API; loopback needs none. Streaming is not offered.
- **Notification center**: `recentNotices` returns the last two weeks of notice entries across bots;
  the bell in the sidebar (and the palette) lists them newest first, with the bot and computer, and a row
  opens that chat.
- **Quick commands**: `/task <title>` adds a board card and `/schedule 30m <text>` schedules a message
  from any chat (not from a document's side conversation); the slash menu lists both. A message's menu
  also has **Add to the board**.
- **Board**: cards can be edited (title, detail and due date), the board has its own global shortcut
  (`Ctrl+Shift+B`), and it is part of the workspace export.
- **Documents**: each save keeps the replaced text as a revision (ten per document); the editor's history
  sheet lists them and puts one back in the editor to save.
- **Prompt library**: the message box's text can be saved as a prompt of the person's own (device only,
  up to 50) and removed from the same sheet.
- **Documents**: the editor's history sheet now compares a revision with the text in the editor
  (`views/bots/doc-diff.ts`, a capped line diff) beside its numbers, and still offers to put the whole
  revision back.

## Connector health — done

- `doctor` now asks whether each connector a bot lists is still installed (`market::connector_ids`), and
  reports `MissingConnector` as a warning. It shows in Diagnostics › Bot check, next to the folder and
  agent checks, so a connector removed from the computer is visible before a turn fails on it.

## Board - done (desktop)

- `store/board.rs` keeps the computer's cards in `kv` (`board.v1`): title, detail, one of four
  statuses, the bot that owns it, and a due date. `boardTasks`, `saveBoardTask`, `moveBoardTask` and
  `deleteBoardTask` are the API; a card keeps its creation time when edited.
- The desktop's rail gains **Board**, with four columns, an add row, a card menu to move or delete, and
  a computer picker when the app is connected to several. A card's bot opens its chat.
- Next: the board moving itself from routine runs and turn states, and drag between columns.

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
