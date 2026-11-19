//! Secrets a bot keeps for its own work: an API key pinned to one HTTPS origin, which the host
//! attaches to `api_request` calls, a command variable the host exports into
//! `run_with_secrets` commands, or a website login with an optional two-factor seed. The bot
//! asks for one with a secure card and never gets the password, key or seed back. One encrypted
//! vault record holds them all.
//!
//! Idea from rakazo's `docs/bot-secrets.md` (Apache-2.0); no code copied.

use super::totp;
use crate::LockExt;
use crate::store::Store;
use anyhow::{Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const SLOT: &str = "botSecrets";
/// Secrets one bot may keep.
const MAX_PER_BOT: usize = 100;
/// Largest secret value.
const MAX_VALUE: usize = 16 * 1024;
/// Shortest secret value: shorter ones couldn't be blanked out of output safely.
const MIN_VALUE: usize = 3;
/// One read-modify-write of the record at a time.
static EDIT: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BotSecret {
    pub id: String,
    pub bot_id: String,
    /// What the bot calls it: letters, digits, `-` and `_`.
    pub name: String,
    #[serde(rename = "use")]
    pub usage: Usage,
    /// The secret, or an `op://` reference resolved each time it's used.
    pub value: String,
}

/// Where a secret goes.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Usage {
    /// Attached to `api_request` calls to this origin (`https://host[:port]`).
    Http { origin: String, auth: Auth },
    /// Exported as `NAME` (upper case, `-` → `_`) to `run_with_secrets` commands.
    Command,
    /// A website login: `value` holds the password and `totp` the base32 two-factor seed,
    /// both in the encrypted record. The seed is never put in a listing or a tool answer.
    Login { username: String, origin: String, totp: Option<String> },
}

/// How an `Http` secret is attached.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "scheme", rename_all = "camelCase")]
pub enum Auth {
    /// `Authorization: Bearer <secret>`.
    Bearer,
    /// `<header>: <secret>`.
    Header { header: String },
    /// `Authorization: Basic` with this username and the secret as password.
    Basic { username: String },
}

impl Usage {
    /// What the secret is for, in the words the card and Credentials show.
    pub fn destination(&self, name: &str) -> String {
        match self {
            Usage::Http { origin, auth: Auth::Bearer } => format!("{origin} · bearer token"),
            Usage::Http { origin, auth: Auth::Header { header } } => format!("{origin} · {header} header"),
            Usage::Http { origin, auth: Auth::Basic { username } } => format!("{origin} · password for {username}"),
            Usage::Command => format!("{} in commands", crate::shell::var_ref(&env_name(name))),
            Usage::Login { origin, username, totp } => {
                let who = if username.is_empty() { "login".to_owned() } else { format!("login for {username}") };
                format!("{origin} · {who}{}", if totp.is_some() { " · 2FA" } else { "" })
            }
        }
    }

    /// The use a client or a tool may see: a login's two-factor seed is replaced by a flag.
    pub fn public(&self) -> Value {
        if let Usage::Login { username, origin, totp } = self {
            json!({"kind":"login","username":username,"origin":origin,"hasTotp":totp.is_some()})
        } else {
            serde_json::to_value(self).unwrap_or(Value::Null)
        }
    }
}

impl BotSecret {
    /// Everything but the value and the two-factor seed.
    pub fn public(&self) -> Value {
        let mut value = json!({"id": self.id, "botId": self.bot_id, "name": self.name,
            "use": self.usage.public(), "destination": self.usage.destination(&self.name)});
        if let Usage::Login { username, origin, .. } = &self.usage {
            value["username"] = json!(username);
            value["origin"] = json!(origin);
            value["hasTotp"] = json!(self.has_totp());
        }
        value
    }

    /// Whether this is a saved website login.
    pub fn has_login(&self) -> bool {
        matches!(&self.usage, Usage::Login { .. })
    }

    /// Whether a two-factor code can be made for this secret.
    pub fn has_totp(&self) -> bool {
        matches!(&self.usage, Usage::Login { totp: Some(_), .. })
    }

    /// The seed the host uses to make a code; never returned to a client or a tool answer.
    pub fn totp(&self) -> Option<&str> {
        match &self.usage {
            Usage::Login { totp: Some(seed), .. } => Some(seed),
            _ => None,
        }
    }
}

/// `netbird-setup-key` → `NETBIRD_SETUP_KEY`.
pub fn env_name(name: &str) -> String {
    name.to_ascii_uppercase().replace('-', "_")
}

/// Variables a secret's name may not become: they'd change how the shell or its programs run.
const RESERVED_VARS: [&str; 14] = [
    "PATH",
    "HOME",
    "IFS",
    "ENV",
    "BASH_ENV",
    "SHELL",
    "USER",
    "COMSPEC",
    "PATHEXT",
    "SYSTEMROOT",
    "TEMP",
    "TMP",
    "TMPDIR",
    "USERPROFILE",
];

pub fn check_name(name: &str) -> Result<()> {
    let ok = (1..=64).contains(&name.len())
        && name.starts_with(|c: char| c.is_ascii_alphabetic())
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
    if !ok {
        bail!("A secret's name is 1-64 letters, digits, `-` or `_`, starting with a letter");
    }
    let var = env_name(name);
    if RESERVED_VARS.contains(&var.as_str()) || ["LD_", "DYLD_", "CODYNC_"].iter().any(|p| var.starts_with(p)) {
        bail!("`{name}` would replace the {var} variable; pick another name");
    }
    Ok(())
}

/// `https://api.example.com/v1/` → `https://api.example.com`; anything but a bare HTTPS origin is refused.
pub fn origin(url: &str) -> Result<String> {
    let u = reqwest::Url::parse(url.trim()).map_err(|_| anyhow!("`{url}` isn't an address"))?;
    if u.scheme() != "https" || u.host_str().is_none() || !u.username().is_empty() || u.password().is_some() {
        bail!("A secret's origin is an https address, like https://api.example.com");
    }
    Ok(u.origin().ascii_serialization())
}

/// Validates the parts of an `Http` secret a bot names.
pub fn check_auth(auth: &Auth) -> Result<()> {
    match auth {
        Auth::Bearer => Ok(()),
        Auth::Header { header } => {
            let token = !header.is_empty()
                && header.len() <= 64
                && reqwest::header::HeaderName::from_bytes(header.as_bytes()).is_ok();
            let reserved = ["host", "cookie", "content-length", "transfer-encoding", "connection"];
            if token && !reserved.contains(&header.to_ascii_lowercase().as_str()) {
                Ok(())
            } else {
                bail!("`{header}` can't carry a secret")
            }
        }
        Auth::Basic { username } if !username.is_empty() && username.len() <= 256 && !username.contains(':') => Ok(()),
        Auth::Basic { .. } => bail!("Give the username for basic auth (no `:`)"),
    }
}

/// A login's username is only ever displayed; keep it short and on one line.
pub fn check_login_username(username: &str) -> Result<()> {
    let username = username.trim();
    if username.is_empty() || username.len() > 256 || username.chars().any(char::is_control) {
        bail!("Give a username of 1-256 characters, without line breaks");
    }
    Ok(())
}

fn all(store: &Store) -> Result<Vec<BotSecret>> {
    Ok(match super::vault::read(store, SLOT)? {
        Some(s) if !s.is_empty() => serde_json::from_str(&s)?,
        _ => vec![],
    })
}

fn save_all(store: &Store, items: &[BotSecret]) -> Result<()> {
    super::vault::write(store, SLOT, &serde_json::to_string(items)?)
}

pub fn for_bot(store: &Store, bot: &str) -> Result<Vec<BotSecret>> {
    Ok(all(store)?.into_iter().filter(|s| s.bot_id == bot).collect())
}

pub fn find(store: &Store, bot: &str, name: &str) -> Result<BotSecret> {
    for_bot(store, bot)?
        .into_iter()
        .find(|s| s.name.eq_ignore_ascii_case(name))
        .ok_or_else(|| anyhow!("No secret named `{name}`; check list_secrets, or ask for one"))
}

/// Saves `bot`'s secret `name`, replacing the one it had by that name.
pub fn save(store: &Store, bot: &str, name: &str, usage: Usage, value: &str) -> Result<BotSecret> {
    check_name(name)?;
    // A pasted value often ends in a newline that isn't part of the key.
    let value = value.trim();
    if value.len() < MIN_VALUE || value.len() > MAX_VALUE {
        bail!("A secret is {MIN_VALUE} characters to {} KB", MAX_VALUE / 1024);
    }
    let _one = EDIT.locked();
    let mut items = all(store)?;
    let secret = if let Some(s) = items.iter_mut().find(|s| s.bot_id == bot && s.name.eq_ignore_ascii_case(name)) {
        s.usage = usage;
        value.clone_into(&mut s.value);
        s.clone()
    } else {
        if items.iter().filter(|s| s.bot_id == bot).count() >= MAX_PER_BOT {
            bail!("This bot already keeps {MAX_PER_BOT} secrets; remove one in Credentials first");
        }
        let taken: Vec<String> = items.iter().map(|s| s.id.clone()).collect();
        let s = BotSecret {
            id: super::unique_id(&format!("{bot}-{name}"), &taken),
            bot_id: bot.to_owned(),
            name: name.to_owned(),
            usage,
            value: value.to_owned(),
        };
        items.push(s.clone());
        s
    };
    save_all(store, &items)?;
    Ok(secret)
}

/// Saves a login, replacing one with the same name. Its password and optional base32
/// two-factor seed go into the encrypted value alongside the rest of the record.
pub fn save_login(
    store: &Store,
    bot: &str,
    name: &str,
    username: &str,
    site: &str,
    password: &str,
    seed: Option<&str>,
) -> Result<BotSecret> {
    let origin = origin(site)?;
    let username = username.trim().to_owned();
    check_login_username(&username)?;
    if let Some(seed) = seed {
        totp::check_secret(seed)?;
    }
    let usage = Usage::Login { username, origin, totp: seed.map(str::to_owned) };
    save(store, bot, name, usage, password)
}

pub fn find_by_id(store: &Store, id: &str) -> Result<BotSecret> {
    all(store)?.into_iter().find(|s| s.id == id).ok_or_else(|| anyhow!("No secret with that id"))
}

/// Changes a saved login. An omitted field keeps its old value; `Some(None)` clears 2FA.
pub fn update_login(
    store: &Store,
    id: &str,
    username: Option<&str>,
    site: Option<&str>,
    password: Option<&str>,
    seed: Option<Option<&str>>,
) -> Result<BotSecret> {
    let old = find_by_id(store, id)?;
    let Usage::Login { username: old_username, origin: old_origin, totp: old_seed } = &old.usage else {
        bail!("That secret isn't a login");
    };
    let username = username.unwrap_or(old_username);
    let site = site.unwrap_or(old_origin);
    let password = password.filter(|p| !p.is_empty()).unwrap_or(&old.value);
    let seed = match seed {
        Some(seed) => seed.filter(|s| !s.is_empty()),
        None => old_seed.as_deref(),
    };
    save_login(store, &old.bot_id, &old.name, username, site, password, seed)
}

pub fn remove(store: &Store, id: &str) -> Result<()> {
    let _one = EDIT.locked();
    let mut items = all(store)?;
    items.retain(|s| s.id != id);
    save_all(store, &items)
}

/// Forgets every secret of a deleted bot.
pub fn remove_bot(store: &Store, bot: &str) -> Result<()> {
    let _one = EDIT.locked();
    let mut items = all(store)?;
    let before = items.len();
    items.retain(|s| s.bot_id != bot);
    if items.len() == before { Ok(()) } else { save_all(store, &items) }
}

/// Every bot's secrets for Credentials, with the bot's name.
pub fn list(store: &Store) -> Result<Value> {
    listed(store, false)
}

/// Just the saved logins, for a client that shows them separately.
pub fn logins(store: &Store) -> Result<Value> {
    listed(store, true)
}

/// Only one bot's logins, for its own tool. It never sees another bot's entries.
pub fn logins_for_bot(store: &Store, bot: &str) -> Result<Value> {
    let items: Vec<Value> = for_bot(store, bot)?.iter().filter(|s| s.has_login()).map(BotSecret::public).collect();
    Ok(json!({"items": items}))
}

fn listed(store: &Store, only_logins: bool) -> Result<Value> {
    let bots = store.bots()?;
    let items: Vec<Value> = all(store)?
        .iter()
        .filter(|s| !only_logins || s.has_login())
        .map(|s| {
            let mut v = s.public();
            v["botName"] = bots.iter().find(|b| b.config.id == s.bot_id).map(|b| b.config.name.clone()).into();
            v
        })
        .collect();
    Ok(json!({"items": items}))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> Store {
        Store::open(std::path::Path::new(":memory:")).unwrap()
    }

    #[test]
    fn names_become_variables() {
        assert_eq!(env_name("netbird-setup-key"), "NETBIRD_SETUP_KEY");
        assert!(check_name("openai_key").is_ok());
        assert!(check_name("9lives").is_err());
        assert!(check_name("a b").is_err());
        assert!(check_name("").is_err());
        assert!(check_name("path").is_err());
        assert!(check_name("ld-preload").is_err());
    }

    #[test]
    fn origins_are_bare_https() {
        assert_eq!(origin("https://api.example.com/v1/").unwrap(), "https://api.example.com");
        assert_eq!(origin("https://api.example.com:8443").unwrap(), "https://api.example.com:8443");
        assert!(origin("http://api.example.com").is_err());
        assert!(origin("https://user:pw@api.example.com").is_err());
        assert!(origin("api.example.com").is_err());
    }

    #[test]
    fn reserved_headers_carry_no_secret() {
        assert!(check_auth(&Auth::Header { header: "X-Api-Key".into() }).is_ok());
        assert!(check_auth(&Auth::Header { header: "Host".into() }).is_err());
        assert!(check_auth(&Auth::Header { header: "bad header".into() }).is_err());
        assert!(check_auth(&Auth::Basic { username: "a:b".into() }).is_err());
    }

    #[test]
    fn saving_again_replaces_and_listing_hides_values() {
        let store = store();
        let http = Usage::Http { origin: "https://api.example.com".into(), auth: Auth::Bearer };
        let first = save(&store, "bot", "weather", http.clone(), "one").unwrap();
        let again = save(&store, "bot", "Weather", Usage::Command, "two").unwrap();
        assert_eq!(first.id, again.id);
        assert_eq!(find(&store, "bot", "weather").unwrap().value, "two");
        assert!(find(&store, "other", "weather").is_err());
        let listed = list(&store).unwrap().to_string();
        assert!(!listed.contains("\"two\""));
        assert!(listed.contains(&format!("{} in commands", crate::shell::var_ref("WEATHER"))));
        remove(&store, &first.id).unwrap();
        assert_eq!(for_bot(&store, "bot").unwrap().len(), 0);
    }

    #[test]
    fn values_have_limits() {
        let store = store();
        assert!(save(&store, "bot", "k", Usage::Command, "").is_err());
        assert!(save(&store, "bot", "k", Usage::Command, &"x".repeat(MAX_VALUE + 1)).is_err());
    }

    #[test]
    fn a_login_hides_its_password_and_two_factor_seed() {
        let store = store();
        let seed = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";
        let login =
            save_login(&store, "bot", "github", "ada", "https://github.com/login", "hunter2", Some(seed)).unwrap();
        assert!(login.has_totp());
        assert_eq!(login.totp(), Some(seed));
        let public = login.public().to_string();
        assert!(!public.contains("hunter2"));
        assert!(!public.contains(seed));
        assert_eq!(login.public()["hasTotp"], true);
        assert_eq!(login.public()["username"], "ada");
        assert_eq!(login.public()["origin"], "https://github.com");
        assert_eq!(login.public()["use"]["hasTotp"], true);
        let listed = list(&store).unwrap().to_string();
        let only_logins = logins(&store).unwrap().to_string();
        assert!(!listed.contains(seed));
        assert!(!only_logins.contains(seed));
        assert!(!only_logins.contains("hunter2"));
        assert_eq!(logins(&store).unwrap()["items"].as_array().unwrap().len(), 1);
        assert!(!totp::code_for(login.totp().unwrap(), 59).unwrap().0.is_empty());
    }

    #[test]
    fn a_bot_only_sees_its_own_logins() {
        let store = store();
        save_login(&store, "bot", "a", "ada", "https://a.example", "one", None).unwrap();
        save_login(&store, "other", "b", "bob", "https://b.example", "two", None).unwrap();
        let listed = logins_for_bot(&store, "bot").unwrap().to_string();
        assert!(listed.contains("a.example"));
        assert!(!listed.contains("b.example"));
        assert!(!listed.contains("one"));
    }

    #[test]
    fn malformed_logins_and_seeds_are_refused() {
        let store = store();
        assert!(save_login(&store, "bot", "a", "ada", "http://github.com", "hunter2", None).is_err());
        assert!(save_login(&store, "bot", "a", "", "https://github.com", "hunter2", None).is_err());
        assert!(save_login(&store, "bot", "a", "ada", "https://github.com", "hunter2", Some("not base32!")).is_err());
        assert!(save_login(&store, "bot", "a", "ada", "https://github.com", "hunter2", Some("GEZDGNBV")).is_err());
    }

    #[test]
    fn updating_a_login_can_change_or_clear_its_two_factor_seed() {
        let store = store();
        let seed = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";
        let first = save_login(&store, "bot", "github", "ada", "https://github.com", "one", Some(seed)).unwrap();
        let changed = update_login(&store, &first.id, Some("ada2"), None, Some("two"), Some(None)).unwrap();
        assert_eq!(changed.id, first.id);
        assert_eq!(changed.value, "two");
        assert_eq!(changed.public()["username"], "ada2");
        assert_eq!(changed.public()["hasTotp"], false);
        let rotated =
            update_login(&store, &first.id, None, Some("https://gitlab.com"), None, Some(Some(seed))).unwrap();
        assert_eq!(rotated.value, "two");
        assert_eq!(rotated.public()["origin"], "https://gitlab.com");
        assert_eq!(rotated.public()["hasTotp"], true);
        let kept = update_login(&store, &first.id, None, None, None, None).unwrap();
        assert_eq!(kept.value, "two");
        assert_eq!(kept.public()["hasTotp"], true);
        remove(&store, &first.id).unwrap();
        assert!(find_by_id(&store, &first.id).is_err());
    }
}
