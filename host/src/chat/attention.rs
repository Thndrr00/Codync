//! Attention lifecycle: an open permission or question card stops being answerable once the request
//! it stands for has timed out, so a forgotten card neither blocks its bot nor stays "pending" forever.
//! Ported from OpenBot nightly's attention registry and reusing Codync's approval request timeout and
//! the card-sweep pattern from [`crate::chat::takeover`]. A card with no actor waiting on it is the
//! case the actor's own aging can't close; this sweep is the backstop.

use crate::hub::Hub;
use crate::store::Store;
use anyhow::Result;
use serde_json::Value;

/// How long a card waits before it is stale: the same request timeout the approval card's tool uses.
pub(super) const TTL_MS: i64 = crate::chat::manage::APPROVAL_TIMEOUT.as_millis() as i64;

/// The card's data once its time is up at `now`; `None` while it still waits or already ended. Pure so
/// the expiry of every kind of attention card has a test without a store or a host. `created_at` is
/// the card's own timestamp, so the caller need not read it from the store for the decision.
pub(super) fn lapsed(data: &Value, created_at: i64, now: i64) -> Option<Value> {
    if now.saturating_sub(created_at) < TTL_MS {
        return None;
    }
    let mut next = data.clone();
    if data["status"] == "pending" {
        // A permission card: close it the way a denied or expired approval closes.
        next["status"] = "expired".into();
    } else if data["memberQuestion"]["status"] == "pending" {
        // A group question card: its answers can no longer reach the bot that waited.
        next["memberQuestion"]["status"] = "expired".into();
    } else {
        return None;
    }
    Some(next)
}

/// Every still-pending attention card's id, data and timestamp.
fn waiting(store: &Store) -> Result<Vec<(String, Value, i64)>> {
    store.with_connection(|c| {
        let mut st = c.prepare(
            "SELECT id, data, created_at FROM entries
               WHERE (kind = 'permission' AND json_extract(data, '$.status') = 'pending')
                  OR (kind = 'notice' AND json_extract(data, '$.memberQuestion.status') = 'pending')",
        )?;
        let rows = st
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows.into_iter().filter_map(|(id, data, at)| Some((id, serde_json::from_str(&data).ok()?, at))).collect())
    })
}

/// The stale cards whose time is up at `now`, with their expired data.
pub(super) fn due(store: &Store, now: i64) -> Result<Vec<(String, Value)>> {
    Ok(waiting(store)?.into_iter().filter_map(|(id, data, at)| Some((id, lapsed(&data, at, now)?))).collect())
}

/// Closes the stale permission and question cards, for as long as the host runs.
pub fn sweep(hub: &Hub, now: i64) {
    match due(&hub.store, now) {
        Ok(expired) => {
            for (id, data) in expired {
                hub.set_entry(&id, &data);
            }
        }
        Err(error) => tracing::warn!(error = format!("{error:#}"), "couldn't look for stale attention cards"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_pending_permission_card_expires_after_the_request_timeout_only_once() {
        let card = json!({"status": "pending", "title": "Allow"});
        assert!(lapsed(&card, 1_000, 1_000 + TTL_MS - 1).is_none(), "still waiting just before the limit");
        let next = lapsed(&card, 1_000, 1_000 + TTL_MS).expect("expires at the limit");
        assert_eq!(next["status"], "expired");
        assert!(lapsed(&next, 1_000, 9_999_999_999).is_none(), "an expired card does not expire again");
    }

    #[test]
    fn a_question_card_expires_but_an_answered_one_does_not() {
        let question = json!({"memberQuestion": {"status": "pending", "answers": []}});
        assert_eq!(lapsed(&question, 0, TTL_MS).unwrap()["memberQuestion"]["status"], "expired");
        let answered = json!({"memberQuestion": {"status": "answered", "answers": ["yes"]}});
        assert!(lapsed(&answered, 0, i64::MAX).is_none());
        let permission_answered = json!({"status": "answered"});
        assert!(lapsed(&permission_answered, 0, i64::MAX).is_none());
    }
}
