//! Secrets a bot keeps for its own work: an API key pinned to one HTTPS origin, which the host
//! attaches to `api_request` calls, or a command variable the host exports into
//! `run_with_secrets` commands. The bot asks for one with a secure card (`secret_tools`) and
//! never gets the value back. One encrypted vault record holds them all.
//!
//! Idea from rakazo's `docs/bot-secrets.md` (Apache-2.0); no code copied.

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
        }
    }
}

impl BotSecret {
    /// Everything but the value.
    pub fn public(&self) -> Value {
        json!({"id": self.id, "botId": self.bot_id, "name": self.name, "use": self.usage,
            "destination": self.usage.destination(&self.name)})
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
    let bots = store.bots()?;
    let items: Vec<Value> = all(store)?
        .iter()
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
}
