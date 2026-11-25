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
| `parentChat`, `parentThread` | where this task's final answer is delivered; empty for a local task |
| `run` | `idle`, `queued`, `running` or `stopped` in the parallel scheduler |
| `routedSeq` | last task message already delivered to the parent chat |
| `prompt` | the split's instruction, kept in the host and never serialized to clients |

`run` is a scheduling mark, not a second lifecycle: a `closed` task is `idle`, an `archived` one is
`stopped`, and a reopen never silently restarts work. Only an explicit follow-up can requeue a
stopped task.

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
archived one is refused. `taskStop` interrupts the bot's current turn and stamps `run: stopped` so
the scheduler cannot pick it up again — the person is told in the task's own thread not to restart
it, and the parent agent gets the same note as a teammate result. `taskClose` sets `closedAt` and
pauses (`idle`); `taskClose` with `archived: true` stops it for good; `taskReopen` clears `closedAt`
without starting it.

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
| `taskBranch` | `botId`, `taskId` (source), `name`, `before?`, `requestId?` | `{ task, seeded }` |
| `taskDeleteMessage` | `taskId`, `messageId` | `{ deleted }` |
| `taskSend` | `botId`, `taskId`, `text`, `clientNonce?`, `replyToMessageId?` | `{ entry }` |
| `taskSplit` | `botId`, `tasks: [{ name, text }]`, `parentChat?`, `parentThread?` | `{ tasks, scheduled, maxConcurrent }` |
| `taskFollowUp` | `botId`, `taskId`, `text` | `{ task }` |
| `taskCollectResults` | `botId` | the number of results routed |
| `taskAwaitEvent` | `taskId`, `runId?` or `source?` + `event?`, `timeoutSeconds?` | `{ event?, status?, continuedInThread? }` or `{ cancelled }` / `{ timedOut }` |

`limit` is clamped to 1–100 (default 50). `before` and `after` are entry `seq` cursors: `before`
pages older, `after` pages newer, neither gives the newest page. `taskReadAround` centres on a
message id. `taskBranch` copies the newest 50 user/final messages before `before` into a new task;
the copy is a snapshot with a `branchedFrom` marker, not a live link. It only branches from the
newest final answer of a task whose turn is idle or stopped, never while it is queued or running
(OpenBot 010): the pure host decision is `branch_allowed` and the phone menus use the same rule.

A client may send `requestId` with `taskBranch`. The host stores the resulting task id and seed
count under `task-branch.v1:<requestId>`, so a repeated click returns the original answer instead
of creating a second branch. Without an id the call always creates a task; ids are opaque to the
host and never reach analytics. The pure decision is `branch_dedupe`. Desktop and Android generate
one id per click and pass it on the request.

`taskStart`, `taskStop`, `taskSend`, `taskBranch`, `taskDeleteMessage`, `taskDelete`, `taskSplit`,
`taskFollowUp`, `taskCollectResults` and `taskAwaitEvent` are local-only because they drive this
computer's single agent or remove its transcript lines. Reads and metadata
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

## Parallel split, results and side-thread waits

`taskSplit` validates the whole list before writing anything, creates the tasks, and admits at most
**four** of them at a time (`MAX_CONCURRENT` in `chat::tasks::parallel`). Each admitted task's prompt
is posted through `post_in_lane` into that task's thread, so the bot's one actor and queue run it in
its own session; there is no second agent loop. The rest stay `queued` and are admitted as slots free.
A claim is atomic, so a task cannot be posted twice. A post failure puts the slot back in the queue
and surfaces the error to the caller. The cap bounds how many tasks may be admitted at once; the
existing actor still decides when each admitted turn actually runs, so a provider that can only do
one turn at a time simply sees them in queue order rather than starting an unbounded pile.

When a task's turn finishes, `taskCollectResults` (also called by `taskList`) picks the newest final
agent message after `routedSeq` and posts one teammate-style result into the parent chat — or into
`parentThread` when the work was split from a side thread. The prompt is framed as a result, not a
user instruction, so the parent combines answers. Drafts, tool lines and already-routed messages are
ignored; the cursor advances only after a successful route. A stop routes a note instead of a stale
answer, and writes the "do not restart" instruction into the task's own thread.

`taskAwaitEvent` lets a side thread wait for a routine run or script event in the event stream the
host already keeps (`routines.runs_for`). It checks for a stop on every tick, has a 10-minute cap
and a 500 ms poll, and wakes the wait's own thread with a "continue the request" prompt when a
matching run reaches a terminal state. A stop wins over an event at the same instant; expiry and
cancellation return a result instead of an error.

The desktop Tasks surface (`views/tasks`) lists status, opens a task's messages, branches a task
with a request id, and sends follow-up, stop, archive and reopen. It is reachable from an agent's
context menu, beside the existing Tasks sheet for approvals and routine runs. The host auto-names
a branch opened without a title from its source (pure decision `branch_title` in the thread store);
an explicit title is left alone. Both phone thread lists nest branches of the same source message
together under that message's row.

## Busy steps, steering and recovery

The host's turn rules decide which messages may join a running turn, all in pure functions with unit
tests (`host/src/agent/bot/turn_rules.rs`):

- A message the person asked to send now while a turn is still opening its ACP session is held for that
  session and steered in as soon as it opens, instead of waiting for the whole turn to end (OpenBot
  0.78.0). Antigravity takes a steer only after its first update; the host waits briefly for it. Devin
  has no Codync backend.
- A Stop the provider ignored does not keep blocking a later message: while the provider still runs,
  the stale stop flag is reconciled and the message can join it (0.78.0).
- Only the person's own message may interrupt a running Claude tool step. Another agent's or a
  routine's message stays queued until the step ends, and the team tool's answer says so
  (OpenBot nightly triage-15730).
- A tool call still running past 30 minutes ends its turn as stopped and frees the queue; open cards
  are detached so the person can still answer them (triage-15551).
- When a Claude process exits while its subagents or other background work still run, the host posts
  one notice and schedules a short recovery turn that says what stopped and resumes the unfinished
  work without repeating completed steps (triage-15555).
- An agent can answer a whole turn without the MCP servers Codync handed it. When a `session/new`
  reply reports its servers and any declared server is missing, the host posts one plain notice in
  the turn's lane — "The agent ran without Codync's tools" — and marks the tools as changed, so the
  next message opens a fresh session and declares them again (OpenBot triage-15811). An agent that
  does not report its servers is left alone; only server names are compared, never tool or page
  content. The pure decision is `tools_loaded` in `host/src/agent/bot/session.rs`.
- A message the person sends while a sign-in/password/code card is open waits with that card: it
  joins the card's held messages and goes out when they answer, instead of interrupting the turn or
  cancelling the card (0.78.0, triage-15776; `message_during_card` in `agent/bot/queue.rs`).
- A routine configured to run in a separate thread still continues a woken open wait in the bot's
  main chat, not in the routine thread (triage-15775; `wake_target` in `routines/runner.rs`).
- A later link or status line from a teammate does not replace a result already shown in the
  delegating agent's chat; a short "Starting now" note still leaves the eventual result to reach
  the chat (triage-16136/16168; `teammate_update` in `chat/team/assistant.rs`). An optional
  `ask_member` question ending in "(optional)" ends its card quietly when nobody answers, without
  another ask (triage-16368; `optional_answer_done` in `chat/team/members.rs`).
- A background-command answer that a provider reports twice is shown once (triage-16149;
  `background_answer_seen` in `agent/bot/turn_rules.rs`). A message read by a busy Claude whose
  session then closes is requeued instead of lost, while a self-continued background reply still
  reaches the chat (triage-16361/16364; `late_message_action`).
- A turn that stopped in every place and answered nothing says so plainly — "The agent stopped
  before it answered. Send your message again…" — so the person knows to resend (triage-15306;
  `stopped_before_answer`). A "model after this turn" change waits until running turns end
  (`deferred_change` in `agent/bot/queue.rs`), and a slow model list keeps the chosen model across a
  restart (`stable_model` in `agent/auth/models.rs`, triage-16212).
- Vault storage that wasn't ready at startup is reopened on the next settings or agent use
  (`vault_retry` in `market/vault.rs`, triage-16215).

Not in this pass: catalog keys, a dedicated `task` type in the SSE catch-up (metadata is refreshed
with `taskList`), per-task queue reorder and per-task analytics events. No method described here
sends task text to analytics.
