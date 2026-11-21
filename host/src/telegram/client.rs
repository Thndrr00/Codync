//! The Telegram Bot API: every call the host makes to it, through the egress policy.
//!
//! The token is part of the URL Telegram wants, so no error leaves this module with the URL in it.

use crate::egress::{self, Policy};
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::time::Duration;

/// Telegram's limit for one text message.
pub(super) const MAX_TEXT: usize = 4096;

/// Where the Bot API is and how patiently to talk to it; tests point it at a fake.
#[derive(Clone)]
pub(super) struct Endpoint {
    pub api: String,
    pub policy: Policy,
    /// Seconds Telegram may hold a `getUpdates` open.
    pub poll_seconds: u64,
    /// The least time between two edits of the message that is being streamed.
    pub edit_every: Duration,
    /// Wait after a failed poll.
    pub retry: Duration,
    /// How often the set of Telegram bots to serve is compared with the configuration.
    pub reconcile: Duration,
}

impl Endpoint {
    pub(super) fn telegram() -> Self {
        Self {
            api: "https://api.telegram.org".into(),
            policy: Policy::Only(&["api.telegram.org"]),
            poll_seconds: 25,
            edit_every: Duration::from_secs(1),
            retry: Duration::from_secs(3),
            reconcile: Duration::from_secs(2),
        }
    }
}

async fn send(
    ep: &Endpoint,
    token: &str,
    method: &str,
    build: impl FnOnce(reqwest::RequestBuilder) -> reqwest::RequestBuilder,
) -> Result<Value> {
    let url = format!("{}/bot{token}/{method}", ep.api);
    let client = egress::client(ep.policy, &url).await.map_err(|e| anyhow::anyhow!("Telegram {method}: {e:#}"))?;
    let request = build(client.post(&url)).timeout(Duration::from_secs(ep.poll_seconds + 15));
    let response =
        request.send().await.map_err(|e| anyhow::anyhow!("Telegram {method} failed: {}", e.without_url()))?;
    let status = response.status();
    let body = egress::json(response, 4 * 1024 * 1024)
        .await
        .with_context(|| format!("Telegram {method} answered {status}"))?;
    if body["ok"] != true {
        bail!("Telegram {method}: {}", body["description"].as_str().unwrap_or("refused"));
    }
    Ok(body["result"].clone())
}

pub(super) async fn call(ep: &Endpoint, token: &str, method: &str, body: &Value) -> Result<Value> {
    send(ep, token, method, |r| r.json(body)).await
}

/// Download only a Telegram-issued relative file path, through the existing endpoint policy.
/// Errors deliberately omit the request URL: it contains the bot's token.
pub(super) async fn voice_note(ep: &Endpoint, token: &str, id: &str) -> Result<Vec<u8>> {
    let file = call(ep, token, "getFile", &json!({"file_id":id})).await?;
    if file["file_size"].as_u64().is_some_and(|size| size > 20 * 1024 * 1024) {
        bail!("Telegram voice note exceeds 20 MiB");
    }
    let path = file["file_path"]
        .as_str()
        .filter(|path| valid_file_path(path))
        .context("Telegram returned an invalid voice-note path")?;
    let url = format!("{}/file/bot{token}/{path}", ep.api);
    let client =
        egress::client(ep.policy, &url).await.map_err(|_| anyhow::anyhow!("Telegram file endpoint unavailable"))?;
    let response = client
        .get(url)
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .map_err(|_| anyhow::anyhow!("Telegram voice-note download failed"))?;
    if !response.status().is_success() {
        bail!("Telegram voice-note download was refused ({})", response.status());
    }
    egress::body(response, crate::voice::transcription::MAX_NOTE_BYTES)
        .await
        .map_err(|_| anyhow::anyhow!("Telegram voice-note download failed or exceeded 20 MiB"))
}

fn valid_file_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 1024
        && path.split('/').all(|part| !part.is_empty() && part != "." && part != "..")
        && path.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'/' | b'.' | b'_' | b'-'))
}

/// New updates after `offset`, waiting up to the endpoint's poll time for one.
pub(super) async fn updates(ep: &Endpoint, token: &str, offset: i64) -> Result<Vec<Value>> {
    let body = json!({"offset": offset, "timeout": ep.poll_seconds, "allowed_updates": ["message", "callback_query"]});
    let result = call(ep, token, "getUpdates", &body).await?;
    Ok(result.as_array().cloned().unwrap_or_default())
}

/// The well-formed Markdown span at the start of `rest`, if any. The combined marker is checked
/// **before** bold and italic: `***bold italic***` must not be read as a bold span with a stray
/// asterisk, which would drop the italic and leave a `*` in the message.
fn span(rest: &str) -> Option<(&'static str, &str, &str)> {
    for marker in ["***", "**", "*"] {
        if let Some(after) = rest.strip_prefix(marker)
            && let Some((inner, tail)) = after.split_once(marker)
            && !inner.is_empty()
            && !inner.contains('\n')
            && !inner.starts_with(' ')
            && !inner.ends_with(' ')
        {
            return Some((marker, inner, tail));
        }
    }
    None
}

/// The Markdown link at the start of `rest`: `[label](url)`. Kept whole so a link never has its
/// label read as bold or italic, and Telegram's `parse_mode` still turns it into a real link.
fn link(rest: &str) -> Option<(&str, &str, &str)> {
    let after = rest.strip_prefix('[')?;
    let (label, after_label) = after.split_once("](")?;
    let (url, tail) = after_label.split_once(')')?;
    (!label.is_empty() && !label.contains('\n') && !url.is_empty() && !url.contains('\n')).then_some((label, url, tail))
}

/// Converts the Markdown an agent writes into Telegram's `Markdown` parse mode. Bold (`**…**`)
/// becomes `*…*`, italic (`*…*`) becomes `_…_`, and the combined `***…***` and `[text](url)` links
/// are kept. Pure so the combined marker and links can be tested without the Bot API.
pub(super) fn markdown(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while !rest.is_empty() {
        if let Some((marker, inner, tail)) = span(rest) {
            match marker {
                "***" => {
                    out.push_str("***");
                    out.push_str(&markdown(inner));
                    out.push_str("***");
                }
                "**" => {
                    out.push('*');
                    out.push_str(&markdown(inner));
                    out.push('*');
                }
                _ => {
                    out.push('_');
                    out.push_str(&markdown(inner));
                    out.push('_');
                }
            }
            rest = tail;
            continue;
        }
        if let Some((label, url, tail)) = link(rest) {
            out.push('[');
            out.push_str(&markdown(label));
            out.push_str("](");
            out.push_str(url);
            out.push(')');
            rest = tail;
            continue;
        }
        if let Some(ch) = rest.chars().next() {
            out.push(ch);
            rest = &rest[ch.len_utf8()..];
        }
    }
    out
}

fn text_body(chat: i64, text: &str, formatted: bool) -> Value {
    if formatted {
        json!({"chat_id": chat, "text": clip(&markdown(text)), "parse_mode": "Markdown", "link_preview_options": {"is_disabled": true}})
    } else {
        json!({"chat_id": chat, "text": clip(text), "link_preview_options": {"is_disabled": true}})
    }
}

/// Sends text and returns the id of the message it made. Text beyond Telegram's limit is cut.
pub(super) async fn message(ep: &Endpoint, token: &str, chat: i64, text: &str, keyboard: Option<Value>) -> Result<i64> {
    let mut body = text_body(chat, text, false);
    if let Some(keyboard) = keyboard {
        body["reply_markup"] = json!({"inline_keyboard": keyboard});
    }
    let sent = call(ep, token, "sendMessage", &body).await?;
    sent["message_id"].as_i64().context("Telegram sent no message id")
}

/// Sends an agent's reply, keeping its bold, italic and links (and `***bold italic***`).
pub(super) async fn reply(ep: &Endpoint, token: &str, chat: i64, text: &str) -> Result<i64> {
    let sent = call(ep, token, "sendMessage", &text_body(chat, text, true)).await?;
    sent["message_id"].as_i64().context("Telegram sent no message id")
}

/// Replaces a message's text (and drops its buttons).
pub(super) async fn edit(ep: &Endpoint, token: &str, chat: i64, message: i64, text: &str) -> Result<()> {
    edit_text(ep, token, chat, message, text, false).await
}

/// Replaces an agent's streamed reply, keeping the same formatting as [`reply`].
pub(super) async fn edit_reply(ep: &Endpoint, token: &str, chat: i64, message: i64, text: &str) -> Result<()> {
    edit_text(ep, token, chat, message, text, true).await
}

async fn edit_text(ep: &Endpoint, token: &str, chat: i64, message: i64, text: &str, formatted: bool) -> Result<()> {
    let mut body = text_body(chat, text, formatted);
    body["message_id"] = json!(message);
    match call(ep, token, "editMessageText", &body).await {
        Ok(_) => Ok(()),
        // Editing to the same text is not a failure worth reporting.
        Err(error) if format!("{error:#}").contains("message is not modified") => Ok(()),
        Err(error) => Err(error),
    }
}

/// Sends `bytes` as a file called `name`, for a reply too long for one message.
pub(super) async fn document(
    ep: &Endpoint,
    token: &str,
    chat: i64,
    name: &str,
    bytes: &[u8],
    caption: &str,
) -> Result<()> {
    let boundary = format!("codync-{}", uuid::Uuid::new_v4().simple());
    let mut body = Vec::new();
    for (field, value) in [("chat_id", chat.to_string()), ("caption", clip(caption))] {
        body.extend(
            format!("--{boundary}\r\nContent-Disposition: form-data; name=\"{field}\"\r\n\r\n{value}\r\n").bytes(),
        );
    }
    let safe: String = name.chars().filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_')).collect();
    body.extend(
        format!("--{boundary}\r\nContent-Disposition: form-data; name=\"document\"; filename=\"{safe}\"\r\nContent-Type: text/plain\r\n\r\n")
            .bytes(),
    );
    body.extend(bytes);
    body.extend(format!("\r\n--{boundary}--\r\n").bytes());
    let content_type = format!("multipart/form-data; boundary={boundary}");
    send(ep, token, "sendDocument", |r| r.header("content-type", content_type).body(body)).await?;
    Ok(())
}

fn clip(text: &str) -> String {
    text.chars().take(MAX_TEXT).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_text_is_cut_at_telegrams_limit() {
        let long = "é".repeat(MAX_TEXT + 10);
        assert_eq!(clip(&long).chars().count(), MAX_TEXT);
        assert_eq!(clip("short"), "short");
    }

    #[test]
    fn markdown_keeps_bold_italic_and_links_for_telegram() {
        // The combined marker is checked before bold, so `***…***` is not read as a bold span
        // with a stray asterisk and its italic is kept.
        assert_eq!(markdown("***bold italic***"), "***bold italic***");
        assert_eq!(markdown("**bold**"), "*bold*");
        assert_eq!(markdown("*italic*"), "_italic_");
        assert_eq!(markdown("_italic_"), "_italic_");
        assert_eq!(markdown("`code`"), "`code`");
        assert_eq!(
            markdown("See [the docs](https://example.com/a_b) and ***really*** *matter*"),
            "See [the docs](https://example.com/a_b) and ***really*** _matter_"
        );
        // A marker inside a link's label is converted, but the URL is left untouched.
        assert_eq!(markdown("[**bold** link](https://x.dev)"), "[*bold* link](https://x.dev)");
        // Unpaired markers stay literal rather than being swallowed into a span.
        assert_eq!(markdown("2 * 3 ** four"), "2 * 3 ** four");
        assert_eq!(markdown("plain text"), "plain text");
    }
}
