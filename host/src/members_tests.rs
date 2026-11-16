//! Tests for member accounts: invitations, sessions, roles and what a member can reach.
//! A child module of `members` (`docs/features/people.md`), so the module's own helpers stay in reach.

use super::*;
use crate::api::dispatch;
use crate::remote::identity::Identity;
use crate::store::{BotConfig, Store};

fn hub() -> Arc<Hub> {
    let dir = std::env::temp_dir().join(format!("codync-members-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let store = Store::open(std::path::Path::new(":memory:")).unwrap();
    Hub::new(store, "x".into(), Identity::load_or_create(&dir).unwrap(), "t".into(), 0)
}

fn bot(hub: &Hub, id: &str, person: Option<&str>) {
    let cfg: BotConfig = serde_json::from_value(json!({
        "id": id, "name": id, "backend": "fixture", "command": "true", "cwd": "/tmp",
        "person": person, "notify": false,
    }))
    .unwrap();
    hub.store.save_bot(&cfg).unwrap();
}

/// An invitation for `email`, accepted at the computer, and the caller it becomes.
async fn signed_in(hub: &Arc<Hub>, email: &str) -> (Value, Caller) {
    let invited = call(hub, &Caller::Local, "inviteMember", &json!({"email": email, "name": "Ana"})).await.unwrap();
    let code = invited["invite"]["code"].as_str().unwrap();
    let answer = call(hub, &Caller::Local, "acceptMemberInvite", &json!({"code": code})).unwrap();
    let token = answer["token"].as_str().unwrap();
    let caller = caller_for_token(hub, token).expect("a live session");
    (answer, caller)
}

#[tokio::test]
async fn an_invitation_is_accepted_once_and_becomes_that_person() {
    let hub = hub();
    let invited =
        call(&hub, &Caller::Local, "inviteMember", &json!({"email": "Ana@Example.com", "name": "Ana"})).await.unwrap();
    assert_eq!(invited["person"]["role"], "member");
    assert_eq!(invited["person"]["email"], "Ana@Example.com");
    let code = invited["invite"]["code"].as_str().unwrap();
    assert!(invite_open(&hub, code), "the code is live while its invitation is");

    let answer = call(&hub, &Caller::Local, "acceptMemberInvite", &json!({"code": code})).unwrap();
    assert_eq!(answer["person"]["email"], "Ana@Example.com");
    let token = answer["token"].as_str().unwrap();
    let caller = caller_for_token(&hub, token).expect("a session");
    assert_eq!(people::caller_person(&hub, &caller).unwrap(), answer["person"]["id"].as_str().unwrap());
    assert!(!invite_open(&hub, code), "single use");
    assert!(call(&hub, &Caller::Local, "acceptMemberInvite", &json!({"code": code})).is_err());
    assert_eq!(profile(&hub, &caller)["role"], "member");

    // The session lists what it is, and revoking it ends it.
    let list = call(&hub, &Caller::Local, "memberSessions", &json!({})).unwrap();
    let sessions = list["sessions"].as_array().unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0]["kind"], "session");
    let id = sessions[0]["id"].as_str().unwrap().to_owned();
    assert_eq!(call(&hub, &Caller::Local, "revokeMemberSession", &json!({"session": id})).unwrap()["revoked"], true);
    assert!(caller_for_token(&hub, token).is_none(), "the token is dead");
    assert!(
        call(&hub, &Caller::Local, "memberSessions", &json!({})).unwrap()["sessions"].as_array().unwrap().is_empty()
    );
}

#[tokio::test]
async fn inviting_the_same_email_again_replaces_its_code() {
    let hub = hub();
    let first = call(&hub, &Caller::Local, "inviteMember", &json!({"email": "ana@example.com"})).await.unwrap();
    let second = call(&hub, &Caller::Local, "inviteMember", &json!({"email": "ANA@example.com"})).await.unwrap();
    assert_eq!(first["person"]["id"], second["person"]["id"], "one person per email");
    assert!(!invite_open(&hub, first["invite"]["code"].as_str().unwrap()));
    assert!(invite_open(&hub, second["invite"]["code"].as_str().unwrap()));
}

#[tokio::test]
async fn a_member_cannot_invite_list_or_revoke_but_an_admin_can_invite_members() {
    let hub = hub();
    let (_, ana) = signed_in(&hub, "ana@example.com").await;
    assert!(call(&hub, &ana, "inviteMember", &json!({"email": "ben@example.com"})).await.is_err());
    assert!(call(&hub, &ana, "memberSessions", &json!({})).is_err());
    assert!(call(&hub, &ana, "revokeMemberSession", &json!({"session": "x"})).is_err());

    let ana_id = people::caller_person(&hub, &ana).unwrap();
    call(&hub, &Caller::Local, "updatePerson", &json!({"id": ana_id, "name": "Ana", "role": "admin"})).unwrap();
    let ben = call(&hub, &ana, "inviteMember", &json!({"email": "ben@example.com"})).await.unwrap();
    assert_eq!(ben["person"]["role"], "member");
    assert!(
        call(&hub, &ana, "inviteMember", &json!({"email": "cid@example.com", "role": "admin"})).await.is_err(),
        "only the owner makes an admin"
    );
    // An admin sees members and themself, and can end a member's invitation.
    let list = call(&hub, &ana, "memberSessions", &json!({})).unwrap();
    let rows = list["sessions"].as_array().unwrap();
    assert_eq!(rows.len(), 2, "the member's invitation and the admin's own session");
    assert!(rows.iter().any(|row| row["role"] == "admin"), "the admin's own session is theirs to see");
    let ben_invite = rows.iter().find(|row| row["kind"] == "invite").unwrap();
    let id = ben_invite["id"].as_str().unwrap();
    assert!(call(&hub, &ana, "revokeMemberSession", &json!({"session": id})).is_ok());
    assert!(!invite_open(&hub, ben["invite"]["code"].as_str().unwrap()));
}

#[tokio::test]
async fn only_the_owner_adds_or_manages_an_admin() {
    let hub = hub();
    let invited = call(&hub, &Caller::Local, "inviteMember", &json!({"email": "ana@example.com", "role": "admin"}))
        .await
        .unwrap();
    assert_eq!(invited["person"]["role"], "admin");
    let code = invited["invite"]["code"].as_str().unwrap();
    let answer = call(&hub, &Caller::Local, "acceptMemberInvite", &json!({"code": code})).unwrap();
    let ana = caller_for_token(&hub, answer["token"].as_str().unwrap()).unwrap();

    // Another admin's invitation is the owner's to cancel, not an admin's.
    let other = call(&hub, &Caller::Local, "inviteMember", &json!({"email": "cid@example.com", "role": "admin"}))
        .await
        .unwrap();
    let rows = call(&hub, &ana, "memberSessions", &json!({})).unwrap();
    assert!(
        rows["sessions"].as_array().unwrap().iter().all(|row| row["role"] == "member"
            || Some(row["personId"].as_str().unwrap()) == people::caller_person(&hub, &ana).as_deref()),
        "an admin sees members and themself"
    );
    let other_id = rows["sessions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["email"] == "cid@example.com")
        .map(|row| row["id"].as_str().unwrap().to_owned());
    assert!(other_id.is_none(), "another admin's invitation isn't listed");
    let other_invite = call(&hub, &Caller::Local, "memberSessions", &json!({})).unwrap();
    let cid = other_invite["sessions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["email"] == "cid@example.com")
        .map(|row| row["id"].as_str().unwrap().to_owned())
        .unwrap();
    assert!(call(&hub, &ana, "revokeMemberSession", &json!({"session": cid})).is_err());
    assert_eq!(other["person"]["role"], "admin");

    // Only the owner changes a role.
    let ben = call(&hub, &Caller::Local, "inviteMember", &json!({"email": "ben@example.com"})).await.unwrap();
    let ben_id = ben["person"]["id"].as_str().unwrap();
    assert!(call(&hub, &ana, "inviteMember", &json!({"email": "ben@example.com", "role": "admin"})).await.is_err());
    assert!(call(&hub, &ana, "updatePerson", &json!({"id": ben_id, "name": "Ben", "role": "admin"})).is_err());
}

#[tokio::test]
async fn company_sign_in_needs_a_configured_domain_and_an_invited_email() {
    let hub = hub();
    let store = &hub.store;
    *store.secret_key.lock().unwrap() = Some(zeroize::Zeroizing::new(vec![7; 32]));
    assert!(call(&hub, &Caller::Local, "memberSignIn", &json!({"email": "ana@example.com"})).await.is_err());
    let config = crate::enterprise::Config {
        issuer: "https://id.example.com".into(),
        client_id: "client".into(),
        domains: vec!["example.com".into()],
        endpoints: json!({}),
    };
    crate::market::vault::write(store, "enterprise.oidc", &serde_json::to_string(&config).unwrap()).unwrap();
    assert!(
        call(&hub, &Caller::Local, "memberSignIn", &json!({"email": "ana@other.com"})).await.is_err(),
        "a domain the company doesn't own"
    );
    assert!(
        call(&hub, &Caller::Local, "memberSignIn", &json!({"email": "ana@example.com"})).await.is_err(),
        "not invited yet"
    );
    call(&hub, &Caller::Local, "inviteMember", &json!({"email": "ana@example.com"})).await.unwrap();
    let answer = call(&hub, &Caller::Local, "memberSignIn", &json!({"email": "ANA@example.com"})).await.unwrap();
    assert_eq!(answer["role"], "member");
    assert!(caller_for_token(&hub, answer["token"].as_str().unwrap()).is_some());
}

#[tokio::test]
async fn a_member_session_cannot_reach_the_owners_bots_or_connectors() {
    let hub = hub();
    bot(&hub, "owner-bot", None);
    let (answer, ana) = signed_in(&hub, "ana@example.com").await;
    let ana_id = answer["person"]["id"].as_str().unwrap();
    bot(&hub, "ana-bot", Some(ana_id));

    // A member session is a Control device: it can't list people or devices, or read the owner's chat.
    assert!(call(&hub, &ana, "people", &json!({})).is_err());
    assert!(dispatch(&hub, &ana, "devices", json!({})).await.is_err());
    assert!(dispatch(&hub, &ana, "history", json!({"botId": "owner-bot"})).await.is_err());
    assert!(dispatch(&hub, &ana, "updateBot", json!({"botId": "owner-bot", "name": "mine"})).await.is_err());
    assert!(dispatch(&hub, &ana, "history", json!({"botId": "ana-bot"})).await.is_ok());
    let listed = dispatch(&hub, &ana, "connectors", json!({})).await.expect("connectors");
    assert!(listed["items"].is_array());
    // Control only, so the screen methods stay the owner's.
    assert!(dispatch(&hub, &ana, "screenPrepare", json!({})).await.is_err());
}

#[tokio::test]
async fn removing_a_person_ends_their_invitation_and_session() {
    let hub = hub();
    let invited = call(&hub, &Caller::Local, "inviteMember", &json!({"email": "ana@example.com"})).await.unwrap();
    let id = invited["person"]["id"].as_str().unwrap().to_owned();
    let code = invited["invite"]["code"].as_str().unwrap().to_owned();
    let answer = call(&hub, &Caller::Local, "acceptMemberInvite", &json!({"code": code})).unwrap();
    let token = answer["token"].as_str().unwrap().to_owned();
    // Inviting her again leaves a live invitation next to the session.
    let again = call(&hub, &Caller::Local, "inviteMember", &json!({"email": "ana@example.com"})).await.unwrap();
    assert!(invite_open(&hub, again["invite"]["code"].as_str().unwrap()));

    dispatch(&hub, &Caller::Local, "removePerson", json!({"id": id})).await.expect("removed");
    assert!(caller_for_token(&hub, &token).is_none(), "the session ended");
    assert!(!invite_open(&hub, again["invite"]["code"].as_str().unwrap()), "the invitation ended");
}

#[tokio::test]
async fn a_revoked_session_stops_authorizing_at_once() {
    let hub = hub();
    let (answer, ana) = signed_in(&hub, "ana@example.com").await;
    let token = answer["token"].as_str().unwrap().to_owned();
    assert!(dispatch(&hub, &ana, "hello", json!({})).await.is_ok());
    let key = match &ana {
        Caller::Device { key, .. } => key.clone(),
        _ => panic!("a member session is a device"),
    };
    call(&hub, &Caller::Local, "revokeMemberSession", &json!({"session": key})).unwrap();
    assert!(caller_for_token(&hub, &token).is_none());
    // The person stays, with their bots; only the access ends.
    assert!(hub.store.people().unwrap().iter().any(|person| person.email.as_deref() == Some("ana@example.com")));
}
