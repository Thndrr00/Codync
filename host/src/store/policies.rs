//! This computer's policy rules (`docs/features/policy-engine.md`): a named CEL expression with an
//! effect (`allow` or `deny`) and a message, saved in the order the person arranged them, versioned on
//! every change and enabled or disabled without being lost.
//!
//! The store owns the rule's shape and limits; compiling and evaluating the expression is
//! `crate::policy`'s, so a rule is refused before it is saved when its expression is broken.

use super::{Store, now_ms};
use crate::LockExt;
use anyhow::{Result, bail};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Rules one computer keeps.
pub const MAX_POLICIES: usize = 100;
/// Longest rule name, in characters.
pub const MAX_NAME: usize = 80;
/// Longest expression, in characters.
pub const MAX_EXPRESSION: usize = 500;
/// Longest message, in characters.
pub const MAX_MESSAGE: usize = 200;
/// Versions kept per rule.
const KEEP_VERSIONS: i64 = 20;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PolicyEffect {
    /// Permits what the expression matches.
    Allow,
    /// Refuses what the expression matches, before any allow rule is looked at.
    Deny,
}

impl PolicyEffect {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Deny => "deny",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "allow" => Some(Self::Allow),
            "deny" => Some(Self::Deny),
            _ => None,
        }
    }
}

/// One rule as it is saved.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Policy {
    /// Empty to add a new rule.
    #[serde(default)]
    pub id: String,
    pub name: String,
    pub expression: String,
    pub effect: PolicyEffect,
    #[serde(default)]
    pub message: String,
    #[serde(default = "enabled_by_default")]
    pub enabled: bool,
    /// 1 when it was first saved; one more on every change (`policyVersions`).
    #[serde(default)]
    pub version: i64,
    #[serde(default)]
    pub updated_at: i64,
}

fn enabled_by_default() -> bool {
    true
}

pub(super) fn open(db: &Connection) -> Result<()> {
    db.execute_batch(include_str!("policies.sql"))?;
    Ok(())
}

/// The rule's own bounds: what can be stored at all, before its expression is compiled.
pub fn check(policy: &Policy) -> Result<()> {
    let name = policy.name.trim();
    if name.is_empty() || name.chars().count() > MAX_NAME {
        bail!("A policy's name is 1 to {MAX_NAME} characters");
    }
    let expression = policy.expression.trim();
    if expression.is_empty() || expression.chars().count() > MAX_EXPRESSION {
        bail!("A policy's expression is 1 to {MAX_EXPRESSION} characters");
    }
    if policy.message.chars().count() > MAX_MESSAGE {
        bail!("A policy's message is at most {MAX_MESSAGE} characters");
    }
    Ok(())
}

fn row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Policy> {
    let effect: String = r.get(3)?;
    Ok(Policy {
        id: r.get(0)?,
        name: r.get(1)?,
        expression: r.get(2)?,
        effect: match effect.as_str() {
            "allow" => PolicyEffect::Allow,
            _ => PolicyEffect::Deny,
        },
        message: r.get(4)?,
        enabled: r.get::<_, i64>(5)? == 1,
        version: r.get(6)?,
        updated_at: r.get(7)?,
    })
}

const COLUMNS: &str = "id,name,expression,effect,message,enabled,version,updated_at";

fn one(db: &Connection, id: &str) -> Result<Option<Policy>> {
    Ok(db.query_row(&format!("SELECT {COLUMNS} FROM policies WHERE id=?1"), [id], row).optional()?)
}

impl Store {
    /// Every saved rule, enabled or not, in the order the person put them.
    pub fn policies(&self) -> Result<Vec<Policy>> {
        let db = self.db.locked();
        let mut query = db.prepare(&format!("SELECT {COLUMNS} FROM policies ORDER BY position, name"))?;
        let rows = query.query_map([], row)?;
        Ok(rows.collect::<rusqlite::Result<Vec<Policy>>>()?)
    }

    pub fn policy(&self, id: &str) -> Result<Option<Policy>> {
        one(&self.db.locked(), id)
    }

    /// Adds (`id` empty) or updates a rule, one version up, keeping the last 20 versions.
    pub fn save_policy(&self, policy: Policy) -> Result<Policy> {
        check(&policy)?;
        // Stored as trimmed text, and returned the same way the next read gives it back.
        let policy = Policy {
            name: policy.name.trim().to_owned(),
            expression: policy.expression.trim().to_owned(),
            message: policy.message.trim().to_owned(),
            ..policy
        };
        let mut db = self.db.locked();
        let tx = db.transaction()?;
        let now = now_ms();
        let saved = if policy.id.trim().is_empty() {
            let count: i64 = tx.query_row("SELECT COUNT(*) FROM policies", [], |r| r.get(0))?;
            if count >= i64::try_from(MAX_POLICIES).unwrap_or(i64::MAX) {
                bail!("A computer keeps at most {MAX_POLICIES} policies");
            }
            let id = format!("pol_{}", &uuid::Uuid::new_v4().simple().to_string()[..12]);
            let position: i64 = tx.query_row("SELECT COALESCE(MAX(position),0)+1 FROM policies", [], |r| r.get(0))?;
            tx.execute(
                "INSERT INTO policies(id,name,expression,effect,message,enabled,position,version,created_at,updated_at)
                 VALUES(?1,?2,?3,?4,?5,?6,?7,1,?8,?8)",
                params![
                    id,
                    policy.name.trim(),
                    policy.expression.trim(),
                    policy.effect.as_str(),
                    policy.message.trim(),
                    i64::from(policy.enabled),
                    position,
                    now
                ],
            )?;
            Policy { id, version: 1, updated_at: now, ..policy }
        } else {
            let current = one(&tx, &policy.id)?.ok_or_else(|| anyhow::anyhow!("Unknown policy"))?;
            let version = current.version + 1;
            tx.execute(
                "UPDATE policies SET name=?2,expression=?3,effect=?4,message=?5,enabled=?6,version=?7,updated_at=?8
                 WHERE id=?1",
                params![
                    policy.id,
                    policy.name.trim(),
                    policy.expression.trim(),
                    policy.effect.as_str(),
                    policy.message.trim(),
                    i64::from(policy.enabled),
                    version,
                    now
                ],
            )?;
            Policy { version, updated_at: now, ..policy }
        };
        tx.execute(
            "INSERT INTO policy_versions(policy_id,version,body,saved_at) VALUES(?1,?2,?3,?4)",
            params![saved.id, saved.version, serde_json::to_string(&saved)?, now],
        )?;
        tx.execute(
            "DELETE FROM policy_versions WHERE policy_id=?1 AND version<=?2",
            params![saved.id, saved.version - KEEP_VERSIONS],
        )?;
        tx.commit()?;
        Ok(saved)
    }

    /// Takes a rule out for good, versions and all. False when there was none.
    pub fn remove_policy(&self, id: &str) -> Result<bool> {
        Ok(self.db.locked().execute("DELETE FROM policies WHERE id=?1", [id])? == 1)
    }

    /// Switches a rule on or off; its expression and place in the list stay.
    pub fn set_policy_enabled(&self, id: &str, enabled: bool) -> Result<Policy> {
        let mut policy = self.policy(id)?.ok_or_else(|| anyhow::anyhow!("Unknown policy"))?;
        policy.enabled = enabled;
        self.save_policy(policy)
    }

    /// A rule's versions, newest first: what it was saved as, each time.
    pub fn policy_versions(&self, id: &str) -> Result<Vec<Value>> {
        let db = self.db.locked();
        let mut query =
            db.prepare("SELECT version,body,saved_at FROM policy_versions WHERE policy_id=?1 ORDER BY version DESC")?;
        let rows = query.query_map([id], |r| {
            let version: i64 = r.get(0)?;
            let body: String = r.get(1)?;
            let saved_at: i64 = r.get(2)?;
            Ok(json!({"version": version, "savedAt": saved_at, "policy": serde_json::from_str::<Value>(&body).unwrap_or(Value::Null)}))
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<Value>>>()?)
    }
}
