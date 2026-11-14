# Search

Ask a bot what it knows about some words, and get the source back with a snippet: that is the whole
feature. It is the host's (`host/src/store/search.rs`), so every client gets the same answers.

## How it works

- One FTS5 table (`search_fts`) holds the text of every source, with its **kind**: a `message` (the
  user's or the agent's `text`, a tool's `title` or `detail`) or a `document` (its title and body).
- Sources are indexed as they are written: messages in `insert_entry`/`insert_entry_once`, and again on
  `update_entry` (a message still streaming grows), pages in `save_document`. A page that changes is
  replaced in the index; a page that is deleted leaves it, and so does a deleted message or a deleted
  bot (all of its sources).
- A database written before search existed is backfilled, a couple of thousand entries per pass, from
  a marker in `kv` that moves as it goes: a large history never blocks one search on all of it, and a
  pass that was cut short carries on where it stopped.
- `search {botId, query, limit?, tools?, author?, since?, until?}` returns
  `{hits: [{kind, id, at, snippet, entryKind, author, title, threadId}]}`, best match first. The
  snippet marks the match with `[...]` and elides the rest; `threadId` is where a hit lives, so a
  screen can open it in place. Tool traces, thoughts, permission prompts and notices are left out
  unless `tools` is true: what a person searches for is what was said. The limit counts what is
  kept, so a screenful of traces never empties the answer.
- `author` keeps one voice: `user` or `agent` (which also covers a plan). Pages belong to no voice,
  so naming one leaves them out. `since` and `until` are epoch milliseconds. A value the index does
  not know is ignored, never an error.
- Ranking is BM25, nudged by how recent a hit is: the search reads `limit × 4` candidates and sorts
  them by `bm25 + days/8.3` (capped at three). An old exact match still beats a recent weak one;
  a hit from this week beats an equal one from last year. The limit is applied after that.
- `searchReindex` rebuilds the whole index from the store: the repair for a bad migration or a
  hand-edited database. It answers `{indexed}` and is safe to run twice.
- Every word must appear and each is a prefix, so `deploy api` finds sources with both and `deploy`
  finds `deployment`. A stray quote, `AND` or `*` is treated as a word instead of failing.

## In the apps

Every client asks the same question from inside a chat: a magnifying glass in the chat header, a
field, and three tabs for whose words to keep (Everyone, Me, Bot) with a tool-traces switch beside
them. A message opens in its context; a page opens in place, before its library. The Android sheet
also offers saved searches, whose scope narrows the answer to the main chat or to threads. The
terminal's sheet answers `Ctrl+S` and the palette's Search, and keeps the same keys as its other
overlays (tab side, `^t` traces, esc back).

## What is never indexed

- **Internal entries** (`data.internal`): a turn's own plumbing.
- **Keys.** A token that looks like one (`sk-`, `ghp_`, `AKIA`, `xox…`, `Bearer …`) is stored as its
  first six characters and `…`, so the index does not hold the secret twice. The message itself, in
  the chat, is untouched.
- **Binary attachments.** Only text fields are read; a file's bytes are never in the index.

## The turn context

`chat::recall` turns a search into what the agent is told before the person's message: earlier
messages and pages of this chat that match it, each with its source and date, within a character
budget (`BotConfig::recall_chars`, default 900, `0` to turn it off, 4000 at most). Messages already in
front of the agent (the newest 40) are skipped, so nothing is repeated. A line that does not fit is
dropped, never cut in half.

## What it does not do

- **No cross-bot search in the index.** A bot's chat and library are searched on their own. The apps'
  full-history search is a separate query (`store::roster`) that reads this index too.
- **No memory, files or routines in the index.** They are separate stores; adding them is a builder
  each, the same way documents were added.
- **No channel filter yet.** A hit carries its thread; filtering by one is not a query parameter. The
  phone app narrows All, Main chat and Threads on its own.
- **No embeddings.** agentes.chat searches with a model alongside keywords; Codync's host has no
  provider of its own, so this is keyword only.

## Code

`host/src/store/search.rs` (`message_body`, `scrub`, `index_message`, `index_document`, `forget`,
`search`, `search_filtered`, `search_with_tools`, `backfill_search`, `reindex_search`),
`host/src/chat/recall.rs`, the routes in `host/src/api/mod.rs`, the sheets
(`apps/desktop/src/renderer/views/thread/SearchSheet.tsx`,
`apps/ios/Kit/Sources/CodyncUI/Thread/ChatSearchView.swift`,
`apps/android/app/src/main/kotlin/com/codync/android/ConversationSearch.kt`,
`host/src/tui/app/chat_search.rs`) and the tests: what a hit contains, that a page and a message are
two kinds, that a changed page is indexed once, that internal entries, keys and tool traces stay out,
that the backfill resumes, what another chat hides, whose words and which dates a filter keeps, how
recency moves the order, and what a broken query does.

Idea from agentes.chat's search (`packages/engine/src/search.ts`, AGPL-3.0-only): an index of several
kinds of source, each with its own builder, a run's memory assembled within a budget, and search
narrowed by who said it and when. The behavior, not the code; written against Codync's own `entries`
and `documents`.
