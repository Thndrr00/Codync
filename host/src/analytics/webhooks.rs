//! `webhook_received`: one delivery that reached a routine's webhook. Only what a chart needs: the
//! owner kind, the result, the HTTP status, the body size and the answer time. Never the URL, the
//! secret, the event type or the body.

use super::{Analytics, Event, capture};
use crate::LockExt as _;
use crate::hub::Hub;
use serde_json::json;

/// At most this many deliveries are reported each minute; a burst stays one small number.
const PER_MINUTE: u32 = 60;

impl Analytics {
    /// Whether a delivery may be reported now: [`PER_MINUTE`] each UTC minute, in memory only.
    fn allow_webhook(&self, now_ms: i64) -> bool {
        let minute = now_ms.div_euclid(60_000);
        let mut seen = self.webhooks.locked();
        if seen.0 != minute {
            *seen = (minute, 0);
        }
        if seen.1 >= PER_MINUTE {
            return false;
        }
        seen.1 += 1;
        true
    }
}

/// `webhook_received`: a delivery was answered with `status` after `ms`, for a routine (`local`: the
/// request came to this computer rather than through the relay). `body_bytes` is the size only.
pub fn webhook_received(hub: &Hub, local: bool, status: u16, body_bytes: usize, ms: i64) {
    if !hub.analytics.allow_webhook(crate::store::now_ms()) {
        return;
    }
    let result = if status < 400 { "ok" } else { "refused" };
    let bytes = i64::try_from(body_bytes).unwrap_or(i64::MAX).clamp(0, 16 * 1024 * 1024);
    capture(
        hub,
        Event::WebhookReceived,
        json!({
            "owner": "routine",
            "result": result,
            "status": status,
            "body_bytes": bytes,
            "ms": ms.clamp(0, 86_400_000),
            "local": local,
        })
        .as_object()
        .cloned()
        .unwrap_or_default(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LockExt;
    use std::path::Path;

    fn hub() -> (std::sync::Arc<Hub>, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("codync-webhook-event-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = crate::store::Store::open(Path::new(":memory:")).unwrap();
        let identity = crate::remote::identity::Identity::load_or_create(&dir).unwrap();
        (Hub::new(store, "h".into(), identity, "t".into(), 0), dir)
    }

    #[test]
    fn a_delivery_reports_only_counts_and_its_owner_kind() {
        let (hub, dir) = hub();
        webhook_received(&hub, false, 200, 512, 1_200);
        let sent = hub.store.pending_analytics().unwrap();
        let p = &sent[0]["properties"];
        assert_eq!((p["owner"].as_str(), p["result"].as_str()), (Some("routine"), Some("ok")));
        assert_eq!((p["status"].as_u64(), p["body_bytes"].as_u64()), (Some(200), Some(512)));
        assert_eq!((p["ms"].as_u64(), p["local"].as_bool()), (Some(1_200), Some(false)));
        for secret in ["url", "secret", "event", "body", "headers", "hook"] {
            assert!(p.get(secret).is_none(), "{secret} must never be reported");
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn sixty_deliveries_a_minute_are_reported_and_the_rest_dropped() {
        let (hub, dir) = hub();
        let now = 1_000 * 60_000;
        for i in 0..=PER_MINUTE {
            assert_eq!(hub.analytics.allow_webhook(now), i < PER_MINUTE);
        }
        assert!(hub.analytics.allow_webhook(now + 60_000), "the next minute starts over");
        // A reading never stores more than the cap.
        *hub.analytics.webhooks.locked() = (now.div_euclid(60_000), 0);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
