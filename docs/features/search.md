# Search

Ask a bot what it knows about some words, and get the source back with a snippet: that is the whole
feature. It is the host's (`host/src/store/search.rs`), so every client gets the same answers.

## How it works

- One FTS5 table (`search_fts`) holds the text of every source, with its **kind**: a `message` (the
  user's or the agent's `text`, a tool's `title` or `detail`) or a `document` (its title and body).
- Sources are indexed as they are written: messages in `insert_entry`/`insert_entry_once`, and again on
  `update_entry` (a message still streaming grows), pages in `save_document`. A page that changes is
  replaced in the index; a page that is deleted leaves it, and so does a deleted message.
- A database written before search existed is backfilled, a couple of thousand entries per pass, from
  a marker in `kv` that moves as it goes: a large history never blocks one search on all of it, and a
  pass that was cut short carries on where it stopped.
- `search {botId, query, limit?, tools?}` returns `{hits: [{kind, id, at, snippet}]}`, best match first.
  The snippet marks the match with `[...]` and elides the rest. Tool traces, thoughts, permission
  prompts and notices are left out unless `tools` is true: what a person searches for is what was said.
  The limit counts what is kept, so a screenful of traces never empties the answer.
- `searchReindex` rebuilds the whole index from the store: the repair for a bad migration or a
  hand-edited database. It answers `{indexed}` and is safe to run twice.
- Every word must appear and each is a prefix, so `deploy api` finds sources with both and `deploy`
  finds `deployment`. A stray quote, `AND` or `*` is treated as a word instead of failing.

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
- **No embeddings.** agentes.chat searches with a model alongside keywords; Codync's host has no
  provider of its own, so this is keyword only.
- **No ranking beyond FTS5's `rank`** (BM25 with the column's default weights). Recency weighting and
  the other filters (bot, channel, author, dates) are not applied yet.

## Code

`host/src/store/search.rs` (`message_body`, `scrub`, `index_message`, `index_document`, `forget`,
`search`, `search_with_tools`, `backfill_search`, `reindex_search`), `host/src/chat/recall.rs`, the
routes in `host/src/api/mod.rs`, and the tests: what a hit contains, that a page and a message are two
kinds, that a changed page is indexed once, that internal entries, keys and tool traces stay out, that
the backfill resumes, what another chat hides, and what a broken query does.

Idea from agentes.chat's search (`packages/engine/src/search.ts`, AGPL-3.0-only): an index of several
kinds of source, each with its own builder, and a run's memory assembled within a budget. The
behavior, not the code; written against Codync's own `entries` and `documents`.
