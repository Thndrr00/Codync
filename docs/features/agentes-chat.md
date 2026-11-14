# Patterns taken from agentes.chat

agentes.chat (Codari Global, `AGPL-3.0-only`) is read-only reference: its engine is TypeScript over
Bun, so nothing is copied. Each item below is a **behavior** rewritten against Codync's own tables,
with the files where it lives and the tests that hold it. Its ADRs are read before designing; its
`LICENSE` and `NOTICE` stay behind if any line is ever lifted verbatim.

## What Codync already had

| Pattern | Already here |
|---|---|
| A1 Approval cards, one decision each | `PermissionCard`, `chat/*/approve` |
| A3 GitHub pull-request triggers | `host/src/github`, routine `pull_request` triggers |
| A6 A board of what the bots are doing | `botTasks` |

## What was reimplemented

| Pattern | What Codync does now | Code |
|---|---|---|
| A2 An approval is recorded once | Durable ledger with a fingerprint of the card (tool kind, command, folder, diffs); consuming it twice fails | `store/approvals.rs`, `agent/approval_ledger.rs` |
| A4 Pages that name each other | `[[Title]]` links, backlinks, and the graph of a library | `store/documents/links.rs` |
| A5 Every published file keeps its versions | `fileVersions` lists the snapshots of a shared name, newest first | `chat/library.rs` |
| A7 A mailbox through connected apps | Gmail and Outlook through the existing connectors, documented | `docs/features/mailbox.md` |
| A8 Questions answered from a database | Read-only SQLite, one statement, row/column/time caps | `sql.rs`, `docs/features/read-only-database.md` |
| A9 A takeover handed back with a note | `resolveTakeover` ends the takeover with the reason the agent resumes from | `chat/takeover.rs` |
| A10 An import says what it leaves behind | The credential report counts keys a person must re-enter | `import/manifest.rs` |
| A11 A rule learned from two refusals | Two identical denials propose one deny rule, never applied by itself | `agent/approval_lessons.rs` |
| A12 A run at the token limit | Holds (`wait`) or skips per the routine's `limit_policy`; test runs are exempt | `routines/runner.rs` |
| Search over what a bot knows | One index, two kinds (message, document), keys masked, internal entries out, resumable backfill | `store/search.rs`, `chat/recall.rs` |

## Not taken

- **Missions** (block 1 of the plan): their runs carry an objective across turns; Codync's routines
  end. Not started.
- **Brains** (block 3): a third memory scope per project. Not started.
- **Modules** (block 5): the extension point. Not started.
- **Embeddings**, organization permissions and the other search filters: no provider in the host and
  no organization model.

The plan for these is `docs/plan-agentes-chat.md`.
