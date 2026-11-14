//! The `signal-cli` JSON-RPC contract the host speaks: one HTTP endpoint for requests (the
//! daemon's `POST /api/v1/rpc`) and one Server-Sent Events stream for what arrives
//! (`GET /api/v1/events`).
//!
//! The daemon, not the host, owns the Signal account: `signal-cli -a +15551234567 daemon
//! --http 127.0.0.1:7583 --receive-mode=on-start`. Nothing is linked by this module; the person
//! links the number once with `signal-cli -a +15551234567 link` (a device QR code) or with
//! `register` plus `verify`, and the host only talks to that daemon.

use anyhow::{Context, Result, bail};
use base64::Engine as _;
use futures::StreamExt;
use serde_json::{Value, json};
use std::time::Duration;

use crate::egress;

const MAX_FRAME: usize = 1024 * 1024;
const MAX_ATTACHMENT: usize = 20 * 1024 * 1024;

fn rpc_url(daemon: &str) -> String {
    format!("{}/api/v1/rpc", daemon.trim_end_matches('/'))
}

fn events_url(daemon: &str) -> String {
    format!("{}/api/v1/events", daemon.trim_end_matches('/'))
}

/// One JSON-RPC call. The daemon answers `{"result": ...}` or `{"error": {"message": ...}}`.
pub(super) async fn rpc(daemon: &str, method: &str, params: Value) -> Result<Value> {
    let url = rpc_url(daemon);
    let client = egress::client(egress::Policy::UserChosen, &url).await?;
    let body = json!({"jsonrpc": "2.0", "id": uuid::Uuid::new_v4().to_string(), "method": method, "params": params});
    let response = client
        .post(&url)
        .json(&body)
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .map_err(|error| anyhow::anyhow!("signal-cli is not answering: {}", error.without_url()))?;
    let status = response.status();
    let answer = egress::json(response, MAX_FRAME).await.unwrap_or_default();
    if !status.is_success() {
        let why = answer["error"]["message"].as_str().or_else(|| answer["error"].as_str()).unwrap_or("refused");
        bail!("signal-cli {method} failed ({status}): {why}");
    }
    if let Some(error) = answer.get("error").filter(|error| !error.is_null()) {
        let why = error["message"].as_str().or_else(|| error.as_str()).unwrap_or("refused");
        bail!("signal-cli {method} failed: {why}");
    }
    Ok(answer.get("result").cloned().unwrap_or(Value::Null))
}

/// The numbers the daemon is linked to. Also the reachability check: a daemon that answers has
/// the account the person configured.
pub(super) async fn accounts(daemon: &str) -> Result<Vec<String>> {
    let result = rpc(daemon, "listAccounts", json!({})).await?;
    let mut numbers = Vec::new();
    if let Some(list) = result.as_array().or_else(|| result["accounts"].as_array()) {
        for account in list {
            if let Some(number) = account["number"].as_str().or_else(|| account.as_str()) {
                numbers.push(number.to_owned());
            }
        }
    }
    Ok(numbers)
}

/// Whether `number` is linked to the daemon.
pub(super) async fn has_account(daemon: &str, number: &str) -> Result<bool> {
    Ok(accounts(daemon).await?.iter().any(|account| account == number))
}

fn params(account: &str, chat: &str, message: &str, attachments: &[String]) -> Value {
    let mut params = json!({"account": account, "message": message});
    if let Some(group) = chat.strip_prefix("group:") {
        params["groupId"] = group.into();
    } else {
        params["recipient"] = json!([chat]);
    }
    if !attachments.is_empty() {
        params["attachments"] = json!(attachments);
    }
    params
}

/// Sends `text` to a phone number or a `group:<id>` chat, as one Signal message; returns its
/// timestamp, which is also the id a reply quotes.
pub(super) async fn send_text(
    daemon: &str,
    account: &str,
    chat: &str,
    text: &str,
    attachments: &[String],
) -> Result<String> {
    let result = rpc(daemon, "send", params(account, chat, text, attachments)).await?;
    let timestamp = result["timestamp"]
        .as_u64()
        .map_or_else(|| result.as_u64().map_or_else(String::new, |t| t.to_string()), |t| t.to_string());
    anyhow::ensure!(!timestamp.is_empty(), "Signal accepted the message without a timestamp");
    Ok(timestamp)
}

/// Downloads one incoming attachment to `dest`. signal-cli answers with base64 data; the content
/// type is kept for the caller's file name.
pub(super) async fn download(daemon: &str, account: &str, attachment_id: &str, dest: &std::path::Path) -> Result<()> {
    let result = rpc(daemon, "getAttachment", json!({"account": account, "attachmentId": attachment_id})).await?;
    let encoded = result["data"].as_str().or_else(|| result.as_str()).context("Signal sent no attachment data")?;
    anyhow::ensure!(encoded.len() <= MAX_ATTACHMENT * 4 / 3 + 4, "Signal attachment exceeds 20 MiB");
    let bytes = base64::engine::general_purpose::STANDARD.decode(encoded).context("Signal attachment is not base64")?;
    anyhow::ensure!(bytes.len() <= MAX_ATTACHMENT, "Signal attachment exceeds 20 MiB");
    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(dest, bytes).await?;
    Ok(())
}

/// One notification on the daemon's event stream.
pub(super) struct Frame {
    pub method: String,
    pub params: Value,
}

fn frame(data: &str) -> Option<Frame> {
    let value: Value = serde_json::from_str(data).ok()?;
    let method = value["method"].as_str()?.to_owned();
    Some(Frame { method, params: value["params"].clone() })
}

/// Reads the daemon's SSE stream until it closes. `on_open` runs once the daemon accepted the
/// stream (that is what "connected" means); every `receive` notification is handed to `on`.
pub(super) async fn listen(
    daemon: &str,
    on_open: impl FnOnce(),
    mut on: impl FnMut(Frame) -> Result<()>,
) -> Result<()> {
    let url = events_url(daemon);
    let client = egress::client(egress::Policy::UserChosen, &url).await?;
    let response = client
        .get(&url)
        .header(reqwest::header::ACCEPT, "text/event-stream")
        .timeout(Duration::from_secs(20))
        .send()
        .await
        .map_err(|error| anyhow::anyhow!("signal-cli events are not answering: {}", error.without_url()))?;
    if !response.status().is_success() {
        bail!("signal-cli events answered {}", response.status());
    }
    on_open();
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("signal-cli event stream broke")?;
        anyhow::ensure!(buffer.len() + chunk.len() <= MAX_FRAME, "signal-cli event exceeds 1 MiB");
        buffer.push_str(&String::from_utf8_lossy(&chunk));
        if buffer.contains('\r') {
            buffer = buffer.replace("\r\n", "\n");
        }
        while let Some(end) = buffer.find("\n\n") {
            let event = buffer[..end].to_owned();
            buffer.drain(..end + 2);
            let mut data = String::new();
            for line in event.lines() {
                if let Some(rest) = line.strip_prefix("data:") {
                    if !data.is_empty() {
                        data.push('\n');
                    }
                    data.push_str(rest.trim_start());
                }
            }
            if let Some(frame) = frame(&data)
                && frame.method == "receive"
            {
                on(frame)?;
            }
        }
    }
    bail!("signal-cli event stream closed");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_message_carries_its_target_and_files() {
        let dm = params("+15550000001", "+15550000002", "hola", &[]);
        assert_eq!(dm["account"], "+15550000001");
        assert_eq!(dm["recipient"], json!(["+15550000002"]));
        assert_eq!(dm["message"], "hola");
        assert!(dm.get("groupId").is_none());
        assert!(dm.get("attachments").is_none());

        let group = params("+15550000001", "group:abc=", "hola", &["/tmp/a.png".into()]);
        assert_eq!(group["groupId"], "abc=");
        assert!(group.get("recipient").is_none());
        assert_eq!(group["attachments"], json!(["/tmp/a.png"]));
    }

    #[test]
    fn an_sse_frame_is_the_receive_notification() {
        let value = json!({"jsonrpc": "2.0", "method": "receive",
            "params": {"envelope": {"source": "+15550000002", "timestamp": 12}}});
        let frame = frame(&value.to_string()).expect("a frame");
        assert_eq!(frame.method, "receive");
        assert_eq!(frame.params["envelope"]["timestamp"], 12);
        assert!(frame(&json!({"jsonrpc":"2.0","method":"heartbeat"}).to_string()).is_some());
        assert!(frame("not json").is_none());
    }
}
