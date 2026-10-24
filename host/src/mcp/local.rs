//! Secret-injecting stdio proxy with the same immutable approval boundary as HTTP MCP.
use super::{cancellation::Cancellation, policy, proxy_replies::ProxyReplies, tasks::Tasks};
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::{process::Stdio, sync::Arc};
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{Mutex, mpsc};

struct CloseOnDrop(Arc<Cancellation>);
impl Drop for CloseOnDrop {
    fn drop(&mut self) {
        self.0.close();
    }
}
async fn read_value(reader: &mut (impl AsyncBufRead + Unpin), limit: u64) -> Result<Option<Value>> {
    let mut bytes = Vec::new();
    (&mut *reader).take(limit + 1).read_until(b'\n', &mut bytes).await?;
    if bytes.is_empty() {
        return Ok(None);
    }
    if bytes.len() as u64 > limit {
        bail!("MCP line exceeded its size limit.");
    }
    Ok(Some(serde_json::from_slice(&bytes)?))
}

pub(super) async fn serve(connector: String, port: u16) -> Result<()> {
    let token = tokio::fs::read_to_string(crate::service::data_dir().join("token")).await?;
    let token = token.trim().to_owned();
    let runtime = policy::proxy_call(port, &token, "connectorRuntime", &json!({"id":connector})).await?;
    let command = runtime["command"].as_str().context("Not a local connector.")?;
    let observed = Arc::new(ProxyReplies::bound(
        port,
        &token,
        &connector,
        runtime["fingerprint"].as_str().context("Missing runtime fingerprint.")?.into(),
    ));
    let mut process = tokio::process::Command::new(crate::shell::program(command));
    process.args(runtime["args"].as_array().into_iter().flatten().filter_map(Value::as_str));
    for (key, value) in runtime["env"].as_object().into_iter().flatten() {
        if let Some(value) = value.as_str() {
            process.env(key, value);
        }
    }
    process.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).kill_on_drop(true);
    #[cfg(unix)]
    process.process_group(0);
    #[cfg(windows)]
    process.creation_flags(
        windows_sys::Win32::System::Threading::CREATE_SUSPENDED
            | windows_sys::Win32::System::Threading::CREATE_NO_WINDOW,
    );
    let mut child = process.spawn()?;
    let input = Arc::new(Mutex::new(child.stdin.take().context("Missing connector stdin.")?));
    let output = child.stdout.take().context("Missing connector stdout.")?;
    let mut child = crate::agent::process::Process::new(child)?;
    let cancellation = Arc::new(Cancellation::default());
    let mut tasks = Tasks::default();
    let (reply_tx, mut reply_rx) = mpsc::channel::<Value>(16);
    let seen = observed.clone();
    let writer_cancel = cancellation.clone();
    let writer = tokio::spawn(async move {
        let mut stdout = tokio::io::stdout();
        while let Some(reply) = reply_rx.recv().await {
            seen.observe(&reply).await;
            writer_cancel.finish(&reply["id"]);
            let mut line = serde_json::to_vec(&reply)?;
            line.push(b'\n');
            stdout.write_all(&line).await?;
            stdout.flush().await?;
        }
        anyhow::Ok(())
    });
    tasks.track(&writer);
    let replies = reply_tx.clone();
    let closed = cancellation.clone();
    let upstream = tokio::spawn(async move {
        let _closed = CloseOnDrop(closed);
        let mut lines = BufReader::new(output);
        while let Some(reply) = read_value(&mut lines, 1024 * 1024).await? {
            if replies.send(reply).await.is_err() {
                break;
            }
        }
        anyhow::Ok(())
    });
    tasks.track(&upstream);
    let (incoming, mut inbox) = mpsc::channel::<Value>(100);
    let cancelled = cancellation.clone();
    let immediate = reply_tx.clone();
    let reader = tokio::spawn(async move {
        let _closed = CloseOnDrop(cancelled.clone());
        let mut lines = BufReader::new(tokio::io::stdin());
        while let Ok(Some(message)) = read_value(&mut lines, 128 * 1024).await {
            cancelled.observe(&message);
            if message["method"] == "ping" {
                if !message["id"].is_null()
                    && immediate.try_send(json!({"jsonrpc":"2.0","id":message["id"],"result":{}})).is_err()
                {
                    break;
                }
                continue;
            }
            if incoming.try_send(message).is_err() {
                break;
            }
        }
        cancelled.close();
    });
    tasks.track(&reader);
    let mut failure = None;
    loop {
        let message = tokio::select! {
            () = cancellation.wait_closed() => break,
            status = child.wait_owned() => {
                match status { Ok(status) if status.success() => {}, Ok(_) => failure=Some(anyhow::anyhow!("Connector exited unsuccessfully.")), Err(error) => failure=Some(error) }
                break;
            }
            message = inbox.recv() => { let Some(message) = message else { break; }; message }
        };
        let id = message["id"].clone();
        let (message, approval) =
            match policy::authorize(port, &token, &connector, message, &cancellation, &observed.fingerprint).await {
                Ok(authorized) => authorized,
                Err(error) => {
                    if !id.is_null() {
                        let _ = reply_tx
                            .send(
                                json!({"jsonrpc":"2.0","id":id,"error":{"code":-32603,"message":format!("{error:#}")}}),
                            )
                            .await;
                    }
                    cancellation.finish(&id);
                    continue;
                }
            };
        observed.track(&message, approval.as_deref());
        if let Err(error) = policy::check(port, &token, approval.as_deref()).await {
            observed.forget(&id);
            let _ = reply_tx
                .send(json!({"jsonrpc":"2.0","id":id,"error":{"code":-32603,"message":format!("{error:#}")}}))
                .await;
            continue;
        }
        let mut line = serde_json::to_vec(&message)?;
        line.push(b'\n');
        if let Err(error) = input.lock().await.write_all(&line).await {
            failure = Some(error.into());
            break;
        }
        if let Some(approval) = approval {
            let watched = observed.clone();
            let input = input.clone();
            let token = token.clone();
            let cancelled = cancellation.clone();
            let monitor = tokio::spawn(async move {
                while watched.pending(&id) {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    if cancelled.cancelled(&id) || policy::check(port, &token, Some(&approval)).await.is_err() {
                        let notification = json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":id,"reason":"Authorization revoked or caller cancelled"}});
                        if let Ok(mut line) = serde_json::to_vec(&notification) {
                            line.push(b'\n');
                            if let Err(error) = input.lock().await.write_all(&line).await {
                                tracing::debug!(%error,"connector already closed during cancellation");
                            }
                        }
                        break;
                    }
                }
            });
            tasks.track(&monitor);
        }
    }
    reader.abort();
    let _ = reader.await;
    drop(input);
    drop(child);
    if let Ok(Ok(Ok(()))) = tokio::time::timeout(std::time::Duration::from_secs(5), upstream).await {
    } else {
        tracing::debug!("connector output stopped before it could be fully drained")
    }
    drop(reply_tx);
    if let Ok(Ok(Ok(()))) = tokio::time::timeout(std::time::Duration::from_secs(35), writer).await {
    } else {
        tracing::warn!("MCP replies did not finish draining; missing receipts have an unknown outcome")
    }
    if let Some(error) = failure {
        return Err(error);
    }
    Ok(())
}
