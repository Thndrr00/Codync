# Patterns taken from agentes.chat

agentes.chat (Codari Global, `AGPL-3.0-only`) is read-only reference: its engine is TypeScript over
Bun, so nothing is copied. Each item below is a **behavior** rewritten against Codync's own tables,
with the files where it lives and the tests that hold it. Its ADRs are read before designing; where
one decides differently from us and we keep ours, it is written down in [ADR deltas](agentes-chat-adr-deltas.md).
Its `LICENSE` and `NOTICE` stay behind if any line is ever lifted verbatim.

## What Codync already had

| Pattern | Already here |
|---|---|
| A1 Approval cards, one decision each | `PermissionCard`, `chat/*/approve` |
| A3 GitHub pull-request triggers | `host/src/github`, routine `pull_request` triggers |
| A6 A board of what the bots are doing | `botTasks` |

## What was reimplemented

| Pattern | What Codync does now | Code |
|---|---|---|
| A2 An approval is recorded once | Durable ledger with a fingerprint of the card (tool kind, command, folder, diffs); consuming it twice fails, and an automatic rule or the bot's automatic setting leaves its own record naming what let it through | `store/approvals.rs`, `agent/approval_ledger.rs` |
| A4 Pages that name each other | `[[Title]]` links, backlinks, and the graph of a library | `store/documents/links.rs` |
| A5 Every published file keeps its versions | `fileVersions` lists the snapshots of a shared name, newest first | `chat/library.rs` |
| A7 A mailbox through connected apps | Gmail and Outlook through the existing connectors, documented | `docs/features/mailbox.md` |
| A8 Questions answered from a database | Read-only SQLite, one statement, row/column/time caps (the deadline held by a real slow query); SQLite only, with the reasons and what another engine would need | `sql.rs`, `docs/features/read-only-database.md` |
| A9 A takeover handed back with a note | `resolveTakeover` ends the takeover with the reason the agent resumes from; taking it records who and since when and holds the agent's queue | `chat/takeover.rs`, `hub.rs` |
| A10 An import says what it leaves behind | The credential report counts keys a person must re-enter | `import/manifest.rs` |
| An export a person can open elsewhere | HTML, Word or PDF through the machine's converters, naming the one to install when there is none | `document_export.rs`, `docs/features/opendots-documents.md` |
| An export uploaded to object storage | A path-style, SigV4-signed `PUT` to the bucket set on the computer | `s3.rs` |
| A11 A rule learned from two refusals | Two identical denials propose one deny rule, never applied by itself | `agent/approval_lessons.rs` |
| A12 A run at the token limit | Holds (`wait`) or skips per the routine's `limit_policy`; test runs are exempt | `routines/runner.rs` |
| Search over what a bot knows | One index, two kinds (message, document), keys masked, internal entries out, resumable backfill | `store/search.rs`, `chat/recall.rs` |
| A person waiting goes first (ADR 0046) | A bot's queue keeps the three urgency levels (person, agent, background) and inserts a new turn ahead of lower ones, stable within a level; a burst of routines never overtakes a person's message | `agent/bot/cmd.rs`, `agent/bot/queue.rs` |

## Not taken

- **Embeddings**, organization permissions and the other search filters: no provider in the host and
  no organization model.
- From ADR 0046, **aging and reserved slots**: Codync queues turns per bot, not jobs in one global
  outbox, so there are no shared slots to reserve and no enqueue time on background items to age.
  The three priority levels and the "person first" order are kept.

Built since this report was written: **missions** are their own feature now
([missions](../features/missions.md), block 1), GitHub pull-request work runs from a **GitHub App**
([github-app](../features/github-app.md), block 4), a bot works in a **brain**
([brains](brains.md), block 3), and a **module** is the extension point
([modules](../architecture/modules.md), block 5), so none of them is a departure from agentes.chat
any more.

The plan for the rest is `docs/plan-agentes-chat.md`.
