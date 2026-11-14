//! Who wrote a message, and who is in a chat right now: the people are the devices paired with
//! this computer, each by its own name and colour, so two people on one computer are told apart.
//! This computer's own apps have no author: they are the owner.
//!
//! The presence list is what `OpenBot`'s `TeamPresenceSnapshot` carries (a member, `online`, a typing
//! flag, a time), cut down to this computer: `people` has the devices seen in the chat in the last
//! half minute, one of them typing for four seconds after its last key.

use crate::LockExt;
use crate::api::devices::Caller;
use crate::hub::Hub;
use crate::store::AVATAR_COLORS;
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// How long a device counts as being in the chat after it last looked.
const PRESENT_FOR: Duration = Duration::from_secs(30);
/// How long after its last key a device counts as typing.
const TYPING_FOR: Duration = Duration::from_secs(4);

/// One device in one chat.
struct Here {
    name: String,
    color: &'static str,
    seen: Instant,
    typing: Option<Instant>,
}

/// The devices seen in each chat, in memory only: presence is about now, not history.
#[derive(Default)]
pub struct People {
    seen: Mutex<HashMap<String, HashMap<String, Here>>>,
    /// The chats whose presence already went out as one analytics event.
    reported: Mutex<HashSet<String>>,
}

impl People {
    pub fn new() -> Self {
        Self::default()
    }
}

/// The author of a message: the sending device, named and coloured. `None` from this computer.
pub fn author(hub: &Hub, caller: &Caller) -> Option<Value> {
    let (name, color) = her(caller, hub)?;
    Some(json!({"key": caller.device_key(), "name": name, "color": color}))
}

/// A colour that stays with a device: the bot avatar palette, picked by its key.
fn color(key: &str) -> &'static str {
    let sum = key.bytes().fold(0usize, |sum, b| sum.wrapping_mul(31).wrapping_add(b as usize));
    AVATAR_COLORS[sum % AVATAR_COLORS.len()]
}

/// The name and colour of the caller, when it is a device this computer knows.
fn her(caller: &Caller, hub: &Hub) -> Option<(String, &'static str)> {
    let key = caller.device_key();
    if matches!(caller, Caller::Local) {
        return None;
    }
    let device = hub.store.device(key)?;
    Some((device.name, color(key)))
}

/// Notes that `caller` is in `bot`'s chat, typing or not, and returns the others there now.
pub fn look(hub: &Hub, caller: &Caller, bot: &str, typing: Option<bool>) -> Value {
    let key = caller.device_key();
    let now = Instant::now();
    let others = {
        let mut chats = hub.people.seen.locked();
        let here = chats.entry(bot.to_owned()).or_default();
        here.retain(|_, h| now.duration_since(h.seen) < PRESENT_FOR);
        // This computer itself is here, but not as one of the people.
        if let Some((name, color)) = her(caller, hub) {
            let h = here.entry(key.to_owned()).or_insert_with(|| Here { name, color, seen: now, typing: None });
            h.seen = now;
            match typing {
                Some(true) => h.typing = Some(now),
                Some(false) => h.typing = None,
                None => {}
            }
        }
        here.iter()
            .filter(|(id, _)| id.as_str() != key)
            .map(|(id, h)| {
                let typing = h.typing.is_some_and(|at| now.duration_since(at) < TYPING_FOR);
                json!({"key": id, "name": h.name, "color": h.color, "typing": typing})
            })
            .collect::<Vec<Value>>()
    };
    report(hub, bot, &others);
    json!({"people": others, "at": crate::store::now_ms()})
}

/// One `presence_shown` per chat, once somebody else is in it.
fn report(hub: &Hub, bot: &str, others: &[Value]) {
    if others.is_empty() || !hub.people.reported.locked().insert(bot.to_owned()) {
        return;
    }
    let typing = others.iter().any(|p| p["typing"] == true);
    let mut properties = serde_json::Map::new();
    properties.insert("others".into(), others.len().into());
    properties.insert("typing".into(), typing.into());
    crate::analytics::capture(hub, crate::analytics::Event::PresenceShown, properties);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::identity::Identity;
    use crate::store::{Device, DeviceAccount, DeviceSource, Scope, Store};
    use std::sync::Arc;

    fn hub() -> (Arc<Hub>, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("codync-people-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::open(&dir.join("t.db")).unwrap();
        let identity = Identity::load_or_create(&dir).unwrap();
        for (key, name) in [("a", "Ana"), ("b", "Bo")] {
            store
                .put_device(&Device {
                    key: key.into(),
                    name: name.into(),
                    platform: "ios".into(),
                    source: DeviceSource::Local,
                    grant_id: None,
                    scopes: vec![Scope::Control],
                    lease_until: None,
                    created_at: 0,
                    last_seen_at: None,
                    account: DeviceAccount::Unrecorded,
                })
                .unwrap();
        }
        (Hub::new(store, "test".into(), identity, "token".into(), 0), dir)
    }

    fn device(key: &str) -> Caller {
        Caller::Device { key: key.into(), scopes: vec![Scope::Control] }
    }

    #[test]
    fn a_colour_stays_with_its_device() {
        assert_eq!(color("phone-a"), color("phone-a"));
        assert!(AVATAR_COLORS.contains(&color("phone-b")));
    }

    #[test]
    fn the_owner_has_no_author_and_a_device_does() {
        let (hub, dir) = hub();
        assert_eq!(author(&hub, &Caller::Local), None);
        let some = author(&hub, &device("a")).unwrap();
        assert_eq!(some["key"], "a");
        assert_eq!(some["name"], "Ana");
        assert!(some["color"].is_string());
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn looking_shows_the_other_devices_and_their_typing() {
        let (hub, dir) = hub();
        look(&hub, &device("a"), "chat", Some(true));
        let seen = look(&hub, &device("b"), "chat", None);
        let people = seen["people"].as_array().unwrap();
        assert_eq!(people.len(), 1);
        assert_eq!(people[0]["name"], "Ana");
        assert_eq!(people[0]["typing"], true);
        look(&hub, &device("a"), "chat", Some(false));
        assert_eq!(look(&hub, &device("b"), "chat", None)["people"][0]["typing"], false);
        // Another chat does not carry them.
        assert_eq!(look(&hub, &device("b"), "other", None)["people"].as_array().unwrap().len(), 0);
        std::fs::remove_dir_all(dir).ok();
    }
}
