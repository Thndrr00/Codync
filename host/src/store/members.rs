//! The people who share this computer's Codync: one owner, admins, members and viewers. A member's
//! token is handed out once when they are invited and stored only as a SHA-256 hash; their phone
//! redeems it on the pairing channel and connects as `Caller::Member`, so `permit` applies the
//! role to every call (a viewer reads, a member works with the bots, admins also invite and
//! remove people). Removing a member closes their devices on the next recheck.

use super::Store;
use crate::LockExt;
use anyhow::{Result, ensure};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

const KEY: &str = "workspace-members.v1";
const LIMIT: usize = 50;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MemberRole {
    Owner,
    Admin,
    Member,
    Viewer,
}

impl MemberRole {
    fn parse(value: &str) -> Result<Self> {
        Ok(match value {
            "owner" => Self::Owner,
            "admin" => Self::Admin,
            "member" => Self::Member,
            "viewer" => Self::Viewer,
            other => anyhow::bail!("unknown member role: {other}"),
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Admin => "admin",
            Self::Member => "member",
            Self::Viewer => "viewer",
        }
    }

    /// Owners and admins run the workspace: they may invite, remove and re-role people.
    pub fn is_admin(self) -> bool {
        matches!(self, Self::Owner | Self::Admin)
    }

    /// What a member's own device may do. Everyone can work with the bots (a viewer is
    /// limited to read methods by `permit`); screen access is for admins only.
    pub fn scopes(self) -> Vec<crate::store::Scope> {
        match self {
            Self::Owner | Self::Admin => vec![crate::store::Scope::Control, crate::store::Scope::Screen],
            Self::Member | Self::Viewer => vec![crate::store::Scope::Control],
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Member {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub email: String,
    pub role: MemberRole,
    pub created_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_seen_at: Option<i64>,
    /// SHA-256 of the member's token; the token itself is shown once, when invited.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_hash: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Invite {
    pub member: Member,
    pub token: String,
}

fn read(store: &Store) -> Result<Vec<Member>> {
    match store.kv_read(KEY)? {
        Some(value) => Ok(serde_json::from_str(&value)?),
        None => Ok(Vec::new()),
    }
}

fn write(store: &Store, members: &[Member]) -> Result<()> {
    store.kv_set(KEY, &serde_json::to_string(members)?)
}

fn hash(token: &str) -> String {
    crate::chat::files::digest_hex(&Sha256::digest(token.as_bytes()))
}

impl Store {
    /// The workspace's people, the owner first. The first call seeds the owner as "You".
    pub fn members(&self) -> Result<Vec<Member>> {
        let mut members = read(self)?;
        if members.is_empty() {
            let owner = Member {
                id: uuid::Uuid::new_v4().to_string(),
                name: "You".into(),
                email: String::new(),
                role: MemberRole::Owner,
                created_at: crate::store::now_ms(),
                last_seen_at: None,
                token_hash: None,
            };
            members.push(owner);
            write(self, &members)?;
        }
        members.sort_by_key(|member| match member.role {
            MemberRole::Owner => 0,
            MemberRole::Admin => 1,
            MemberRole::Member => 2,
            MemberRole::Viewer => 3,
        });
        Ok(members)
    }

    /// Adds a person and returns their one-time token. There is only one owner.
    pub fn invite_member(&self, name: &str, email: &str, role: &str) -> Result<Invite> {
        let name = name.trim();
        ensure!(!name.is_empty(), "a member needs a name");
        ensure!(name.chars().count() <= 120, "a member name is at most 120 characters");
        let role = MemberRole::parse(role)?;
        ensure!(role != MemberRole::Owner, "the workspace already has an owner");
        let mut members = self.members()?;
        ensure!(members.len() < LIMIT, "the workspace holds at most {LIMIT} people");
        ensure!(
            email.trim().is_empty() || !members.iter().any(|member| member.email.eq_ignore_ascii_case(email.trim())),
            "that email is already a member"
        );
        let token = format!("{}{}", uuid::Uuid::new_v4().simple(), uuid::Uuid::new_v4().simple());
        let member = Member {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_owned(),
            email: email.trim().to_owned(),
            role,
            created_at: crate::store::now_ms(),
            last_seen_at: None,
            token_hash: Some(hash(&token)),
        };
        members.push(member.clone());
        write(self, &members)?;
        Ok(Invite { member, token })
    }

    /// Removes a person; the owner cannot be removed.
    pub fn remove_member(&self, id: &str) -> Result<bool> {
        let mut members = self.members()?;
        let Some(member) = members.iter().find(|member| member.id == id) else { return Ok(false) };
        ensure!(member.role != MemberRole::Owner, "the owner cannot be removed");
        members.retain(|member| member.id != id);
        write(self, &members)?;
        Ok(true)
    }

    /// Changes a person's role. The owner's role stays owner; nobody else can become owner.
    pub fn set_member_role(&self, id: &str, role: &str) -> Result<Member> {
        let role = MemberRole::parse(role)?;
        let mut members = self.members()?;
        let member =
            members.iter_mut().find(|member| member.id == id).ok_or_else(|| anyhow::anyhow!("unknown member"))?;
        if member.role == MemberRole::Owner {
            ensure!(role == MemberRole::Owner, "the owner's role cannot change");
        } else {
            ensure!(role != MemberRole::Owner, "there is already an owner");
            member.role = role;
        }
        let changed = member.clone();
        write(self, &members)?;
        Ok(changed)
    }

    /// Records that a member's token was seen, for the members page.
    pub fn touch_member(&self, token_hash: &str) -> Result<()> {
        let mut members = self.members()?;
        let mut changed = false;
        for member in &mut members {
            if member.token_hash.as_deref() == Some(token_hash) {
                member.last_seen_at = Some(crate::store::now_ms());
                changed = true;
            }
        }
        if changed {
            write(self, &members)?;
        }
        Ok(())
    }

    /// The person a member token belongs to, and the hash the caller should record.
    pub fn member_by_token(&self, token: &str) -> Result<Option<(Member, String)>> {
        if token.len() != 64 || !token.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Ok(None);
        }
        let hash = hash(token);
        Ok(self
            .members()?
            .into_iter()
            .find(|member| member.token_hash.as_deref() == Some(hash.as_str()))
            .map(|m| (m, hash)))
    }

    /// The person behind an id, for a channel that carries their role.
    pub fn member(&self, id: &str) -> Result<Option<Member>> {
        Ok(self.members()?.into_iter().find(|member| member.id == id))
    }

    /// Records that a member connected, without rehashing their token.
    pub fn touch_member_id(&self, id: &str) -> Result<()> {
        let mut members = self.members()?;
        for member in &mut members {
            if member.id == id {
                member.last_seen_at = Some(crate::store::now_ms());
                write(self, &members)?;
                return Ok(());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_owner_is_seeded_and_cannot_be_removed_or_demoted() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let members = store.members().unwrap();
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].role, MemberRole::Owner);
        assert!(!store.remove_member(&members[0].id).unwrap());
        assert!(store.set_member_role(&members[0].id, "member").is_err());
        assert_eq!(store.members().unwrap()[0].role, MemberRole::Owner);
    }

    #[test]
    fn an_invite_returns_a_token_once_and_keeps_only_its_hash() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let invite = store.invite_member("Ada", "ada@example.com", "admin").unwrap();
        assert_eq!(invite.member.role, MemberRole::Admin);
        assert_eq!(invite.token.len(), 64);
        let saved = store.members().unwrap();
        assert_eq!(saved.len(), 2);
        assert!(saved.iter().all(|member| member.id != invite.token));
        assert_eq!(saved[1].token_hash.as_deref(), Some(hash(&invite.token).as_str()));
        store.touch_member(&hash(&invite.token)).unwrap();
        assert!(store.members().unwrap()[1].last_seen_at.is_some());
    }

    #[test]
    fn a_member_token_finds_its_person_and_nothing_else() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        assert!(store.member_by_token("nope").unwrap().is_none());
        let invite = store.invite_member("Ada", "", "member").unwrap();
        let (member, hash) = store.member_by_token(&invite.token).unwrap().unwrap();
        assert_eq!(member.id, invite.member.id);
        assert_eq!(Some(hash), member.token_hash.clone());
        assert!(store.member_by_token(&"0".repeat(64)).unwrap().is_none());
        store.touch_member_id(&member.id).unwrap();
        assert!(store.member(&member.id).unwrap().unwrap().last_seen_at.is_some());
        assert!(store.member("gone").unwrap().is_none());
    }

    #[test]
    fn roles_are_checked_and_the_email_is_unique() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        assert!(store.invite_member("Ada", "ada@example.com", "boss").is_err());
        assert!(store.invite_member("Ada", "ada@example.com", "owner").is_err());
        let invite = store.invite_member("Ada", "ada@example.com", "viewer").unwrap();
        assert!(store.invite_member("Ada 2", "ADA@example.com", "member").is_err());
        assert_eq!(store.set_member_role(&invite.member.id, "member").unwrap().role, MemberRole::Member);
        assert!(store.remove_member(&invite.member.id).unwrap());
        assert_eq!(store.members().unwrap().len(), 1);
    }
}
