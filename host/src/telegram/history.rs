//! What Telegram delivered to this computer, kept so an agent can read earlier messages after a
//! restart (OpenBot 0.75.0). This is a thin adapter over the shared channel history store in
//! `whatsapp::history`: the same 90-day and row-count trim, the same data-dir-relative files, and the
//! same disconnect cleanup, with Telegram's own table, media folder and source label. A message that
//! arrived with a file keeps that file beside its text, so a line that never reached the bot still
//! has what was sent with it.

use crate::store::Store;
use crate::whatsapp::history::{self, Channel, KEEP_MS, MAX_KEPT, Message};
use anyhow::Result;
use serde_json::Value;

/// Telegram sometimes leaves a chat's title out of an update. Like WhatsApp's fallback, the chat
/// still reads as "Telegram group" instead of leaving an agent with an empty name.
pub(super) fn place_name<'a>(name: Option<&'a str>, group: bool, sender: &'a str) -> &'a str {
    let named = name.unwrap_or_default().trim();
    if !named.is_empty() {
        return named;
    }
    if group {
        "Telegram group"
    } else {
        let sender = sender.trim();
        if sender.is_empty() { "someone" } else { sender }
    }
}

/// The sender shown for a Telegram message: a person's name, then their handle or numeric id.
pub(super) fn sender(message: &Value) -> String {
    let from = &message["from"];
    let first = from["first_name"].as_str().unwrap_or_default().trim();
    let last = from["last_name"].as_str().unwrap_or_default().trim();
    let full = format!("{first} {last}");
    let full = full.trim();
    if !full.is_empty() {
        return full.to_owned();
    }
    if let Some(username) = from["username"].as_str().filter(|name| !name.trim().is_empty()) {
        return username.trim().to_owned();
    }
    from["id"].as_i64().map_or_else(|| "someone".to_owned(), |id| id.to_string())
}

/// One delivered message to keep, named after the Telegram connector it came through.
pub(super) struct Incoming<'a> {
    pub connector: &'a str,
    /// Telegram's chat id, kept with the connector so a disconnect only removes its own chats.
    pub chat: i64,
    pub group: bool,
    pub sender: &'a str,
    /// A group title, when Telegram sent one.
    pub title: Option<&'a str>,
    pub text: &'a str,
    /// The file that arrived with it, as an absolute path under the data folder.
    pub file: Option<&'a str>,
    pub at: i64,
}

/// Keeps one delivered message before it becomes a bot event. Telegram has no resend copy to
/// reconcile, so the shared store's WhatsApp dedup stays idle (the id is empty here).
pub(super) fn remember(store: &Store, message: &Incoming<'_>) -> Result<bool> {
    let chat = format!("{}:{}", message.connector, message.chat);
    let name = place_name(message.title, message.group, message.sender);
    history::remember_in(
        Channel::Telegram,
        store,
        &Message {
            id: String::new(),
            chat: &chat,
            sender: message.sender,
            name,
            text: message.text,
            file: message.file,
            at: message.at,
        },
    )
}

/// Keeps a whole update that never reaches the bot, such as a group line. Only the text is kept: a
/// group message brings no file the host could save.
pub(super) fn remember_update(store: &Store, connector: &str, message: &Value) -> Result<bool> {
    let sender = sender(message);
    remember(
        store,
        &Incoming {
            connector,
            chat: message["chat"]["id"].as_i64().unwrap_or_default(),
            group: message["chat"]["type"].as_str() != Some("private"),
            sender: &sender,
            title: message["chat"]["title"].as_str(),
            text: message["text"].as_str().or_else(|| message["caption"].as_str()).unwrap_or_default().trim(),
            file: None,
            at: crate::store::now_ms(),
        },
    )
}

/// Saves a file that arrived with a message under the data folder, like WhatsApp's media. The path
/// the store keeps is relative to that folder, so a moved data folder still finds the file.
pub(super) async fn save_file(bytes: &[u8], extension: &str) -> Result<String> {
    let dir = crate::service::data_dir().join(Channel::Telegram.media());
    tokio::fs::create_dir_all(&dir).await?;
    let suffix: String = extension.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    let suffix = if suffix.is_empty() { "bin" } else { &suffix };
    let path = dir.join(format!("{}.{}", uuid::Uuid::new_v4().simple(), suffix));
    tokio::fs::write(&path, bytes).await?;
    Ok(path.to_string_lossy().into_owned())
}

/// The kept messages holding every word of `words`, for the channel-history tool's `query` and
/// `since` arguments, newest first.
pub(crate) fn search(store: &Store, words: &[String], since: i64, before: i64, limit: usize) -> Result<Vec<Value>> {
    history::search_in(Channel::Telegram, store, words, since, before, limit)
}

/// Disconnecting a connector deletes the messages and files it kept, and nobody else's.
pub(super) fn forget(store: &Store, connector: &str) -> Result<()> {
    history::forget_prefix(store, Channel::Telegram, connector)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// A file under the Telegram media store, as an arriving message would leave it.
    fn saved_file(name: &str) -> (PathBuf, String) {
        let dir = crate::service::data_dir().join(Channel::Telegram.media());
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("{}-{name}", uuid::Uuid::new_v4().simple()));
        std::fs::write(&path, b"ogg").unwrap();
        let shown = path.to_string_lossy().into_owned();
        (path, shown)
    }

    fn incoming<'a>(connector: &'a str, chat: i64, text: &'a str, file: Option<&'a str>, at: i64) -> Incoming<'a> {
        Incoming { connector, chat, group: false, sender: "sender", title: None, text, file, at }
    }

    #[test]
    fn a_message_is_kept_and_read_back_with_the_channels_query_and_since() {
        let store = crate::store::tests::temp_store();
        let at = 1_800_000_000_000;
        assert!(remember(&store, &incoming("home", 42, "hola", None, at)).unwrap());
        let kept = search(&store, &[], 0, i64::MAX, 10).unwrap();
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0]["text"], "hola");
        assert_eq!(kept[0]["from"], "sender");
        assert_eq!(kept[0]["chat"], "home:42");
        let words = vec!["HOLA".to_owned()];
        assert_eq!(search(&store, &words, 0, i64::MAX, 10).unwrap().len(), 1, "query matches any case");
        assert!(search(&store, &[], at + 1, i64::MAX, 10).unwrap().is_empty(), "since skips older messages");
    }

    #[test]
    fn a_file_that_arrived_is_kept_beside_its_text() {
        let store = crate::store::tests::temp_store();
        let (path, shown) = saved_file("voice.ogg");
        assert!(remember(&store, &incoming("home", 42, "", Some(&shown), 1_800_000_000_000)).unwrap());
        let kept = search(&store, &[], 0, i64::MAX, 10).unwrap();
        assert_eq!(kept.len(), 1, "a message with only a file is kept");
        assert_eq!(kept[0]["text"], "");
        assert_eq!(kept[0]["file"], format!("telegram-media/{}", path.file_name().unwrap().to_string_lossy()));
    }

    #[test]
    fn messages_past_ninety_days_take_their_files_with_them() {
        let store = crate::store::tests::temp_store();
        let (old, old_shown) = saved_file("old.ogg");
        let (recent, recent_shown) = saved_file("recent.ogg");
        let now = 1_800_000_000_000;
        remember(&store, &incoming("home", 42, "old", Some(&old_shown), now - KEEP_MS - 1)).unwrap();
        remember(&store, &incoming("home", 42, "recent", Some(&recent_shown), now)).unwrap();
        assert!(!old.exists(), "the trimmed row's file goes with it");
        assert!(recent.exists(), "the kept row's file stays");
        let kept = search(&store, &[], 0, i64::MAX, 10).unwrap();
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0]["text"], "recent");
    }

    #[test]
    fn the_row_cap_deletes_the_oldest_file() {
        let store = crate::store::tests::temp_store();
        let (old, old_shown) = saved_file("old.ogg");
        let (new, new_shown) = saved_file("new.ogg");
        let now = 1_800_000_000_000;
        remember(&store, &incoming("home", 42, "old", Some(&old_shown), now)).unwrap();
        store
            .with_connection(|c| {
                for i in 0..(MAX_KEPT - 1) {
                    c.execute(
                        "INSERT INTO telegram_history(chat, sender, name, text, file, at) \
                         VALUES(?1, ?2, ?3, ?4, NULL, ?5)",
                        rusqlite::params![format!("home:{i}"), "sender", "Name", "filler", now + i],
                    )?;
                }
                Ok(())
            })
            .unwrap();
        remember(&store, &incoming("home", 42, "new", Some(&new_shown), now + MAX_KEPT)).unwrap();
        assert!(!old.exists(), "the row past the cap takes its file with it");
        assert!(new.exists(), "the newest file stays");
        let kept = search(&store, &[], 0, i64::MAX, 10).unwrap();
        assert!(kept.iter().all(|message| message["text"] != "old"), "the old row is gone");
    }

    #[test]
    fn disconnecting_a_connector_deletes_its_messages_and_files() {
        let store = crate::store::tests::temp_store();
        let (a, a_shown) = saved_file("a.ogg");
        let (b, b_shown) = saved_file("b.ogg");
        let (other, other_shown) = saved_file("other.ogg");
        let at = 1_800_000_000_000;
        remember(&store, &incoming("home", 42, "a", Some(&a_shown), at)).unwrap();
        remember(&store, &incoming("home", 43, "b", Some(&b_shown), at + 1)).unwrap();
        remember(&store, &incoming("work", 42, "other", Some(&other_shown), at + 2)).unwrap();
        forget(&store, "home").unwrap();
        assert!(!a.exists() && !b.exists(), "the disconnected connector's files go");
        assert!(other.exists(), "another connector's file stays");
        let kept = search(&store, &[], 0, i64::MAX, 10).unwrap();
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0]["text"], "other");
        assert_eq!(kept[0]["chat"], "work:42");
    }

    #[test]
    fn a_group_with_no_name_reads_as_telegram_group() {
        assert_eq!(place_name(Some("Family"), true, "sender"), "Family");
        assert_eq!(place_name(Some("  "), true, "sender"), "Telegram group");
        assert_eq!(place_name(None, true, "sender"), "Telegram group");
        assert_eq!(place_name(None, false, "sender"), "sender");
        assert_eq!(place_name(None, false, ""), "someone");
        // The fallback reaches the kept row, so an agent reading history sees it.
        let store = crate::store::tests::temp_store();
        remember(
            &store,
            &Incoming {
                connector: "home",
                chat: -100,
                group: true,
                sender: "sender",
                title: None,
                text: "hola",
                file: None,
                at: 1_800_000_000_000,
            },
        )
        .unwrap();
        let kept = search(&store, &[], 0, i64::MAX, 10).unwrap();
        assert_eq!(kept[0]["name"], "Telegram group");
    }
}
