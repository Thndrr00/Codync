//! People on this computer: the owner, who is implicit, and everyone else who uses it with their own
//! paired phone or their own bots (docs/features/people.md).
//!
//! Codync has no member sign-in. A person is a record the owner creates here, and a device or a bot
//! names which person it belongs to. The host authorizes every call by that pair: a device assigned to a
//! person acts as that person (`api::devices::caller_person`), and a bot's person decides which connectors
//! it may use and whose chats it may read. Everything is checked here, never in a client.
//!
//! Removing a person never deletes their work: their devices, bots and connectors become the owner's, and
//! the chat stays where it was.

use crate::hub::Hub;
use crate::market;
use anyhow::{Result, bail};
use serde_json::{Value, json};
use std::sync::Arc;

/// The methods this module answers (loopback only: only the owner manages who is on their computer).
pub const METHODS: [&str; 5] = ["people", "addPerson", "updatePerson", "removePerson", "setDevicePerson"];

/// Longest a person's name may be, in characters.
const MAX_NAME_CHARS: usize = 80;
/// Longest email a person may carry.
const MAX_EMAIL_CHARS: usize = 200;

pub fn call(hub: &Arc<Hub>, method: &str, b: &Value) -> Result<Value> {
    match method {
        "people" => list(hub),
        "addPerson" => add(hub, text(b, "name")?, b["email"].as_str()),
        "updatePerson" => update(hub, required(b, "id")?, text(b, "name")?, b["email"].as_str()),
        "removePerson" => remove(hub, required(b, "id")?),
        "setDevicePerson" => {
            let person = match &b["person"] {
                Value::String(id) if !id.is_empty() => Some(id.as_str()),
                Value::Null | Value::String(_) => None,
                _ => bail!("`person` must be a person id or null"),
            };
            set_device_person(hub, required(b, "device")?, person)
        }
        _ => bail!("unknown people method: {method}"),
    }
}

/// Everyone on this computer besides its owner.
pub fn list(hub: &Hub) -> Result<Value> {
    Ok(json!({"people": hub.store.people()?}))
}

pub fn add(hub: &Arc<Hub>, name: &str, email: Option<&str>) -> Result<Value> {
    let name = checked_name(name)?;
    let person = hub.store.add_person(name, checked_email(email)?)?;
    Ok(json!({"person": person}))
}

pub fn update(hub: &Arc<Hub>, id: &str, name: &str, email: Option<&str>) -> Result<Value> {
    let name = checked_name(name)?;
    let person =
        hub.store.update_person(id, name, checked_email(email)?)?.ok_or_else(|| anyhow::anyhow!("unknown person"))?;
    Ok(json!({"person": person}))
}

/// Removes a person: their devices, bots and connectors are unassigned and become the owner's, and their
/// chats stay with their bots. Nothing they set up is deleted.
pub fn remove(hub: &Arc<Hub>, id: &str) -> Result<Value> {
    if hub.store.person(id).is_none() {
        return Ok(json!({"removed": false}));
    }
    // Bots first: their connectors stay on them, so no bot loses a tool it had.
    for row in hub.store.bots()?.into_iter().filter(|row| row.config.person.as_deref() == Some(id)) {
        let mut cfg = row.config;
        cfg.person = None;
        hub.store.save_bot(&cfg)?;
        hub.emit_bot(&cfg.id);
    }
    market::update_connectors(&hub.store, |all| {
        for c in all.iter_mut().filter(|c| c.person.as_deref() == Some(id)) {
            c.person = None;
        }
        Ok(())
    })?;
    Ok(json!({"removed": hub.store.remove_person(id)?}))
}

/// Makes `device` belong to `person`, or gives it back to the owner with `None`. A member's phone then acts
/// as that person: it lists that person's bots and connectors, and nothing of another person's.
pub fn set_device_person(hub: &Arc<Hub>, device: &str, person: Option<&str>) -> Result<Value> {
    if hub.store.device(device).is_none() {
        bail!("unknown device");
    }
    if let Some(person) = person
        && hub.store.person(person).is_none()
    {
        bail!("unknown person");
    }
    Ok(json!({"changed": hub.store.set_device_person(device, person)?}))
}

fn required<'a>(b: &'a Value, key: &str) -> Result<&'a str> {
    b[key].as_str().filter(|v| !v.is_empty()).ok_or_else(|| anyhow::anyhow!("`{key}` is required"))
}

fn text<'a>(b: &'a Value, key: &str) -> Result<&'a str> {
    b[key].as_str().ok_or_else(|| anyhow::anyhow!("`{key}` is required"))
}

/// A name, trimmed and nonempty.
fn checked_name(name: &str) -> Result<&str> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > MAX_NAME_CHARS {
        bail!("Use a name of up to {MAX_NAME_CHARS} characters.");
    }
    Ok(name)
}

/// An optional email: trimmed, empty is none, and it has to look like an address.
fn checked_email(email: Option<&str>) -> Result<Option<&str>> {
    let Some(email) = email.map(str::trim).filter(|e| !e.is_empty()) else { return Ok(None) };
    if email.len() > MAX_EMAIL_CHARS || !email.contains('@') || email.chars().any(char::is_whitespace) {
        bail!("Use an email like person@example.com, or leave it empty.");
    }
    Ok(Some(email))
}

/// The person a caller acts as, from the device's row: `None` for the owner (this computer, or a device
/// that was never assigned to a person). Every check in the host starts here.
pub fn caller_person(hub: &Hub, caller: &crate::api::devices::Caller) -> Option<String> {
    match caller {
        crate::api::devices::Caller::Device { key, .. } => hub.store.device(key).and_then(|d| d.person_id),
        _ => None,
    }
}

/// Refuses a call from a member's device that names a bot (or a message or thread in it) belonging to
/// someone else. Called once, in `api::dispatch`, for every method that names one.
pub fn check_bot(hub: &Hub, b: &Value, person: &str) -> Result<()> {
    if let Some(id) = b["botId"].as_str().or_else(|| b["bot"].as_str()) {
        return check_bot_id(hub, id, person);
    }
    for key in ["id", "entryId", "threadId", "rootId"] {
        let Some(id) = b[key].as_str() else { continue };
        if hub.store.bot(id)?.is_some() {
            return check_bot_id(hub, id, person);
        }
        if let Some(entry) = hub.store.entry(id) {
            return check_bot_id(hub, &entry.bot_id, person);
        }
        if let Some(thread) = hub.store.thread_by_id(id)? {
            return check_bot_id(hub, &thread.bot_id, person);
        }
    }
    Ok(())
}

fn check_bot_id(hub: &Hub, id: &str, person: &str) -> Result<()> {
    if let Some(row) = hub.store.bot(id)?
        && row.config.person.as_deref() != Some(person)
    {
        bail!("That bot belongs to someone else on this computer.");
    }
    Ok(())
}

/// Refuses a group whose members belong to someone else: a member can't put the owner's bot in their channel,
/// or a bot of another person's.
pub fn check_group_members(hub: &Hub, b: &Value, person: &str) -> Result<()> {
    let Some(members) = b["members"].as_array() else { return Ok(()) };
    for id in members.iter().filter_map(Value::as_str) {
        if let Some(row) = hub.store.bot(id)?
            && row.config.person.as_deref() != Some(person)
        {
            bail!("That bot belongs to someone else on this computer.");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::identity::Identity;
    use crate::store::{BotConfig, Device, DeviceAccount, DeviceSource, Scope, Store};
    use std::path::Path;

    fn hub() -> Arc<Hub> {
        let dir = std::env::temp_dir().join(format!("codync-people-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::open(Path::new(":memory:")).unwrap();
        Hub::new(store, "x".into(), Identity::load_or_create(&dir).unwrap(), "t".into(), 0)
    }

    fn device(hub: &Hub, key: &str, person: Option<&str>) {
        hub.store
            .put_device(&Device {
                key: key.into(),
                name: "Phone".into(),
                platform: "ios".into(),
                source: DeviceSource::Local,
                grant_id: None,
                scopes: vec![Scope::Control, Scope::Screen],
                lease_until: None,
                created_at: 1,
                last_seen_at: None,
                account: DeviceAccount::Unrecorded,
                person_id: person.map(str::to_owned),
            })
            .unwrap();
    }

    fn bot(hub: &Hub, id: &str, person: Option<&str>) {
        let cfg: BotConfig = serde_json::from_value(json!({
            "id": id, "name": id, "backend": "fixture", "command": "true", "cwd": "/tmp",
            "person": person, "notify": false,
        }))
        .unwrap();
        hub.store.save_bot(&cfg).unwrap();
    }

    #[test]
    fn only_the_owner_manages_people_and_their_names_are_checked() {
        let hub = hub();
        assert!(super::add(&hub, "   ", None).is_err());
        assert!(super::add(&hub, "Ana", Some("nonsense")).is_err());
        let ana = super::add(&hub, " Ana ", Some("ana@example.com")).unwrap();
        let id = ana["person"]["id"].as_str().unwrap();
        assert_eq!(super::update(&hub, id, "Ana B", None).unwrap()["person"]["name"], "Ana B");
        assert!(super::update(&hub, "nope", "Nobody", None).is_err());
        assert_eq!(super::list(&hub).unwrap()["people"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn removing_a_person_gives_their_bots_and_devices_to_the_owner() {
        let hub = hub();
        let person = hub.store.add_person("Ana", None).unwrap();
        device(&hub, "ana-phone", Some(&person.id));
        bot(&hub, "ana-bot", Some(&person.id));
        assert_eq!(
            caller_person(
                &hub,
                &crate::api::devices::Caller::Device { key: "ana-phone".into(), scopes: vec![Scope::Control] }
            ),
            Some(person.id.clone())
        );

        assert_eq!(remove(&hub, &person.id).unwrap()["removed"], true);
        assert_eq!(hub.store.device("ana-phone").unwrap().person_id, None);
        assert_eq!(hub.store.bot("ana-bot").unwrap().unwrap().config.person, None);
        assert!(!remove(&hub, &person.id).unwrap()["removed"].as_bool().unwrap());
    }

    #[test]
    fn a_member_cannot_name_another_persons_bot() {
        let hub = hub();
        let ana = hub.store.add_person("Ana", None).unwrap();
        let ben = hub.store.add_person("Ben", None).unwrap();
        bot(&hub, "owned", Some(&ana.id));
        assert!(check_bot(&hub, &json!({"botId": "owned"}), &ana.id).is_ok());
        assert!(check_bot(&hub, &json!({"botId": "owned"}), &ben.id).is_err());
        assert!(check_bot(&hub, &json!({"id": "owned"}), &ben.id).is_err(), "updateBot names a bot by id");
        assert!(check_bot(&hub, &json!({"botId": "missing"}), &ben.id).is_ok(), "not a bot: left to the method");
    }

    use crate::api::dispatch;

    fn member(hub: &Hub, key: &str, person: &str) -> crate::api::devices::Caller {
        device(hub, key, Some(person));
        crate::api::devices::Caller::Device { key: key.into(), scopes: vec![Scope::Control] }
    }

    async fn install(hub: &Arc<Hub>, caller: &crate::api::devices::Caller, name: &str) -> String {
        let answer = dispatch(
            hub,
            caller,
            "installConnector",
            json!({"name": name, "command": "node", "args": [format!("{name}.js")]}),
        )
        .await
        .expect("install");
        answer["connector"]["id"].as_str().expect("id").to_owned()
    }

    /// The connectors a caller's own `connectors` call lists, by id.
    async fn listed(hub: &Arc<Hub>, caller: &crate::api::devices::Caller) -> Vec<String> {
        dispatch(hub, caller, "connectors", json!({})).await.expect("connectors")["items"]
            .as_array()
            .expect("items")
            .iter()
            .filter_map(|c| c["id"].as_str().map(str::to_owned))
            .collect()
    }

    #[tokio::test]
    async fn a_member_lists_and_changes_only_their_own_connectors() {
        let hub = hub();
        let ana = hub.store.add_person("Ana", None).unwrap();
        let ben = hub.store.add_person("Ben", None).unwrap();
        let ana_caller = member(&hub, "ana-phone", &ana.id);
        let ben_caller = member(&hub, "ben-phone", &ben.id);

        let shared = install(&hub, &crate::api::devices::Caller::Local, "Owner Server").await;
        // The body names Ben, but Ana's device makes it Ana's.
        let answer = dispatch(
            &hub,
            &ana_caller,
            "installConnector",
            json!({"name": "Ana Server", "command": "node", "args": ["ana.js"], "person": ben.id}),
        )
        .await
        .expect("member install");
        let ana_connector = answer["connector"]["id"].as_str().expect("id").to_owned();
        let stored = crate::market::connectors(&hub.store).unwrap();
        assert_eq!(
            stored.iter().find(|c| c.id == ana_connector).unwrap().person.as_deref(),
            Some(ana.id.as_str()),
            "a member's install is theirs even when the body says otherwise"
        );
        let ben_connector = install(&hub, &ben_caller, "Ben Server").await;

        assert_eq!(listed(&hub, &crate::api::devices::Caller::Local).await.len(), 3, "the owner lists all");
        assert_eq!(listed(&hub, &ana_caller).await, [shared.clone(), ana_connector.clone()]);
        assert_eq!(listed(&hub, &ben_caller).await, [shared.clone(), ben_connector.clone()]);

        // A member can't touch a shared connector, or another person's.
        assert!(dispatch(&hub, &ana_caller, "removeConnector", json!({"id": shared})).await.is_err());
        assert!(dispatch(&hub, &ana_caller, "removeConnector", json!({"id": ben_connector})).await.is_err());
        assert!(
            dispatch(&hub, &ana_caller, "credentialUpdateConnector", json!({"id": shared, "fields": {"TOKEN": "x"}}))
                .await
                .is_err()
        );
        // Their own goes, and only theirs.
        dispatch(&hub, &ana_caller, "removeConnector", json!({"id": ana_connector})).await.expect("own");
        assert_eq!(listed(&hub, &ben_caller).await, [shared, ben_connector]);
    }

    #[tokio::test]
    async fn a_new_bot_gets_the_shared_connectors_and_its_persons() {
        let hub = hub();
        let ana = hub.store.add_person("Ana", None).unwrap();
        let ana_caller = member(&hub, "ana-phone", &ana.id);
        let shared = install(&hub, &crate::api::devices::Caller::Local, "Owner Server").await;
        let own = install(&hub, &ana_caller, "Ana Server").await;

        let created = dispatch(
            &hub,
            &ana_caller,
            "createBot",
            json!({
                "name": "Ana bot", "backend": "fixture", "command": "true",
                "cwd": std::env::temp_dir().to_string_lossy(),
            }),
        )
        .await
        .expect("create");
        assert_eq!(created["bot"]["person"], json!(ana.id), "a member's bot is theirs");
        let cfg = hub.store.bot(created["bot"]["id"].as_str().unwrap()).unwrap().unwrap().config;
        assert_eq!(cfg.connectors, [shared, own.clone()], "shared first, then its person's");
        // The owner's own bot gets the shared one only.
        let owner_bot: crate::store::BotConfig = serde_json::from_value(json!({
            "id": "owner-bot", "name": "Owner bot", "backend": "fixture", "command": "true", "cwd": "/tmp",
        }))
        .unwrap();
        assert_eq!(crate::market::connector_ids_for(&hub.store, owner_bot.person.as_deref()).unwrap(), [shared]);
        let anas = crate::market::connectors(&hub.store).unwrap();
        assert!(crate::market::for_bot(anas.iter().find(|c| c.id == own).unwrap(), Some(&ana.id)));
        assert!(!crate::market::for_bot(anas.iter().find(|c| c.id == own).unwrap(), None));
    }

    #[tokio::test]
    async fn a_members_cross_chat_search_cannot_open_the_owners_chat() {
        let hub = hub();
        let ana = hub.store.add_person("Ana", None).unwrap();
        bot(&hub, "owner-bot", None);
        bot(&hub, "ana-bot", Some(&ana.id));
        let owner_entry = hub
            .store
            .insert_entry(
                &crate::store::Lane::main("owner-bot"),
                crate::store::EntryKind::User,
                1,
                &json!({"text": "the owner's private note"}),
            )
            .unwrap();
        let ana_entry = hub
            .store
            .insert_entry(
                &crate::store::Lane::main("ana-bot"),
                crate::store::EntryKind::User,
                1,
                &json!({"text": "Ana's note"}),
            )
            .unwrap();
        let ana_cfg = hub.store.bot("ana-bot").unwrap().unwrap().config;
        let chats = crate::chat::memory::chats(&hub, &ana_cfg);
        assert_eq!(chats, ["ana-bot"], "never the owner's chat");
        assert!(
            crate::chat::memory::tool_call(&hub, "ana-bot", "read_history", &json!({"id": owner_entry.id}))
                .await
                .is_err(),
            "reading around the owner's message is refused"
        );
        assert!(
            crate::chat::memory::tool_call(&hub, "ana-bot", "read_history", &json!({"id": ana_entry.id})).await.is_ok(),
            "its own chat stays readable"
        );
        let found = crate::chat::memory::tool_call(&hub, "ana-bot", "search_history", &json!({"query": "private"}))
            .await
            .expect("search");
        assert_eq!(found["results"].as_array().map(Vec::len), Some(0), "the owner's message is not in its chats");
    }
}
