//! Keeps one `signal-cli` event stream open while the Signal channel is configured, and hands each
//! arriving envelope to [`super::ingress`]. A daemon that isn't running is a state, not an error
//! loop: the status says exactly what to start, and the stream reconnects when it appears.

use super::{Hub, SLOT, client, ingress, load};
use crate::{LockExt, Result};
use std::sync::Arc;
use std::time::Duration;

pub(super) async fn run(hub: Arc<Hub>) {
    loop {
        let ready = hub.store.kv_read(SLOT).ok().flatten().is_some();
        if ready {
            if let Err(error) = supervise(&hub).await {
                tracing::warn!(error = format!("{error:#}"), "Signal channel is not connected");
                let state = format!("notRunning: {error:#}");
                let _ = hub.store.kv_set("signal.state", &state);
            }
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

/// One connection lifetime: stays until the daemon closes, the configuration changes or the hub
/// stops. Returns what ended it.
async fn supervise(hub: &Arc<Hub>) -> Result<()> {
    crate::market::vault::unlock(hub.clone()).await?;
    let Some(c) = load(&hub.store)? else { return Ok(()) };
    let generation = c.generation.clone();
    let _ = hub.store.kv_set("signal.state", "starting");
    let (frames, mut received) = tokio::sync::mpsc::unbounded_channel();
    let daemon = c.daemon.clone();
    let connected = hub.clone();
    let listener = tokio::spawn(async move {
        client::listen(
            &daemon,
            || {
                let _ = connected.store.kv_set("signal.state", "ok");
            },
            |frame| frames.send(frame).map_err(|_| anyhow::anyhow!("Signal stream closed")),
        )
        .await
    });
    while let Some(frame) = received.recv().await {
        {
            let _configuration = hub.store.connector_lock.locked();
            let current = load(&hub.store)?;
            if current.as_ref().is_none_or(|current| current.generation != generation) {
                listener.abort();
                return Ok(());
            }
        }
        if let Err(error) = ingress::accept(hub, &c, &frame.params).await {
            tracing::warn!(chat = c.number.as_str(), error = format!("{error:#}"), "Signal envelope was not accepted");
        }
    }
    match listener.await {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error),
        Err(error) if error.is_cancelled() => Ok(()),
        Err(error) => Err(error.into()),
    }
}
