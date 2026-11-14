//! One read-only question to a database the person chose for a bot. The connection is opened
//! read-only and `query_only`, so nothing it runs can write even if a statement got past the check:
//! the check is what tells the bot which statements are allowed, the connection is what keeps the
//! promise. The row cap and the deadline bound the work a question may do.
//!
//! Idea from agentes.chat's read-only database connector (`packages/engine`, AGPL-3.0-only): the
//! behavior, not the code.

use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OpenFlags};
use serde_json::{Value, json};
use std::path::Path;
use std::time::{Duration, Instant};

/// Rows one question may return.
pub const MAX_ROWS: usize = 500;
/// Columns one question may return.
pub const MAX_COLUMNS: usize = 40;
/// How long one question may run.
pub const TIMEOUT: Duration = Duration::from_secs(5);

/// The words that write, attach a file or change the connection. Refused anywhere outside a string
/// literal, even inside a CTE.
const WRITES: [&str; 14] = [
    "INSERT",
    "UPDATE",
    "DELETE",
    "REPLACE",
    "DROP",
    "ALTER",
    "CREATE",
    "TRUNCATE",
    "ATTACH",
    "DETACH",
    "PRAGMA",
    "VACUUM",
    "REINDEX",
    "SAVEPOINT",
];

/// Whether `sql` is one read-only statement: a single `SELECT` or `WITH`, with no word that writes.
pub fn is_read_only(sql: &str) -> bool {
    let bare = bare(sql);
    let statement = bare.trim().trim_end_matches(';').trim();
    if statement.is_empty() || statement.contains(';') {
        return false;
    }
    let first = statement.split_whitespace().next().unwrap_or_default().to_ascii_uppercase();
    if first != "SELECT" && first != "WITH" {
        return false;
    }
    let upper = statement.to_ascii_uppercase();
    !WRITES.iter().any(|word| keyword(&upper, word))
}

/// Runs one read-only statement against the SQLite file at `path`: `{columns, rows, truncated}`.
pub fn query(path: &Path, sql: &str) -> Result<Value> {
    if !is_read_only(sql) {
        bail!("only one read-only statement is allowed (SELECT or WITH)");
    }
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("opening {}", path.display()))?;
    connection.pragma_update(None, "query_only", true)?;
    let deadline = Instant::now() + TIMEOUT;
    connection.progress_handler(10_000, Some(move || Instant::now() > deadline))?;
    let mut statement = connection.prepare(sql).context("preparing the statement")?;
    if statement.column_count() > MAX_COLUMNS {
        bail!("the question returns more than {MAX_COLUMNS} columns");
    }
    let columns: Vec<String> = statement.column_names().into_iter().map(str::to_owned).collect();
    let mut rows = statement.query([])?;
    let mut values = Vec::new();
    let mut truncated = false;
    while let Some(row) = rows.next()? {
        if values.len() == MAX_ROWS {
            truncated = true;
            break;
        }
        let mut item = Vec::with_capacity(columns.len());
        for index in 0..columns.len() {
            item.push(cell(row, index)?);
        }
        values.push(Value::Array(item));
    }
    Ok(json!({"columns": columns, "rows": values, "truncated": truncated}))
}

/// The `query_database` tool: one question to the database this bot was given.
pub fn call(hub: &crate::hub::Hub, bot: &str, args: &Value) -> Result<Value> {
    hub.store.bot(bot)?.filter(|row| !row.deleted).context("unknown bot")?;
    let path = database(hub, bot).context("this bot has no database: the person picks one first")?;
    let sql = args["sql"].as_str().context("`sql` is required")?;
    query(Path::new(&path), sql)
}

/// The database this bot may read, when the person picked one.
pub fn database(hub: &crate::hub::Hub, bot: &str) -> Option<String> {
    hub.store.kv_get(&key(bot)).filter(|path| !path.trim().is_empty())
}

/// Picks (`Some`) or clears (`None`) the database a bot may read. It must be a file on this computer.
pub fn set_database(store: &crate::store::Store, bot: &str, path: Option<&str>) -> Result<()> {
    match path.map(str::trim).filter(|path| !path.is_empty()) {
        Some(path) => {
            if !Path::new(path).is_file() {
                bail!("the database must be a file on this computer: {path}");
            }
            store.kv_set(&key(bot), path)
        }
        None => store.kv_delete(&key(bot)),
    }
}

fn key(bot: &str) -> String {
    format!("database.{bot}")
}

/// The tool's definition, for the chat MCP server.
pub fn tool() -> Value {
    json!({
        "name": "query_database",
        "description": "Ask one read-only question of the database the person gave this bot. One SELECT or WITH \
                        statement, at most 500 rows and 5 seconds. Anything that writes is refused, and the \
                        connection cannot write even if a statement tried.",
        "inputSchema": {"type": "object", "properties": {"sql": {"type": "string"}}, "required": ["sql"]},
        "annotations": {"readOnlyHint": true}
    })
}

/// `sql` with its string literals and quoted names blanked, so a word inside them is not a keyword.
fn bare(sql: &str) -> String {
    let mut out = String::with_capacity(sql.len());
    let mut quote: Option<char> = None;
    let mut chars = sql.chars().peekable();
    while let Some(c) = chars.next() {
        match quote {
            Some(open) => {
                if c == open {
                    if chars.peek() == Some(&open) {
                        chars.next();
                    } else {
                        quote = None;
                    }
                }
                out.push(' ');
            }
            None => {
                if matches!(c, '\'' | '"' | '`') {
                    quote = Some(c);
                    out.push(' ');
                } else {
                    out.push(c);
                }
            }
        }
    }
    out
}

/// Whether `word` appears in `haystack` as a whole word.
fn keyword(haystack: &str, word: &str) -> bool {
    let bytes = haystack.as_bytes();
    let mut from = 0;
    while let Some(at) = haystack[from..].find(word) {
        let start = from + at;
        let end = start + word.len();
        let before = start == 0 || !is_name(bytes[start - 1]);
        let after = end >= bytes.len() || !is_name(bytes[end]);
        if before && after {
            return true;
        }
        from = end;
    }
    false
}

fn is_name(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// One cell as JSON: numbers and text as they are, a blob as its size.
fn cell(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<Value> {
    Ok(match row.get_ref(index)? {
        rusqlite::types::ValueRef::Null => Value::Null,
        rusqlite::types::ValueRef::Integer(number) => json!(number),
        rusqlite::types::ValueRef::Real(number) => json!(number),
        rusqlite::types::ValueRef::Text(text) => Value::String(String::from_utf8_lossy(text).into_owned()),
        rusqlite::types::ValueRef::Blob(bytes) => json!(format!("<{} bytes>", bytes.len())),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_one_read_statement_is_allowed() {
        assert!(is_read_only("SELECT * FROM people"));
        assert!(is_read_only("  select count(*) from people;  "));
        assert!(is_read_only("SELECT 1; "), "a trailing semicolon is not a second statement");
        assert!(is_read_only("WITH t AS (SELECT 1) SELECT * FROM t"));
        assert!(!is_read_only("SELECT 1; DROP TABLE people"));
        assert!(!is_read_only("DELETE FROM people"));
        assert!(!is_read_only("PRAGMA journal_mode=WAL"));
        assert!(!is_read_only(""));
    }

    #[test]
    fn a_word_inside_a_string_is_not_a_keyword() {
        assert!(is_read_only("SELECT * FROM log WHERE action = 'delete'"));
        assert!(is_read_only("SELECT 'a;b' AS text"));
    }

    #[test]
    fn a_cte_that_writes_is_refused() {
        assert!(!is_read_only("WITH x AS (DELETE FROM people RETURNING *) SELECT * FROM x"));
    }

    #[test]
    fn a_question_returns_its_rows_and_refuses_a_write() {
        let path = std::env::temp_dir().join(format!("codync-sql-{}.db", uuid::Uuid::new_v4()));
        {
            let connection = Connection::open(&path).unwrap();
            connection
                .execute_batch(
                    "CREATE TABLE people(name TEXT, age INTEGER);
                     INSERT INTO people VALUES('Ana', 30), ('Luis', 41);",
                )
                .unwrap();
        }
        let answer = query(&path, "SELECT name, age FROM people ORDER BY age").unwrap();
        assert_eq!(answer["columns"], json!(["name", "age"]));
        assert_eq!(answer["rows"], json!([["Ana", 30], ["Luis", 41]]));
        assert_eq!(answer["truncated"], false);
        assert!(query(&path, "INSERT INTO people VALUES('Eve', 1)").is_err(), "a write never runs");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_question_that_returns_too_many_rows_says_so() {
        let path = std::env::temp_dir().join(format!("codync-sql-{}.db", uuid::Uuid::new_v4()));
        {
            let connection = Connection::open(&path).unwrap();
            connection.execute_batch("CREATE TABLE n(x INTEGER);").unwrap();
            let mut statement = connection.prepare("INSERT INTO n(x) VALUES(?1)").unwrap();
            for value in 0..(MAX_ROWS + 10) {
                statement.execute([i64::try_from(value).unwrap()]).unwrap();
            }
        }
        let answer = query(&path, "SELECT x FROM n").unwrap();
        assert_eq!(answer["rows"].as_array().unwrap().len(), MAX_ROWS);
        assert_eq!(answer["truncated"], true);
        let _ = std::fs::remove_file(&path);
    }
}
