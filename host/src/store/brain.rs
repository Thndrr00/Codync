//! The `brains` table: memory several bots share. A brain is a project's, or the Fleet one, which stands over the
//! shared-memory folder every bot used to share. Which brain a bot works in is `bots.brain` (and `BotConfig::brain`);
//! its members are the bots that point at it.

use super::{Store, logged, next_rev};
use crate::LockExt;
use anyhow::Result;
use codync_mem::layers::Brain;
use rusqlite::{OptionalExtension, params};

/// The Fleet brain's id: the one brain that always exists.
pub const FLEET: &str = "fleet";

/// A brain as stored.
type Row = (String, String);

/// A stored brain as the memory layers know it; the Fleet one keeps the shared folder.
fn brain((id, name): Row) -> Brain {
    if id == FLEET { Brain::fleet(&id, &name) } else { Brain::project(&id, &name) }
}

impl Store {
    /// Every brain, the Fleet one first.
    pub fn brains(&self) -> Result<Vec<Brain>> {
        let c = self.db.locked();
        let rows = c
            .prepare("SELECT id, name FROM brains ORDER BY id = ?1 DESC, name")?
            .query_map([FLEET], |r| Ok(brain((r.get(0)?, r.get(1)?))))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn brain(&self, id: &str) -> Result<Option<Brain>> {
        let c = self.db.locked();
        let row = c.query_row("SELECT id, name FROM brains WHERE id = ?1", [id], |r| Ok(brain((r.get(0)?, r.get(1)?))));
        Ok(row.optional()?)
    }

    /// The Fleet brain, which always exists.
    pub fn fleet_brain(&self) -> Result<Option<Brain>> {
        self.brain(FLEET)
    }

    /// A new brain named `name`, or `name` renaming the brain `id`; says which. The Fleet brain can be renamed but
    /// not made twice.
    pub fn save_brain(&self, id: Option<&str>, name: &str) -> Result<String> {
        let c = self.db.locked();
        let id = id.map_or_else(|| format!("brain-{}", uuid::Uuid::new_v4()), str::to_owned);
        let rev = next_rev(&c)?;
        c.execute(
            "INSERT INTO brains(id, name, created_at) VALUES(?1, ?2, ?3)
             ON CONFLICT(id) DO UPDATE SET name = ?2",
            params![id, name, rev],
        )?;
        Ok(id)
    }

    /// Deletes a brain other than the Fleet one and takes every bot in it out: they keep their own memory. What the
    /// brain held is gone with it. Says how many bots left it.
    pub fn delete_brain(&self, id: &str) -> Result<usize> {
        let c = self.db.locked();
        let left = c.execute("UPDATE bots SET brain = NULL WHERE brain = ?1", [id])?;
        c.execute("DELETE FROM brains WHERE id = ?1 AND id != ?2", params![id, FLEET])?;
        // Its layer's memory goes too: files, their versions, the facts' provenance and the marks of what the user
        // had forgotten there. The Fleet brain keeps the shared folder, whose layer is `''`.
        let layer = format!("brain:{id}");
        for table in ["memory_dream_files", "memory_dream_changes", "memory_dream_lines"] {
            c.execute(
                &format!("DELETE FROM {table} WHERE dream IN (SELECT id FROM memory_dreams WHERE bot = ?1)"),
                [layer.as_str()],
            )?;
        }
        c.execute("DELETE FROM memory_dreams WHERE bot = ?1", [layer.as_str()])?;
        c.execute("DELETE FROM memory_files WHERE bot = ?1", [layer.as_str()])?;
        c.execute("DELETE FROM memory_file_versions WHERE bot = ?1", [layer.as_str()])?;
        c.execute("DELETE FROM memory_facts WHERE bot = ?1", [layer.as_str()])?;
        c.execute("DELETE FROM memory_tombstones WHERE scope = ?1", [layer.as_str()])?;
        Ok(left)
    }

    /// The brains with how many bots work in each, the Fleet one included.
    pub fn brain_sizes(&self) -> Vec<(String, i64)> {
        let c = self.db.locked();
        let rows = c
            .prepare(
                "SELECT b.id, (SELECT COUNT(*) FROM bots WHERE bots.brain = b.id AND bots.deleted = 0)
                 FROM brains b ORDER BY b.id = ?1 DESC, b.name",
            )
            .and_then(|mut st| st.query_map([FLEET], |r| Ok((r.get(0)?, r.get(1)?)))?.collect());
        logged("brain sizes", rows).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::tests::temp_store;

    #[test]
    fn a_brain_is_created_joined_and_deleted() {
        let s = temp_store();
        let fleet = s.fleet_brain().unwrap().expect("the Fleet brain always exists");
        assert_eq!((fleet.id.as_str(), fleet.layer), (FLEET, None));
        let id = s.save_brain(None, "Weather").unwrap();
        let brain = s.brain(&id).unwrap().unwrap();
        assert_eq!(brain.layer.as_deref(), Some(format!("brain:{id}").as_str()));
        assert_eq!(s.brains().unwrap().first().map(|b| b.id.as_str()), Some(FLEET), "Fleet comes first");
        assert_eq!(s.brain_sizes(), [(FLEET.into(), 0), (id.clone(), 0)]);
        assert!(s.delete_brain(FLEET).is_ok(), "Fleet can be asked to go and stays");
        assert!(s.brain(FLEET).unwrap().is_some());
        let layer = format!("brain:{id}");
        s.memory_write(|c, at, rev| {
            c.execute(
                "INSERT INTO memory_files(bot, name, hash, size, mtime, description, tokens, version, updated_at, rev)
                 VALUES(?1, 'MEMORY.md', 'h', 1, ?2, NULL, 1, 1, ?2, ?3)",
                params![layer, at, rev],
            )?;
            c.execute("INSERT INTO memory_tombstones(scope, hash) VALUES(?1, 'f')", [layer.as_str()])?;
            Ok(())
        })
        .unwrap();
        s.delete_brain(&id).unwrap();
        assert!(s.brain(&id).unwrap().is_none());
        let (files, marks) = s
            .memory_read(|c| {
                Ok((
                    c.query_row("SELECT COUNT(*) FROM memory_files WHERE bot = ?1", [layer.as_str()], |r| r.get(0))?,
                    c.query_row("SELECT COUNT(*) FROM memory_tombstones WHERE scope = ?1", [layer.as_str()], |r| {
                        r.get(0)
                    })?,
                ))
            })
            .unwrap();
        assert_eq!((files, marks), (0, 0), "the brain's memory goes with it");
    }
}
