//! People on this computer: the owner, who is implicit, and everyone else who uses it with their
//! own paired phone, their own session or their own bots (see `docs/features/people.md`).
//!
//! A person is a member account: an invited email and a one-time invitation redeem it into a device or
//! a session (`store::member_accounts`). A bot, connector or device with no person is the owner's.
//!
//! A person is a `member` (their own bots, chats and connectors) or an `admin` (they also manage
//! everyone's people, connectors and bots). Only the owner makes someone an admin.

use super::{Store, logged, now_ms};
use crate::LockExt;
use anyhow::Result;
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

/// What a person may manage on this computer. The owner is implicit and is not one of these.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Admin,
    #[default]
    Member,
}

impl Role {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "admin" => Some(Self::Admin),
            "member" => Some(Self::Member),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Admin => "admin",
            Self::Member => "member",
        }
    }
}

/// A person on this computer, besides its owner.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    pub created_at: i64,
    #[serde(default)]
    pub role: Role,
}

const PERSON_COLS: &str = "id, name, email, created_at, role";

fn row_person(r: &rusqlite::Row) -> rusqlite::Result<Person> {
    let role: String = r.get(4)?;
    Ok(Person {
        id: r.get(0)?,
        name: r.get(1)?,
        email: r.get(2)?,
        created_at: r.get(3)?,
        role: Role::parse(&role).unwrap_or_default(),
    })
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

    /// The person whose email is `email`, whatever its case: the address a company signs in with.
    pub fn person_by_email(&self, email: &str) -> Option<Person> {
        let email = email.trim();
        if email.is_empty() {
            return None;
        }
        let c = self.db.locked();
        let row = c.query_row(
            &format!("SELECT {PERSON_COLS} FROM people WHERE codync_lower(email) = codync_lower(?1)"),
            [email],
            row_person,
        );
        logged("person_by_email", row.optional()).flatten()
    }

    /// Adds a person; `name` is trimmed by the caller. `email` stays `None` when it is empty.
    pub fn add_person(&self, name: &str, email: Option<&str>) -> Result<Person> {
        self.add_person_with_role(name, email, Role::Member)
    }

    /// Adds a person with their role; the owner alone may add an admin.
    pub fn add_person_with_role(&self, name: &str, email: Option<&str>, role: Role) -> Result<Person> {
        let person = Person {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_owned(),
            email: email.map(str::to_owned),
            created_at: now_ms(),
            role,
        };
        self.db.locked().execute(
            "INSERT INTO people(id, name, email, created_at, role) VALUES(?1, ?2, ?3, ?4, ?5)",
            params![person.id, person.name, person.email, person.created_at, person.role.as_str()],
        )?;
        Ok(person)
    }

    /// Renames a person and sets or clears their email; `None` when there's no such person. The role
    /// stays as it is (`set_person_role` changes it).
    pub fn update_person(&self, id: &str, name: &str, email: Option<&str>) -> Result<Option<Person>> {
        let changed = self
            .db
            .locked()
            .execute("UPDATE people SET name = ?2, email = ?3 WHERE id = ?1", params![id, name, email])?;
        Ok((changed > 0).then(|| self.person(id)).flatten())
    }

    /// Makes a person an admin or a member; `None` when there's no such person.
    pub fn set_person_role(&self, id: &str, role: Role) -> Result<Option<Person>> {
        let changed =
            self.db.locked().execute("UPDATE people SET role = ?2 WHERE id = ?1", params![id, role.as_str()])?;
        Ok((changed > 0).then(|| self.person(id)).flatten())
    }

    /// Removes a person and revokes their devices, with their push and Live Activity tickets, so a
    /// phone of theirs loses access at once. Their bots and connectors are unassigned by the caller,
    /// so this alone never drops them. False when it wasn't there.
    pub fn remove_person(&self, id: &str) -> Result<bool> {
        let mut c = self.db.locked();
        let tx = c.transaction()?;
        let devices: Vec<String> = {
            let mut st = tx.prepare("SELECT key FROM devices WHERE person_id = ?1")?;
            st.query_map([id], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?
        };
        for key in devices {
            super::devices::delete_device(&tx, &key)?;
        }
        // Their invitations and session tokens end too; a member session is backed by a device row,
        // so this is the second half of what `delete_device` above already removed.
        tx.execute("DELETE FROM member_invites WHERE person_id = ?1", [id])?;
        tx.execute("DELETE FROM member_sessions WHERE person_id = ?1", [id])?;
        let gone = tx.execute("DELETE FROM people WHERE id = ?1", [id])?;
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
    fn people_are_added_renamed_and_removed_without_their_bots() {
        let s = temp_store();
        let ana = s.add_person("Ana", Some("ana@example.com")).unwrap();
        let ben = s.add_person_with_role("Ben", None, Role::Admin).unwrap();
        assert_eq!(s.people().unwrap().len(), 2);
        assert_eq!(s.person(&ana.id).unwrap().email.as_deref(), Some("ana@example.com"));
        assert_eq!(s.person(&ben.id).unwrap().email, None);
        assert_eq!((ana.role, ben.role), (Role::Member, Role::Admin));

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
        assert_eq!(s.set_person_role(&ben.id, Role::Member).unwrap().unwrap().role, Role::Member);

        // Removing a person revokes their devices instead of handing them to the owner.
        assert!(s.remove_person(&ben.id).unwrap());
        assert!(!s.remove_person(&ben.id).unwrap());
        assert!(s.person(&ben.id).is_none());
        assert!(s.device("dk").is_none(), "a removed person's phone loses access");
        assert_eq!(s.people().unwrap(), [ana]);
    }
}
