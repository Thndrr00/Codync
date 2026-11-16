//! Asking the person at this computer whether a device may control it, for the "Ask on this computer"
//! remote desktop policy. A request nobody answers in 30 seconds does not start, and a request replaces
//! the one before it: one person answers one session at a time.

use crate::LockExt;
use anyhow::{Result, bail};
use serde_json::{Value, json};
use std::sync::Mutex;

/// How long the person at this computer has to allow a session.
pub const ASK_WINDOW: i64 = 30_000;
/// How long an allowed session may take to start after the person allowed it.
const APPROVAL_WINDOW: i64 = 60_000;

#[derive(Clone)]
struct Request {
    id: String,
    device: String,
    until: i64,
}

#[derive(Clone)]
struct Approval {
    device: String,
    until: i64,
}

/// The pending question and the last answer, both expiring.
#[derive(Default)]
pub struct Consent {
    pending: Mutex<Option<Request>>,
    approved: Mutex<Option<Approval>>,
}

impl Consent {
    /// A new request for `device`, replacing an older one. Returns what the apps show.
    pub fn request(&self, device: &str) -> Value {
        let now = crate::store::now_ms();
        let request =
            Request { id: uuid::Uuid::new_v4().to_string(), device: device.to_owned(), until: now + ASK_WINDOW };
        let value = view(&request);
        *self.pending.locked() = Some(request);
        value
    }

    /// The pending question, if it hasn't expired.
    pub fn pending(&self) -> Option<Value> {
        let mut pending = self.pending.locked();
        if pending.as_ref().is_some_and(|r| r.until <= crate::store::now_ms()) {
            *pending = None;
        }
        pending.as_ref().map(view)
    }

    /// Answers the pending question; true when it was still waiting. An allowed session may start once.
    pub fn answer(&self, id: &str, allow: bool) -> Result<bool> {
        let now = crate::store::now_ms();
        let mut pending = self.pending.locked();
        let Some(request) = pending.as_ref() else { return Ok(false) };
        if request.id != id {
            bail!("That request is no longer waiting.");
        }
        let request = pending.take().expect("just checked");
        if request.until <= now {
            return Ok(false);
        }
        *self.approved.locked() = allow.then(|| Approval { device: request.device, until: now + APPROVAL_WINDOW });
        Ok(true)
    }

    /// Whether `device` was allowed just now, consuming the approval: one answer starts one session.
    pub fn take_approval(&self, device: &str) -> bool {
        let now = crate::store::now_ms();
        let mut approved = self.approved.locked();
        let matches = approved.as_ref().is_some_and(|a| a.device == device && a.until > now);
        if matches {
            *approved = None;
        }
        matches
    }

    /// Drops the pending question when its window closed unanswered.
    pub fn cancel(&self, id: &str) {
        let mut pending = self.pending.locked();
        if pending.as_ref().is_some_and(|r| r.id == id) {
            *pending = None;
        }
    }
}

fn view(request: &Request) -> Value {
    json!({"id": request.id, "device": request.device, "expiresAt": request.until, "seconds": ASK_WINDOW / 1000})
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LockExt;

    #[test]
    fn an_unanswered_request_expires_and_an_allowed_one_starts_one_session() {
        let consent = Consent::default();
        let request = consent.request("phone");
        let id = request["id"].as_str().unwrap().to_owned();
        assert!(consent.pending().is_some());
        assert!(consent.answer(&id, true).unwrap());
        assert!(consent.take_approval("phone"), "the person allowed this device");
        assert!(!consent.take_approval("phone"), "one answer starts one session");
        assert!(consent.pending().is_none());

        let denied = consent.request("tablet");
        let id = denied["id"].as_str().unwrap().to_owned();
        assert!(consent.answer(&id, false).unwrap());
        assert!(!consent.take_approval("tablet"));
    }

    #[test]
    fn an_old_request_is_replaced_and_a_late_answer_changes_nothing() {
        let consent = Consent::default();
        let first = consent.request("phone");
        let second = consent.request("phone");
        assert_ne!(first["id"], second["id"]);
        assert!(consent.answer(first["id"].as_str().unwrap(), true).is_err(), "it was replaced");
        assert!(!consent.take_approval("phone"));
    }

    #[test]
    fn an_expired_request_cannot_be_allowed() {
        let consent = Consent::default();
        let request = consent.request("phone");
        let id = request["id"].as_str().unwrap().to_owned();
        {
            let mut pending = consent.pending.locked();
            pending.as_mut().unwrap().until = 0;
        }
        assert!(!consent.answer(&id, true).unwrap());
        assert!(!consent.take_approval("phone"));
    }
}
