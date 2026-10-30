//! Private search shortcuts belong to the host, isolated by the authenticated device.
use super::{OptionalExtension, Store};
use crate::LockExt;
use anyhow::{Result, ensure};
use rusqlite::params;
use serde::{Deserialize, Serialize};

const LIMIT: usize = 20;

#[derive(Clone, Copy, Default, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SearchScope {
    #[default]
    All,
    Main,
    Threads,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedSearch {
    pub id: String,
    pub query: String,
    pub scope: SearchScope,
}

pub fn search_terms(query: &str) -> Result<Vec<String>> {
    ensure!(!query.trim().is_empty() && query.chars().count() <= 256, "query must contain 1–256 characters");
    let terms: Vec<String> = query.split_whitespace().map(str::to_owned).collect();
    ensure!(terms.len() <= 8, "query may contain at most 8 terms");
    Ok(terms)
}

impl Store {
    pub fn saved_searches(&self, device: &str) -> Result<Vec<SavedSearch>> {
        let key = format!("saved-searches.v1:{device}");
        let value = self.kv_read(&key)?;
        value.map_or_else(|| Ok(Vec::new()), |value| Ok(serde_json::from_str(&value)?))
    }

    /// One database lock covers reading, validation and writing; simultaneous saves cannot overwrite each other.
    pub fn change_saved_searches(
        &self,
        device: &str,
        query: Option<(&str, SearchScope)>,
        remove: Option<&str>,
    ) -> Result<Vec<SavedSearch>> {
        let key = format!("saved-searches.v1:{device}");
        let c = self.db.locked();
        let value: Option<String> =
            c.query_row("SELECT v FROM kv WHERE k = ?1", [&key], |row| row.get(0)).optional()?;
        let mut searches: Vec<SavedSearch> = match value {
            Some(value) => serde_json::from_str(&value)?,
            None => Vec::new(),
        };
        if let Some((query, scope)) = query {
            let query = query.trim();
            search_terms(query)?;
            if !searches.iter().any(|saved| saved.scope == scope && saved.query.to_lowercase() == query.to_lowercase())
            {
                ensure!(searches.len() < LIMIT, "You can save up to 20 searches. Remove one before saving another.");
                searches
                    .insert(0, SavedSearch { id: uuid::Uuid::new_v4().to_string(), query: query.to_owned(), scope });
            }
        }
        if let Some(id) = remove {
            searches.retain(|saved| saved.id != id);
        }
        c.execute(
            "INSERT INTO kv(k,v) VALUES(?1,?2) ON CONFLICT(k) DO UPDATE SET v = excluded.v",
            params![key, serde_json::to_string(&searches)?],
        )?;
        Ok(searches)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcuts_are_private_deduplicated_and_bounded() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let saved = store.change_saved_searches("phone", Some((" release ", SearchScope::Threads)), None).unwrap();
        assert_eq!(saved[0].query, "release");
        assert!(store.saved_searches("local").unwrap().is_empty());
        assert_eq!(
            store.change_saved_searches("phone", Some(("RELEASE", SearchScope::Threads)), None).unwrap().len(),
            1
        );
        for i in 1..LIMIT {
            store.change_saved_searches("phone", Some((&format!("query{i}"), SearchScope::All)), None).unwrap();
        }
        assert!(store.change_saved_searches("phone", Some(("overflow", SearchScope::All)), None).is_err());
        assert_eq!(store.saved_searches("phone").unwrap().len(), LIMIT);
        store.change_saved_searches("phone", None, Some(&saved[0].id)).unwrap();
        assert_eq!(store.saved_searches("phone").unwrap().len(), LIMIT - 1);
        assert!(store.change_saved_searches("phone", Some((" ", SearchScope::All)), None).is_err());
    }

    #[test]
    fn invalid_saved_state_is_not_overwritten() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        store.kv_set("saved-searches.v1:phone", "broken").unwrap();
        assert!(store.saved_searches("phone").is_err());
        assert!(store.change_saved_searches("phone", Some(("new", SearchScope::All)), None).is_err());
        assert_eq!(store.kv_read("saved-searches.v1:phone").unwrap().as_deref(), Some("broken"));
    }
}
