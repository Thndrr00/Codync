//! Member accounts on this computer: an invited email and the sessions it opens
//! (docs/features/people.md).
//!
//! A person in `people` is the role and the scope; this module only holds the two secrets that let
//! someone act as that person. Both are stored as SHA-256 digests, never in the clear:
//!
//! - an **invite** is a one-time code the owner gives a member, for a phone or a desktop that pairs
//!   with it. Redeeming it through the pairing channel files that device under the person.
//! - a **session** is a bearer token a client at this computer exchanges for, backed by a device row
//!   whose `person_id` is the member. The host authorizes every call from it exactly like a paired
//!   device (`api::devices::permit` plus `people::caller_person`), so there is no second
//!   authorization system.

use super::{Store, logged, now_ms};
use crate::LockExt;
use anyhow::Result;
use rusqlite::{OptionalExtension, params};
use std::fmt::Write as _;

/// How long an invitation stays usable.
pub const INVITE_TTL_MS: i64 = 7 * 24 * 60 * 60 * 1000;
/// How long a member session token stays usable before the member signs in again.
pub const SESSION_TTL_MS: i64 = 30 * 24 * 60 * 60 * 1000;

/// An email invitation waiting for a member to redeem it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemberInvite {
    pub id: String,
    pub person_id: String,
    pub created_at: i64,
    pub expires_at: i64,
}

/// A member's session: the device row it authorizes as, and when it ends.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemberSession {
    pub device_key: String,
    pub person_id: String,
    pub created_at: i64,
    pub expires_at: i64,
}

/// What a secret is matched by: its digest, so the database never holds it.
fn digest(secret: &str) -> String {
    let mut out = String::with_capacity(64);
    for byte in crate::remote::crypto::sha256(&[secret.as_bytes()]) {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

pub(super) fn open(c: &rusqlite::Connection) -> Result<()> {
    c.execute_batch(
        "CREATE TABLE IF NOT EXISTS member_invites(
            id TEXT PRIMARY KEY, person_id TEXT NOT NULL, code_hash TEXT NOT NULL UNIQUE,
            created_at INTEGER NOT NULL, expires_at INTEGER NOT NULL);
         CREATE INDEX IF NOT EXISTS member_invites_person ON member_invites(person_id);
         CREATE TABLE IF NOT EXISTS member_sessions(
            token_hash TEXT PRIMARY KEY, device_key TEXT NOT NULL UNIQUE, person_id TEXT NOT NULL,
            created_at INTEGER NOT NULL, expires_at INTEGER NOT NULL);
         CREATE INDEX IF NOT EXISTS member_sessions_person ON member_sessions(person_id);",
    )?;
    Ok(())
}

fn row_invite(r: &rusqlite::Row<'_>) -> rusqlite::Result<MemberInvite> {
    Ok(MemberInvite { id: r.get(0)?, person_id: r.get(1)?, created_at: r.get(2)?, expires_at: r.get(3)? })
}

fn row_session(r: &rusqlite::Row<'_>) -> rusqlite::Result<MemberSession> {
    Ok(MemberSession { device_key: r.get(0)?, person_id: r.get(1)?, created_at: r.get(2)?, expires_at: r.get(3)? })
}

impl Store {
    /// A new invitation for `person`, replacing any they had: one live invite per person keeps the
    /// list a client shows honest, and an old code stops working the moment a new one is made alive.
    pub fn add_member_invite(&self, person: &str, code: &str, expires_at: i64) -> Result<MemberInvite> {
        let now = now_ms();
        let invite = MemberInvite {
            id: uuid::Uuid::new_v4().to_string(),
            person_id: person.to_owned(),
            created_at: now,
            expires_at,
        };
        let mut c = self.db.locked();
        let tx = c.transaction()?;
        tx.execute("DELETE FROM member_invites WHERE person_id = ?1", [person])?;
        tx.execute("DELETE FROM member_invites WHERE expires_at <= ?1", [now])?;
        tx.execute(
            "INSERT INTO member_invites(id, person_id, code_hash, created_at, expires_at)
             VALUES(?1, ?2, ?3, ?4, ?5)",
            params![invite.id, invite.person_id, digest(code), invite.created_at, invite.expires_at],
        )?;
        tx.commit()?;
        Ok(invite)
    }

    /// Every invitation still usable, oldest first.
    pub fn member_invites(&self) -> Result<Vec<MemberInvite>> {
        let c = self.db.locked();
        let mut st = c.prepare(
            "SELECT id, person_id, created_at, expires_at FROM member_invites
             WHERE expires_at > ?1 ORDER BY created_at",
        )?;
        Ok(st.query_map([now_ms()], row_invite)?.collect::<rusqlite::Result<_>>()?)
    }

    pub fn member_invite(&self, id: &str) -> Option<MemberInvite> {
        let c = self.db.locked();
        let row = c.query_row(
            "SELECT id, person_id, created_at, expires_at FROM member_invites WHERE id = ?1",
            [id],
            row_invite,
        );
        logged("member_invite", row.optional()).flatten()
    }

    /// Whether `code` is a live invitation, without using it up.
    pub fn member_invite_for_code(&self, code: &str) -> Option<MemberInvite> {
        if code.is_empty() {
            return None;
        }
        let c = self.db.locked();
        let row = c.query_row(
            "SELECT id, person_id, created_at, expires_at FROM member_invites
             WHERE code_hash = ?1 AND expires_at > ?2",
            params![digest(code), now_ms()],
            row_invite,
        );
        logged("member_invite_for_code", row.optional()).flatten()
    }

    /// Uses up the invitation `code` names, if it is still live.
    pub fn take_member_invite(&self, code: &str) -> Result<Option<MemberInvite>> {
        let Some(invite) = self.member_invite_for_code(code) else { return Ok(None) };
        let gone = self.db.locked().execute("DELETE FROM member_invites WHERE id = ?1", [&invite.id])?;
        Ok((gone > 0).then_some(invite))
    }

    /// Cancels one invitation; false when it wasn't there.
    pub fn remove_member_invite(&self, id: &str) -> Result<bool> {
        Ok(self.db.locked().execute("DELETE FROM member_invites WHERE id = ?1", [id])? > 0)
    }

    /// A session token for `person`, acting as `device`; true when it was saved.
    pub fn add_member_session(
        &self,
        device: &str,
        person: &str,
        token: &str,
        expires_at: i64,
    ) -> Result<MemberSession> {
        let now = now_ms();
        let session =
            MemberSession { device_key: device.to_owned(), person_id: person.to_owned(), created_at: now, expires_at };
        let mut c = self.db.locked();
        let tx = c.transaction()?;
        tx.execute("DELETE FROM member_sessions WHERE expires_at <= ?1", [now])?;
        // One token per device row: signing in again on the same machine replaces the old token.
        tx.execute(
            "DELETE FROM member_sessions WHERE device_key = ?1 OR token_hash = ?2",
            params![device, digest(token)],
        )?;
        tx.execute(
            "INSERT INTO member_sessions(token_hash, device_key, person_id, created_at, expires_at)
             VALUES(?1, ?2, ?3, ?4, ?5)",
            params![digest(token), session.device_key, session.person_id, session.created_at, session.expires_at],
        )?;
        tx.commit()?;
        Ok(session)
    }

    /// The session `token` opens, while it is live.
    pub fn member_session(&self, token: &str) -> Option<MemberSession> {
        if token.is_empty() {
            return None;
        }
        let c = self.db.locked();
        let row = c.query_row(
            "SELECT device_key, person_id, created_at, expires_at FROM member_sessions
             WHERE token_hash = ?1 AND expires_at > ?2",
            params![digest(token), now_ms()],
            row_session,
        );
        logged("member_session", row.optional()).flatten()
    }

    /// Every live member session, oldest first.
    pub fn member_sessions(&self) -> Result<Vec<MemberSession>> {
        let c = self.db.locked();
        let mut st = c.prepare(
            "SELECT device_key, person_id, created_at, expires_at FROM member_sessions
             WHERE expires_at > ?1 ORDER BY created_at",
        )?;
        Ok(st.query_map([now_ms()], row_session)?.collect::<rusqlite::Result<_>>()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::tests::temp_store;

    #[test]
    fn an_invitation_is_single_use_and_a_new_one_replaces_it() {
        let s = temp_store();
        let ana = s.add_person("Ana", Some("ana@example.com")).unwrap();
        let now = now_ms();
        let first = s.add_member_invite(&ana.id, "first-code", now + INVITE_TTL_MS).unwrap();
        assert_eq!(s.member_invites().unwrap(), [first.clone()]);
        assert!(s.member_invite_for_code("first-code").is_some());
        assert!(s.member_invite_for_code("nope").is_none());

        let second = s.add_member_invite(&ana.id, "second-code", now + INVITE_TTL_MS).unwrap();
        assert_ne!(first.id, second.id);
        assert_eq!(s.member_invites().unwrap(), [second.clone()], "one live invite per person");
        assert!(s.member_invite_for_code("first-code").is_none(), "the replaced code stops working");

        let taken = s.take_member_invite("second-code").unwrap().expect("live invite");
        assert_eq!(taken.person_id, ana.id);
        assert!(s.take_member_invite("second-code").unwrap().is_none(), "single use");
        assert!(s.member_invites().unwrap().is_empty());
    }

    #[test]
    fn an_expired_invitation_and_session_authorize_nothing() {
        let s = temp_store();
        let ana = s.add_person("Ana", None).unwrap();
        let past = now_ms() - 1;
        s.add_member_invite(&ana.id, "stale", past).unwrap();
        assert!(s.member_invite_for_code("stale").is_none());
        s.add_member_session("ana-desktop", &ana.id, "stale-token", past).unwrap();
        assert!(s.member_session("stale-token").is_none());
        assert!(s.member_sessions().unwrap().is_empty());
    }

    #[test]
    fn a_session_token_is_matched_by_digest_and_ends_with_its_device() {
        let s = temp_store();
        let ana = s.add_person("Ana", None).unwrap();
        s.db.locked()
            .execute(
                "INSERT INTO devices(key, name, platform, source, scopes, created_at)
                 VALUES('ana-desktop', 'Ana''s desktop', 'macos', 'local', '[\"control\"]', 1)",
                [],
            )
            .unwrap();
        let session = s.add_member_session("ana-desktop", &ana.id, "secret-token", now_ms() + SESSION_TTL_MS).unwrap();
        assert_eq!(session.person_id, ana.id);
        assert!(s.member_session("wrong").is_none());
        assert_eq!(s.member_session("secret-token").unwrap().device_key, "ana-desktop");

        // Signing in again on the same machine replaces its token.
        s.add_member_session("ana-desktop", &ana.id, "newer-token", now_ms() + SESSION_TTL_MS).unwrap();
        assert!(s.member_session("secret-token").is_none());
        assert!(s.member_session("newer-token").is_some());
        assert_eq!(s.member_sessions().unwrap().len(), 1);

        // The session is backed by its device: revoking the device ends the token with it.
        assert!(s.remove_device("ana-desktop").unwrap());
        assert!(!s.remove_device("ana-desktop").unwrap());
        assert!(s.member_session("newer-token").is_none());
    }
}
