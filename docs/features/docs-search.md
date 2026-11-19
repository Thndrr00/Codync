# Documentation and code search

Codync's host indexes a bot's repository and serves it to the bot over MCP. The bot can search the
project's documentation and source, then read the exact lines it found. Nothing leaves the computer:
the index is built from the bot's working folder (or a folder its owner names) and is bounded in
files, bytes and time.

## Tools

The `codync-docs` MCP server is registered for every bot the way `chat`, `team` and `memory` are: the
host starts `codync-host mcp docs --bot <id>` and each tool call goes through the authenticated loopback
API (`docsCall`). Three tools:

- `docs_search({query, limit?})` searches documentation (Markdown, MDX, text, reStructuredText,
  AsciiDoc, and files under a `docs/` folder). Every result has `path`, `line`, `snippet`, `score` and
  `citation` (`path:line`).
- `code_search({query, limit?})` searches every indexed file, source included.
- `docs_read({path, start?, end?})` returns numbered lines of one indexed file, capped at 400 lines and
  60,000 characters, with a `citation`.

All query terms must appear in a document or its path. A path match ranks higher than a body match,
and a heading gets a small bonus, so `docs_search("authentication token")` returns the auth page
rather than an incidental mention. Results are deterministic and capped at 20.

## Indexing

The root is the bot's `cwd`, unless the bot's settings name another folder for it (desktop, iPhone and
Android show **Documentation search** with the indexed folder, its files and size; `setDocsFolder({botId, path})`
does the same, and an empty path clears it). `docsStatus({botId})` reports the folder, file count, bytes,
build time and whether a limit truncated the walk.

- `.gitignore` files are read per directory, including nested ones. A rule without a slash matches any
  path segment; a rule with a slash is anchored to its own directory; `!` re-includes and the last
  match wins. Skipped directories always win over an ignore rule.
- Hidden files and folders are skipped, as are `node_modules`, `target`, `build`, `dist`, `.next`,
  `vendor`, virtual environments, `DerivedData`, `Pods` and other generated trees.
- Files are read only when they are UTF-8 text without NUL bytes, at most 400 KiB each. The walk stops
  at 4,000 files or 24 MiB of text and marks the index `truncated`.
- The index lives in memory for ten minutes and is cached as JSON under the host's data directory
  (`docs-index/<digest>.json`, at most 16 MiB), so a restart does not walk the repository again.

Only paths that are in the index can be read: `docs_read` never touches the filesystem, so a path such
as `../.env` is simply not found. `.env` and every other hidden file are never indexed in the first
place.

## Why not package Pathfinder

`CopilotKit/pathfinder` was read as the pattern for these tools, but it is not packaged. Its license
is **Elastic License 2.0 with an Additional Use Grant** (verified in the repository's `LICENSING.md`
and the npm package, which declares `Elastic-2.0`). That is a source-available license, not an OSI
open-source one: it forbids providing Pathfinder itself as a hosted or managed service and requires
keeping its notices, so Codync cannot vendor or redistribute it as part of its own binary without
separate permission.

Pathfinder also assumes more than Codync needs: a PostgreSQL database with pgvector and an embedding
provider (OpenAI, Ollama or transformers.js) to answer semantic search. Codync's bots need exact
file-and-line citations from a repository on the same computer, with no database, no API key and no
network call. A bounded local index is smaller, auditable, works offline and ships under Codync's own
license, so the tools are Codync's own implementation of the pattern.

Teams, by contrast, is a transport question rather than a licensing one: see
[the Matrix channel](matrix-channel.md#teams).
