//! What a linked `WhatsApp` number received, kept so an agent can read it back later. Ported from
//! `OpenBot` 0.72.0: a number keeps its messages for 90 days, also the group messages that did not
//! mention it, and unlinking the number deletes them. `OpenBot` keeps the files too; here the text is
//! what a routine summarizes, and a message that arrived with a file keeps its file beside it, so a
//! line that never reached the bot still has what was sent with it.
//!
//! The table is made on first use, indexed by time, and pruned on every write, so it never needs a
//! migration or a background sweep. Telegram reuses exactly this store (the same retention, files and
//! trim); only the table, media folder and source label differ ([`Channel`]). WhatsApp's own
//! resend handling stays WhatsApp-only.

use crate::LockExt;
use crate::store::Store;
use anyhow::Result;
use rusqlite::params;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;

/// The channel a kept message came from. The two channels share this store; only these three names
/// differ, so a WhatsApp row can never land in Telegram's table or the other way around.
#[derive(Clone, Copy)]
pub(crate) enum Channel {
    WhatsApp,
    Telegram,
}

impl Channel {
    fn table(self) -> &'static str {
        match self {
            Self::WhatsApp => "whatsapp_history",
            Self::Telegram => "telegram_history",
        }
    }

    /// The folder under the data directory where this channel's kept files live.
    pub(crate) fn media(self) -> &'static str {
        match self {
            Self::WhatsApp => "whatsapp-media",
            Self::Telegram => "telegram-media",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::WhatsApp => "WhatsApp",
            Self::Telegram => "Telegram",
        }
    }
}

/// How long a message is kept.
pub(crate) const KEEP_MS: i64 = 90 * 24 * 60 * 60 * 1000;
/// Most messages one channel keeps; the oldest go first.
pub(crate) const MAX_KEPT: i64 = 20_000;
/// Most media one number keeps, in bytes. Beyond this the oldest files go first (their messages keep
/// their text, just no file path), so one number can't fill the data folder.
pub(crate) const MAX_MEDIA_BYTES: i64 = 2 * 1024 * 1024 * 1024;

/// Pure budget decision: how many of the oldest files must go for their total to fit under `cap`.
/// `sizes` is oldest first; a file bigger than the cap on its own is dropped after every older one.
fn media_to_prune(sizes: &[i64], cap: i64) -> usize {
    // i128 keeps the sum exact even for absurd file lengths, so one huge file can't make the rest look free.
    let total: i128 = sizes.iter().map(|&size| i128::from(size)).sum();
    let mut over = total - i128::from(cap);
    if over <= 0 {
        return 0;
    }
    let mut count = 0;
    for &size in sizes {
        if over <= 0 {
            break;
        }
        over -= i128::from(size);
        count += 1;
    }
    count
}

/// How long a message WhatsApp sends again after a newly linked number couldn't read it at first is
/// still treated as its first copy (OpenBot 0.70.0), not as an ordinary duplicate.
pub(crate) const RESEND_WINDOW_MS: i64 = 10 * 60 * 1000;

/// The first copy of each message id (chat and id), with when it arrived and whether it left a row.
/// Kept in memory: a resend follows its failed first copy within minutes, and a restart needn't
/// remember one from before.
static FIRST_COPIES: Mutex<HashMap<String, (i64, bool)>> = Mutex::new(HashMap::new());

/// Pure decision for a copy of a message. `first` is the first copy's time and whether it left a
/// row; `at` is this copy's time. A copy that arrives within the window of a first copy that left no
/// row is kept (WhatsApp sends an undecryptable group message again); every other repeat is dropped.
pub(crate) fn keep_copy(first: Option<(i64, bool)>, at: i64) -> bool {
    match first {
        None => true,
        Some((_, true)) => false,
        Some((when, false)) => at.saturating_sub(when) <= RESEND_WINDOW_MS,
    }
}

fn copy_key(chat: &str, id: &str) -> Option<String> {
    (!chat.is_empty() && !id.is_empty()).then(|| format!("{chat}\0{id}"))
}

/// Notes that a message arrived with nothing a history row could hold (it couldn't be decrypted
/// yet). WhatsApp sends it again once the number can read it; that copy must reach the history and
/// the orchestrator instead of being dropped as a duplicate.
pub(crate) fn note_unreadable(chat: &str, id: &str, at: i64) {
    let Some(key) = copy_key(chat, id) else { return };
    let mut copies = FIRST_COPIES.locked();
    copies.retain(|_, (when, _)| at.saturating_sub(*when) <= RESEND_WINDOW_MS);
    copies.entry(key).or_insert((at, false));
}

/// One message the number received, as [`remember`] keeps it.
pub(crate) struct Message<'a> {
    /// The bridge's message id, used only to tell a resend from an ordinary duplicate.
    pub id: String,
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
/// `table` only ever comes from [`Channel`], so it cannot be SQL from a caller.
fn open(c: &rusqlite::Connection, channel: Channel) -> Result<()> {
    let table = channel.table();
    c.execute_batch(&format!(
        "CREATE TABLE IF NOT EXISTS {table}(
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            chat TEXT NOT NULL,
            sender TEXT NOT NULL,
            name TEXT NOT NULL,
            text TEXT NOT NULL,
            file TEXT,
            at INTEGER NOT NULL);
         CREATE INDEX IF NOT EXISTS {table}_at ON {table}(at);"
    ))?;
    let file: bool = c.query_row(
        &format!("SELECT EXISTS(SELECT 1 FROM pragma_table_info('{table}') WHERE name='file')"),
        [],
        |row| row.get(0),
    )?;
    if !file {
        c.execute(&format!("ALTER TABLE {table} ADD COLUMN file TEXT"), [])?;
    }
    Ok(())
}

/// Keeps one message the number received. One with neither text nor a file is not kept. Returns
/// whether a row was kept, so an ordinary duplicate can stop before it reaches a bot again.
pub(crate) fn remember(store: &Store, message: &Message<'_>) -> Result<bool> {
    remember_in(Channel::WhatsApp, store, message)
}

/// Keeps one message for `channel`. The WhatsApp resend handling only applies when an id is given,
/// so Telegram (which passes an empty id, having no such resends) gets the same store without it.
pub(crate) fn remember_in(channel: Channel, store: &Store, message: &Message<'_>) -> Result<bool> {
    let text = message.text.trim();
    let file = message.file.and_then(relative);
    if (text.is_empty() && file.is_none()) || message.chat.is_empty() {
        return Ok(false);
    }
    let first = copy_key(message.chat, &message.id).and_then(|key| {
        let mut copies = FIRST_COPIES.locked();
        copies.retain(|_, (when, _)| message.at.saturating_sub(*when) <= RESEND_WINDOW_MS);
        let first = copies.get(&key).copied();
        if keep_copy(first, message.at) {
            copies.insert(key, (message.at, true));
            true
        } else {
            false
        }
    });
    if first == Some(false) {
        return Ok(false);
    }
    let table = channel.table();
    let pruned = store.with_connection(|c| {
        open(c, channel)?;
        c.execute(
            &format!("INSERT INTO {table}(chat, sender, name, text, file, at) VALUES(?1, ?2, ?3, ?4, ?5, ?6)"),
            params![message.chat, message.sender, message.name, text, file, message.at],
        )?;
        trim(c, channel, message.at)
    })?;
    remove_files(channel, &pruned);
    Ok(true)
}

/// Deletes the rows the 90-day limit and the row count remove, and the oldest files past the media
/// cap, answering the files the caller must delete. Rows trimmed for media keep their text: only their
/// `file` path is cleared, so no row still needs a file that is about to go.
fn trim(c: &rusqlite::Connection, channel: Channel, at: i64) -> Result<Vec<String>> {
    let table = channel.table();
    let mut files = {
        let mut statement = c.prepare(&format!(
            "SELECT file FROM {table} WHERE file IS NOT NULL
             AND (at < ?1 OR id NOT IN (SELECT id FROM {table} ORDER BY id DESC LIMIT ?2))"
        ))?;
        let rows = statement.query_map(params![cutoff(at), MAX_KEPT], |row| row.get::<_, String>(0))?;
        rows.collect::<rusqlite::Result<Vec<String>>>()?
    };
    c.execute(
        &format!(
            "DELETE FROM {table} WHERE at < ?1
             OR id NOT IN (SELECT id FROM {table} ORDER BY id DESC LIMIT ?2)"
        ),
        params![cutoff(at), MAX_KEPT],
    )?;
    files.extend(trim_media(c, channel, MAX_MEDIA_BYTES)?);
    Ok(files)
}

/// Clears the `file` path of the oldest rows whose files push the channel over `cap`, oldest first,
/// and answers those paths for `remove_files`. A file already gone has size zero and is never picked.
fn trim_media(c: &rusqlite::Connection, channel: Channel, cap: i64) -> Result<Vec<String>> {
    let table = channel.table();
    let root = crate::service::data_dir();
    let rows: Vec<(i64, String, i64)> = {
        let mut statement =
            c.prepare(&format!("SELECT id, file FROM {table} WHERE file IS NOT NULL ORDER BY id ASC"))?;
        let found = statement.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)))?;
        let mut kept = Vec::new();
        for row in found {
            let (id, file) = row?;
            let size = kept_file(&root, &file)
                .and_then(|path| std::fs::metadata(path).ok())
                .map_or(0, |meta| meta.len() as i64);
            if size > 0 {
                kept.push((id, file, size));
            }
        }
        kept
    };
    let sizes: Vec<i64> = rows.iter().map(|row| row.2).collect();
    let mut files = Vec::new();
    for (id, file, _) in rows.iter().take(media_to_prune(&sizes, cap)) {
        // Clear every row naming this path before the caller deletes the file, so no row needs it.
        c.execute(&format!("UPDATE {table} SET file = NULL WHERE id = ?1"), params![id])?;
        c.execute(&format!("UPDATE {table} SET file = NULL WHERE file = ?1"), params![file])?;
        if !files.contains(file) {
            files.push(file.clone());
        }
    }
    Ok(files)
}

/// The kept messages holding every word of `words` (already lowercased), at or after `since` and at
/// or before `before`, newest first.
pub(crate) fn search(store: &Store, words: &[String], since: i64, before: i64, limit: usize) -> Result<Vec<Value>> {
    search_in(Channel::WhatsApp, store, words, since, before, limit)
}

/// The kept messages for `channel` holding every word of `words`, newest first.
pub(crate) fn search_in(
    channel: Channel,
    store: &Store,
    words: &[String],
    since: i64,
    before: i64,
    limit: usize,
) -> Result<Vec<Value>> {
    store.with_connection(|c| {
        open(c, channel)?;
        let mut sql =
            format!("SELECT chat, sender, name, text, file, at FROM {} WHERE at >= ?1 AND at <= ?2", channel.table());
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
    forget_in(Channel::WhatsApp, store)
}

/// Deletes everything kept for `channel`, and the files those rows named.
pub(crate) fn forget_in(channel: Channel, store: &Store) -> Result<()> {
    let table = channel.table();
    let files = store.with_connection(|c| {
        open(c, channel)?;
        let files = {
            let mut statement = c.prepare(&format!("SELECT file FROM {table} WHERE file IS NOT NULL"))?;
            let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
            rows.collect::<rusqlite::Result<Vec<String>>>()?
        };
        c.execute(&format!("DELETE FROM {table}"), [])?;
        Ok(files)
    })?;
    remove_files(channel, &files);
    Ok(())
}

/// Deletes the rows of one connector's chats, when it is disconnected, and their files. A row's
/// `chat` is stored as `connector:chat-id`; connector names only hold a-z, 0-9 and '-', so the
/// `LIKE` pattern cannot match another connector and has no wildcard of its own.
pub(crate) fn forget_prefix(store: &Store, channel: Channel, connector: &str) -> Result<()> {
    let table = channel.table();
    let pattern = format!("{connector}:%");
    let files = store.with_connection(|c| {
        open(c, channel)?;
        let files = {
            let mut statement =
                c.prepare(&format!("SELECT file FROM {table} WHERE file IS NOT NULL AND (chat = ?1 OR chat LIKE ?2)"))?;
            let rows = statement.query_map(params![connector, pattern], |row| row.get::<_, String>(0))?;
            rows.collect::<rusqlite::Result<Vec<String>>>()?
        };
        c.execute(&format!("DELETE FROM {table} WHERE chat = ?1 OR chat LIKE ?2"), params![connector, pattern])?;
        Ok(files)
    })?;
    remove_files(channel, &files);
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
fn remove_files(channel: Channel, files: &[String]) {
    let root = crate::service::data_dir();
    for file in files {
        let Some(path) = kept_file(&root, file) else { continue };
        if let Err(error) = std::fs::remove_file(&path)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            tracing::warn!(%error, path = %path.display(), "couldn't delete a kept {} file", channel.label());
        }
    }
}

/// Deletes the file `save_media` wrote for a queued message that is dropped before history keeps its
/// row. The path is the absolute one saved in `_codyncPath`; `relative` keeps it inside the data
/// folder, so the same traversal guard and NotFound handling as a row's file apply.
pub(crate) fn remove_queued_file(path: &str) {
    if let Some(relative) = relative(path) {
        remove_files(Channel::WhatsApp, &[relative]);
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
        Message { id: uuid::Uuid::new_v4().to_string(), chat, sender: "sender", name: "Name", text, file, at }
    }

    #[test]
    fn a_resend_of_an_unreadable_message_is_kept_but_ordinary_duplicates_are_not() {
        let at = 1_800_000_000_000;
        assert!(keep_copy(None, at), "a first copy is kept");
        assert!(!keep_copy(Some((at - 1_000, true)), at), "an ordinary repeat is a duplicate");
        assert!(keep_copy(Some((at - 1_000, false)), at), "the readable resend takes the failed first copy's place");
        assert!(
            !keep_copy(Some((at - RESEND_WINDOW_MS - 1, false)), at),
            "past the window the resend is a duplicate too"
        );
    }

    #[test]
    fn the_readable_copy_of_a_message_that_left_no_row_is_saved_once() {
        let store = crate::store::tests::temp_store();
        let at = 1_800_000_000_000;
        note_unreadable("group", "id-1", at);
        let resent = Message {
            id: "id-1".to_owned(),
            chat: "group",
            sender: "sender",
            name: "Name",
            text: "hola",
            file: None,
            at: at + 1_000,
        };
        assert!(remember(&store, &resent).expect("the resend is saved"));
        assert_eq!(search(&store, &[], 0, i64::MAX, 10).unwrap().len(), 1);
        assert!(!remember(&store, &resent).expect("a normal repeat is dropped"));
        assert_eq!(search(&store, &[], 0, i64::MAX, 10).unwrap().len(), 1, "only one row is kept");
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
    fn the_media_budget_drops_only_as_many_oldest_files_as_it_needs() {
        assert_eq!(media_to_prune(&[], 4), 0, "nothing kept fits any budget");
        assert_eq!(media_to_prune(&[1, 2, 3], 6), 0, "at the cap nothing goes");
        assert_eq!(media_to_prune(&[1, 2, 3], 5), 1, "one oldest file pays for the overrun");
        assert_eq!(media_to_prune(&[1, 2, 3], 1), 3, "every older file goes before the newest");
        assert_eq!(media_to_prune(&[4, 1, 1], 4), 1, "the oldest file alone covers the overrun");
        assert_eq!(media_to_prune(&[i64::MAX, i64::MAX], MAX_MEDIA_BYTES), 2, "sizes can't overflow the sum");
    }

    #[test]
    fn files_over_the_media_cap_are_pruned_oldest_first_and_leave_their_messages() {
        let store = crate::store::tests::temp_store();
        let (old, old_shown) = saved_file("old.jpg");
        let (recent, recent_shown) = saved_file("recent.jpg");
        let now = 1_800_000_000_000;
        remember(&store, &message("chat", "old", Some(old_shown.as_str()), now)).unwrap();
        remember(&store, &message("chat", "recent", Some(recent_shown.as_str()), now + 1)).unwrap();

        // Two four-byte files with a six-byte cap: the older one goes, its row keeps the text.
        let pruned = store.with_connection(|c| trim_media(c, Channel::WhatsApp, 6)).unwrap();
        assert_eq!(pruned.len(), 1);
        remove_files(Channel::WhatsApp, &pruned);
        assert!(!old.exists(), "the oldest file over budget goes");
        assert!(recent.exists(), "the kept file stays");

        let kept = search(&store, &[], 0, i64::MAX, 10).unwrap();
        assert_eq!(kept.len(), 2, "pruning media doesn't delete messages");
        let old_row = kept.iter().find(|row| row["text"] == "old").unwrap();
        let recent_row = kept.iter().find(|row| row["text"] == "recent").unwrap();
        assert!(old_row.get("file").is_none(), "no row still needs the deleted file");
        assert!(recent_row.get("file").is_some(), "the other row keeps its file");
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

    #[test]
    fn a_queued_file_is_deleted_when_its_message_never_reaches_history() {
        let (path, shown) = saved_file("queued.jpg");
        remove_queued_file(&shown);
        assert!(!path.exists(), "a dropped queued message takes its file with it");
        remove_queued_file(&shown);
        // Removing a file already gone is fine; only other errors are logged.
    }
}
