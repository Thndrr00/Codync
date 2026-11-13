//! What a bot learns from a "no": each denied action is counted, and a kind of action denied twice
//! becomes a rule the bot proposes. Nothing changes on its own: the rule waits until the person
//! accepts it, and a dismissed proposal doesn't come back.
//!
//! Idea from agentes.chat's memory that learns from rejected approvals (`packages/engine`,
//! AGPL-3.0-only): the behavior, not the code.

use super::{Store, now_ms};
use crate::LockExt;
use anyhow::Result;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

/// How many denials of the same kind of action make a proposal.
pub const PROPOSE_AFTER: i64 = 2;

const COLS: &str = "id, bot_id, fingerprint, subject, kind, denials, status, rule, created_at, updated_at";

/// A rule the bot proposes after the person denied the same kind of action twice.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Lesson {
    pub id: String,
    pub bot_id: String,
    /// The kind of action, hashed: its tool kind and its command (or title).
    pub fingerprint: String,
    /// What was denied, as the person saw it.
    pub subject: String,
    pub kind: String,
    pub denials: i64,
    /// `proposed`, `accepted` or `dismissed`.
    pub status: String,
    /// The approval rule the bot proposes, as its config takes it.
    pub rule: serde_json::Value,
    pub created_at: i64,
    pub updated_at: i64,
}

pub(super) fn open(c: &Connection) -> Result<()> {
    c.execute_batch(
        "CREATE TABLE IF NOT EXISTS approval_lessons (
            id TEXT PRIMARY KEY,
            bot_id TEXT NOT NULL,
            fingerprint TEXT NOT NULL,
            subject TEXT NOT NULL,
            kind TEXT NOT NULL,
            denials INTEGER NOT NULL,
            status TEXT NOT NULL,
            rule TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            UNIQUE(bot_id, fingerprint)
        );
        CREATE INDEX IF NOT EXISTS approval_lessons_bot ON approval_lessons(bot_id, status, updated_at DESC);",
    )?;
    Ok(())
}

impl Store {
    /// Counts one more denial of an action for `bot`, and returns the lesson when this denial is the
    /// one that turns it into a proposal (see [`PROPOSE_AFTER`]).
    pub fn approval_denied(
        &self,
        bot: &str,
        kind: &str,
        subject: &str,
        fingerprint: &str,
        rule: &serde_json::Value,
    ) -> Result<Option<Lesson>> {
        let c = self.db.locked();
        let now = now_ms();
        c.execute(
            "INSERT INTO approval_lessons(id, bot_id, fingerprint, subject, kind, denials, status, rule, created_at, updated_at)
             VALUES(?1, ?2, ?3, ?4, ?5, 1, 'proposed', ?6, ?7, ?7)
             ON CONFLICT(bot_id, fingerprint) DO UPDATE SET
                denials = denials + 1, updated_at = ?7,
                subject = ?4, kind = ?5, rule = ?6
             WHERE approval_lessons.status = 'proposed'",
            params![uuid::Uuid::new_v4().to_string(), bot, fingerprint, subject, kind, rule.to_string(), now],
        )?;
        let lesson = lesson_row(&c, bot, fingerprint)?;
        Ok(lesson.filter(|l| l.status == "proposed" && l.denials == PROPOSE_AFTER))
    }

    pub fn approval_lesson(&self, id: &str) -> Option<Lesson> {
        let c = self.db.locked();
        c.query_row(&format!("SELECT {COLS} FROM approval_lessons WHERE id = ?1"), [id], read)
            .optional()
            .ok()
            .flatten()
    }

    /// A bot's lessons, newest first, all of them or only the ones in `status`.
    pub fn approval_lessons(&self, bot: &str, status: Option<&str>) -> Vec<Lesson> {
        let c = self.db.locked();
        let Ok(mut stmt) = c.prepare(&format!(
            "SELECT {COLS} FROM approval_lessons WHERE bot_id = ?1 AND (?2 IS NULL OR status = ?2)
             ORDER BY updated_at DESC"
        )) else {
            return Vec::new();
        };
        stmt.query_map(params![bot, status], read).map(|rows| rows.filter_map(std::result::Result::ok).collect()).unwrap_or_default()
    }

    /// Marks a lesson `accepted` or `dismissed`; `false` when it was already decided.
    pub fn decide_approval_lesson(&self, id: &str, status: &str) -> Result<bool> {
        let c = self.db.locked();
        let changed = c.execute(
            "UPDATE approval_lessons SET status = ?2, updated_at = ?3 WHERE id = ?1 AND status = 'proposed'",
            params![id, status, now_ms()],
        )?;
        Ok(changed > 0)
    }
}

fn lesson_row(c: &Connection, bot: &str, fingerprint: &str) -> Result<Option<Lesson>> {
    Ok(c.query_row(
        &format!("SELECT {COLS} FROM approval_lessons WHERE bot_id = ?1 AND fingerprint = ?2"),
        params![bot, fingerprint],
        read,
    )
    .optional()?)
}

fn read(r: &rusqlite::Row<'_>) -> rusqlite::Result<Lesson> {
    let rule: String = r.get(7)?;
    Ok(Lesson {
        id: r.get(0)?,
        bot_id: r.get(1)?,
        fingerprint: r.get(2)?,
        subject: r.get(3)?,
        kind: r.get(4)?,
        denials: r.get(5)?,
        status: r.get(6)?,
        rule: serde_json::from_str(&rule).unwrap_or(serde_json::Value::Null),
        created_at: r.get(8)?,
        updated_at: r.get(9)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::Path;

    fn store() -> Store {
        Store::open(Path::new(":memory:")).unwrap()
    }

    fn rule() -> serde_json::Value {
        json!({"pattern": "git push", "kind": "execute", "decision": "deny"})
    }

    #[test]
    fn the_second_denial_is_the_proposal() {
        let s = store();
        assert!(s.approval_denied("bot", "execute", "git push", "fp", &rule()).unwrap().is_none());
        let proposed = s.approval_denied("bot", "execute", "git push", "fp", &rule()).unwrap().expect("a proposal");
        assert_eq!(proposed.denials, PROPOSE_AFTER);
        assert_eq!(proposed.status, "proposed");
        assert_eq!(proposed.rule["decision"], "deny");
        assert!(s.approval_denied("bot", "execute", "git push", "fp", &rule()).unwrap().is_none(), "said once");
    }

    #[test]
    fn an_accepted_lesson_is_not_counted_again() {
        let s = store();
        s.approval_denied("bot", "execute", "git push", "fp", &rule()).unwrap();
        let proposed = s.approval_denied("bot", "execute", "git push", "fp", &rule()).unwrap().unwrap();
        assert!(s.decide_approval_lesson(&proposed.id, "accepted").unwrap());
        assert!(!s.decide_approval_lesson(&proposed.id, "dismissed").unwrap(), "decided once");
        s.approval_denied("bot", "execute", "git push", "fp", &rule()).unwrap();
        assert_eq!(s.approval_lesson(&proposed.id).unwrap().denials, PROPOSE_AFTER);
    }

    #[test]
    fn lessons_belong_to_their_bot_and_can_be_listed_by_status() {
        let s = store();
        s.approval_denied("a", "execute", "git push", "fp", &rule()).unwrap();
        s.approval_denied("a", "execute", "git push", "fp", &rule()).unwrap();
        assert_eq!(s.approval_lessons("a", None).len(), 1);
        assert_eq!(s.approval_lessons("a", Some("proposed")).len(), 1);
        assert_eq!(s.approval_lessons("a", Some("accepted")).len(), 0);
        assert!(s.approval_lessons("b", None).is_empty());
    }
}
