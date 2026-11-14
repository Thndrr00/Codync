//! The Matrix Client-Server API calls the host makes: logging in, syncing, joining, sending text,
//! uploading and sending files, and reacting. Every address comes from the homeserver the owner
//! named, so the requests go out under the person-chosen egress policy.

use anyhow::{Context as _, Result, bail};
use serde_json::{Value, json};
use std::time::Duration;

/// Encoded as one path segment: `!room:server` keeps meaning in a URL only when escaped.
const SEGMENT: &percent_encoding::AsciiSet =
    &percent_encoding::NON_ALPHANUMERIC.remove(b'_').remove(b'-').remove(b'.').remove(b'~');
const TOKEN_CHARS: usize = 8192;
/// One text event’s practical ceiling, under Matrix’s 64 KiB limit after JSON escaping.
const PIECE: usize = 16_000;
const MAX_FILES: usize = 10;
const MAX_FILE_BYTES: u64 = 25 * 1024 * 1024;

pub(super) struct Session {
    pub user_id: String,
    pub access_token: String,
    pub device_id: String,
}

fn seg(value: &str) -> String {
    percent_encoding::utf8_percent_encode(value, SEGMENT).to_string()
}

fn url(base: &str, path: &str) -> String {
    format!("{base}/_matrix/client/v3/{path}")
}

async fn answer(response: reqwest::Response, what: &str) -> Result<Value> {
    let status = response.status();
    let value = crate::egress::json(response, 16 * 1024 * 1024).await.unwrap_or_default();
    if !status.is_success() {
        let why = value["error"].as_str().filter(|e| !e.is_empty()).unwrap_or("refused");
        bail!("Matrix {what} failed ({status}): {why}");
    }
    Ok(value)
}

async fn request(base: &str, method: reqwest::Method, path: &str, body: &Value, timeout: Duration) -> Result<Value> {
    let target = url(base, path);
    let client = crate::egress::client(crate::egress::Policy::UserChosen, &target).await?;
    answer(client.request(method, target.as_str()).json(body).timeout(timeout).send().await?, "request").await
}

/// `POST /login`, the password flow every homeserver supports. The password never leaves this call.
pub(super) async fn login(base: &str, user: &str, password: &str) -> Result<Session> {
    anyhow::ensure!(!user.is_empty() && user.len() <= 255, "Write the Matrix user.");
    anyhow::ensure!(!password.is_empty() && password.len() <= 1024, "Write the Matrix password.");
    let body = json!({
        "type": "m.login.password",
        "identifier": {"type": "m.id.user", "user": user},
        "password": password,
        "initial_device_display_name": "Codync",
    });
    let value = request(base, reqwest::Method::POST, "login", &body, Duration::from_secs(30)).await?;
    let access_token = value["access_token"].as_str().filter(|t| !t.is_empty() && t.len() <= TOKEN_CHARS);
    Ok(Session {
        user_id: value["user_id"].as_str().context("Matrix signed in without a user id.")?.into(),
        access_token: access_token.context("Matrix signed in without an access token.")?.into(),
        device_id: value["device_id"].as_str().unwrap_or_default().into(),
    })
}

/// `GET /account/whoami`: the account an access token belongs to.
pub(super) async fn whoami(base: &str, token: &str) -> Result<String> {
    let target = url(base, "account/whoami");
    let client = crate::egress::client(crate::egress::Policy::UserChosen, &target).await?;
    let value = answer(
        client.get(target.as_str()).bearer_auth(token).timeout(Duration::from_secs(20)).send().await?,
        "sign-in check",
    )
    .await?;
    value["user_id"].as_str().filter(|u| !u.is_empty()).map(str::to_owned).context("That token has no account.")
}

/// Resolves `#alias:server` to the room id it names.
pub(super) async fn resolve_alias(base: &str, token: &str, alias: &str) -> Result<String> {
    let path = format!("directory/room/{}", seg(alias));
    let target = url(base, &path);
    let client = crate::egress::client(crate::egress::Policy::UserChosen, &target).await?;
    let value = answer(
        client.get(target.as_str()).bearer_auth(token).timeout(Duration::from_secs(20)).send().await?,
        "room lookup",
    )
    .await?;
    value["room_id"].as_str().filter(|r| r.starts_with('!')).map(str::to_owned).context("That alias is not a room.")
}

/// Joins a room or accepts its invite. A room the bot is already in answers with its id too.
pub(super) async fn join(base: &str, token: &str, room: &str) -> Result<()> {
    request(base, reqwest::Method::POST, &format!("join/{}", seg(room)), &json!({}), Duration::from_secs(30))
        .await
        .map(drop)
}

/// One `GET /sync` page. `since` empty asks for the initial page; a long `timeout` holds the request
/// open until something happens or the server gives up.
pub(super) async fn sync(base: &str, token: &str, since: &str, timeout: Duration) -> Result<Value> {
    let filter = json!({"room":{"timeline":{"limit":20},"state":{"lazy_load_members":true}},"presence":{"types":[]}});
    let mut target = reqwest::Url::parse(&url(base, "sync"))?;
    target.query_pairs_mut().append_pair("timeout", &timeout.as_millis().to_string());
    target.query_pairs_mut().append_pair("filter", &filter.to_string());
    if !since.is_empty() {
        target.query_pairs_mut().append_pair("since", since);
    }
    let client = crate::egress::client(crate::egress::Policy::UserChosen, target.as_str()).await?;
    answer(client.get(target).bearer_auth(token).timeout(timeout + Duration::from_secs(20)).send().await?, "sync").await
}

/// The `m.room.message` content of a text reply, answering `reply_to` when it is not empty.
pub(super) fn text_content(text: &str, reply_to: &str) -> Value {
    let mut content = json!({"msgtype": "m.text", "body": text});
    if !reply_to.is_empty() {
        content["m.relates_to"] = json!({"rel_type": "m.in_reply_to", "event_id": reply_to});
    }
    content
}

/// The `m.room.message` content of one uploaded file.
pub(super) fn file_content(name: &str, mimetype: &str, mxc: &str, size: u64) -> Value {
    let msgtype = if mimetype.starts_with("image/") {
        "m.image"
    } else if mimetype.starts_with("audio/") {
        "m.audio"
    } else if mimetype.starts_with("video/") {
        "m.video"
    } else {
        "m.file"
    };
    json!({"msgtype": msgtype, "body": name, "url": mxc, "info": {"mimetype": mimetype, "size": size}})
}

/// The `m.reaction` content that puts `key` on `event`.
pub(super) fn reaction_content(event: &str, key: &str) -> Value {
    json!({"m.relates_to": {"rel_type": "m.annotation", "event_id": event, "key": key}})
}

/// Sends one content object and returns the event id the homeserver gave it.
pub(super) async fn send_content(base: &str, token: &str, room: &str, txn: &str, content: &Value) -> Result<String> {
    let path = format!("rooms/{}/send/m.room.message/{}", seg(room), seg(txn));
    let value = request(base, reqwest::Method::PUT, &path, content, Duration::from_secs(40)).await?;
    value["event_id"].as_str().filter(|e| !e.is_empty()).map(str::to_owned).context("Matrix sent no event id.")
}

/// Puts `key` on an event; a reaction is a normal event of its own.
pub(super) async fn react(base: &str, token: &str, room: &str, event: &str, key: &str, txn: &str) -> Result<()> {
    let path = format!("rooms/{}/send/m.reaction/{}", seg(room), seg(txn));
    request(base, reqwest::Method::PUT, &path, &reaction_content(event, key), Duration::from_secs(30)).await.map(drop)
}

/// Replaces an event's text (a settled card), the way Matrix edits a message.
pub(super) async fn replace(base: &str, token: &str, room: &str, event: &str, text: &str, txn: &str) -> Result<()> {
    let content = json!({
        "msgtype": "m.text",
        "body": format!("* {text}"),
        "m.new_content": {"msgtype": "m.text", "body": text},
        "m.relates_to": {"rel_type": "m.replace", "event_id": event},
    });
    send_content(base, token, room, txn, &content).await.map(drop)
}

/// Sends `text`, cut into pieces the homeserver accepts; returns the last event id.
pub(super) async fn send_text(
    base: &str,
    token: &str,
    room: &str,
    reply_to: &str,
    text: &str,
    txn: &str,
) -> Result<String> {
    let mut last = String::new();
    for (index, piece) in pieces(text).into_iter().enumerate() {
        let reply = if index == 0 { reply_to } else { "" };
        let id = send_content(base, token, room, &format!("{txn}.{index}"), &text_content(&piece, reply)).await?;
        last = id;
    }
    Ok(last)
}

/// Uploads each file (its bytes, named as it was) and sends it after the text; returns the event ids.
pub(super) async fn send_files(base: &str, token: &str, room: &str, files: &[Value], txn: &str) -> Result<Vec<String>> {
    let mut sent = Vec::new();
    for (index, file) in files.iter().take(MAX_FILES).enumerate() {
        let Some(path) = file["path"].as_str() else { continue };
        let Ok(metadata) = tokio::fs::metadata(path).await else { continue };
        if metadata.len() > MAX_FILE_BYTES {
            tracing::warn!(path, "Matrix file is larger than 25 MiB and was not sent");
            continue;
        }
        let Ok(bytes) = tokio::fs::read(path).await else { continue };
        let name = file["name"].as_str().unwrap_or("file").chars().take(120).collect::<String>();
        let mimetype = file["mimetype"].as_str().unwrap_or("application/octet-stream");
        let base_url = format!("{base}/_matrix/media/v3/upload");
        let mut target = reqwest::Url::parse(&base_url)?;
        target.query_pairs_mut().append_pair("filename", &name);
        let client = crate::egress::client(crate::egress::Policy::UserChosen, target.as_str()).await?;
        let response = client
            .post(target)
            .bearer_auth(token)
            .header(reqwest::header::CONTENT_TYPE, mimetype)
            .body(bytes)
            .timeout(Duration::from_mins(2))
            .send()
            .await
            .map_err(|error| anyhow::anyhow!("Matrix upload failed: {}", error.without_url()))?;
        let uploaded = answer(response, "upload").await?;
        let Some(mxc) = uploaded["content_uri"].as_str() else { continue };
        let id = send_content(
            base,
            token,
            room,
            &format!("{txn}.file{index}"),
            &file_content(&name, mimetype, mxc, metadata.len()),
        )
        .await?;
        sent.push(id);
    }
    Ok(sent)
}

/// `text` cut into pieces at character boundaries, so a long reply is not refused.
pub(super) fn pieces(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut count = 0;
    for ch in text.chars() {
        if count >= PIECE {
            out.push(std::mem::take(&mut current));
            count = 0;
        }
        current.push(ch);
        count += 1;
    }
    if !current.is_empty() || out.is_empty() {
        out.push(current);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_text_reply_names_the_event_it_answers() {
        assert_eq!(text_content("hola", ""), json!({"msgtype": "m.text", "body": "hola"}));
        let reply = text_content("hola", "$abc");
        assert_eq!(reply["m.relates_to"], json!({"rel_type": "m.in_reply_to", "event_id": "$abc"}));
    }

    #[test]
    fn a_file_gets_the_msgtype_its_mimetype_asks_for() {
        let image = file_content("a.png", "image/png", "mxc://x/a", 12);
        assert_eq!(image["msgtype"], "m.image");
        assert_eq!(image["body"], "a.png");
        assert_eq!(image["url"], "mxc://x/a");
        assert_eq!(image["info"]["size"], 12);
        assert_eq!(file_content("a.pdf", "application/pdf", "mxc://x/b", 1)["msgtype"], "m.file");
        assert_eq!(file_content("a.mp3", "audio/mpeg", "mxc://x/c", 1)["msgtype"], "m.audio");
    }

    #[test]
    fn long_text_is_cut_and_room_ids_stay_one_segment() {
        let text = "é".repeat(PIECE + 400);
        let out = pieces(&text);
        assert_eq!(out.len(), 2);
        assert!(out.iter().all(|p| p.chars().count() <= PIECE));
        assert_eq!(out.concat(), text);
        assert_eq!(seg("!room:example.org"), "%21room%3Aexample.org");
    }

    #[test]
    fn a_reaction_annotates_its_event() {
        assert_eq!(
            reaction_content("$e", "1️⃣"),
            json!({"m.relates_to": {"rel_type": "m.annotation", "event_id": "$e", "key": "1️⃣"}})
        );
    }
}
