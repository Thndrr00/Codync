//! The record of every approval card: what it asked for, who answered it, and the one moment its
//! answer reached the agent. The decision is written once and claimed once, so a repeated tap, a
//! second device or a replayed request can never authorize the same action twice.
//!
//! Idea from agentes.chat's single-use approvals (`packages/engine`, AGPL-3.0-only): the behavior,
//! not the code.

use super::{Store, now_ms};
use crate::LockExt;
use anyhow::Result;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

/// Longest subject kept, in characters: enough to recognize the action, never a whole diff.
pub const MAX_SUBJECT: usize = 500;
/// Decisions older than this go, like the audit log's 90 days.
const KEEP_MS: i64 = 90 * 24 * 60 * 60 * 1000;

const COLS: &str = "entry_id, bot_id, nonce, fingerprint, subject, kind, decision, option_id, actor, \
                    asked_at, decided_at, consumed_at";

/// One card's approval: what it asked for, who answered, and whether that answer was used.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Approval {
    /// The card entry's id.
    pub entry_id: String,
    pub bot_id: String,
    /// Identifies this card's answer; a replay of the same card never counts twice.
    pub nonce: String,
    /// What the card asked for, hashed: its tool kind, command or title, folder and diffs.
    pub fingerprint: String,
    /// What the person saw, for the record.
    pub subject: String,
    pub kind: String,
    /// `allow`, `deny` or `cancelled`; absent while the card waits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub option_id: Option<String>,
    /// Who answered: a device's name, or this computer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor: Option<String>,
    pub asked_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decided_at: Option<i64>,
    /// When the answer was handed to the agent: the one moment it takes effect.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consumed_at: Option<i64>,
}

pub(super) fn open(c: &Connection) -> Result<()> {
    c.execute_batch(
        "CREATE TABLE IF NOT EXISTS approval_decisions (
            entry_id TEXT PRIMARY KEY,
            bot_id TEXT NOT NULL,
            nonce TEXT NOT NULL,
            fingerprint TEXT NOT NULL,
            subject TEXT NOT NULL,
            kind TEXT NOT NULL,
            decision TEXT,
            option_id TEXT,
            actor TEXT,
            asked_at INTEGER NOT NULL,
            decided_at INTEGER,
            consumed_at INTEGER
        );
        CREATE INDEX IF NOT EXISTS approval_decisions_bot ON approval_decisions(bot_id, asked_at DESC);
        CREATE INDEX IF NOT EXISTS approval_decisions_action
            ON approval_decisions(fingerprint, asked_at DESC);",
    )?;
    c.execute("DELETE FROM approval_decisions WHERE asked_at < ?", params![now_ms() - KEEP_MS])?;
    Ok(())
}

impl Store {
    /// Records that card `entry_id` asked for `subject`. Asking twice about the same card keeps
    /// the first record and its nonce: a card shown again is the same approval, not a new one.
    pub fn approval_asked(
        &self,
        entry_id: &str,
        bot_id: &str,
        kind: &str,
        subject: &str,
        fingerprint: &str,
    ) -> Result<Approval> {
        let c = self.db.locked();
        c.execute(
            "INSERT INTO approval_decisions(entry_id, bot_id, nonce, fingerprint, subject, kind, asked_at)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7) ON CONFLICT(entry_id) DO NOTHING",
            params![entry_id, bot_id, nonce(), fingerprint, subject, kind, now_ms()],
        )?;
        row(&c, entry_id)?.ok_or_else(|| anyhow::anyhow!("the approval record vanished"))
    }

    /// The one decision on a card. `Ok(None)` when it was already decided: the first answer is the
    /// one that counts, and a later different one changes nothing.
    pub fn approval_decide(
        &self,
        entry_id: &str,
        bot_id: &str,
        decision: &str,
        option_id: Option<&str>,
        actor: Option<&str>,
    ) -> Result<Option<Approval>> {
        let c = self.db.locked();
        // A card from before the ledger is adopted here, so its answer still leaves a record.
        c.execute(
            "INSERT INTO approval_decisions(entry_id, bot_id, nonce, fingerprint, subject, kind, asked_at)
             VALUES(?1, ?2, ?3, '', '', '', ?4) ON CONFLICT(entry_id) DO NOTHING",
            params![entry_id, bot_id, nonce(), now_ms()],
        )?;
        let changed = c.execute(
            "UPDATE approval_decisions SET decision = ?2, option_id = ?3, actor = ?4, decided_at = ?5
             WHERE entry_id = ?1 AND decision IS NULL",
            params![entry_id, decision, option_id, actor, now_ms()],
        )?;
        Ok(if changed == 0 { None } else { row(&c, entry_id)? })
    }

    /// Claims a card's answer for the one time it takes effect: `true` only for the first caller.
    /// A card the ledger never saw (one from before it existed) is claimed rather than refused.
    pub fn approval_consume(&self, entry_id: &str) -> Result<bool> {
        let c = self.db.locked();
        let changed = c.execute(
            "UPDATE approval_decisions SET consumed_at = ?2 WHERE entry_id = ?1 AND consumed_at IS NULL",
            params![entry_id, now_ms()],
        )?;
        if changed > 0 {
            return Ok(true);
        }
        Ok(row(&c, entry_id)?.is_none())
    }

    pub fn approval(&self, entry_id: &str) -> Option<Approval> {
        row(&self.db.locked(), entry_id).ok().flatten()
    }

    /// The newest decisions of a bot, newest first: what it was allowed to do and who said so.
    pub fn approvals_recent(&self, bot_id: &str, limit: usize) -> Vec<Approval> {
        let c = self.db.locked();
        let Ok(mut stmt) = c.prepare(&format!(
            "SELECT {COLS} FROM approval_decisions WHERE bot_id = ?1 ORDER BY asked_at DESC LIMIT ?2"
        )) else {
            return Vec::new();
        };
        stmt.query_map(params![bot_id, i64::try_from(limit).unwrap_or(i64::MAX)], read)
            .map(|rows| rows.filter_map(std::result::Result::ok).collect())
            .unwrap_or_default()
    }
}

fn row(c: &Connection, entry_id: &str) -> Result<Option<Approval>> {
    Ok(c.query_row(&format!("SELECT {COLS} FROM approval_decisions WHERE entry_id = ?1"), [entry_id], read)
        .optional()?)
}

fn read(r: &rusqlite::Row<'_>) -> rusqlite::Result<Approval> {
    Ok(Approval {
        entry_id: r.get(0)?,
        bot_id: r.get(1)?,
        nonce: r.get(2)?,
        fingerprint: r.get(3)?,
        subject: r.get(4)?,
        kind: r.get(5)?,
        decision: r.get(6)?,
        option_id: r.get(7)?,
        actor: r.get(8)?,
        asked_at: r.get(9)?,
        decided_at: r.get(10)?,
        consumed_at: r.get(11)?,
    })
}

/// A fresh nonce for a card: 16 random bytes in hex.
fn nonce() -> String {
    let mut bytes = [0u8; 16];
    let _ = getrandom::fill(&mut bytes);
    bytes.iter().fold(String::with_capacity(32), |mut s, b| {
        use std::fmt::Write as _;
        let _ = write!(s, "{b:02x}");
        s
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn store() -> Store {
        Store::open(Path::new(":memory:")).unwrap()
    }

    #[test]
    fn a_card_keeps_one_nonce_when_it_is_recorded_twice() {
        let s = store();
        let first = s.approval_asked("e1", "bot", "execute", "git push", "fp").unwrap();
        let again = s.approval_asked("e1", "bot", "execute", "git push", "fp").unwrap();
        assert_eq!(first.nonce, again.nonce);
        assert_eq!(again.decision, None);
    }

    #[test]
    fn the_first_decision_is_the_only_one() {
        let s = store();
        s.approval_asked("e1", "bot", "execute", "git push", "fp").unwrap();
        let allowed = s.approval_decide("e1", "bot", "allow", Some("allow_once"), Some("iPhone")).unwrap();
        assert_eq!(allowed.unwrap().actor.as_deref(), Some("iPhone"));
        assert!(s.approval_decide("e1", "bot", "deny", Some("reject_once"), Some("Mac")).unwrap().is_none());
        let kept = s.approval("e1").unwrap();
        assert_eq!(kept.decision.as_deref(), Some("allow"));
        assert_eq!(kept.actor.as_deref(), Some("iPhone"));
    }

    #[test]
    fn an_answer_takes_effect_once() {
        let s = store();
        s.approval_asked("e1", "bot", "execute", "git push", "fp").unwrap();
        s.approval_decide("e1", "bot", "allow", Some("allow_once"), None).unwrap();
        assert!(s.approval_consume("e1").unwrap());
        assert!(!s.approval_consume("e1").unwrap());
        assert!(s.approval("e1").unwrap().consumed_at.is_some());
    }

    #[test]
    fn a_card_the_ledger_never_saw_is_claimed_rather_than_refused() {
        let s = store();
        assert!(s.approval_consume("old").unwrap());
    }

    #[test]
    fn a_card_from_before_the_ledger_still_leaves_a_record() {
        let s = store();
        let decided = s.approval_decide("old", "bot", "allow", Some("allow_once"), Some("This computer")).unwrap();
        assert_eq!(decided.unwrap().decision.as_deref(), Some("allow"));
    }

    #[test]
    fn a_decision_survives_reopening_the_database() {
        let dir = std::env::temp_dir().join(format!("codync-approvals-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("t.db");
        {
            let s = Store::open(&path).unwrap();
            s.approval_asked("e1", "bot", "execute", "git push", "fp").unwrap();
            s.approval_decide("e1", "bot", "deny", Some("reject_once"), Some("iPhone")).unwrap();
        }
        let s = Store::open(&path).unwrap();
        let kept = s.approval("e1").unwrap();
        assert_eq!(kept.decision.as_deref(), Some("deny"));
        assert_eq!(kept.actor.as_deref(), Some("iPhone"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
