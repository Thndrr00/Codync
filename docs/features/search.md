# Search

Ask a chat what it said about some words, and get the message back with a snippet: that is the whole
feature. It is the host's (`host/src/store/search.rs`), so every client gets the same answers.

## How it works

- An FTS5 table (`entries_fts`) holds the text of each entry: the user's or the agent's `text`, and a
  tool's `title` or `detail`. Entries with none of those are not indexed.
- Every entry is indexed as it is written (`insert_entry`, `insert_entry_once`), so a search finds a
  message the moment it exists.
- A database written before search existed is backfilled on its first search, once. No manual step.
- `search {botId, query, limit?}` returns `{hits: [{entryId, at, snippet}]}`, best match first. The
  snippet marks the match with `[...]` and elides the rest.
- Every word must appear: a query is a list of quoted terms joined by space, so `deploy api` finds
  messages with both. A stray quote, `AND` or `*` is treated as a word instead of failing.

## What it does not do

- **No cross-chat search.** A bot's chat is searched on its own; there is no "search everything".
- **No documents or memory.** They are separate stores with their own text; indexing them is the
  next step of the same idea.
- **No ranking beyond FTS5's `rank`** (BM25 with the column's default weights). Recency weighting is
  not applied yet.

## Code

`host/src/store/search.rs` (`body`, `index`, `search`, `reindex_search`), the route in
`host/src/api/mod.rs`, and the tests that cover what a hit contains, what another chat hides, and
what a broken query does.

Idea from agentes.chat's search and context (AGPL-3.0-only): the behavior, not the code. Nothing here
is copied; it is written against Codync's own `entries` table.
