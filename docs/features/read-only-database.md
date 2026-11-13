# A database a bot may only read

A bot can answer questions about the person's data without ever being able to change it. The person
picks one SQLite file for the bot (`setBotDatabase {botId, path}`, a file on the computer that runs
it; `path: null` clears it, and `botDatabase {botId}` reads it back); the bot gets one tool,
`query_database {sql}`, and nothing else.

## What a question may be

- **One statement.** A single `SELECT` or `WITH`. A second statement after a semicolon is refused, and
  so is anything that doesn't start with those two words.
- **No writing.** `INSERT`, `UPDATE`, `DELETE`, `REPLACE`, `DROP`, `ALTER`, `CREATE`, `TRUNCATE`,
  `ATTACH`, `DETACH`, `PRAGMA`, `VACUUM`, `REINDEX` and `SAVEPOINT` are refused anywhere outside a
  string literal, even inside a CTE. A word inside a quoted value (`WHERE action = 'delete'`) is not a
  keyword.
- **Bounded.** At most 500 rows, 40 columns and 5 seconds; a question that returns more rows says
  `truncated: true` instead of growing.

## Why it cannot write even if the check is wrong

The check tells the bot what it may ask; the connection is what keeps the promise. It is opened with
`SQLITE_OPEN_READ_ONLY` and `query_only` is set, so a statement that slipped past the check still
cannot write, attach a file or change the connection.

## What the bot gets back

`{columns: [...], rows: [[...]], truncated: bool}`. Numbers and text come as they are, `NULL` as
`null`, and a blob as `"<N bytes>"`. The answer is the tool's result, so the bot shows it as a table
in its reply.

Code: `host/src/sql.rs` (`is_read_only`, `query`, `call`, `tool`), the `botDatabase` and
`setBotDatabase` methods in `host/src/api/bots.rs`, and the setting kept per bot in the store's
`kv` (`database.<bot>`).

Idea from agentes.chat's read-only database connector (`packages/engine`, AGPL-3.0-only): the
behavior, not the code.

## Not here yet

Only SQLite files. A server database (Postgres, MySQL) needs a driver this host doesn't link, and a
sign-in to go with it; the guard, the row cap and the deadline are the same.
