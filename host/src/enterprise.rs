//! Company sign-in and user sync: an `OpenID Connect` provider the owner names, checked by discovery, and a `bearer token`
//! for a SCIM client. Members live in `people`: SCIM creates and removes them there, and a verified
//! company email maps to one (`member_from_email`) for the login exchange in `members`.

use crate::hub::Hub;
use crate::store::Store;
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const CONFIG: &str = "enterprise.oidc";
const SCIM: &str = "enterprise.scim";

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub issuer: String,
    pub client_id: String,
    #[serde(default)]
    pub domains: Vec<String>,
    /// What discovery answered, kept so the settings can show it without asking again.
    #[serde(default)]
    pub endpoints: Value,
}

fn load(store: &Store) -> Result<Option<Config>> {
    crate::market::vault::read(store, CONFIG)?
        .filter(|s| !s.is_empty())
        .map(|s| serde_json::from_str(&s).map_err(Into::into))
        .transpose()
}

fn discovery_url(issuer: &str) -> String {
    format!("{}/.well-known/openid-configuration", issuer.trim_end_matches('/'))
}

/// Checks the issuer answers OIDC discovery, the client id has a sane shape and every domain looks like one.
async fn verify(issuer: &str, client_id: &str, domains: &[String]) -> Result<Value> {
    if !issuer.starts_with("https://") || issuer.len() > 300 || issuer.chars().any(char::is_whitespace) {
        bail!("The issuer must be an https address.");
    }
    if client_id.is_empty() || client_id.len() > 200 || client_id.chars().any(char::is_whitespace) {
        bail!("Enter the client id from the provider.");
    }
    if domains.len() > 20 || domains.iter().any(|d| d.is_empty() || d.len() > 200 || !d.contains('.')) {
        bail!("Use up to 20 email domains, such as example.com.");
    }
    let response = crate::http()
        .get(discovery_url(issuer))
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .context("The provider did not answer discovery.")?;
    if !response.status().is_success() {
        bail!("The provider answered {} for its discovery document.", response.status());
    }
    let answer: Value = response.json().await.context("The discovery document is not JSON.")?;
    if answer["issuer"].as_str().is_none()
        || answer["authorization_endpoint"].as_str().is_none()
        || answer["token_endpoint"].as_str().is_none()
    {
        bail!("The discovery document lacks its issuer or endpoints.");
    }
    Ok(json!({
        "issuer": answer["issuer"],
        "authorizationEndpoint": answer["authorization_endpoint"],
        "tokenEndpoint": answer["token_endpoint"],
        "userinfoEndpoint": answer["userinfo_endpoint"],
    }))
}

/// What the settings show: the saved company sign-in, without secrets, and whether a SCIM token exists.
pub fn status(store: &Store) -> Result<Value> {
    let config = load(store)?;
    let configured = config.is_some();
    Ok(json!({
        "configured": configured,
        "issuer": config.as_ref().map(|c| &c.issuer),
        "clientId": config.as_ref().map(|c| &c.client_id),
        "domains": config.as_ref().map(|c| &c.domains),
        "endpoints": config.as_ref().map(|c| &c.endpoints),
        "scimTokenSet": crate::market::vault::read(store, SCIM)?.is_some_and(|t| !t.is_empty()),
    }))
}

pub async fn set(hub: &std::sync::Arc<Hub>, body: &Value) -> Result<Value> {
    crate::market::vault::unlock(hub.clone()).await.ok();
    let issuer = body["issuer"].as_str().unwrap_or_default().trim().to_owned();
    let client_id = body["clientId"].as_str().unwrap_or_default().trim().to_owned();
    let domains: Vec<String> = body["domains"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(|d| d.trim().trim_start_matches('@').to_lowercase())
        .filter(|d| !d.is_empty())
        .collect();
    let endpoints = verify(&issuer, &client_id, &domains).await?;
    let config = Config { issuer, client_id, domains, endpoints };
    crate::market::vault::write(&hub.store, CONFIG, &serde_json::to_string(&config)?)?;
    status(&hub.store)
}

pub fn disconnect(store: &Store) -> Result<Value> {
    crate::market::vault::write(store, CONFIG, "")?;
    status(store)
}

/// Makes a new SCIM bearer token (32 bytes, base64url), replacing the old one.
pub fn rotate_scim(store: &Store) -> Result<String> {
    let token = crate::remote::crypto::b64(&crate::remote::crypto::random::<32>());
    crate::market::vault::write(store, SCIM, &token)?;
    Ok(token)
}

/// A person as the directory and SCIM see them (`docs/features/people.md`).
pub fn member_json(person: &crate::store::Person) -> Value {
    json!({
        "id": person.id,
        "userName": person.email.clone().unwrap_or_else(|| person.name.clone()),
        "displayName": person.name,
        "active": true,
        "createdAt": person.created_at,
        "role": person.role.as_str(),
    })
}

/// The people a SCIM client or an owner created, without secrets. The `people` table is the one
/// source of truth: a member is a person with an email (see `docs/features/people.md`).
pub fn member_list(store: &Store) -> Result<Value> {
    let members: Vec<Value> = store.people()?.iter().map(member_json).collect();
    Ok(json!({"members": members}))
}

/// Adds a member from an email (the SCIM `userName`), or updates the one with that email. A person
/// created this way is a `member`: a directory sync never grants an admin.
pub fn add_member(store: &Store, user_name: &str, display_name: &str) -> Result<Value> {
    let user_name = user_name.trim().to_lowercase();
    if user_name.len() < 3
        || user_name.len() > 200
        || !user_name.contains('@')
        || user_name.chars().any(char::is_whitespace)
    {
        bail!("Use the person's email as their user name.");
    }
    let display_name = display_name.trim();
    if display_name.chars().count() > 120 {
        bail!("That name is too long.");
    }
    let person = match store.person_by_email(&user_name) {
        Some(existing) => {
            let name = if display_name.is_empty() { existing.name.clone() } else { display_name.to_owned() };
            store.update_person(&existing.id, &name, Some(&user_name))?.unwrap_or(existing)
        }
        None => {
            let name =
                if display_name.is_empty() { user_name.split('@').next().unwrap_or(&user_name) } else { display_name };
            store.add_person_with_role(name, Some(&user_name), crate::store::Role::Member)?
        }
    };
    Ok(json!({"member": member_json(&person)}))
}

/// Removes the member with this id, the same way Settings does: their devices and invitations go,
/// their bots and connectors become the owner's, and nothing is deleted. False when it wasn't there.
pub fn remove_member(hub: &std::sync::Arc<Hub>, id: &str) -> Result<bool> {
    Ok(crate::people::remove(hub, id)?["removed"].as_bool().unwrap_or(false))
}

/// The member a SCIM resource id names, if any.
pub fn member_by_id(store: &Store, id: &str) -> Option<crate::store::Person> {
    store.person(id)
}

/// The person a signed-in company email belongs to, whatever its case. Company sign-in maps the
/// provider's verified email through this, then exchanges it for a member session at the computer
/// (`members::memberSignIn`).
pub fn member_from_email(store: &Store, email: &str) -> Option<crate::store::Person> {
    store.person_by_email(email)
}

/// Whether `email` is at a domain the configured company owns. Company sign-in never maps an email
/// from anywhere else to a member, even if a person happens to carry that address.
pub fn domain_allows(store: &Store, email: &str) -> Result<bool> {
    let Some(config) = load(store)? else { return Ok(false) };
    let Some((_, domain)) = email.trim().to_lowercase().rsplit_once('@') else { return Ok(false) };
    Ok(config.domains.iter().any(|allowed| allowed == domain))
}

/// The SCIM list this computer can answer: the account owner and the people on it. A person is a
/// real member account now, so its devices are its sessions, not users of their own.
pub fn scim_users(hub: &Hub) -> Result<Value> {
    let mut resources = Vec::new();
    if let Some(owner) = hub.cloud.status().owner {
        resources.push(json!({
            "schemas": ["urn:ietf:params:scim:api:messages:2.0:User"],
            "id": owner.user_id,
            "userName": owner.email.unwrap_or_else(|| owner.user_id.clone()),
            "active": true,
            "displayName": "Owner",
        }));
    }
    for person in hub.store.people()? {
        let mut resource = member_json(&person);
        resource["schemas"] = json!(["urn:ietf:params:scim:api:messages:2.0:User"]);
        resources.push(resource);
    }
    let total = resources.len();
    Ok(json!({"schemas": ["urn:ietf:params:scim:api:messages:2.0:ListResponse"], "totalResults": total,
        "startIndex": 1, "itemsPerPage": total, "Resources": resources}))
}

/// Whether `authorization: Bearer …` carries the SCIM token.
pub fn scim_allowed(store: &Store, header: Option<&str>) -> bool {
    let Some(given) = header.and_then(|h| h.strip_prefix("Bearer ")) else { return false };
    crate::market::vault::read(store, SCIM).ok().flatten().is_some_and(|token| {
        !token.is_empty()
            && token.len() == given.len()
            && token.bytes().zip(given.bytes()).fold(0_u8, |d, (a, b)| d | (a ^ b)) == 0
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::identity::Identity;
    use crate::store::Store;

    fn hub() -> std::sync::Arc<Hub> {
        let dir = std::env::temp_dir().join(format!("codync-enterprise-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        Hub::new(store, "x".into(), Identity::load_or_create(&dir).unwrap(), "t".into(), 0)
    }

    #[test]
    fn discovery_lives_under_the_issuer() {
        assert_eq!(discovery_url("https://id.example.com/"), "https://id.example.com/.well-known/openid-configuration");
    }

    #[test]
    fn a_scim_member_is_a_person_who_can_sign_in() {
        let hub = hub();
        *hub.store.secret_key.lock().unwrap() = Some(zeroize::Zeroizing::new(vec![7; 32]));
        add_member(&hub.store, "Ana@Example.com", "Ana").unwrap();
        add_member(&hub.store, "ana@example.com", "Ana B").unwrap();
        let people = hub.store.people().unwrap();
        assert_eq!(people.len(), 1, "the same email updates the member");
        assert_eq!(people[0].name, "Ana B");
        assert_eq!(people[0].role, crate::store::Role::Member, "a directory sync never grants an admin");
        assert_eq!(member_from_email(&hub.store, "ANA@example.com").unwrap().id, people[0].id);
        assert!(member_by_id(&hub.store, &people[0].id).is_some());
        assert!(member_list(&hub.store).unwrap()["members"].as_array().unwrap().len() == 1);

        assert!(remove_member(&hub, &people[0].id).unwrap());
        assert!(!remove_member(&hub, &people[0].id).unwrap());
        assert!(member_from_email(&hub.store, "ana@example.com").is_none());
        assert!(add_member(&hub.store, "nonsense", "").is_err());
    }

    #[test]
    fn company_sign_in_only_maps_the_domains_the_owner_named() {
        let hub = hub();
        *hub.store.secret_key.lock().unwrap() = Some(zeroize::Zeroizing::new(vec![7; 32]));
        add_member(&hub.store, "ana@example.com", "Ana").unwrap();
        assert!(!domain_allows(&hub.store, "ana@example.com").unwrap(), "no company sign-in yet");
        let config = Config {
            issuer: "https://id.example.com".into(),
            client_id: "client".into(),
            domains: vec!["example.com".into()],
            endpoints: json!({}),
        };
        crate::market::vault::write(&hub.store, CONFIG, &serde_json::to_string(&config).unwrap()).unwrap();
        assert!(domain_allows(&hub.store, "ANA@Example.com").unwrap());
        assert!(!domain_allows(&hub.store, "ana@elsewhere.com").unwrap());
        assert!(domain_allows(&hub.store, "nonsense").is_ok_and(|allowed| !allowed));
        assert_eq!(member_from_email(&hub.store, "ana@example.com").unwrap().name, "Ana");
    }

    #[test]
    fn only_the_current_token_is_accepted() {
        let hub = hub();
        *hub.store.secret_key.lock().unwrap() = Some(zeroize::Zeroizing::new(vec![7; 32]));
        assert!(!scim_allowed(&hub.store, None));
        let token = rotate_scim(&hub.store).unwrap();
        assert!(scim_allowed(&hub.store, Some(&format!("Bearer {token}"))));
        assert!(!scim_allowed(&hub.store, Some("Bearer nope")));
        let newer = rotate_scim(&hub.store).unwrap();
        assert!(!scim_allowed(&hub.store, Some(&format!("Bearer {token}"))));
        assert!(scim_allowed(&hub.store, Some(&format!("Bearer {newer}"))));
    }
}
