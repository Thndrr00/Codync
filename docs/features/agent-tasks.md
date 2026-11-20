# Agent tasks

A task is a named, long-lived turn container of one agent bot, separate from the chat. It is meant
for work that outlives a single exchange: a person can list tasks, send into one, close it, reopen
it later and branch a new one from what happened before. The host owns the whole lifecycle; the
phone, desktop and terminal only render it and send intents.

Origin: adapted from OpenBot nightly's `agent-tasks` subsystem (0.70.0) to Codync's existing
threads, entries and bot queue. This page describes the host side; clients are wired separately.

## Data model

A task is **one bounded row plus existing transcript entries** — there is no second message store:

| Field | Meaning |
| --- | --- |
| `id` | `task:<uuid>`, the id clients pass back |
| `botId` | the agent that owns it |
| `name` | trimmed, non-empty, at most 120 characters |
| `state` | `open`, `closed` or `archived` |
| `root` | a notice entry in the bot's chat that anchors the task's thread |
| `startedAt`, `closedAt` | milliseconds; `closedAt` is 0 while open |
| `readRev` | the host's last-read mark over the task's entries |
| `rev`, `updatedAt` | the global mutation rev and the page ordering key |

Messages are ordinary `entries` in `Lane::in_thread(bot, root)`. That gives a task its own agent
session, the transcript's rev/stamp, history search, file attachments and the existing thread paging
for free. The root also doubles as a thread root for the chat chip: incoming task messages emit the
existing `thread` event, and `taskMarkRead` writes both the task's `readRev` and `thread_reads`, so
the chip's unread dot follows the task. Each bot is capped at **100 tasks**; creating past the cap is
refused rather than silently dropping an older one. The `tasks` table is created/altered idempotently
on first use, so databases from before this feature need no manual migration.

Unread is not copied into clients: it is counted in the host as final agent messages (and pending
permission cards) whose `contentRev`/`rev` is newer than the task's `readRev`. `taskList` and the
`task` events carry it.

## States

```text
create -> open
open    -> closed | archived
closed  -> open | archived
archive -> open            (an archived task must be reopened before it can be closed again)
```

A same-state call is harmless. Only an `open` task accepts a message; sending into a closed or
archived one is refused. `taskStop` interrupts the bot's current turn but does not change the state;
`taskClose` sets `closedAt`, and `taskReopen` clears it.

## Host API

`POST /api/<method>` with a JSON body. All names and bodies are camelCase.

| Method | Body | Answer |
| --- | --- | --- |
| `taskCreate` | `botId`, `name` | `{ task }` |
| `taskList` | `botId`, `status?` (`open`/`closed`/`archived`) | `{ tasks }` |
| `taskRename` | `taskId`, `name` | `{ task }` |
| `taskStart` | `taskId` | `{ task }` |
| `taskStop` | `taskId` | `{ stopped, rev }` |
| `taskClose` | `taskId`, `archived?` (true archives) | `{ task }` |
| `taskReopen` | `taskId` | `{ task }` |
| `taskDelete` | `taskId` | `{ deleted, rev }` |
| `taskMarkRead` | `taskId`, `throughMessageId?` | `{ task }` |
| `taskReadPage` | `taskId`, `limit?`, `before?`, `after?` | `{ entries, hasMore }` |
| `taskReadAround` | `taskId`, `messageId`, `limit?` | `{ entries }` |
| `taskBranch` | `botId`, `taskId` (source), `name`, `before?` | `{ task, seeded }` |
| `taskDeleteMessage` | `taskId`, `messageId` | `{ deleted }` |
| `taskSend` | `botId`, `taskId`, `text`, `clientNonce?`, `replyToMessageId?` | `{ entry }` |

`limit` is clamped to 1–100 (default 50). `before` and `after` are entry `seq` cursors: `before`
pages older, `after` pages newer, neither gives the newest page. `taskReadAround` centres on a
message id. `taskBranch` copies the newest 50 user/final messages before `before` into a new task;
the copy is a snapshot with a `branchedFrom` marker, not a live link.

`taskStart`, `taskStop`, `taskSend`, `taskBranch`, `taskDeleteMessage` and `taskDelete` are local-only
because they drive this computer's single agent or remove its transcript lines. Reads and metadata
changes work for paired devices too. A `task` event on the existing event stream carries the new
`rev` with every change, so clients refresh by `rev` and call `taskList` for metadata after a
reconnect. Task messages are `entry` events like any other and are included in the normal catch-up.
No task name, text or message body is sent to analytics.

## Reuse and boundaries

- `taskSend` calls `chat::messages::post_in_lane`, which posts into the task's thread and hands the
  `Cmd::Send` to the bot's one actor. The normal queue and the computer's Sending/steer setting
  decide when it runs; the per-task queue is the bot's shared queue, addressed by the task's entry
  ids through the existing `steer` and `cancelQueuedMessage` methods.
- `taskStop` calls `hub.stop_bot` (`agent::bot`'s stop path). Starting is implicit in the first
  send, as for every Codync bot; no second agent process or loop is started.
- Message deletes go through `hub.delete_entry`, so clients receive the usual `entryDeleted`.

Not in this pass: client screens and catalog keys, a dedicated `task` type in the SSE catch-up
(metadata is refreshed with `taskList`), per-task queue reorder, and per-task analytics events.
