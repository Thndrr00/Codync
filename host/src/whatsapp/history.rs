//! What a linked `WhatsApp` number received, kept so an agent can read it back later. Ported from
//! `OpenBot` 0.72.0: a number keeps its messages for 90 days, also the group messages that did not
//! mention it, and unlinking the number deletes them. `OpenBot` keeps the files too; here the text is
//! what a routine summarizes, and a message that arrived with a file keeps its file beside it, so a
//! line that never reached the bot still has what was sent with it.
//!
//! The table is made on first use, indexed by time, and pruned on every write, so it never needs a
//! migration or a background sweep.

use crate::store::Store;
use anyhow::Result;
use rusqlite::params;
use serde_json::{Value, json};
use std::fmt::Write as _;
use std::path::{Component, Path, PathBuf};

/// How long a message is kept.
pub(crate) const KEEP_MS: i64 = 90 * 24 * 60 * 60 * 1000;
/// Most messages one number keeps; the oldest go first.
const MAX_KEPT: i64 = 20_000;

/// One message the number received, as [`remember`] keeps it.
pub(crate) struct Message<'a> {
    pub chat: &'a str,
    pub sender: &'a str,
    pub name: &'a str,
    /// The message's text; empty for a bare attachment.
    pub text: &'a str,
    /// The file the bridge saved for it, when it had one.
    pub file: Option<&'a str>,
    pub at: i64,
}

/// The oldest message kept at `at`.
fn cutoff(at: i64) -> i64 {
    at.saturating_sub(KEEP_MS)
}

/// Makes the table and its index, and adds `file` to a table kept before the column existed.
fn open(c: &rusqlite::Connection) -> Result<()> {
    c.execute_batch(
        "CREATE TABLE IF NOT EXISTS whatsapp_history(
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            chat TEXT NOT NULL,
            sender TEXT NOT NULL,
            name TEXT NOT NULL,
            text TEXT NOT NULL,
            file TEXT,
            at INTEGER NOT NULL);
         CREATE INDEX IF NOT EXISTS whatsapp_history_at ON whatsapp_history(at);",
    )?;
    let file: bool = c.query_row(
        "SELECT EXISTS(SELECT 1 FROM pragma_table_info('whatsapp_history') WHERE name='file')",
        [],
        |row| row.get(0),
    )?;
    if !file {
        c.execute("ALTER TABLE whatsapp_history ADD COLUMN file TEXT", [])?;
    }
    Ok(())
}

/// Keeps one message the number received. One with neither text nor a file is not kept.
pub(crate) fn remember(store: &Store, message: &Message<'_>) -> Result<()> {
    let text = message.text.trim();
    let file = message.file.and_then(relative);
    if (text.is_empty() && file.is_none()) || message.chat.is_empty() {
        return Ok(());
    }
    let pruned = store.with_connection(|c| {
        open(c)?;
        c.execute(
            "INSERT INTO whatsapp_history(chat, sender, name, text, file, at) VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
            params![message.chat, message.sender, message.name, text, file, message.at],
        )?;
        trim(c, message.at)
    })?;
    remove_files(&pruned);
    Ok(())
}

/// Deletes the rows the 90-day limit and the row count remove, answering the files they named.
fn trim(c: &rusqlite::Connection, at: i64) -> Result<Vec<String>> {
    let files = {
        let mut statement = c.prepare(
            "SELECT file FROM whatsapp_history WHERE file IS NOT NULL
             AND (at < ?1 OR id NOT IN (SELECT id FROM whatsapp_history ORDER BY id DESC LIMIT ?2))",
        )?;
        let rows = statement.query_map(params![cutoff(at), MAX_KEPT], |row| row.get::<_, String>(0))?;
        rows.collect::<rusqlite::Result<Vec<String>>>()?
    };
    c.execute(
        "DELETE FROM whatsapp_history WHERE at < ?1
         OR id NOT IN (SELECT id FROM whatsapp_history ORDER BY id DESC LIMIT ?2)",
        params![cutoff(at), MAX_KEPT],
    )?;
    Ok(files)
}

/// The kept messages holding every word of `words` (already lowercased), at or after `since` and at
/// or before `before`, newest first.
pub(crate) fn search(store: &Store, words: &[String], since: i64, before: i64, limit: usize) -> Result<Vec<Value>> {
    store.with_connection(|c| {
        open(c)?;
        let mut sql =
            String::from("SELECT chat, sender, name, text, file, at FROM whatsapp_history WHERE at >= ?1 AND at <= ?2");
        let mut bound: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(since), Box::new(before)];
        for word in words {
            // The word is matched as text: its own `%` and `_` must not become wildcards.
            bound.push(Box::new(format!("%{}%", word.replace(['%', '_'], ""))));
            let _ = write!(sql, " AND lower(text) LIKE ?{}", bound.len());
        }
        let _ = write!(sql, " ORDER BY id DESC LIMIT {limit}");
        let mut statement = c.prepare(&sql)?;
        let refs: Vec<&dyn rusqlite::ToSql> = bound.iter().map(|value| value.as_ref()).collect();
        let rows = statement.query_map(refs.as_slice(), |row| {
            let mut message = json!({
                "chat": row.get::<_, String>(0)?,
                "from": row.get::<_, String>(1)?,
                "name": row.get::<_, String>(2)?,
                "text": row.get::<_, String>(3)?,
                "at": row.get::<_, i64>(5)?,
            });
            if let Some(file) = row.get::<_, Option<String>>(4)? {
                message["file"] = file.into();
            }
            Ok(message)
        })?;
        let mut out = Vec::with_capacity(limit);
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    })
}

/// Deletes everything kept for the number, when it is unlinked, and the files those rows named.
pub(crate) fn forget(store: &Store) -> Result<()> {
    let files = store.with_connection(|c| {
        open(c)?;
        let files = {
            let mut statement = c.prepare("SELECT file FROM whatsapp_history WHERE file IS NOT NULL")?;
            let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
            rows.collect::<rusqlite::Result<Vec<String>>>()?
        };
        c.execute("DELETE FROM whatsapp_history", [])?;
        Ok(files)
    })?;
    remove_files(&files);
    Ok(())
}

/// The file as the table keeps it: relative to the data folder, so a moved data folder still finds
/// it. `None` for a file outside that folder, which the number does not own.
fn relative(path: &str) -> Option<String> {
    let root = crate::service::data_dir();
    relative_to(&root, path).or_else(|| relative_to(&root.canonicalize().ok()?, path))
}

fn relative_to(root: &Path, path: &str) -> Option<String> {
    let rest = Path::new(path).strip_prefix(root).ok()?;
    Some(rest.to_string_lossy().replace('\\', "/"))
}

/// The path a row names, joined to the data folder; `None` when it points outside it.
fn kept_file(root: &Path, file: &str) -> Option<PathBuf> {
    let path = Path::new(file);
    if path.has_root() || path.components().any(|part| part == Component::ParentDir) {
        return None;
    }
    let resolved = root.join(path);
    resolved.starts_with(root).then_some(resolved)
}

/// Deletes the files those rows named. One already gone is not a failure.
fn remove_files(files: &[String]) {
    let root = crate::service::data_dir();
    for file in files {
        let Some(path) = kept_file(&root, file) else { continue };
        if let Err(error) = std::fs::remove_file(&path)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            tracing::warn!(%error, path = %path.display(), "couldn't delete a kept WhatsApp file");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_message_is_kept_for_ninety_days_and_not_a_day_more() {
        let at = 1_800_000_000_000;
        assert_eq!(cutoff(at), at - 90 * 24 * 60 * 60 * 1000);
        assert!(cutoff(at) < at, "the cutoff is in the past");
        assert_eq!(cutoff(i64::MIN), i64::MIN, "a clock before the epoch never overflows");
    }

    /// A file under the media store, as the bridge saves it.
    fn saved_file(name: &str) -> (PathBuf, String) {
        let dir = crate::service::data_dir().join("whatsapp-media");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("{}-{name}", uuid::Uuid::new_v4().simple()));
        std::fs::write(&path, b"jpeg").unwrap();
        let shown = path.to_string_lossy().into_owned();
        (path, shown)
    }

    fn message<'a>(chat: &'a str, text: &'a str, file: Option<&'a str>, at: i64) -> Message<'a> {
        Message { chat, sender: "sender", name: "Name", text, file, at }
    }

    #[test]
    fn a_file_is_kept_beside_the_text_that_arrived_with_it() {
        let store = crate::store::tests::temp_store();
        let (path, shown) = saved_file("photo.jpg");
        remember(&store, &message("chat", "", Some(shown.as_str()), 1_800_000_000_000)).unwrap();
        let kept = search(&store, &[], 0, i64::MAX, 10).unwrap();
        assert_eq!(kept.len(), 1, "a message with only a file is kept");
        assert_eq!(kept[0]["text"], "");
        assert_eq!(kept[0]["file"], format!("whatsapp-media/{}", path.file_name().unwrap().to_string_lossy()));
    }

    #[test]
    fn a_message_with_neither_text_nor_a_file_is_not_kept() {
        let store = crate::store::tests::temp_store();
        remember(&store, &message("chat", "   ", None, 1)).unwrap();
        assert!(search(&store, &[], 0, i64::MAX, 10).unwrap().is_empty());
    }

    #[test]
    fn a_table_kept_before_files_existed_gets_its_column() {
        let store = crate::store::tests::temp_store();
        store
            .with_connection(|c| {
                c.execute_batch(
                    "CREATE TABLE whatsapp_history(
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        chat TEXT NOT NULL,
                        sender TEXT NOT NULL,
                        name TEXT NOT NULL,
                        text TEXT NOT NULL,
                        at INTEGER NOT NULL);",
                )?;
                Ok(())
            })
            .unwrap();
        remember(&store, &message("chat", "hola", None, 2)).unwrap();
        let kept = search(&store, &[], 0, i64::MAX, 10).unwrap();
        assert_eq!(kept.len(), 1);
        assert!(kept[0].get("file").is_none(), "a message without a file has no field");
    }

    #[test]
    fn a_message_past_ninety_days_takes_its_file_with_it() {
        let store = crate::store::tests::temp_store();
        let (old, old_shown) = saved_file("old.jpg");
        let (recent, recent_shown) = saved_file("recent.jpg");
        let now = 1_800_000_000_000;
        remember(&store, &message("chat", "old", Some(old_shown.as_str()), now - KEEP_MS - 1)).unwrap();
        remember(&store, &message("chat", "recent", Some(recent_shown.as_str()), now)).unwrap();
        assert!(!old.exists(), "the trimmed row's file goes with it");
        assert!(recent.exists(), "the kept row's file stays");
        let kept = search(&store, &[], 0, i64::MAX, 10).unwrap();
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0]["text"], "recent");
    }

    #[test]
    fn unlinking_the_number_deletes_the_files_it_kept() {
        let store = crate::store::tests::temp_store();
        let (path, shown) = saved_file("photo.jpg");
        remember(&store, &message("chat", "hola", Some(shown.as_str()), 1_800_000_000_000)).unwrap();
        forget(&store).unwrap();
        assert!(!path.exists());
        assert!(search(&store, &[], 0, i64::MAX, 10).unwrap().is_empty());
    }

    #[test]
    fn a_kept_file_is_named_from_the_data_folder_and_never_escapes_it() {
        let root = Path::new("/data/codync");
        assert_eq!(relative_to(root, "/data/codync/whatsapp-media/a.jpg").as_deref(), Some("whatsapp-media/a.jpg"));
        assert_eq!(relative_to(root, "/elsewhere/a.jpg"), None, "a file the number doesn't own");
        assert_eq!(kept_file(root, "whatsapp-media/a.jpg"), Some(root.join("whatsapp-media/a.jpg")));
        assert_eq!(kept_file(root, "../a.jpg"), None, "a row can't point outside the folder");
        assert_eq!(kept_file(root, "/etc/passwd"), None);
    }
}
