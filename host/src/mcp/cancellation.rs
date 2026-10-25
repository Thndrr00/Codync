use crate::LockExt;
use serde_json::Value;
use std::collections::HashSet;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Default)]
pub(super) struct Cancellation {
    closed: AtomicBool,
    ids: Mutex<HashSet<String>>,
    ended: tokio::sync::Notify,
}

impl Cancellation {
    pub(super) fn observe(&self, message: &Value) {
        if message["method"] == "notifications/cancelled" && !message["params"]["requestId"].is_null() {
            self.ids.locked().insert(message["params"]["requestId"].to_string());
        }
    }
    pub(super) fn close(&self) {
        self.closed.store(true, Ordering::Release);
        self.ended.notify_waiters();
    }
    pub(super) async fn wait_closed(&self) {
        loop {
            let ended = self.ended.notified();
            tokio::pin!(ended);
            ended.as_mut().enable();
            if self.closed.load(Ordering::Acquire) {
                return;
            }
            ended.await;
        }
    }
    pub(super) fn cancelled(&self, id: &Value) -> bool {
        self.closed.load(Ordering::Acquire) || self.ids.locked().contains(&id.to_string())
    }
    pub(super) fn finish(&self, id: &Value) {
        self.ids.locked().remove(&id.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn cancellation_is_bound_to_the_exact_rpc_id_and_eof_cancels_everything() {
        let state = Cancellation::default();
        state.observe(&json!({"method":"notifications/cancelled","params":{"requestId":1}}));
        assert!(state.cancelled(&json!(1)));
        assert!(!state.cancelled(&json!("1")));
        assert!(!state.cancelled(&json!(2)));
        state.finish(&json!(1));
        assert!(!state.cancelled(&json!(1)));
        state.close();
        assert!(state.cancelled(&json!(2)));
    }
}
