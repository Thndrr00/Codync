//! Who may write through a linked `WhatsApp` number, and the code an unknown person gets on a number the
//! owner restricted: one short code each, kept until the owner answers it in the app.

use crate::store::Store;
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// How many people may wait for the owner on one number, so a flood of strangers cannot fill the list.
const MAX_PENDING: usize = 3;
/// How long a code waits before the person needs a new one.
const TTL_MS: i64 = 24 * 60 * 60 * 1000;
/// The code has no 0/O or 1/I, so one read aloud is not mistaken.
const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
/// The codes and pending requests of every linked number.
const KEY: &str = "whatsapp.requests";

/// Someone waiting for the owner to let them write.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pending {
    pub account: String,
    pub sender: String,
    pub name: String,
    pub code: String,
    /// When the request arrived, in Unix milliseconds.
    pub at: i64,
}

/// What one inbound message may do, before anything shows that it was seen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Gate {
    Allow,
    /// A restricted number answers an unknown direct sender once, with a code for the owner.
    Pair,
    Ignore,
}

/// The owner's own chat always counts, an allowed chat or person counts, and on a restricted number an
/// unknown direct sender is asked to wait for a code. A group that was not allowed is ignored.
pub(super) fn gate(allowed: &[String], me: &str, chat: &str, sender: &str, group: bool) -> Gate {
    let own = !me.is_empty() && (chat == me || sender == me);
    if own || allowed.is_empty() {
        return Gate::Allow;
    }
    let listed = allowed.iter().any(|entry| entry == if group { chat } else { sender });
    if listed {
        Gate::Allow
    } else if group {
        Gate::Ignore
    } else {
        Gate::Pair
    }
}

/// The requests waiting for an answer on one number.
pub(super) fn pending(store: &Store, account: &str) -> Result<Vec<Pending>> {
    Ok(list(store)?.into_iter().filter(|request| request.account == account).collect())
}

/// The code for this person, if they already have one, or a new one. `None` when three others wait, so
/// one more stranger gets no answer until the owner clears the list.
pub(super) fn greet(store: &Store, account: &str, sender: &str, name: &str) -> Result<Option<Pending>> {
    if account.is_empty() || sender.is_empty() {
        return Ok(None);
    }
    let mut requests = list(store)?;
    if let Some(waiting) = requests.iter().find(|request| request.account == account && request.sender == sender) {
        return Ok(Some(waiting.clone()));
    }
    if requests.iter().filter(|request| request.account == account).count() >= MAX_PENDING {
        return Ok(None);
    }
    let request = Pending {
        account: account.to_owned(),
        sender: sender.to_owned(),
        name: name.trim().chars().take(120).collect(),
        code: new_code(&requests),
        at: crate::store::now_ms(),
    };
    requests.push(request.clone());
    write(store, &requests)?;
    Ok(Some(request))
}

/// Tells an unknown person on a restricted number the code the owner answers in the app. A repeat
/// message sends the same code again; a person whose request ended starts over with a new one.
pub(super) async fn greet_person(
    app: &super::WhatsApp,
    hub: &super::Hub,
    me: &str,
    chat: &str,
    sender: &str,
    name: &str,
) {
    let request = {
        let _configuration = crate::LockExt::locked(&hub.store.connector_lock);
        greet(&hub.store, me, sender, name)
    };
    match request {
        Ok(Some(request)) => {
            let line = format!(
                "Para escribir a este asistente, el dueño de este número debe aprobar este código: {}",
                request.code
            );
            if let Err(error) = app.send_message(chat, &line, None, None, None).await {
                tracing::warn!(%error, "WhatsApp could not send a code to a new person");
            }
        }
        Ok(None) => {}
        Err(error) => tracing::warn!(%error, "WhatsApp could not keep a request to write"),
    }
}

/// Removes the request of a code and returns it, or `None` when it ended, as a second answer finds it.
pub(super) fn take(store: &Store, code: &str) -> Result<Option<Pending>> {
    let wanted = code.trim().to_uppercase().replace(['-', ' '], "");
    if wanted.is_empty() {
        return Ok(None);
    }
    let mut requests = list(store)?;
    let found = requests.iter().position(|request| request.code == wanted);
    let found = found.map(|at| requests.remove(at));
    if found.is_some() {
        write(store, &requests)?;
    }
    Ok(found)
}

/// The waiting requests, without the ones that ran out: an old code needs a new message to work.
fn list(store: &Store) -> Result<Vec<Pending>> {
    let raw: Vec<Pending> = store.kv_read(KEY)?.as_deref().map(serde_json::from_str).transpose().unwrap_or_default();
    let now = crate::store::now_ms();
    let live: Vec<Pending> = raw.into_iter().filter(|request| request.at + TTL_MS > now).collect();
    Ok(live)
}

fn write(store: &Store, requests: &[Pending]) -> Result<()> {
    store.kv_set(KEY, &serde_json::to_string(requests)?)
}

/// Six characters from the alphabet, never one another request already shows.
fn new_code(taken: &[Pending]) -> String {
    for _ in 0..64 {
        let mut bytes = [0_u8; 6];
        if getrandom::fill(&mut bytes).is_err() {
            break;
        }
        let code: String = bytes.iter().map(|byte| ALPHABET[usize::from(*byte) % ALPHABET.len()] as char).collect();
        if !taken.iter().any(|request| request.code == code) {
            return code;
        }
    }
    // Without entropy a code from the clock still works; it is replaced as soon as one is generated.
    let mut now = crate::store::now_ms().unsigned_abs();
    (0..6)
        .map(|_| {
            let character = ALPHABET[(now % ALPHABET.len() as u64) as usize] as char;
            now /= ALPHABET.len() as u64;
            character
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hub::Hub;
    use crate::remote::identity::Identity;
    use crate::whatsapp::settings;
    use std::path::PathBuf;
    use std::sync::Arc;

    fn store() -> (Arc<Hub>, PathBuf) {
        let dir = std::env::temp_dir().join(format!("codync-whatsapp-senders-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::open(&dir.join("t.db")).unwrap();
        let hub = Hub::new(store, "test".into(), Identity::load_or_create(&dir).unwrap(), "token".into(), 0);
        (hub, dir)
    }

    #[test]
    fn an_unknown_person_gets_one_code_and_a_repeat_returns_it() {
        let (hub, dir) = store();
        let (account, alice) = ("5215555555555@s.whatsapp.net", "5215555555556@s.whatsapp.net");
        assert_eq!(gate(&[], account, alice, alice, false), Gate::Allow, "anyone writes while the list is empty");
        let allowed = vec![account.to_owned()];
        assert_eq!(gate(&allowed, account, alice, alice, false), Gate::Pair);

        let first = greet(&hub.store, account, alice, "Alice").unwrap().expect("a code");
        assert_eq!(first.code.len(), 6);
        assert!(first.code.chars().all(|character| !matches!(character, '0' | 'O' | '1' | 'I')), "{}", first.code);
        let again = greet(&hub.store, account, alice, "Alice").unwrap().expect("the same code");
        assert_eq!(again.code, first.code);
        assert_eq!(pending(&hub.store, account).unwrap().len(), 1);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn approving_lets_the_person_write_and_a_fourth_stranger_gets_nothing() {
        let (hub, dir) = store();
        let (account, alice) = ("5215555555555@s.whatsapp.net", "5215555555556@s.whatsapp.net");
        let code = greet(&hub.store, account, alice, "Alice").unwrap().expect("a code").code;
        assert_eq!(settings::answer_request(&hub, &code, true).unwrap().as_deref(), Some(alice));
        assert!(crate::whatsapp::allowed(&hub).iter().any(|entry| entry == alice), "now allowed");
        assert_eq!(gate(&crate::whatsapp::allowed(&hub), account, alice, alice, false), Gate::Allow);
        assert!(settings::answer_request(&hub, &code, true).is_err(), "answered once");

        for index in 0..3 {
            let sender = format!("52155555555{index}@s.whatsapp.net");
            assert!(greet(&hub.store, account, &sender, "").unwrap().is_some(), "three may wait");
        }
        // Alice is allowed now, so the three are strangers four, five and six.
        let fourth = "5215555555599@s.whatsapp.net";
        assert_eq!(gate(&crate::whatsapp::allowed(&hub), account, fourth, fourth, false), Gate::Pair);
        assert!(greet(&hub.store, account, fourth, "").unwrap().is_none(), "the list is full");
        assert_eq!(pending(&hub.store, account).unwrap().len(), MAX_PENDING);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn a_denied_request_is_gone_without_allowing_anyone() {
        let (hub, dir) = store();
        let (account, bob) = ("5215555555555@s.whatsapp.net", "5215555555557@s.whatsapp.net");
        let code = greet(&hub.store, account, bob, "Bob").unwrap().expect("a code").code;
        assert!(settings::answer_request(&hub, &code, false).unwrap().is_none(), "denied, nobody is told");
        assert!(!crate::whatsapp::allowed(&hub).iter().any(|entry| entry == bob));
        assert!(pending(&hub.store, account).unwrap().is_empty());
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn pending_requests_survive_a_restart() {
        let (hub, dir) = store();
        let (account, alice) = ("5215555555555@s.whatsapp.net", "5215555555556@s.whatsapp.net");
        let code = greet(&hub.store, account, alice, "Alice").unwrap().expect("a code").code;
        let path = dir.join("t.db");
        drop(hub);

        let reopened = Store::open(&path).unwrap();
        let waiting = pending(&reopened, account).unwrap();
        assert_eq!(waiting.len(), 1);
        assert_eq!(waiting[0].code, code, "the code a person already has still works");
        drop(reopened);
        std::fs::remove_dir_all(dir).ok();
    }
}
