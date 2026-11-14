# OpenBot gap census: F10 (0.58-0.65 residue)

This is the census for card F10 of the OpenBot gap list. For each bullet of F10 it records
whether Codync already has the behaviour, with the file and line that shows it, or writes a
short F11 card for what is missing: what to build, where, which clients, and the test that
would prove it.

- Checked 2026-11-13, read-only. No product code was changed for this document.
- Worktree: `C:\Users\Edwin Villeda\Documents\Codync-openbot-experience`
- Branch: `feat/openbot-connector-experience` at `e89d62cb9`
- Reference: `C:\Users\Edwin Villeda\Documents\openbot-nightly`, OpenBot 0.65.0
  (`CHANGELOG.md` and `src/`). OpenBot is a reference for behaviour only; its source is
  PolyForm Noncommercial, so the F11 cards describe Codync's own build, never an import.

## How to read the tables

| Verdict | Meaning |
| --- | --- |
| EXISTS | The behaviour is in the code now. The evidence column names it. |
| PARTIAL | Some clients or some cases have it; the row says which. |
| MISSING | No code path does it. Its F11 card is in the second half of this file. |

Codync calls OpenBot's channels **group chats** (`BotKind::Group`). "Channel memory" in
Codync also names a different feature: the memory of a Slack, Telegram or Discord channel
(`host/src/channels/mod.rs`). Both readings are checked below.

## 1. Channels

| Sub-part | Verdict | Evidence and behaviour |
| --- | --- | --- |
| A person creates a group with a name, instructions and members | EXISTS | `apps/desktop/src/renderer/views/bots/GroupEditorView.tsx:25-57` (name, about, members saved by `updateGroup`); `apps/desktop/src/renderer/store/bot-store.ts:581-582` (`createGroup`); `apps/android/app/src/main/kotlin/com/codync/android/GroupEditor.kt:95`. The host creates groups through `createBot` (`host/src/api/bots.rs:210-216`) and `Hub::create_bot` branches on `is_group` (`host/src/hub/bots.rs:42`); a marketplace team installs one at `host/src/api/catalog.rs:183-190`. |
| The lead is chosen by a person | PARTIAL | Desktop: `GroupEditorView.tsx:28,114` (`chiefBotId` picker). Android: `GroupEditor.kt:95` ("Chief of Staff"). iOS: no control (`apps/ios/Kit/Sources/CodyncUI/Bots/GroupEditorView.swift:45-108`). The host routes to it in `host/src/chat/group.rs:155-162`. |
| A channel title (a short line under the name) | MISSING | No title on `BotConfig` (`host/src/store/model.rs:131-197`); the editors hold name and description only. F11.4. |
| Archive a channel | MISSING | `BotConfig` has no archived flag. Archiving exists for threads (`apps/ios/Kit/Sources/CodyncKit/Client/HostClient.swift:197-219`) and memory, never for a group. F11.4. |
| A channel image | MISSING | The group editor draws `GroupAvatar` from its members (`GroupEditorView.tsx:65`) and has no picture control. A bot can carry `avatar_photo` (`model.rs:158-160`), but no group path sets or shows a channel picture. F11.4. |
| The agent creates a channel | MISSING | The agent's MCP servers are chat, connectors, routines, computer, team, manage and memory (`host/src/chat/tool.rs:27-33`); `manage` lists and changes bots only and filters groups out (`host/src/chat/manage.rs:99,224`). F11.1. |
| The agent changes a channel (title, instructions, members, lead, archive, image) | MISSING | Same as above: no tool reaches a group. F11.1. |
| The lead discards earlier work | MISSING | The chief only routes (`host/src/chat/group.rs:155-162`); `stop` is the person's (`group.rs:96-104`). No run or task can be dropped by a member. F11.2. |
| Sub-tasks with Stop | MISSING | Groups have room turns, not tasks (`group.rs:41-57`); the only Stop ends the whole room turn. F11.3. |
| Sub-tasks with Continue | MISSING | `continueTurn` continues one stopped *turn* (`host/src/api/bots.rs:566`), not a channel sub-task. F11.3. |
| Sub-tasks with Reassign | MISSING | Nothing names a task owner, so nothing can be reassigned. F11.3. |
| Sub-tasks with Cancel | MISSING | No task tree to cancel. F11.3. |
| `$` skills of the channel | EXISTS (as `/`) | The group composer lists every member's skills: `apps/desktop/src/renderer/views/thread/slash-commands.ts:46-55` (`bot.kind === 'group' ? members.flatMap((m) => m.skills)`). Each member expands `/<skill>` with its own skills: `host/src/chat/group.rs:461-473` and `host/src/market/skills.rs:420-436`. The prefix is `/`, not OpenBot's `$`. |
| "Send to a channel" by a person | PARTIAL | Host: `host/src/chat/forward.rs:15-61`, exposed as `forward` (`host/src/api/bots.rs:465`). Desktop: `views/thread/ForwardSheet.tsx`. Android: `ChatRows.kt:212-213` and `ForwardSheet.kt:34`. iOS: the message menu has Copy, Reply, Branch, Resend and Delete only (`apps/ios/Kit/Sources/CodyncUI/Thread/ChatRows.swift:166-168`). F11.6. |
| "Send to a channel" by an agent | MISSING | `forward` is an API method for clients; no agent tool posts work into a group from its own chat. F11.5. |
| Channel memory (a messaging channel keeps facts) | EXISTS | `host/src/channels/mod.rs:70-81` (`memory_arg`, `with_memory`) and `:115-141` (notes), applied to Telegram, Slack and Discord. |
| Channel memory (a group's members share saved facts) | MISSING | `host/src/chat/memory/api.rs:107-109` refuses memory for a group ("a group has no memory of its own"), and the member prompt carries no channel memory (`group.rs:410-459`): each member answers from its own session's memory. F11.7. |

Upstream for the missing rows:

- `src/backend/agent/channel-agent-tools.ts:21` -
  `export const CHANNEL_AGENT_TOOL_NAMES: ReadonlySet<string> = new Set(["create_channel", "update_channel"]);`
- `src/backend/agent/channel-agent-tools.ts:59-95` - the create and update schemas carry
  `title`, `instructions`, `memberAgentIds`, `leadAgentId`, `isolated`, `icon`/`imagePath`
  and, for update, `addMemberAgentIds`, `removeMemberAgentIds`, `archived`.
- `src/backend/channel-tools.ts:84-97` - `channel_add_member`, `channel_remove_member` and
  `channel_set_instructions`, each "when a person asks for it".
- `CHANGELOG.md:2825-2827` (0.65.0) - "In a channel, you can ask the lead to drop earlier
  work... The lead cancels that whole run with its running and queued sub-tasks" and "Each
  row of 'Waiting for sub-tasks' in a channel now has Stop. A stopped sub-task then offers
  Continue, Reassign and Cancel."
- `src/backend/channel-service.ts:1108-1135` - the `stop` / `resume` / `reassign` /
  `cancel` commands and what each does to the task branch.
- `src/backend/openbot-tools.ts:251,397-400` - the `send_to_channel` tool schema and its
  registration.
- `CHANGELOG.md:3063` (0.62.0) - "A channel setting, 'Keep memories in this channel'. When
  it is on, an agent saves what it learns in the channel as a channel memory, and its other
  chats do not get it."

## 2. WhatsApp

| Sub-part | Verdict | Evidence and behaviour |
| --- | --- | --- |
| An approval answered with 1 or 2 | EXISTS | `host/src/whatsapp/outgoing.rs:58-64` posts the pending card as a numbered list of its options; `:80-118` answers the card by that number. The number is the option's position, so a two-option card is answered with 1 or 2. |
| A code for strangers | MISSING | Only an allow list exists: `host/src/whatsapp/incoming.rs:17-26` and `host/src/whatsapp/inbox.rs:66-74`. An unknown sender is dropped; nobody gets a code and the owner gets no request. F11.8. |
| Authorised groups | EXISTS | `incoming.rs:22` matches the group chat id against `whatsapp.allowed`; `inbox.rs:71-73` does the same before queueing. `host/src/whatsapp/settings.rs:45-63` (`set_allowed`) writes the list. |
| Files in both directions (inbound) | EXISTS | `host/src/whatsapp/incoming.rs:55-68` saves the media and hands the bot its path; `host/src/whatsapp.rs:338-369` writes it under `whatsapp-media/` (jpeg, png, webp, mp4, ogg, pdf). |
| Files in both directions (outbound) | EXISTS | `host/src/whatsapp/outgoing.rs:23-33` and `host/src/whatsapp/transport.rs:121-133` send each file; `host/whatsapp-bridge/bridge.mjs:190-195` sends it. |
| JPEG/PNG/WebP arrive as photos | MISSING | The bridge sends every file as a `document` (`bridge.mjs:192`), so a photo arrives as a file to open, not a picture to save. F11.9. |
| SVG | MISSING | Inbound, `image/svg+xml` falls to the default and is saved as `.bin` (`host/src/whatsapp.rs:349-357`); outbound, SVG goes as an untyped document (`bridge.mjs:192`). F11.9. |
| Durable person data between conversations | MISSING | The store keeps one row per chat and bot with the last sender (`host/src/store/whatsapp.rs:54-61`), and a new chat starts a new conversation root (`incoming.rs:107-129`). A fact learned in one chat is not known in another, or after a new chat, unless the bot happens to save it in its own memory. F11.10. |

Upstream for the missing rows:

- `src/backend/messaging/whatsapp/whatsapp-senders.ts:6-7,22,94` - "No 0/O or 1/I, so a code
  read aloud is not mistaken", "A dedicated number answers an unknown person once, with a
  code for the owner", `request(...): { code, created } | null` (at most three wait).
- `src/backend/messaging/whatsapp/whatsapp-events.ts:24-25` -
  `APPROVE_TEXT = /^(1|approve|yes)$/iu`, `DECLINE_TEXT = /^(2|deny|no)$/iu`.
- `src/backend/messaging/whatsapp/whatsapp-driver.ts:64-71` -
  `IMAGE_TYPES = { ".jpg": "image/jpeg", ".jpeg": "image/jpeg", ".png": "image/png", ".webp": "image/webp" }`;
  `:268-276` sends `{ image: { url }, mimetype, ...dimensions }` for those and a `document`
  for everything else.
- `CHANGELOG.md:2570` (0.63.0) - "On WhatsApp, JPEG, PNG and WebP images arrive as photos
  that the recipient can save."
- `CHANGELOG.md:2704-2706` (0.61.0) - "The agent that answers Slack, Discord, Telegram and
  WhatsApp now saves a lasting fact that a person gives... Before, it kept the fact only in
  that conversation, so a new conversation or a restart lost it."

## 3. Providers

Codync's "provider" is the agent runtime (Claude, Codex, Cursor, Hermes, custom) and its
accounts. There is no separate provider page as OpenBot has.

| Sub-part | Verdict | Evidence and behaviour |
| --- | --- | --- |
| Delete a provider without deleting accounts or agents | MISSING | No provider delete exists. "Connectors & skills" removes only connectors and skills (`apps/desktop/src/renderer/views/marketplace/InstalledView.tsx:37-40`). Accounts have their own delete (`host/src/api/host.rs:348-352`, `host/src/agent/account.rs`), and a downloaded runtime stays in `~/.codync/agents` (`host/src/agent/registry.rs:26-27`) with no way to remove it. F11.11. |
| Effort per model (None, Minimal, Ultra...) | EXISTS | The picker shows the selected model's own options: `apps/desktop/src/renderer/views/bots/AgentModelPicker.tsx:53,125-134`; the slider is `ReasoningSlider.tsx:10-50`. The host passes through whatever the agent advertises (`host/src/agent/auth/models.rs:124-130`). Levels are the agent's, so `none`/`minimal`/`ultra` appear where a model lists them. |
| New models first | MISSING | The host keeps the agent's order: `host/src/agent/auth/models.rs:150` ("The model ids the agent lists, in its order"). Nothing sorts by version. F11.12. |
| Turn a model off and reset, without removing models in use | EXISTS | Hidden models are saved per backend (`models.rs:229-257`), the picker hides and shows them (`AgentModelPicker.tsx:80-83,105`) and still offers a bot's current model when it is hidden (`:55`). Each option resets to Default (`ReasoningSlider.tsx:26-28`; `AgentModelPicker.tsx:130`). |
| A limit chip | MISSING | The picker shows a caption under the summary (`AgentModelPicker.tsx:62-63,93`) from `spentNote` (`apps/desktop/src/renderer/lib/usage-presentation.ts:126`), not a chip beside the provider name. F11.13. |

Upstream for the missing rows:

- `CHANGELOG.md:2561` (0.63.0) - "A provider page in Settings > Providers now has a Delete
  button at the end. It turns the provider off and deletes the CLI that OpenBot downloaded
  for it. Accounts, sign-ins and agents stay."
- `src/backend/agent/provider-models.ts:60-66` - "Newest version first, so the picker leads
  with the latest model. A name with no version goes last".
- `CHANGELOG.md:3064` (0.62.0) - "Turn a model off to remove it from the model picker...
  Reset to default undoes your changes. Agents keep the models that they use."
- `CHANGELOG.md:3082-3085` (0.62.0) - "The model picker shows a usage limit chip next to
  the provider name when a spent plan window limits all its models, with the time when the
  limit resets. Screen readers also read the limit on each model that it stops."

## 4. Files

| Sub-part | Verdict | Evidence and behaviour |
| --- | --- | --- |
| Uploads with progress | PARTIAL | Desktop: `apps/desktop/src/renderer/store/bot-store.ts:122-130` fills `uploadPercents`, drawn as a strip or a percentage (`views/thread/ComposerAttachment.tsx:33,44`). Android: `AppStoreChat.kt:161-168` and `ChatRows.kt:349` ("Uploading files" with the percent). iOS: the upload is awaited with no progress (`apps/ios/Kit/Sources/CodyncUI/Store/BotStore+Sending.swift:110-124`; `ComposerAttachment.swift` draws no state). F11.14. |
| Uploads with a ring | MISSING | No ring anywhere; the desktop and Android use a strip or a number. F11.14. |
| Cancellation of an upload | MISSING | The composer's remove button drops the file before sending (`ComposerAttachment.tsx:34,45`), but once `send` starts, the upload loop has no cancel (`bot-store.ts:118-131`). F11.14. |
| Retry of a failed upload | MISSING | Retry is per message: it resends the whole message and its files with the same nonce (`bot-store.ts:160-168`; `BotStore+Sending.swift:130-134`). A single failed file has no Retry of its own and the message does not stay with the reason per file. F11.14. |
| The send waits | EXISTS | Each file is uploaded before `send` names it: `bot-store.ts:118-132`, `AppStoreChat.kt:159-169`, `BotStore+Sending.swift:112-124`. |
| Downloads retry | EXISTS | Desktop: `views/thread/FileDownloadCard.tsx:14-26,50` (progress, Cancel download, Retry download, reason). iOS: `apps/ios/Kit/Sources/CodyncUI/Thread/FileDownloadCard.swift:64` ("Tap to retry"). |
| 100 MB to the phone | EXISTS | Host cap `host/src/chat/uploads.rs:13` (`MAX_FILE = 100 * 1024 * 1024`), shared files `host/src/chat/files.rs:11,96,138`; Android `apps/android/core/src/main/kotlin/com/codync/android/core/Conversation.kt:94` (`MAX_FILE_BYTES = 100L * 1024 * 1024`); both phones read in chunks (`FileDownload.swift`). |
| A fixed subagent header | MISSING | Codync has no subagent cards. A turn shows tool steps in the Trace (`views/thread/TraceView.tsx`); nothing renders a subagent card or a docked header. F11.15. |

Upstream for the missing rows:

- `CHANGELOG.md:3056-3057` (0.62.0) - "Files that you attach to a chat show in the message
  box at once, before they reach the server. For a remote OpenBot, a ring shows how much of
  each file the server has." and "A file that fails to upload stays in the message box with
  the reason and a Retry button. The other files of the same pick still arrive. Send waits
  until you send the file again or remove it."
- `CHANGELOG.md:3047` (0.62.0) - "The download button becomes a ring with the bytes
  received below the file name, and a click on the ring stops the download. A failed
  download shows 'Download failed' and a retry button."
- `CHANGELOG.md:2807` (0.62.0) - "When you scroll away from a subagent card while its
  subagents work, its header now stays under the chat header with the progress, the current
  step and the time. Click it to go back to the card."
- `packages/ui/src/features/conversation/styles/subagents.css:122-130` - the dock: "while
  the reader is away from a card whose subagents work, its header hangs under the chat
  header... The dock takes no height, so the chat does not move".

## 5. Routines

| Sub-part | Verdict | Evidence and behaviour |
| --- | --- | --- |
| Four parallel subprocesses, not only the flow | EXISTS | A bot runs up to four routines beside its chat: `host/src/hub.rs:273-274` (`MAX_SIDE = 4`) and `:276-289` (`send_side` starts a side actor per run). The scheduler fills those slots in `host/src/routines/runner.rs:436-451`. Each side actor is its own actor and ACP process (`host/src/agent/bot/mod.rs:100-103`; `session.rs:286`), so a run does not queue behind the chat or another run. Flow runs are separate (`runner.rs:189-197`). |

Nothing missing in this bullet.

## 6. Servers

| Sub-part | Verdict | Evidence and behaviour |
| --- | --- | --- |
| The browser site limit | DEFERRED | Codync hosts no static sites, so it has no site limit or slots. Its agent-built apps are local and served by the host (`host/src/chat/apps.rs:1-3`, `host/src/api/apps_runtime.rs`) with no plan limit. F11.16 explains why this stays deferred. |
| A disk warning | EXISTS | `host/src/health.rs:339-343` marks the disk low at 10% free (clears at 12%), and `:484-495` posts one error notice per episode in the working bots' chats ("This computer's disk is almost full (N GB free). Free space, or saving and agent work may fail."), with an info notice when room comes back. It is a chat notice, not a sidebar card. |
| Updating inside an LXC container | EXISTS | An update names the container from `/run/systemd/container`, `/proc/1/cgroup` or a root marker, and skips only the AppArmor profile step when the container has no `/sys/kernel/security/apparmor` (`host/src/service/unix.rs`). The unit still restarts, and a host that does not report the new version and binary hash within 30 seconds rolls back (`host/src/update/install.rs`). F11.17. |
| Programs in `~/.local/bin` in Docker | EXISTS | `host/src/agent/backends/search_path.rs:42-60` adds `~/.local/bin` to the path it exports, and `:71-73` makes every spawned agent resolve the same way. Codync ships no Docker image, so the image-specific gap does not exist here. |

Upstream for these rows:

- `CHANGELOG.md:2794` (0.62.0) - "One server can have a custom limit of active hosted sites...
  from 1 to 1000 sites"; `:3626` - "Server settings > Sites shows See plans when a server on
  your computer is at its site limit"; `:3681` - "When the site limit stops a publish, the
  agent and the app now say what the limit is and how to publish more sites."
- `src/backend/openbot-tools.ts:275-277` - `list_sites` "List the static sites of this
  OpenBot server, with its site limit and the slots in use."
- `CHANGELOG.md:2518` (0.63.0) - "A hosted or self-hosted server in an LXC container, for
  example on Proxmox, installs updates again. The update no longer stops when the container
  cannot load the AppArmor profile of OpenBot."

## 7. A plush in empty states

| Sub-part | Verdict | Evidence and behaviour |
| --- | --- | --- |
| A plush in empty states | EXISTS | Empty lists and empty searches, on desktop, iOS and Android, show the felt plush above the text: `EmptyState`'s `plush` with `components/Plush.tsx`, `MarketEmpty` in `apps/ios/Kit/Sources/CodyncUI/Marketplace/` and in `apps/android/app/src/main/kotlin/com/codync/android/MarketRows.kt`. The working mood moves and Reduce Motion leaves it still. F11.18. |

Upstream:

- `CHANGELOG.md:2609` (0.62.0) - "When a Marketplace search or filter finds nothing, or a
  tab is empty, a felt plush sits above the text and fishes. With reduced motion, it is a
  still image."
- `CHANGELOG.md:2878` (0.61.0) - "When a search finds nothing, the lilac plush shows above
  'No results'. Its eyes follow the caret... with reduced motion the plush stays still."

## F11 cards

These were the next work after the census. On 2026-11-13 they were implemented on the same
branch, except F11.16:

| Card | Status |
| --- | --- |
| F11.1 An agent creates and changes channels | Shipped: `host/src/chat/group/tools.rs` |
| F11.2 The lead drops earlier work | Shipped: `host/src/chat/group/drop.rs` |
| F11.3 Channel sub-tasks and their controls | Shipped: `host/src/chat/group/tasks.rs`, `host/src/store/group_tasks.rs` |
| F11.4 Group editor parity | Shipped: title, archive, picture and the iOS lead picker |
| F11.5 `send_to_channel` for agents | Shipped: `host/src/chat/group/send.rs` |
| F11.6 "Send to a channel" on iOS | Shipped: `apps/ios/Kit/Sources/CodyncUI/Thread/ForwardSheet.swift` |
| F11.7 Channel memory for a group | Shipped: `host/src/chat/memory/channel.rs` |
| F11.8 WhatsApp pairing code for strangers | Shipped: `host/src/whatsapp/senders.rs` |
| F11.9 WhatsApp photos and SVG | Shipped: `host/whatsapp-bridge/media.mjs` |
| F11.10 Durable person data | Shipped: `host/src/whatsapp/person.rs` |
| F11.11 Delete a provider | Shipped: `deleteAgentDownload` |
| F11.12 Newest models first | Shipped: `host/src/agent/auth/models.rs` |
| F11.13 Usage limit chip | Shipped on every model picker |
| F11.14 Upload ring, cancel and retry | Shipped: per-file uploads, `cancelUpload` |
| F11.15 Fixed subagent header | Shipped: `host/src/chat/subagents.rs` |
| F11.16 Browser site limit | Deferred (see its card) |
| F11.17 Updating inside an LXC container | Shipped: `host/src/service/unix.rs` |
| F11.18 Plush in list and search empty states | Shipped on desktop, iOS and Android |

### F11.1 An agent creates and changes channels

- **What.** Add a channel tool set for agents with `create_channel` and `update_channel`:
  name, title, instructions, members, lead, archive and picture. Under Ask each call waits
  on the existing approval card; under Auto it applies and leaves a notice, exactly as
  `chat/manage.rs` does for bots. A new group needs `title` and `archived` on `BotConfig`,
  and a picture path (or icon) that the clients draw.
- **Where.** New `host/src/chat/group/tools.rs`; register it in `host/src/mcp` and
  `host/src/chat/tool.rs`; extend `host/src/store/model.rs`, `host/src/api/bots.rs`
  (`createBot`/`updateBot` for groups); reuse `chat/manage.rs:202-219` for approval and
  `:337-340` for the notice.
- **Clients.** Host only for the tool. The fields it writes must show in the group editor on
  desktop (`GroupEditorView.tsx`), iOS (`GroupEditorView.swift`), Android (`GroupEditor.kt`)
  and the terminal UI (`host/src/tui/manage`).
- **Test.** In `host/src/chat/group/tests.rs`: a bot with manage-team calls `create_channel`
  and the store holds a group with the title, members and lead; `update_channel` archives it
  and it leaves the open list; a denied approval changes nothing. Then one desktop e2e pass:
  the created channel appears in the roster with its picture.

### F11.2 The lead drops earlier work

- **What.** Let a person ask the lead to drop a run, for example when a newer request
  replaces it. The lead cancels the whole run with its running and queued sub-tasks and the
  channel shows a note naming who asked. This depends on F11.3: there is no run or sub-task
  to drop yet.
- **Where.** `host/src/chat/group/tasks.rs` (new, with F11.3), `host/src/chat/group.rs`
  (a `drop` entry beside `stop`), the `chat` MCP server for the lead's tool.
- **Clients.** The note is an ordinary notice, so all four render it; no new control.
- **Test.** A group test starts a request with two sub-tasks, the lead drops it, both member
  turns are cancelled, the note names the requester, and the messages stay. Repeat with the
  lead's call denied and nothing changes.

### F11.3 Channel sub-tasks and their controls

- **What.** Give a group request a small task tree: the request, its assignments, and their
  children. Each row offers Stop; a stopped row offers Continue, Reassign and Cancel. Stop
  pauses the branch below the row, Continue resumes it, Reassign gives the row another
  member and resumes the paused children with it, Cancel ends the root and every descendant
  and keeps the messages.
- **Where.** New `host/src/chat/group/tasks.rs` (state machine and store rows), wiring in
  `host/src/chat/group.rs` (`run`/`spawn`), `host/src/store` (a tasks table or group state),
  and API methods beside `group.rs`'s `stop`.
- **Clients.** A "Waiting for sub-tasks" list with the four controls in desktop, iOS,
  Android and the terminal UI (`host/src/tui/view`).
- **Test.** Rust: stop pauses the tree and the next speaker does not start; continue resumes
  it; reassign changes the owner of that row and resumes its paused children; cancel ends
  the root and descendants and no member turn starts. Then an e2e pass on desktop and one
  phone: stop, continue, reassign and cancel each land on the host.

### F11.4 Group editor parity: title, archive, picture, iOS lead

- **What.** Add the channel fields the agent tools (F11.1) will write: a title, an archived
  flag and a picture (icon or image), plus the lead picker on iOS. Archived groups leave the
  open roster and can be brought back.
- **Where.** `host/src/store/model.rs`, `host/src/api/bots.rs`, the group list filter in
  `host/src/hub.rs`, `apps/desktop/src/renderer/views/bots/GroupEditorView.tsx`,
  `apps/ios/Kit/Sources/CodyncUI/Bots/GroupEditorView.swift`,
  `apps/android/app/src/main/kotlin/com/codync/android/GroupEditor.kt`, `host/src/tui/manage`.
- **Clients.** All four. The iOS editor is the one missing the lead today.
- **Test.** Create a group with a title, lead and picture, archive it, reopen it: every
  value round-trips and the archived group is out of the open list. An iOS UI test sets the
  lead and reads it back.

### F11.5 `send_to_channel` for agents

- **What.** An agent tool that posts work into a channel from its own chat: the named members get it, and with none the lead routes it, as OpenBot's `handoff` does. It returns the channel id. It is for work a person gives a channel, never for a teammate's request.
- **Where.** New tool in `host/src/chat/group/tools.rs`, reusing `host/src/chat/forward.rs`
  for the message and `chat/group.rs:90-93` to start the room turn; approval through
  `chat/manage.rs:202-219`.
- **Clients.** None for the tool; the channel shows the request as the asking agent's message,
  so all four render it.
- **Test.** A bot in its own chat calls the tool; the group receives the request and the lead
  answers; the caller's chat gets a notice. A call naming a channel the bot cannot see is
  refused with a reason it can act on.

### F11.6 "Send to a channel" on iOS

- **What.** The message menu on iOS gets the item the desktop and Android already have: pick
  a chat or group and send the message on, with its files copied.
- **Where.** `apps/ios/Kit/Sources/CodyncUI/Thread/ChatRows.swift:166-168` (the menu), a
  forward sheet in the iOS UI package, and a `forward` call in
  `apps/ios/Kit/Sources/CodyncKit/Client/HostClient.swift` (the host method exists at
  `host/src/api/bots.rs:465`).
- **Clients.** iOS only; desktop and Android already have it.
- **Test.** An iOS UI test forwards a message with one image to a group: the target shows it
  as a forwarded user message and the image is still there after the source chat is deleted.

### F11.7 Channel memory for a group

- **What.** A group keeps memories every member reads: a `channel_remember` /
  `channel_forget_memory` pair, a cap per memory, and a "Keep memories in this channel"
  switch so a member's other chats do not get them. Members get the memories again only when
  they change.
- **Where.** `host/src/chat/memory` (a group layer beside the bot and shared layers),
  `host/src/chat/group.rs:410-459` (the member prompt), the `memory` MCP server, and the
  group settings in each client.
- **Clients.** A Memory section in the group settings on desktop, iOS, Android and the
  terminal UI; the tool is host-only.
- **Test.** Save a memory in a group; a member's next turn includes it and its own chat does
  not. Save the same text again and it is corrected, not duplicated. A memory over the limit
  is refused with the limit as the reason.

### F11.8 WhatsApp pairing code for strangers

- **What.** On a number used only by Codync, an unknown direct sender gets one short code
  (no 0/O or 1/I); the owner approves or denies it in the app, and the person then gets one
  line. At most three wait per number; a repeat from the same person returns the same code.
- **Where.** New `host/src/whatsapp/senders.rs` (codes and pending requests, kept in `kv`),
  `host/src/whatsapp/incoming.rs` (before the allow-list drop), `host/src/whatsapp/settings.rs`
  (approve/deny), and the WhatsApp sheet (`apps/desktop/src/renderer/views/marketplace/WhatsAppSheet.tsx`,
  iOS and Android equivalents).
- **Clients.** The WhatsApp sheet on desktop, iOS and Android shows the waiting people with
  Approve and Deny; the host does the rest.
- **Test.** Rust: an unknown sender gets a code; a second message returns the same code;
  approving lets the next message through; the fourth waiting stranger gets nothing; after a
  restart the pending requests are still there.

### F11.9 WhatsApp photos and SVG

- **What.** Send JPEG, PNG and WebP as photos (with their size, so WhatsApp does not crop
  them), and every other type as a document named by its extension, including SVG as
  `image/svg+xml`. Inbound, save an SVG as `.svg` instead of `.bin`.
- **Where.** `host/whatsapp-bridge/bridge.mjs:190-195` (choose `image` or `document` by
  extension and read the image size), `host/src/whatsapp.rs:349-357` (the inbound extension
  map).
- **Clients.** None; the bridge and host change.
- **Test.** A Node test of the bridge's send payload: `.jpg` becomes `image` with a
  mimetype and dimensions, `.svg` becomes a `document` with `image/svg+xml`, `.pdf` keeps
  its type. A Rust test on `save_media` maps `image/svg+xml` to `.svg`.

### F11.10 Durable person data between conversations

- **What.** A person record per connected account and sender, with their name and the facts
  they gave (the repository, the account to use). Every conversation with that person reads
  it, so a new chat or a restart does not lose it.
- **Where.** New table and accessors in `host/src/store/whatsapp.rs`; read it into the
  prompt in `host/src/whatsapp/incoming.rs:134-139`; let the bot save a fact through the
  memory tools (`host/src/chat/memory/tools.rs`) scoped to the person.
- **Clients.** None for the store; the WhatsApp sheet may show the saved facts per person.
- **Test.** A sender gives a fact in one chat; a second chat with the same sender and a host
  restart still show it. A different sender does not see it.

### F11.11 Delete a provider

- **What.** A Delete action at the end of an agent's page: it turns the agent off and
  removes the runtime Codync downloaded for it, while every account, sign-in and bot stays.
  Bots on it then show the existing "not installed" state until it is installed again.
- **Where.** `host/src/agent/registry.rs` (remove the install directory under
  `~/.codync/agents`), a new method in `host/src/api/host.rs` beside `deleteAccount`
  (`:348-352`), and the agent page in `apps/desktop/src/renderer/views/marketplace/AgentSheet.tsx`.
- **Clients.** Desktop, iOS (`AgentSheet.swift`), Android (`AgentSheet.kt`) and the terminal
  UI's agent settings; the host does the work.
- **Test.** Delete a downloaded runtime: its directory is gone, the account list is
  unchanged, and a bot on that agent still exists and reports "not installed". Reinstalling
  it starts the same bot with the same account.

### F11.12 Newest models first

- **What.** Order each provider's models newest version first, so the picker leads with the
  latest model; a name with no version goes last and the agent's own order breaks ties.
- **Where.** `host/src/agent/auth/models.rs:150` (keep the list but sort it) or the picker's
  list in `apps/desktop/src/renderer/views/bots/AgentModelPicker.tsx:49-55`.
- **Clients.** All four pickers: desktop, iOS, Android and the terminal UI.
- **Test.** A model list of `GPT-4`, `GPT-5.2`, `GPT-5.10` shows as 5.10, 5.2, 4; a model
  named `120B` is not read as version 120; two models of one version keep the agent's order.

### F11.13 Usage limit chip in the model picker

- **What.** A chip beside the provider name when a spent plan window limits all of that
  provider's models, carrying the reset time, and a readable limit on each model it stops.
  It uses the Usage readings already fetched; it asks the provider for nothing new.
- **Where.** `apps/desktop/src/renderer/views/bots/AgentModelPicker.tsx` (the summary row)
  with `lib/usage-presentation.ts` (`spentNote`, `spentWindow`), plus the iOS, Android and
  terminal model pickers.
- **Clients.** All four.
- **Test.** With a spent report the picker renders the chip and the reset text and the
  stopped models are marked for screen readers; with no report it renders nothing and makes
  no extra request.

### F11.14 Upload ring, cancel and per-file retry

- **What.** Each attached file shows a ring of the bytes the computer has; a click on the
  ring cancels that upload; a failed file stays in the message box with its reason and a
  Retry, and the other files of the pick still arrive. Send waits until the file is sent
  again or removed. iOS gets progress at all.
- **Where.** `apps/desktop/src/renderer/store/bot-store.ts:112-148` and
  `views/thread/ComposerAttachment.tsx`; `apps/ios/Kit/Sources/CodyncUI/Store/BotStore+Sending.swift:105-136`
  and `ComposerAttachment.swift`; `apps/android/app/src/main/kotlin/com/codync/android/AppStoreChat.kt:141-188`
  and `ChatRows.kt:349`. The host needs a way to drop a partial upload
  (`host/src/api/bots.rs:354`, `host/src/chat/uploads.rs`).
- **Clients.** Desktop, iOS, Android and the terminal UI's file list.
- **Test.** A slow upload shows the ring and the received bytes; cancel stops it and the
  message does not send; a failed upload shows Retry and retrying sends only that file while
  the others already arrived.

### F11.15 Fixed subagent header

- **What.** Show a card per turn for the subagents an agent starts, with its task, state and
  time; while the reader is scrolled away from a card whose subagents still work, its header
  stays under the chat header with the progress, the current step and the time, and a click
  goes back to the card.
- **Where.** Host: surface subagent updates from `host/src/agent/acp` as their own entries.
  Clients: `apps/desktop/src/renderer/views/thread/ChatRows.tsx` (card and dock),
  `apps/ios/Kit/Sources/CodyncUI/Thread/ChatRows.swift`,
  `apps/android/app/src/main/kotlin/com/codync/android/ChatRows.kt`, and the terminal UI's
  transcript.
- **Clients.** All four.
- **Test.** Start an agent that opens a subagent, scroll the card out of view: the docked
  header shows the live step and time and clicking it returns to the card; when the subagent
  ends the header goes away.

### F11.16 Browser site limit — deferred

- **What.** Hosted sites with a plan limit and the slots in use, a Sites page that says "See
  plans" at the limit, and a publish that fails with the limit and how to get more. An agent
  tool lists the sites, the limit and the slots.
- **Where.** A new hosting module in `host/src` (Codync has none today), the settings page
  in `apps/desktop/src/renderer/views/settings`, and a `list_sites` tool in the `chat` or
  `connectors` server.
- **Clients.** The Sites page on desktop and the terminal UI; the phone apps show the count
  and the limit read-only.
- **Test.** Publish up to the limit, the next publish says the limit and how to raise it, and
  `list_sites` reports the same numbers. A failed publish leaves no slot taken.
- **Decision: deferred, not built.** Codync hosts no static sites and sells no plans, so a
  site limit would gate nothing: the apps an agent builds are local, served by the host
  (`host/src/chat/apps.rs`, `host/src/api/apps_runtime.rs`) with no plan, and adding a slot
  count there would invent a limit on a feature nobody buys. Build this only if Codync ever
  publishes hosted sites with plans; until then the card stays here as the record of that
  reasoning.

### F11.17 Updating inside an LXC container

- **What.** An update on a server in an LXC container (Proxmox, for example) must not stop
  when the container cannot load the AppArmor profile: skip the profile step, restart the
  unit, and keep the previous binary if the new one does not answer.
- **Where.** `host/src/update/install.rs` (`should_roll_back`, the restart-and-verify step)
  and `host/src/service/unix.rs` (`container`, `sandbox`, `reload_profile`); the container
  rule is `container_from` + `sandbox`, both pure.
- **How.** A container is named from `/run/systemd/container`, `/proc/1/cgroup` or a root
  marker (`/.dockerenv`, `/run/.containerenv`); AppArmor is present when
  `/sys/kernel/security/apparmor` exists. On a machine, a present
  `/etc/apparmor.d/codync-host` profile is reloaded exactly as before (Codync ships none, so
  this is normally a no-op). In a container without AppArmor the reload is skipped with a
  warning and nothing else changes: the service restarts and `hello`/health must report the
  new version and binary hash within 30 seconds, or the previous binary comes back.
- **Clients.** None; host only. The app's update state already shows the result.
- **Test.** Unit tests cover the container reading and the rollback rule
  (`host/src/service/unix.rs`, `host/src/update/tests.rs`). Manual Linux check: in a Proxmox
  LXC container without AppArmor, install a profile at `/etc/apparmor.d/codync-host`, run
  `codync-host update`: the log says the profile step was skipped, the service restarts,
  `codync-host status` and `hello` report the new version, and a deliberately broken
  executable rolls back to the old one.

### F11.18 Plush in list and search empty states

- **What.** The plush sits above the text in empty lists and empty searches ("No results",
  "Nothing installed", an empty tab), and stays a still image with reduced motion.
- **Where.** Desktop: `EmptyState` in `apps/desktop/src/renderer/views/HostStateView.tsx`
  takes a `plush` element (and `inline` for a scrolling page); `components/Plush.tsx` is the
  felt character, and `views/marketplace/InstalledView.tsx`, `MarketplacePanel.tsx` pass it.
  iOS: `MarketEmpty` (`apps/ios/Kit/Sources/CodyncUI/Marketplace/MarketEmpty.swift`) draws
  the `CharacterAvatar` plush above the text, used by `InstalledView.swift` and
  `MarketplaceTabs.swift`. Android: `MarketEmpty` in `MarketRows.kt` draws the same plush
  (`CharacterAvatar("blob", "violet", 64.dp, "working")`), used by `InstalledView.kt` and
  `MarketplaceView.kt`.
- **Motion.** The plush is the user's felt avatar in its working mood: it blinks and glances,
  and each client's Reduce Motion handling stops it (desktop `useReduceMotion`, iOS
  `accessibilityReduceMotion`, Android animations off), so it is a still image there.
- **Clients.** Desktop, iOS and Android. The terminal UI has no pictures, so it is unchanged.
- **Test.** Manual check: a Marketplace search with no results shows the plush above "No
  apps/agents/skills match …", **Connectors & skills** with nothing installed shows it above
  "Nothing installed", and with Reduce Motion the same screens show it still. The repo has no
  harness for client rendering, and the decision is pure presentation, so no unit test fits.

## How to use this

The cards above shipped on this branch except F11.16, which stays deferred until Codync hosts
sites. The "Where" lines name the files that exist today, the "Clients" line follows the
cross-platform rule (a UI change lands in iOS, Android, desktop and the terminal UI together),
and the "Test" line is the check that proves it. Everything still needs the compiler and the
end-to-end walk before merge.

Bullets 1, 3 and 4 are the bulk of the residue; bullets 5 and 7 are done. Bullet 2 is
half done: approvals, allowed groups and files work, but strangers, photos, SVG and durable
people do not. Bullet 6 is missing only its server-side site hosting, which stays deferred
(F11.16); the disk warning, the agent path and container updates exist.
