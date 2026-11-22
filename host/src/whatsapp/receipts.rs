//! Bridge write completion is not delivery: wait for the matching send acknowledgement.

use crate::LockExt;
use anyhow::{Result, anyhow, ensure};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::oneshot;

/// What a bridge event meant to a pending send.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Receipt {
    /// Not a send acknowledgement; the event is still a link event.
    None,
    /// A message id came back: the post was confirmed.
    Confirmed,
    /// WhatsApp did not confirm the post: a stalled send (triage-12196).
    Unconfirmed,
}

#[derive(Default)]
pub(super) struct Receipts {
    entries: Mutex<HashMap<String, oneshot::Sender<Result<()>>>>,
    /// A `wait` timed out: the post was never answered. The link watchdog picks this up.
    stalled: AtomicBool,
}

pub(super) struct Pending<'a> {
    book: &'a Receipts,
    pub id: String,
    receiver: Option<oneshot::Receiver<Result<()>>>,
}

impl Receipts {
    pub fn reserve(&self) -> Result<Pending<'_>> {
        let mut entries = self.entries.locked();
        ensure!(entries.len() < 64, "WhatsApp has too many pending sends");
        let id = uuid::Uuid::new_v4().to_string();
        let (sender, receiver) = oneshot::channel();
        entries.insert(id.clone(), sender);
        Ok(Pending { book: self, id, receiver: Some(receiver) })
    }

    /// Consumes a bridge send event and reports whether it confirmed the post.
    pub fn accept(&self, event: &Value) -> Receipt {
        if !matches!(event["type"].as_str(), Some("sent" | "error")) {
            return Receipt::None;
        }
        let Some(id) = event["id"].as_str() else { return Receipt::None };
        let confirmed = event["type"] == "sent" && event["messageId"].as_str().is_some_and(|id| !id.is_empty());
        let pending = self.entries.locked().remove(id);
        // Provider errors can contain personal data. Keep them out of routine logs.
        if let Some(sender) = pending {
            let result = if confirmed { Ok(()) } else { Err(anyhow!("WhatsApp did not confirm the send")) };
            let _ = sender.send(result);
            if confirmed { Receipt::Confirmed } else { Receipt::Unconfirmed }
        } else {
            Receipt::None
        }
    }

    /// A post that never got an answer is one stop for the self-reconnect watchdog.
    pub fn take_stalled(&self) -> bool {
        self.stalled.swap(false, Ordering::SeqCst)
    }

    pub fn disconnect(&self) {
        for (_, sender) in self.entries.locked().drain() {
            let _ = sender.send(Err(anyhow!("WhatsApp disconnected; the send outcome is unknown")));
        }
    }
}

impl Pending<'_> {
    pub async fn wait(mut self) -> Result<()> {
        let receiver = self.receiver.take().ok_or_else(|| anyhow!("WhatsApp receipt was already consumed"))?;
        tokio::time::timeout(std::time::Duration::from_secs(45), receiver)
            .await
            .map_err(|_| {
                self.book.stalled.store(true, Ordering::SeqCst);
                anyhow!("WhatsApp did not acknowledge the send; its outcome is unknown")
            })?
            .map_err(|_| anyhow!("WhatsApp acknowledgement was interrupted; its outcome is unknown"))?
    }
}

impl Drop for Pending<'_> {
    fn drop(&mut self) {
        self.book.entries.locked().remove(&self.id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn acknowledgements_are_correlated_and_require_a_message_id() {
        let book = Receipts::default();
        let first = book.reserve().unwrap();
        let second = book.reserve().unwrap();
        assert_eq!(book.accept(&json!({"type":"sent","id":second.id,"messageId":"receipt"})), Receipt::Confirmed);
        assert_eq!(book.accept(&json!({"type":"sent","id":first.id,"messageId":null})), Receipt::Unconfirmed);
        assert!(second.wait().await.is_ok());
        assert!(first.wait().await.is_err());
    }

    #[tokio::test]
    async fn an_unconfirmed_post_is_reported_but_an_unrelated_event_is_not() {
        let book = Receipts::default();
        assert_eq!(book.accept(&json!({"type":"state","state":"connected"})), Receipt::None);
        assert!(!book.take_stalled());
        let pending = book.reserve().unwrap();
        assert_eq!(book.accept(&json!({"type":"error","id":pending.id})), Receipt::Unconfirmed);
        assert!(pending.wait().await.is_err());
        assert_eq!(book.accept(&json!({"type":"error","id":"unknown"})), Receipt::None);
    }

    #[tokio::test]
    async fn cancelled_or_disconnected_sends_leave_no_receipt_waiters() {
        let book = Receipts::default();
        let cancelled = book.reserve().unwrap();
        drop(cancelled);
        assert!(book.entries.locked().is_empty());
        let disconnected = book.reserve().unwrap();
        book.disconnect();
        assert!(disconnected.wait().await.is_err());
        assert!(book.entries.locked().is_empty());
    }
}
