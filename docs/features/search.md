# Search

Ask a bot what it knows about some words, and get the source back with a snippet: that is the whole
feature. It is the host's (`host/src/store/search.rs`), so every client gets the same answers.

## How it works

- One FTS5 table (`search_fts`) holds the text of every source, with its **kind**: a `message` (the
  user's or the agent's `text`, a tool's `title` or `detail`) or a `document` (its title and body).
- Sources are indexed as they are written: messages in `insert_entry`/`insert_entry_once`, pages in
  `save_document`. A page that changes is replaced in the index; a page that is deleted leaves it.
- A database written before search existed is backfilled on its first search, once. No manual step.
- `search {botId, query, limit?}` returns `{hits: [{kind, id, at, snippet}]}`, best match first. The
  snippet marks the match with `[...]` and elides the rest.
- Every word must appear: a query is a list of quoted terms joined by space, so `deploy api` finds
  sources with both. A stray quote, `AND` or `*` is treated as a word instead of failing.

## What it does not do

- **No cross-bot search.** A bot's chat and library are searched on their own.
- **No memory, files or routines in the index.** They are separate stores; adding them is the next
  step of the same idea, one builder each.
- **No embeddings.** agentes.chat searches with a model alongside keywords; Codync's host has no
  provider of its own, so this is keyword only.
- **No ranking beyond FTS5's `rank`** (BM25 with the column's default weights). Recency weighting is
  not applied yet.

## Code

`host/src/store/search.rs` (`message_body`, `document_body`, `index_message`, `index_document`,
`forget`, `search`, `reindex_search`), the route in `host/src/api/mod.rs`, and the tests that cover
what a hit contains, that a page and a message are two kinds, that a changed page is indexed once,
what another chat hides, and what a broken query does.

Idea from agentes.chat's search (`packages/engine/src/search.ts`, AGPL-3.0-only): an index of several
kinds of source, each with its own builder. The behavior, not the code; written against Codync's own
`entries` and `documents`.
