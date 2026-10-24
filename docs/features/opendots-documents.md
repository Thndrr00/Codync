# Document library and review protocol

The Rust host owns documents. A bot or group owns its library; each saved page
has a stable ID, optional parent, Markdown body and revision. Parent changes
cannot cross libraries or create cycles. Deleting a page also deletes its
descendants. Bodies retain their original Markdown bytes; titles are trimmed.
Limits are 1,000 live pages per library, 512 title bytes and 128 KiB per body.

## Owner API

These authenticated host methods require the existing control scope:

- `listDocuments {botId, query?, afterId?, limit?}` returns metadata `items` and
  `next`. Pages contain at most 100 results; search matches titles.
- `getDocument {id}` returns `{document}` including the body and revision.
- `saveDocument {botId, id?, parentId?, title, body, ifRevision, clientNonce?}`
  returns `{status: "saved" | "conflict", document}`. New pages use revision
  zero; updates must supply the revision last read. An update's parent is
  changed separately through `moveDocument`.
- `moveDocument {id, parentId, ifRevision}` uses the same result contract.
- `deleteDocument {id, ifRevision}` returns `{status: "deleted", ids, rev}` or
  the conflict result. `parentId: null` moves a page to the library root.

Clients retain a draft on failure or conflict. Reload and overwrite are explicit
owner choices. Creation retries reuse a stable `clientNonce`: the host returns
the original creation result, even after the page has changed. Clients bind the
returned ID/revision while preserving text edited since the original request,
then save that text with a revision check. A nonce is not an overwrite command.

## Agent proposals

The bot-facing MCP tools are `list_documents`, `get_document`,
`review_document` and `get_document_review`. Agents propose a write; they do not
receive the owner's direct save method as a document tool. A group member uses
the active group's library, with membership checked by the host.

`review_document` stores an immutable draft and waits for the owner's decision.
`listDocumentReviews {botId}` returns up to the library's 1,000 review records,
with unexpired pending proposals first. `getDocumentReview {id}` loads the full
draft. `resolveAction {id, decision: "approve" | "decline"}` accepts no replacement
payload. Approval commits the saved document and result in one SQLite
transaction. Repeated decisions return that original result; a stale revision
produces a conflict. Declined or expired proposals do not write a document.
Proposals expire after one hour. A nonce reused with a different draft is
rejected. Records survive restart; each library is limited to 1,000 unexpired
pending proposals. Completed history does not exhaust future creation or review.
The review queue returns the most recent 1,000 records with pending proposals
first; older decisions remain retrievable by their ID for safe replay.

This is a review boundary for the document tools, not a sandbox restricting an
agent's other configured tools or direct filesystem access.

## Conversations and clients

Each saved page has a `doc:<id>` conversation. Each specialist gets a separate
session for that page; group sessions also include the group ID in their key.
New page conversations start without forking the main chat. Every turn receives
the latest document revision and JSON-quoted content marked as untrusted data.

Desktop, Android and TUI expose libraries, editing, conflicts, proposals and
computer activity with CSV export. Desktop and Android offer Markdown previews.
All three clients offer `/doc`, `/routine`, `/skill`, `/task`, `/heading`, `/list`,
`/code` and `/quote` to insert Markdown snippets. These insert content only:
they do not create or execute host resources. Desktop supports an inline chooser
at the caret (arrows, Enter/Tab, Escape); Android offers tap completion on the
final line; TUI completes the first matching snippet with Tab on the current
line. Commands in fenced code, indented code or prose remain literal. TUI keeps
its existing text editor and Markdown display; its terminal has no rich DOM.

## Windows and Android document delivery

Windows offers Visual, Markdown and Preview modes. The lazy-loaded Tiptap
editor supports bold/italic, headings, bullet and numbered lists, tasks, quotes,
code, links, tables already present in Markdown, undo/redo and slash snippets.
Opening a document, changing its editability or switching modes does not emit a
save. User changes serialize to Markdown; rich edits may normalize Markdown
syntax while retaining document structure. Plain Markdown mode retains exact
source bytes. Images retain their Markdown reference but render as text
placeholders, so opening an untrusted document does not load remote images.
Link entry permits HTTP, HTTPS and mailto URLs.

Android follows the catalog's native adaptation: plain Markdown editing, preview,
block insertions and tap-based slash completion. Both clients display a subpage
tree, filter invalid loaded move targets, retain explicit revision conflicts,
offer save-as-copy and local draft discard, and show proposed versus saved
Markdown before approval. The host remains the authority for unloaded ancestors
and concurrent changes. Document conversations show loading, empty, failure and
retry states, and retry when the connection returns.

Windows keeps durable drafts in localStorage scoped by context, computer and
bot; the bot mirror owns one document store per library. Closing/reopening panels
reuses that instance. Retiring a connection stops old responses from replacing
new drafts. Recovered drafts are listed explicitly, including text whose saved
page was deleted, which can be saved as a new document.

Android shares a model per bot within the active context. Atomic draft reads and
writes share a mutex and versioned coalescing; context retirement flushes the last
draft before cancelling writers. View-model teardown also flushes the captured
document models after stopping their jobs. Its account-owned, computer-prefixed no-backup
file is removed by the existing forget-computer cleanup. Neither client clears
unsaved text merely because a request fails.

## Validation and remaining checks

The Windows/Android document implementation is complete in source for catalog
items 1–6. Validation is limited by the user's prohibition on compilation:

- 23 interpreted Node tests exercise the actual document store, navigation tree,
  slash parser and Tiptap editor with JSDOM. They cover format/Markdown reopening,
  no writes from content/editability synchronization, lists/tasks/tables/code,
  safe images/links, durable scoped recovery, stale responses, CAS and copy.
- Three Node/SQLite tests use production schema/queries to verify restart and
  creation receipt isolation, pending-only review limits and recursive ownership.
- New Rust regressions cover completed-history limits. They were not executed.
- Parallel source reviews corrected Tiptap's default update emission, shortcut
  capture outside the editor, a shadowed Android restoration variable and a
  restore/flush race when retiring a context.

Rust/Android compilation, TypeScript type checking, packaging and real-app e2e
were not run in this continuation. Before release, validate these flows on real
Windows and Android apps: create/subpage/move/delete; edit in all offered modes,
save/close/reopen; reconnect with an unsaved draft; concurrent revision conflict;
approve/decline/replay/expire a proposal; isolated bot/group page conversations;
switch account/computer with a pending write. Apple clients remain excluded by
the user's explicit instruction.
