//! Saved agent templates: a bounded, stamped kv blob on this computer.
//!
//! A template is a bot *setup* — a name, a short summary, instructions and safe
//! defaults — never a credential, an account, an image or a computer path. The
//! host sanitizes before it writes (see `market::templates`), so nothing unsafe
//! reaches this store in the first place.
//!
//! The slot is created on first read with `[]`, which is an idempotent migration:
//! a fresh database, a reopened one and several concurrent readers all end up
//! with the same empty list. Every write goes through `kv_set_stamped`, so the
//! global rev moves and synced clients pick the change up through the existing
//! mechanism. No schema version change and no new table are needed.

use super::{Store, now_ms};
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

use crate::LockExt as _;

/// The kv key holding the JSON array.
const SLOT: &str = "agentTemplates";
/// How many templates one computer keeps. The oldest saves go first.
const MAX_TEMPLATES: usize = 100;
/// A guard against a corrupt or hand-edited blob growing without bound.
const MAX_BLOB: usize = 4 * 1024 * 1024;

static LOCK: Mutex<()> = Mutex::new(());

/// Where a template is visible. Only a published one can be reached by a share ref.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TemplateVisibility {
    Local,
    Published,
}

/// A saved bot setup, safe to show on any client and to export as a share.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgentTemplate {
    /// A 22-character share-link id (see `market::templates::is_template_id`).
    pub id: String,
    pub name: String,
    /// A short summary, distinct from the instructions.
    pub description: String,
    /// The instructions a new bot starts with.
    pub instructions: String,
    pub avatar_color: String,
    pub avatar_shape: String,
    /// A catalog agent id, when the template fixes one.
    pub backend: Option<String>,
    /// A catalog model id, when the template fixes one.
    pub model: Option<String>,
    /// Skill ids on this computer, capped by `market::templates`.
    pub skills: Vec<String>,
    /// Connector ids on this computer, capped by `market::templates`.
    pub connectors: Vec<String>,
    pub visibility: TemplateVisibility,
    pub created_at: i64,
    pub updated_at: i64,
    /// Set when published, cleared when unpublished.
    pub published_at: Option<i64>,
}

impl Store {
    /// Every saved template, newest save last. Missing slot means none yet.
    pub fn templates(&self) -> Result<Vec<AgentTemplate>> {
        match self.kv_read(SLOT)? {
            None => {
                self.kv_set(SLOT, "[]")?;
                Ok(Vec::new())
            }
            Some(raw) => {
                let mut all: Vec<AgentTemplate> = serde_json::from_str(&raw).unwrap_or_default();
                if all.len() > MAX_TEMPLATES {
                    all.truncate(MAX_TEMPLATES);
                }
                Ok(all)
            }
        }
    }

    /// Inserts `template` or replaces the one with the same id, capping the list.
    pub fn save_template(&self, template: &AgentTemplate) -> Result<()> {
        let _guard = LOCK.locked();
        let mut all = self.templates()?;
        match all.iter().position(|t| t.id == template.id) {
            Some(index) => all[index] = template.clone(),
            None => {
                if all.len() >= MAX_TEMPLATES {
                    bail!("This computer already has {MAX_TEMPLATES} agent templates; remove one first.");
                }
                all.push(template.clone());
            }
        }
        let value = serde_json::to_string(&all)?;
        if value.len() > MAX_BLOB {
            bail!("The agent templates on this computer are too large.");
        }
        self.kv_set_stamped(SLOT, |_rev| Ok(value.clone()))
    }

    /// Removes a template by id; false when there was none.
    pub fn remove_template(&self, id: &str) -> Result<bool> {
        let _guard = LOCK.locked();
        let mut all = self.templates()?;
        let before = all.len();
        all.retain(|t| t.id != id);
        if all.len() == before {
            return Ok(false);
        }
        self.kv_set_stamped(SLOT, |_rev| Ok(serde_json::to_string(&all)?))
    }

    /// Stamps `now` as the update time of `template` and saves it.
    pub fn touch_template(&self, template: &mut AgentTemplate) -> Result<()> {
        template.updated_at = now_ms();
        self.save_template(template)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn template(id: &str) -> AgentTemplate {
        AgentTemplate {
            id: id.into(),
            name: "Research".into(),
            description: "A helper".into(),
            instructions: "Be careful".into(),
            avatar_color: "blue".into(),
            avatar_shape: "blob".into(),
            backend: None,
            model: None,
            skills: Vec::new(),
            connectors: Vec::new(),
            visibility: TemplateVisibility::Local,
            created_at: 1,
            updated_at: 1,
            published_at: None,
        }
    }

    #[test]
    fn a_fresh_slot_is_created_idempotently() {
        let store = Store::open(std::path::Path::new(":memory:")).expect("memory store");
        assert!(store.templates().expect("read").is_empty());
        assert_eq!(store.kv_get(SLOT).as_deref(), Some("[]"));
        // Reading again, and a second store on the same database, never duplicate or reset it.
        assert!(store.templates().expect("read again").is_empty());
        let dir = std::env::temp_dir().join(format!("codync-templates-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("t.db");
        {
            let first = Store::open(&path).expect("store");
            first.save_template(&template("abc")).expect("save");
        }
        let reopened = Store::open(&path).expect("reopen");
        let all = reopened.templates().expect("read");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "abc");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn saving_replaces_by_id_stamps_the_rev_and_respects_the_cap() {
        let store = Store::open(std::path::Path::new(":memory:")).expect("memory store");
        let rev = store.current_rev();
        let mut first = template("one");
        store.save_template(&first).expect("save");
        assert!(store.current_rev() > rev, "the blob is stamped");
        first.name = "Renamed".into();
        store.touch_template(&mut first).expect("touch");
        assert_eq!(store.templates().expect("read").len(), 1, "same id replaces, no duplicate");
        assert_eq!(store.templates().expect("read")[0].name, "Renamed");
        assert!(first.updated_at >= first.created_at);

        for i in 0..MAX_TEMPLATES - 1 {
            store.save_template(&template(&format!("t{i}"))).expect("save");
        }
        assert_eq!(store.templates().expect("read").len(), MAX_TEMPLATES);
        assert!(store.save_template(&template("one-too-many")).is_err(), "the list is bounded");
        assert!(store.remove_template("t0").expect("remove"));
        assert!(!store.remove_template("t0").expect("remove again"), "a second remove is a no-op");
        store.save_template(&template("now-fits")).expect("save");
        assert_eq!(store.templates().expect("read").len(), MAX_TEMPLATES);
    }
}
