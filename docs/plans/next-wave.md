# Next wave: what Codync can add, and how

Status legend: **done** ships in this branch; **next** is designed here and ready to pick up; **later**
is bigger than one release.

| Area | Status |
| --- | --- |
| Decisions inbox (approvals, unread threads, failed runs) | done (desktop) |
| Team templates with seeded roles and hierarchy | done |
| SBOM for every release | done |
| Local models (chat, embeddings, voice) | done: chat via `local-agent`, embeddings, and the system voice |
| Workspace search with citations | done (lexical + optional semantic rerank) |
| Team meeting: minutes (`/minutes`) and text roundtable done; voice spoken by the host and played on the phones |
| Calendar feeds (`.ics`) and a read-only agenda tool for bots | done (host; editor on desktop, iPhone and Android) |
| RSS/Atom subscriptions and a `news_feed` tool for bots | done (host) |
| Web page clipper for bots (`clip_page`) | done (host) |
| Daily digest and a `daily_digest` tool for bots | done (host) |
| Weather (`weather` tool, Open-Meteo) | done (host) |
| Email accounts (IMAP/SMTP via the local Python bridge) | done (host; screens on desktop, iPhone and Android) |
| Waiting-on tracker that feeds the daily digest | done (host; board section on desktop, iPhone and Android) |
| Meeting notes (microphone recording, transcript and minutes) | done (host; recorder and notes on desktop, iPhone and Android) |
| Encrypted cloud backup to WebDAV (daily or now) | done (host; settings on desktop, iPhone and Android) |
| Standup in a team chat (`/standup`) | done |
| Task board that moves itself | done (desktop and phones) |
| Handoff with context between bots | done |
| Extension SDK (wasm) | done: a manifest and `.wasm` per folder, run by wasmtime or wasmer |
| A2UI (with choices and forms) and AG-UI server done |
| Agent evaluations harness (text, tools, `--bots` comparison) | done (nightly workflow) |
| Policies by role and tool | done (desktop) |
| Retention (preview + delete) | done |
| Approvals from the phone notification | done on iPhone and Android |
| Desktop global shortcut and agent dock | done |
| Global search across chats and documents | done |
| Workspace export (bots + documents) | done |
| Full workspace export (routines, scheduled messages, recent chats) | done |
| Scheduled messages | done (desktop) |
| Notification rules (kinds and quiet hours) | done (desktop, iPhone and Android) |
| Tool rate limits and a read-only tool cache | done |
| Prompt library in the composer | done (desktop, iPhone and Android) |
| Unified calendar (routines + scheduled messages) | done (desktop) |
| Compare models in Diagnostics | done (desktop) |
| Density preference | done (desktop) |
| Presentation mode for artifacts | done (desktop) |
| Bot activity timeline | done (desktop) |
| Policies by role (group editor) | done (desktop) |
| Share a chat as a read-only HTML file | done (desktop) |
| Task board (four columns, bot per card) | done (desktop and phones) |
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
| Read-only links (chat and document) with revocation | done (desktop and phones) |
| Scheduled message editing | done (desktop) |
| Board repeating cards | done |
| Doctor: stale approvals | done |
| Document presentation and Markdown export | done (desktop) |
| Global search: board cards and scheduled messages | done (desktop) |
| Team templates (save and install a group) | done (desktop and phones) |
| Board in a group's details | done (desktop) |
| Board: run its routine, CSV export, clear done | done (desktop) |
| Scheduled message "Send now" | done (desktop) |
| Notification bell count | done (desktop) |
| Save a message as a skill | done (desktop) |
| Database backup from Diagnostics | done |
| Approval rules as JSON (copy/apply) | done (desktop) |
| Share-to-bot on Android and iOS | done |
| Real offline mode | done: text outbox and an encrypted per-computer cache (desktop, iPhone, Android) |
| Marketplace reputation | done: stars, install counts, changelog and verified creators in the app page |
| Skills that improve themselves | done for proposals after turns; review UI exists |
| Multi-user server | done: workspace members with roles, enforced on every call |

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

## Local models — chat, embeddings and voice done

- Chat: the `local-agent` ACP shim over Ollama, with tools; Settings offers `createLocalBot`.
- Embeddings: optional Ollama embeddings rerank workspace search; lexical results stand alone.
- Voice: `host/src/voice/local.rs` reads text aloud with a Piper model when one is installed
  (`CODYNC_PIPER_MODEL` or `<data>/voice/piper.onnx`), else macOS `say` (converted to WAV with
  `afconvert`), Windows System.Speech, or `espeak-ng`/`espeak` on Linux. A keyed cloud provider still
  wins when one is configured; a meeting without either keeps its text.
- Dictation falls back to `whisper.cpp` (`whisper-cli`/`whisper` with a ggml model under
  `CODYNC_WHISPER_MODEL` or `<data>/voice/ggml-base.bin`) when the chosen provider has no key or fails.

## Workspace search — done (lexical)

- Host: `host/src/chat/workspace.rs` adds `search_workspace` and `read_workspace_file`, bounded to 4,000
  files, 512 KB per file and 40 matches, skipping `.git`, `node_modules`, `target` and other build
  folders; each match carries file, line and a snippet, and reading returns numbered lines.
- Next: embeddings (fastembed or llama.cpp) behind `codync-mem` for semantic search, and a citation card
  in the chat that opens the file at the line.

## Calendar feeds, news and web clipping — done (host)

- `host/src/calendar_feeds.rs`: read-only `.ics` subscriptions (Google, Apple, Outlook, any `CalDAV`
  export) with `calendarFeeds`, `addCalendarFeed`, `removeCalendarFeed` and `calendarAgenda`. It parses
  UTC, local and all-day events, unfolds long lines, unescapes text, caches each feed for ten minutes,
  and keeps at most twenty feeds. Bots get `calendar_agenda` as a read-only chat tool.
- `host/src/chat/clip.rs`: `clip_page` fetches one page (http/https, 5 MiB, 20 s) and returns its
  title and readable text, dropping scripts, styles, navigation, header, footer and forms.
- `host/src/feeds.rs`: RSS 2.0 and Atom subscriptions with `feedSubscriptions`, `addFeed`,
  `removeFeed` and `feedItems`; parses titles, links, dates (RFC 3339, RFC 2822, bare dates) and
  summaries, caches each feed for ten minutes and keeps at most thirty. Bots get `news_feed`, which
  lists the newest headlines with links the bot can read with `clip_page`.
- Next: showing subscribed events beside routines in each client's calendar, and a feed editor page.
- Acceptance: a Google Calendar export URL answers "what's on my calendar this week", and a link in a
  chat is explained from its text.

## Email and waiting on others — done

- `host/src/email.rs` and `email_bridge.py`: IMAP inbox headers and SMTP sends through the account's
  own servers. Accounts live in `email-accounts.v1`; passwords go to the encrypted vault. The bridge
  uses only Python's standard library, so no mail crate or OAuth app is needed; it needs Python 3 on
  the computer. API: `emailAccounts`, `saveEmailAccount`, `deleteEmailAccount`, `emailInbox`,
  `emailSend`. Bots get `email_inbox` (read-only) and `email_send`. Desktop Settings › Email and the
  Workspace sheet's Email tab on both phones add and remove accounts, read the newest headers and send
  a short message.
- `host/src/waiting.rs`: replies, reviews and deliveries other people owe the person, kept in
  `waiting.v1`. API: `waitingItems`, `saveWaitingItem`, `setWaitingDone`, `deleteWaitingItem`. The
  daily digest adds them, and bots can list them with the read-only `waiting_on` tool. The board on
  desktop and both phones gains a Waiting on section with an add row, a done action and delete.

## Encrypted cloud backup — done

- `host/src/cloud_backup.rs`: `cloudBackupStatus`, `saveCloudBackup` and `runCloudBackup`. The host
  takes a consistent `VACUUM INTO` copy, seals it with the vault key (`vault::seal`, XChaCha20-Poly1305)
  and uploads it with a WebDAV `PUT` to the person's own folder. Settings live in `cloud-backup.v1` and
  the password in the vault; the server only sees ciphertext. Old uploads are deleted after `keep`
  (1–100, default 14). A six-hour ticker uploads once a day while the schedule is on; **Back up now**
  always works.
- Desktop Settings › Backups and the Backups sheet in each phone's computer settings configure the
  folder, username, password and retention, show the last result and the uploaded files.
- Restoring is a manual copy of the `.codync` file back into place, after the key is available on that
  computer; a restore screen is a later step. Only WebDAV is supported for now (Nextcloud, ownCloud,
  Box, pCloud and similar).

## Meeting notes — done (microphone; system audio is next)

- `host/src/meetings.rs`: `startMeeting`, `appendMeetingAudio` (raw 16 kHz mono PCM16, 20 s pieces),
  `finishMeeting`, `cancelMeeting`, `meetingNotes`, `meetingNote`, `meetingMinutes` and
  `deleteMeetingNote`. The host writes each piece down with the configured speech provider, or
  whisper.cpp when none is configured, and keeps the transcript in `meeting-notes.v1`. `meetingMinutes`
  asks one of the person's agents for Markdown minutes (decisions, owners and open items) and stores
  them. Bots get read-only `meeting_notes` and `read_meeting_note`.
- Desktop (Board), iPhone and Android (Workspace › Board) record the microphone, send pieces while
  they record and show the transcript; **Write minutes** asks a bot. The window or phone must stay
  open while it records. Recording what the computer plays (system audio) and speaker labels are the
  next phase.

## Team meeting - done (text and voice)

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

## Extension SDK — wasm runtime done, panels next

- Each extension is a folder under `<data dir>/extensions/<id>` with `extension.json` (`name`,
  `version`, `description`, `entry`, optional `runtime`, `dirs`, `tools` with JSON schemas) and its
  `.wasm` module. `host/src/extensions.rs` lists them and runs a tool with `wasmtime run` or
  `wasmer run` (whichever is on PATH), as `<tool> <arguments JSON>`; stdout is the result, 60 s and
  1 MiB at most, with no environment or filesystem access beyond the manifest's `dirs`.
- Settings › Extensions lists the installed ones, their tools, permissions and the runtime in use, and
  where the folder is. Nothing installs a runtime: a missing one is shown, not bundled.
- Install from a folder or a `.codync-ext` (tar.gz). The manifest's `permissions` are approved once per
  extension on this computer (`approveExtension`), and nothing runs before that. Each tool is exposed to
  bots as `ext_<folder>_<tool>` through the chat tools, so a turn can call it.
- Next: panels and a per-tool permission list (today permissions cover the whole extension).

## A2UI and AG-UI — done

- A2UI: an agent writes a fenced ```a2ui block whose body is JSON (text, facts, table, bars, note,
  choices, form); the desktop validates it against a closed schema and draws it with Codync's
  components. Nothing executes and the model never supplies HTML. A choice or a submitted form sends
  one message back to the bot that drew it.
- AG-UI: `POST /agui/{bot}` takes the last user message (or `message`) and streams RUN_STARTED,
  TEXT_MESSAGE_START/CONTENT/END and RUN_FINISHED over SSE once the reply exists; `threadId` and a
  generated `runId` are echoed. It uses the same bearer token as the local API and accepts loopback.
- Next: delta streaming per token instead of one content event.

## Evaluations — harness and nightly CI done

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

## Local models, semantic search and evals — partly done

- **Ollama**: `localModels` reports whether `{CODYNC_OLLAMA_URL or 127.0.0.1:11434}/api/tags` answers,
  with the models installed; Diagnostics › Local models shows it.
- **Semantic workspace search**: the `semantic_search` tool runs the lexical pass and reranks the
  matches with Ollama embeddings (`CODYNC_EMBED_MODEL`, default `nomic-embed-text`), returning a
  `score` per match. Without Ollama it returns the lexical order with `semantic: false`.
- **Local chat models**: `codync-host local-agent --model <name>` is a small ACP agent that answers
  through Ollama (`/api/chat`); a custom bot with that command (or **Create a local bot** in
  Diagnostics › Local models) runs it without tools or a provider account.
- **Evals in CI**: `.github/workflows/evals.yml` builds the host, seeds a Claude bot with the
  `ANTHROPIC_API_KEY` secret and runs `evals/*.json` through `tools/eval-bots.mjs`; without the secret it
  skips the model run.

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

## Policies, retention, budgets — done

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

## Phone and desktop comfort — shortcut, prompts, meetings and offline done

- Desktop: done — `CommandOrControl+Shift+Space` brings the chat back from anywhere (or hides it).
- The prompt library and team meetings are on the phones: the same twelve prompts, prompts saved on
  the phone, and a group meeting whose turns the phone reads aloud.
- The board, saved teams, read-only links and alert rules (with quiet hours) are on both phones.
- Offline: done — the desktop keeps each computer's bots and recent messages in a file encrypted with
  the OS key (Keychain, DPAPI, libsecret) and reads it at launch; a plaintext cache from an older build
  moves over once. Typed messages wait in the outbox and go out on reconnect. The phones keep their own
  mirrors, and both phones answer approvals from the notification and take a share from another app as a draft.
- Next: a member sign-in that needs no pairing link (today the invite token travels with the pair code).

## Workspace members — done, a team on one computer

`host/src/store/members.rs` keeps one seeded owner, admins, members and viewers; an invite returns a
one-time token stored only as a SHA-256 hash. A phone redeems it on the pairing channel
(`pair_member`), and the device is saved with `member_id`, so the member caller's role is read from the
member list on every connection and recheck: viewers may only read, members work with the bots,
admins also invite, remove and re-role people, and removing a member closes their devices. The desktop
has a Members page in Settings, and both phones take the invite token on their pairing screen. A phone paired the
usual way can be tied to a person from that page (Devices), which applies the role without pairing
again.

## Later

- Marketplace: none of the flagged items remain; the editor endpoints need `MARKETPLACE_ADMIN`.
- Skills that improve themselves: proposals from real use, reviewed and versioned.
- Local voice models: the meeting speaks through a cloud provider; on-device speech is next.

