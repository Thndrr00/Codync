//! Incoming mail dropped into a bot's workspace (bdots' `onEmail`, adapted).
//!
//! A mail relay posts the message here with the bot's token. Codync writes it under
//! `<cwd>/inbox/email/` and the bot reads it like any other file. The relay and the
//! `user.bot@domain` address are external; this module is the local half.

use crate::hub::Hub;
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::path::{Component, Path, PathBuf};

/// The key a bot's intake token is stored under.
pub fn token_key(bot: &str) -> String {
    format!("email.token.{bot}")
}

/// A message from a mailer daemon or postmaster is not mail to answer.
pub fn is_auto_reply(from: &str) -> bool {
    let from = from.trim().to_ascii_lowercase();
    from.starts_with("mailer-daemon@") || from.starts_with("postmaster@") || from.contains("<>")
}

/// A safe file name: no path separators, no leading dots, bounded length.
pub fn file_name(from: &str, subject: &str, at_ms: i64) -> String {
    let day = chrono::DateTime::from_timestamp_millis(at_ms)
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "undated".to_owned());
    let clean = |text: &str| -> String {
        let out: String =
            text.chars().map(|c| if c.is_alphanumeric() || matches!(c, '-' | '_' | ' ') { c } else { '_' }).collect();
        let out = out.trim().trim_start_matches('.').replace(' ', "_");
        let out: String = out.chars().take(80).collect();
        if out.is_empty() { "message".to_owned() } else { out }
    };
    format!("{day}-{}-{}.eml", clean(from), clean(subject))
}

/// The message body on disk: headers the bot can read, then the text.
pub fn message(from: &str, subject: &str, body: &str) -> String {
    format!("From: {from}\nSubject: {subject}\n\n{body}\n")
}

fn under(root: &Path, rel: &Path) -> Option<PathBuf> {
    let mut out = root.to_path_buf();
    for part in rel.components() {
        match part {
            Component::Normal(p) => out.push(p),
            _ => return None,
        }
    }
    Some(out)
}

/// Writes one message, after checking the token. Auto-replies are acknowledged but not stored.
pub fn deliver(
    hub: &Hub,
    bot_id: &str,
    token: &str,
    from: &str,
    subject: &str,
    body: &str,
    at_ms: i64,
) -> Result<Value> {
    let expected = hub.store.kv_get(&token_key(bot_id)).unwrap_or_default();
    if expected.is_empty() || expected != token {
        bail!("this bot has no matching mail token");
    }
    if is_auto_reply(from) {
        return Ok(json!({"stored": false, "reason": "auto_reply"}));
    }
    let row = hub.store.bot(bot_id)?.context("unknown bot")?;
    let root = Path::new(&row.config.cwd).join("inbox").join("email");
    let rel = Path::new(&file_name(from, subject, at_ms));
    let path = under(&root, rel).context("invalid mail path")?;
    std::fs::create_dir_all(root).with_context(|| "couldn't create the mail folder")?;
    std::fs::write(&path, message(from, subject, body)).with_context(|| "couldn't save the message")?;
    Ok(json!({"stored": true, "file": rel.to_string_lossy()}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mail_paths_are_safe_and_stable() {
        let name = file_name("alice@example.com", "../../Salaries / Q3: 2026", 1_700_000_000_000);
        assert!(name.starts_with("2023-11-14-alice_example.com-"));
        assert!(!name.contains('/'));
        assert!(!name.contains('\\'));
        assert!(name.ends_with(".eml"));
        assert_eq!(is_auto_reply("MAILER-DAEMON@example.com"), true);
        assert_eq!(is_auto_reply("alice@example.com"), false);
    }

    #[test]
    fn the_message_keeps_headers_the_bot_can_read() {
        let text = message("alice@example.com", "Invoice", "Please review it.");
        assert!(text.starts_with("From: alice@example.com\nSubject: Invoice\n\n"));
        assert!(text.ends_with("Please review it.\n"));
    }
}
