//! People on this computer: the owner, who is implicit, and everyone else who uses it with their
//! own paired phone or their own bots (see `docs/features/people.md`).
//!
//! There is no member sign-in: a person is a record the owner creates, and a device or a bot names
//! which person it belongs to. A bot, connector or device with no person is the owner's.

use super::{Store, logged, now_ms};
use crate::LockExt;
use anyhow::Result;
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

/// A person on this computer, besides its owner.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    pub created_at: i64,
}

const PERSON_COLS: &str = "id, name, email, created_at";

fn row_person(r: &rusqlite::Row) -> rusqlite::Result<Person> {
    Ok(Person { id: r.get(0)?, name: r.get(1)?, email: r.get(2)?, created_at: r.get(3)? })
}

impl Store {
    pub fn people(&self) -> Result<Vec<Person>> {
        let c = self.db.locked();
        let mut st = c.prepare(&format!("SELECT {PERSON_COLS} FROM people ORDER BY created_at"))?;
        Ok(st.query_map([], row_person)?.collect::<rusqlite::Result<_>>()?)
    }

    pub fn person(&self, id: &str) -> Option<Person> {
        let c = self.db.locked();
        let row = c.query_row(&format!("SELECT {PERSON_COLS} FROM people WHERE id = ?1"), [id], row_person);
        logged("person", row.optional()).flatten()
    }

    /// Adds a person; `name` is trimmed by the caller. `email` stays `None` when it is empty.
    pub fn add_person(&self, name: &str, email: Option<&str>) -> Result<Person> {
        let person = Person {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_owned(),
            email: email.map(str::to_owned),
            created_at: now_ms(),
        };
        self.db.locked().execute(
            "INSERT INTO people(id, name, email, created_at) VALUES(?1, ?2, ?3, ?4)",
            params![person.id, person.name, person.email, person.created_at],
        )?;
        Ok(person)
    }

    /// Renames a person and sets or clears their email; `None` when there's no such person.
    pub fn update_person(&self, id: &str, name: &str, email: Option<&str>) -> Result<Option<Person>> {
        let changed = self
            .db
            .locked()
            .execute("UPDATE people SET name = ?2, email = ?3 WHERE id = ?1", params![id, name, email])?;
        Ok((changed > 0).then(|| self.person(id)).flatten())
    }

    /// Removes a person and unassigns their devices (which become the owner's); their bots and
    /// connectors are unassigned by the caller, so this alone never drops them. False when it wasn't there.
    pub fn remove_person(&self, id: &str) -> Result<bool> {
        let mut c = self.db.locked();
        let tx = c.transaction()?;
        let gone = tx.execute("DELETE FROM people WHERE id = ?1", [id])?;
        tx.execute("UPDATE devices SET person_id = NULL WHERE person_id = ?1", [id])?;
        tx.commit()?;
        Ok(gone > 0)
    }

    /// Makes `key`'s device belong to `person`; `None` gives it back to the owner.
    pub fn set_device_person(&self, key: &str, person: Option<&str>) -> Result<bool> {
        let changed =
            self.db.locked().execute("UPDATE devices SET person_id = ?2 WHERE key = ?1", params![key, person])?;
        Ok(changed > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::tests::temp_store;

    #[test]
    fn people_are_added_renamed_and_removed_without_their_devices() {
        let s = temp_store();
        let ana = s.add_person("Ana", Some("ana@example.com")).unwrap();
        let ben = s.add_person("Ben", None).unwrap();
        assert_eq!(s.people().unwrap().len(), 2);
        assert_eq!(s.person(&ana.id).unwrap().email.as_deref(), Some("ana@example.com"));
        assert_eq!(s.person(&ben.id).unwrap().email, None);

        s.db.locked()
            .execute(
                "INSERT INTO devices(key, name, platform, source, scopes, created_at, person_id)
                 VALUES('dk', 'Phone', 'ios', 'local', '[]', 1, ?1)",
                [&ana.id],
            )
            .unwrap();
        s.set_device_person("dk", Some(&ben.id)).unwrap();
        assert_eq!(s.device("dk").unwrap().person_id.as_deref(), Some(ben.id.as_str()));

        let renamed = s.update_person(&ben.id, "Ben B", Some("ben@example.com")).unwrap().unwrap();
        assert_eq!((renamed.name.as_str(), renamed.email.as_deref()), ("Ben B", Some("ben@example.com")));

        // Removing a person gives their device back to the owner and leaves nothing behind.
        assert!(s.remove_person(&ben.id).unwrap());
        assert!(!s.remove_person(&ben.id).unwrap());
        assert!(s.person(&ben.id).is_none());
        assert_eq!(s.device("dk").unwrap().person_id, None);
        assert_eq!(s.people().unwrap(), [ana]);
    }
}
