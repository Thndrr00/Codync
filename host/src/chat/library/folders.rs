//! The per-chat Library folder model: which folders a chat has and which of its library items are filed in each.
//! This file holds only data and rules, so every bound and validation can be tested without a database. `state.rs`
//! persists one JSON blob in kv and `tools.rs` offers the same operations to a bot's agent.

use anyhow::{Result, anyhow, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A chat holds at most this many folders; beyond it the tiles stop being scannable.
pub const MAX_FOLDERS: usize = 64;
/// Trimmed characters in a folder name, counting Unicode scalar values.
pub const MAX_NAME: usize = 60;
/// Items filed across all folders of one chat.
pub const MAX_FILED_ITEMS: usize = 1000;
/// Characters in a library item's key (`artifact:<id>`, `file:<entry>:<file>`, `attachment:<entry>:<file>`).
pub const MAX_ITEM_KEY: usize = 300;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Folder {
    pub id: String,
    pub name: String,
    pub created_at: i64,
}

/// A chat's folders and the item-key → folder-id filing. `rev` is the sync revision of the last change.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderState {
    #[serde(default)]
    pub rev: i64,
    #[serde(default)]
    pub folders: Vec<Folder>,
    #[serde(default)]
    pub items: BTreeMap<String, String>,
}

impl FolderState {
    /// A name as it is stored: trimmed, bounded, with no control characters.
    pub fn clean_name(name: &str) -> Result<String> {
        let name = name.trim();
        ensure!(!name.is_empty(), "a folder name is required");
        ensure!(name.chars().count() <= MAX_NAME, "a folder name is at most {MAX_NAME} characters");
        ensure!(!name.chars().any(char::is_control), "a folder name cannot contain control characters");
        Ok(name.to_owned())
    }

    /// Whether `key` names a library item this model can file. No filing happens for a malformed key.
    pub fn check_item(key: &str) -> Result<()> {
        ensure!(!key.is_empty() && key.chars().count() <= MAX_ITEM_KEY, "unknown library item");
        let parts: Vec<&str> = key.split(':').collect();
        let ok = match parts.as_slice() {
            [kind, id] => *kind == "artifact" && !id.is_empty(),
            [kind, entry, file] => (*kind == "file" || *kind == "attachment") && !entry.is_empty() && !file.is_empty(),
            _ => false,
        };
        ensure!(ok, "unknown library item");
        Ok(())
    }

    pub fn find(&self, id: &str) -> Option<&Folder> {
        self.folders.iter().find(|f| f.id == id)
    }

    pub fn folder_of(&self, key: &str) -> Option<&str> {
        self.items.get(key).map(String::as_str)
    }

    /// Adds a folder with a caller-supplied id (a UUID), at the end of the order.
    pub fn create(&mut self, id: String, name: &str, now: i64) -> Result<()> {
        let name = Self::clean_name(name)?;
        ensure!(self.folders.len() < MAX_FOLDERS, "a chat can have at most {MAX_FOLDERS} folders");
        ensure!(!self.folders.iter().any(|f| f.id == id), "a folder with that id already exists");
        ensure!(
            !self.folders.iter().any(|f| f.name.eq_ignore_ascii_case(&name)),
            "a folder with that name already exists"
        );
        self.folders.push(Folder { id, name, created_at: now });
        Ok(())
    }

    pub fn rename(&mut self, id: &str, name: &str) -> Result<()> {
        let name = Self::clean_name(name)?;
        let at = self.folders.iter().position(|f| f.id == id).ok_or_else(|| anyhow!("unknown folder"))?;
        ensure!(
            !self.folders.iter().any(|f| f.id != id && f.name.eq_ignore_ascii_case(&name)),
            "a folder with that name already exists"
        );
        self.folders[at].name = name;
        Ok(())
    }

    /// Places `id` immediately before `before_id`, or at the end when `before_id` is absent. Keeps the rest in order.
    pub fn move_folder(&mut self, id: &str, before_id: Option<&str>) -> Result<()> {
        let from = self.folders.iter().position(|f| f.id == id).ok_or_else(|| anyhow!("unknown folder"))?;
        let to = match before_id {
            None => self.folders.len() - 1,
            Some(before) => {
                ensure!(before != id, "a folder cannot be moved before itself");
                self.folders.iter().position(|f| f.id == before).ok_or_else(|| anyhow!("unknown folder"))?
            }
        };
        let folder = self.folders.remove(from);
        // Removing an earlier item shifts every later target one place left.
        let insert = if from < to { to - 1 } else { to };
        self.folders.insert(insert, folder);
        Ok(())
    }

    /// Removes a folder and returns the keys that became unfiled, so they still show under All files.
    pub fn delete(&mut self, id: &str) -> Result<Vec<String>> {
        let at = self.folders.iter().position(|f| f.id == id).ok_or_else(|| anyhow!("unknown folder"))?;
        self.folders.remove(at);
        let unfiled: Vec<String> =
            self.items.iter().filter(|(_, folder)| folder.as_str() == id).map(|(key, _)| key.clone()).collect();
        for key in &unfiled {
            self.items.remove(key);
        }
        Ok(unfiled)
    }

    /// Files an item in a folder. Re-filing the same item in the same folder is a no-op.
    pub fn file(&mut self, key: &str, folder_id: &str) -> Result<()> {
        Self::check_item(key)?;
        ensure!(self.find(folder_id).is_some(), "unknown folder");
        if self.items.get(key).is_some_and(|f| f == folder_id) {
            return Ok(());
        }
        if !self.items.contains_key(key) {
            ensure!(self.items.len() < MAX_FILED_ITEMS, "a chat can file at most {MAX_FILED_ITEMS} items");
        }
        self.items.insert(key.to_owned(), folder_id.to_owned());
        Ok(())
    }

    pub fn unfile(&mut self, key: &str) -> Result<()> {
        Self::check_item(key)?;
        self.items.remove(key);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(id: &str, name: &str, at: i64) -> Folder {
        Folder { id: id.into(), name: name.into(), created_at: at }
    }

    #[test]
    fn names_are_trimmed_bounded_and_free_of_control_characters() {
        assert_eq!(FolderState::clean_name("  Reports  ").unwrap(), "Reports");
        assert!(FolderState::clean_name("   ").is_err());
        assert!(FolderState::clean_name(&"x".repeat(MAX_NAME + 1)).is_err());
        assert!(FolderState::clean_name("a\nb").is_err());
        assert_eq!(FolderState::clean_name(&"é".repeat(MAX_NAME)).unwrap().chars().count(), MAX_NAME);
    }

    #[test]
    fn only_whole_library_item_keys_are_accepted() {
        for key in ["artifact:a1", "file:e1:f1", "attachment:e1:f2"] {
            assert!(FolderState::check_item(key).is_ok(), "{key}");
        }
        for key in ["", "artifact:", "file:e1", "note:e1:f1", "file::f1", "artifact:a:b", &"x".repeat(MAX_ITEM_KEY + 1)]
        {
            assert!(FolderState::check_item(key).is_err(), "{key}");
        }
    }

    #[test]
    fn folders_are_bounded_and_names_unique_ignoring_case() {
        let mut state = FolderState::default();
        assert!(state.create("1".into(), "Reports", 1).is_ok());
        assert!(state.create("2".into(), " reports ", 2).is_err(), "duplicate name");
        assert!(state.create("1".into(), "Other", 3).is_err(), "duplicate id");
        for n in 2..=MAX_FOLDERS {
            assert!(state.create(format!("f{n}"), format!("Folder {n}"), n as i64).is_ok());
        }
        assert!(state.create("overflow".into(), "Overflow", 99).is_err(), "folder count is bounded");
    }

    #[test]
    fn filing_moves_an_item_between_folders_and_unfiling_clears_it() {
        let mut state = FolderState::default();
        state.create("a".into(), "A", 1).unwrap();
        state.create("b".into(), "B", 2).unwrap();
        state.file("artifact:x", "a").unwrap();
        assert_eq!(state.folder_of("artifact:x"), Some("a"));
        state.file("artifact:x", "b").unwrap();
        assert_eq!(state.folder_of("artifact:x"), Some("b"));
        assert!(state.file("artifact:x", "b").is_ok(), "re-filing in place is harmless");
        state.unfile("artifact:x").unwrap();
        assert_eq!(state.folder_of("artifact:x"), None);
        assert!(state.file("bad", "a").is_err());
        assert!(state.file("artifact:x", "missing").is_err());
    }

    #[test]
    fn moving_a_folder_reorders_it_and_reports_the_new_order() {
        let mut state = FolderState::default();
        for (id, name) in [("a", "A"), ("b", "B"), ("c", "C")] {
            state.create(id.into(), name, 1).unwrap();
        }
        state.move_folder("c", Some("a")).unwrap();
        assert_eq!(state.folders.iter().map(|f| f.id.as_str()).collect::<Vec<_>>(), ["a", "c", "b"]);
        state.move_folder("a", None).unwrap();
        assert_eq!(state.folders.iter().map(|f| f.id.as_str()).collect::<Vec<_>>(), ["c", "b", "a"]);
        assert!(state.move_folder("a", Some("a")).is_err());
        assert!(state.move_folder("missing", None).is_err());
    }

    #[test]
    fn deleting_a_folder_unfiles_only_its_items() {
        let mut state = FolderState::default();
        state.create("a".into(), "A", 1).unwrap();
        state.create("b".into(), "B", 2).unwrap();
        state.file("artifact:1", "a").unwrap();
        state.file("artifact:2", "b").unwrap();
        assert_eq!(state.delete("a").unwrap(), ["artifact:1"]);
        assert_eq!(state.folder_of("artifact:1"), None);
        assert_eq!(state.folder_of("artifact:2"), Some("b"));
        assert!(state.delete("a").is_err());
    }

    #[test]
    fn the_blob_round_trips_with_older_blobs_reading_as_empty() {
        let mut state = FolderState::default();
        state.create("a".into(), "A", 1).unwrap();
        state.file("file:e:f", "a").unwrap();
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(serde_json::from_str::<FolderState>(&json).unwrap(), state);
        assert_eq!(serde_json::from_str::<FolderState>("{}").unwrap(), FolderState::default());
    }
}
