use super::*;
use crate::store::Store;
use std::path::PathBuf;

struct Fixture {
    hub: Arc<Hub>,
    dir: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("codync-pins-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::open(&dir.join("test.db")).unwrap();
        let cfg: BotConfig = serde_json::from_value(json!({
            "id": "b", "name": "Bob", "backend": "fixture", "cwd": dir, "permit": "ask",
        }))
        .unwrap();
        store.save_bot(&cfg).unwrap();
        let hub = Hub::new(
            store,
            "test-host".into(),
            crate::remote::identity::Identity::load_or_create(&dir).unwrap(),
            "test-token".into(),
            19223,
        );
        Self { hub, dir }
    }

    /// A message of the user's in the chat.
    fn message(&self, text: &str) -> Entry {
        self.hub.add_entry(&Lane::main("b"), crate::store::EntryKind::User, 1, &json!({"text": text})).unwrap()
    }

    fn person() -> Value {
        json!({"kind": "person", "device": "Maya's iPhone"})
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn a_pinned_message_lists_with_who_pinned_it() {
    let f = Fixture::new();
    let message = f.message("the deploy key is in the vault");
    let pin = set(&f.hub, "b", &message.id, true, &Fixture::person()).unwrap()["pin"].clone();
    assert_eq!(pin["pinnedBy"], Fixture::person());
    assert_eq!(pin["text"], "the deploy key is in the vault");
    assert_eq!(pin["pinnedBy"]["device"], "Maya's iPhone");
    let listed = list(&f.hub, "b").unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0]["messageId"], message.id);
    assert_eq!(listed[0]["author"], "user");
    assert!(listed[0]["threadId"].is_null());
}

#[test]
fn unpinning_takes_the_marker_out_of_the_message() {
    let f = Fixture::new();
    let message = f.message("temporary");
    set(&f.hub, "b", &message.id, true, &Fixture::person()).unwrap();
    set(&f.hub, "b", &message.id, false, &Fixture::person()).unwrap();
    assert_eq!(list(&f.hub, "b").unwrap().len(), 0);
    assert!(f.hub.store.entry(&message.id).unwrap().data["pinned"].is_null());
}

#[test]
fn only_messages_can_be_pinned() {
    let f = Fixture::new();
    let card = f.hub.add_entry(&Lane::main("b"), crate::store::EntryKind::Tool, 1, &json!({"tool": "shell"})).unwrap();
    assert!(set(&f.hub, "b", &card.id, true, &Fixture::person()).is_err());
}

#[test]
fn a_chat_keeps_fifty_pins() {
    let f = Fixture::new();
    for i in 0..PINS_LIMIT {
        let message = f.message(&format!("message {i}"));
        set(&f.hub, "b", &message.id, true, &Fixture::person()).unwrap();
    }
    let over = f.message("one too many");
    assert!(set(&f.hub, "b", &over.id, true, &Fixture::person()).is_err());
    // Pinning one already pinned doesn't count again.
    let first = list(&f.hub, "b").unwrap()[0]["messageId"].as_str().unwrap().to_owned();
    set(&f.hub, "b", &first, true, &Fixture::person()).unwrap();
}

#[test]
fn the_agent_pins_by_quote_and_cannot_undo_a_persons_pin() {
    let f = Fixture::new();
    let message = f.message("the invoice total rounds wrong");
    f.hub.set_runtime("b", |r| {
        r.lane = Some(Lane::main("b"));
        r.message = Some(message.id.clone());
    });
    let source = f.hub.store.bot("b").unwrap().unwrap().config;

    // No quote: the message of the running turn, pinned by the agent.
    let result = pin(&f.hub, &source, &json!({})).unwrap();
    assert_eq!(result["status"], "pinned");
    assert_eq!(result["pinsInThisChat"][0]["pinnedBy"], "you");
    assert_eq!(list(&f.hub, "b").unwrap()[0]["pinnedBy"]["kind"], "agent");

    // A quote finds an earlier message.
    let earlier = f.message("see src/pace.js");
    let result = pin(&f.hub, &source, &json!({"quote": "src/pace.js"})).unwrap();
    assert_eq!(result["messageId"], earlier.id);

    // The agent takes back its own pin.
    pin(&f.hub, &source, &json!({"quote": "src/pace.js", "pinned": false})).unwrap();
    assert_eq!(list(&f.hub, "b").unwrap().len(), 1);

    // A person's pin stays, unless they take it back.
    set(&f.hub, "b", &earlier.id, true, &Fixture::person()).unwrap();
    assert!(pin(&f.hub, &source, &json!({"quote": "src/pace.js", "pinned": false})).is_err());
}

#[test]
fn a_quote_of_no_message_is_refused() {
    let f = Fixture::new();
    let message = f.message("nothing about it");
    f.hub.set_runtime("b", |r| {
        r.lane = Some(Lane::main("b"));
        r.message = Some(message.id.clone());
    });
    let source = f.hub.store.bot("b").unwrap().unwrap().config;
    assert!(pin(&f.hub, &source, &json!({"quote": "elsewhere"})).is_err());
    // The running turn's own message is not found by its quote, as no quote already means it.
    assert!(pin(&f.hub, &source, &json!({"quote": &message.data["text"]})).is_err());
}

#[test]
fn a_thread_pin_carries_its_title() {
    let f = Fixture::new();
    let root = f.message("dig into this");
    let thread = f.hub.store.create_thread("b", "Rounding bug", Some(&root.id)).unwrap();
    let reply = f
        .hub
        .add_entry(
            &Lane::in_thread("b", &thread),
            crate::store::EntryKind::Agent,
            2,
            &json!({
                "text": "it should round half up", "final": true, "author": "b",
            }),
        )
        .unwrap();
    let pin = set(&f.hub, "b", &reply.id, true, &Fixture::person()).unwrap()["pin"].clone();
    assert_eq!(pin["threadId"], thread);
    assert_eq!(pin["threadTitle"], "Rounding bug");
    assert_eq!(pin["author"], "agent");
}

#[test]
fn a_pin_limit_counts_each_chat_on_its_own() {
    let f = Fixture::new();
    for i in 0..PINS_LIMIT {
        let message = f.message(&format!("message {i}"));
        set(&f.hub, "b", &message.id, true, &Fixture::person()).unwrap();
    }
    let root = f.message("thread root");
    let thread = f.hub.store.create_thread("b", "Elsewhere", Some(&root.id)).unwrap();
    let in_thread = f
        .hub
        .add_entry(
            &Lane::in_thread("b", &thread),
            crate::store::EntryKind::Agent,
            2,
            &json!({"text": "x", "final": true}),
        )
        .unwrap();
    set(&f.hub, "b", &in_thread.id, true, &Fixture::person()).unwrap();
}
