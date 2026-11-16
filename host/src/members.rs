//! Member accounts: an invitation, a session, and the owner or admin who hands them out
//! (docs/features/people.md).
//!
//! A member is a row in `people` with a role, exactly as before. This module adds the two ways
//! someone proves they are that person, and it settles nothing else: a member's calls still go
//! through `api::dispatch`, so `api::devices::permit` checks the scope and `people::caller_person`
//! narrows every bot, chat and connector to them.
//!
//! - An **invitation** is a one-time code bound to a person. A phone or a client that pairs with it
//!   becomes a device for that person with Control scope.
//! - A **session** is a bearer token for a client that reaches this computer's local API (the desktop
//!   app over its own tunnel, or another local client). It is backed by a device row for the same
//!   person, so revoking it is revoking a device.
//!
//! Both secrets are kept as digests (`store::member_accounts`). Company sign-in maps a verified
//! email to a member (`enterprise::member_from_email`) and exchanges it for a session at the
//! computer; it never mints a role.

use crate::LockExt;
use crate::api::devices::Caller;
use crate::hub::Hub;
use crate::people;
use crate::store::{Device, DeviceAccount, DeviceSource, Person, Role, SESSION_TTL_MS, Scope, now_ms};
use anyhow::{Result, bail};
use serde_json::{Value, json};
use std::sync::Arc;

/// The methods this module answers.
pub const METHODS: [&str; 6] =
    ["inviteMember", "memberSessions", "revokeMemberSession", "memberProfile", "memberSignIn", "acceptMemberInvite"];

/// Longest a session device's platform label may be.
const MAX_PLATFORM_CHARS: usize = 20;

pub async fn call(hub: &Arc<Hub>, caller: &Caller, method: &str, b: &Value) -> Result<Value> {
    match method {
        "inviteMember" => invite(hub, caller, b).await,
        "memberSessions" => sessions(hub, caller),
        "revokeMemberSession" => revoke(hub, caller, b),
        "memberProfile" => Ok(profile(hub, caller)),
        // Local only (`api::devices::LOCAL_ONLY`): both hand out a credential.
        "memberSignIn" => sign_in(hub, caller, b).await,
        "acceptMemberInvite" => accept_invite(hub, caller, b),
        _ => bail!("unknown member method: {method}"),
    }
}

/// What a client shows about whoever is calling: the owner, an admin or a member.
pub fn profile(hub: &Hub, caller: &Caller) -> Value {
    let person = people::caller_person(hub, caller).and_then(|id| hub.store.person(&id));
    let scopes = match caller {
        Caller::Device { scopes, .. } => serde_json::to_value(scopes).unwrap_or_default(),
        Caller::Local => json!(["control", "screen"]),
        Caller::Pairing { .. } => json!([]),
    };
    match person {
        None => json!({"owner": true, "role": "owner", "person": null, "scopes": scopes}),
        Some(person) => json!({
            "owner": false,
            "role": person.role.as_str(),
            "person": person,
            "scopes": scopes,
        }),
    }
}

/// The invitation the owner or an admin hands a member: the person record, one code to redeem, and
/// the pairing link that carries it. Inviting the same email again replaces its earlier code.
async fn invite(hub: &Arc<Hub>, caller: &Caller, b: &Value) -> Result<Value> {
    let actor = manager(hub, caller)?;
    let email = people::checked_email(Some(required(b, "email")?))?.expect("an email was required");
    // No `role` keeps an existing person's role; a new person is a member.
    let requested = match b.get("role") {
        None | Some(Value::Null) => None,
        Some(_) => Some(people::role_in(b)?),
    };
    if requested == Some(Role::Admin) && actor.is_some() {
        bail!("Only the owner can add an admin.");
    }
    let person = match hub.store.person_by_email(email) {
        Some(existing) => {
            if existing.role == Role::Admin && actor.is_some() {
                bail!("That email belongs to an admin. Only the owner can invite an admin.");
            }
            match requested {
                Some(role) if role != existing.role => {
                    if actor.is_some() {
                        bail!("Only the owner can change a person's role.");
                    }
                    hub.store.set_person_role(&existing.id, role)?.unwrap_or(existing)
                }
                _ => existing,
            }
        }
        None => {
            let typed = b["name"].as_str().map(str::trim).filter(|n| !n.is_empty());
            let name = people::checked_name(typed.unwrap_or_else(|| email.split('@').next().unwrap_or(email)))?;
            hub.store.add_person_with_role(name, Some(email), requested.unwrap_or(Role::Member))?
        }
    };
    let code = crate::remote::crypto::b64(&crate::remote::crypto::random::<16>());
    let invite = hub.store.add_member_invite(&person.id, &code, now_ms() + crate::store::INVITE_TTL_MS)?;
    let mut answer = json!({"person": person, "invite": {"code": code, "expiresAt": invite.expires_at}});
    // The same code is what a phone scans: the pairing channel files the device under `person`.
    match crate::api::pairing_link(hub, &code, invite.expires_at).await {
        Ok(link) => {
            answer["invite"]["pairingUrl"] = link["pairingUrl"].clone();
            answer["invite"]["urls"] = link["urls"].clone();
        }
        // A computer with no address a phone can reach still hands out the code: a member at this
        // computer redeems it with `acceptMemberInvite`, and the desktop can carry it over its tunnel.
        Err(e) => answer["invite"]["note"] = json!(format!("{e:#}")),
    }
    Ok(answer)
}

/// Everyone with a session or a live invitation: the owner sees all, an admin sees members and themself.
fn sessions(hub: &Arc<Hub>, caller: &Caller) -> Result<Value> {
    let actor = manager(hub, caller)?;
    let admin_sees = actor.as_deref().is_some_and(|id| people::is_admin(hub, id));
    let people = hub.store.people()?;
    let connected = hub.connected.locked().clone();
    let tokens = hub.store.member_sessions()?;
    let visible =
        |person: &Person| !admin_sees || person.role == Role::Member || actor.as_deref() == Some(person.id.as_str());
    let mut rows: Vec<Value> = Vec::new();
    for device in hub.store.devices()? {
        let Some(person_id) = device.person_id.as_deref() else { continue };
        let Some(person) = people.iter().find(|p| p.id == person_id).filter(|p| visible(p)) else { continue };
        let expires_at = tokens.iter().find(|s| s.device_key == device.key).map(|s| s.expires_at);
        rows.push(json!({
            "id": device.key,
            "kind": if expires_at.is_some() { "session" } else { "device" },
            "personId": person.id,
            "person": person.name,
            "email": person.email,
            "role": person.role.as_str(),
            "name": device.name,
            "platform": device.platform,
            "createdAt": device.created_at,
            "lastSeenAt": device.last_seen_at,
            "expiresAt": expires_at,
            "connected": connected.contains_key(&device.key),
        }));
    }
    for invite in hub.store.member_invites()? {
        let Some(person) = people.iter().find(|p| p.id == invite.person_id).filter(|p| visible(p)) else { continue };
        rows.push(json!({
            "id": invite.id,
            "kind": "invite",
            "personId": person.id,
            "person": person.name,
            "email": person.email,
            "role": person.role.as_str(),
            "name": Value::Null,
            "platform": Value::Null,
            "createdAt": invite.created_at,
            "lastSeenAt": Value::Null,
            "expiresAt": invite.expires_at,
            "connected": false,
        }));
    }
    rows.sort_by_key(|row| row["createdAt"].as_i64().unwrap_or_default());
    Ok(json!({"sessions": rows, "canManage": true}))
}

/// Ends one session or cancels one invitation. A device with no person is the owner's own: it is not
/// a member session, so this method leaves it to `revokeDevice`.
fn revoke(hub: &Arc<Hub>, caller: &Caller, b: &Value) -> Result<Value> {
    let actor = manager(hub, caller)?;
    let id = required(b, "session")?;
    let admin_sees = actor.as_deref().is_some_and(|person| people::is_admin(hub, person));
    if let Some(device) = hub.store.device(id) {
        let Some(person_id) = device.person_id.clone() else {
            bail!("That device is this computer's own; revoke it in Authorized devices.");
        };
        let person = hub.store.person(&person_id).ok_or_else(|| anyhow::anyhow!("unknown person"))?;
        if admin_sees && person.role != Role::Member && actor.as_deref() != Some(person.id.as_str()) {
            bail!("Only the owner can end another admin's session.");
        }
        // The device row and its session token go in one change: nothing still authorizes afterwards.
        hub.revoke_device(&device.key)?;
        if let Some(grant) = &device.grant_id {
            tokio::spawn(crate::remote::cloud::revoke_grant(hub.clone(), grant.clone()));
        }
        return Ok(json!({"revoked": true}));
    }
    if let Some(invite) = hub.store.member_invite(id) {
        let person = hub.store.person(&invite.person_id).ok_or_else(|| anyhow::anyhow!("unknown person"))?;
        if admin_sees && person.role != Role::Member {
            bail!("Only the owner can cancel another admin's invitation.");
        }
        return Ok(json!({"revoked": hub.store.remove_member_invite(id)?}));
    }
    bail!("That session isn't on this computer any more.");
}

/// Company sign-in: a verified email at one of the configured domains, exchanged at the computer for
/// a member session (the desktop app, or another local client, reaches the local API here).
async fn sign_in(hub: &Arc<Hub>, caller: &Caller, b: &Value) -> Result<Value> {
    require_local(caller)?;
    let email = people::checked_email(Some(required(b, "email")?))?.expect("an email was required");
    // The provider's settings live in the vault.
    crate::market::vault::unlock(hub.clone()).await.ok();
    if !crate::enterprise::domain_allows(&hub.store, email)? {
        bail!("Company sign-in isn't set up for that email's domain on this computer.");
    }
    let person = crate::enterprise::member_from_email(&hub.store, email)
        .ok_or_else(|| anyhow::anyhow!("No member with that email has been invited on this computer."))?;
    start_session(hub, &person, b, "Member sign-in")
}

/// An invitation redeemed at the computer itself: a desktop app or another local client. A phone
/// redeems the same code through the pairing channel instead.
fn accept_invite(hub: &Arc<Hub>, caller: &Caller, b: &Value) -> Result<Value> {
    require_local(caller)?;
    let code = required(b, "code")?;
    let invite = hub
        .store
        .take_member_invite(code)?
        .ok_or_else(|| anyhow::anyhow!("That invitation isn't valid any more. Ask for a new one."))?;
    let person = hub.store.person(&invite.person_id).ok_or_else(|| anyhow::anyhow!("That person is gone."))?;
    start_session(hub, &person, b, "Member")
}

/// A new member session: a device row for `person` with Control scope, and its bearer token.
fn start_session(hub: &Arc<Hub>, person: &Person, b: &Value, fallback_name: &str) -> Result<Value> {
    let token = crate::remote::crypto::b64(&crate::remote::crypto::random::<32>());
    let key = format!("member-{}", uuid::Uuid::new_v4());
    let name = b["name"].as_str().map(str::trim).filter(|n| !n.is_empty()).unwrap_or(fallback_name);
    let platform = b["platform"].as_str().map(str::trim).filter(|p| !p.is_empty()).unwrap_or("desktop");
    if platform.chars().count() > MAX_PLATFORM_CHARS {
        bail!("Use a shorter platform name.");
    }
    let name = people::checked_name(name)?;
    let device = Device {
        key: key.clone(),
        name: name.to_owned(),
        platform: platform.to_owned(),
        source: DeviceSource::Local,
        grant_id: None,
        // A member session is a Control client: the same methods as a paired phone, without the screen.
        scopes: vec![Scope::Control],
        lease_until: None,
        created_at: now_ms(),
        last_seen_at: None,
        account: DeviceAccount::Unrecorded,
        person_id: Some(person.id.clone()),
    };
    hub.store.put_device(&device)?;
    let expires_at = now_ms() + SESSION_TTL_MS;
    if let Err(e) = hub.store.add_member_session(&key, &person.id, &token, expires_at) {
        // No half-made session: the device row goes with it.
        let _ = hub.store.remove_device(&key);
        return Err(e);
    }
    hub.auth_changed();
    Ok(json!({
        "token": token,
        "person": person,
        "role": person.role.as_str(),
        "expiresAt": expires_at,
        "hostName": crate::remote::managed::computer_name(&hub.store),
    }))
}

/// The caller a member session token acts as, while its device and person exist and it hasn't expired.
pub fn caller_for_token(hub: &Hub, token: &str) -> Option<Caller> {
    let session = hub.store.member_session(token)?;
    let device = hub.store.device(&session.device_key)?;
    if device.person_id.as_deref() != Some(session.person_id.as_str()) || hub.store.person(&session.person_id).is_none()
    {
        return None;
    }
    Some(Caller::Device { key: device.key, scopes: device.scopes })
}

/// Whether `code` is a live member invitation, for the pairing channel's admission check.
pub fn invite_open(hub: &Hub, code: &str) -> bool {
    hub.store.member_invite_for_code(code).is_some()
}

/// Uses up the invitation `code` names and returns the person it was for. The pairing channel calls
/// this only after this computer agreed to let the device in, so a declined pairing leaves the code.
pub fn redeem_invite(hub: &Hub, code: &str) -> Result<Option<Person>> {
    let Some(invite) = hub.store.take_member_invite(code)? else { return Ok(None) };
    Ok(hub.store.person(&invite.person_id))
}

/// The owner, or an admin: the same rule as the other people methods. `None` is the owner.
fn manager(hub: &Hub, caller: &Caller) -> Result<Option<String>> {
    let actor = people::caller_person(hub, caller);
    if let Some(person) = &actor
        && !people::is_admin(hub, person)
    {
        bail!("This can only be done by the owner or an admin on this computer.");
    }
    Ok(actor)
}

fn require_local(caller: &Caller) -> Result<()> {
    if !matches!(caller, Caller::Local) {
        bail!("Signing in as a member happens at the computer itself.");
    }
    Ok(())
}

fn required<'a>(b: &'a Value, key: &str) -> Result<&'a str> {
    b[key].as_str().map(str::trim).filter(|v| !v.is_empty()).ok_or_else(|| anyhow::anyhow!("`{key}` is required"))
}

#[cfg(test)]
#[path = "members_tests.rs"]
mod tests;
