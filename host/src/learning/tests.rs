//! Corrections and repeats become proposals, approval writes one fact to `preferences.md`, dismissal is remembered,
//! and nothing ordinary or repeated too often slips through.

use super::*;
use crate::chat::memory::files;
use crate::remote::identity::Identity;
use crate::store::Store;
use serde_json::json;
use std::sync::Arc;

struct Fixture {
    hub: Arc<Hub>,
    bot: String,
    config: BotConfig,
    dir: std::path::PathBuf,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn fixture() -> Fixture {
    let dir = std::env::temp_dir().join(format!("codync-learning-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).expect("dir");
    let store = Store::open(&dir.join("test.db")).expect("store");
    let hub = Hub::new(store, "learning-test".into(), Identity::load_or_create(&dir).expect("identity"), "t".into(), 0);
    let bot = uuid::Uuid::new_v4().to_string();
    let config: BotConfig =
        serde_json::from_value(json!({"id": bot, "name": "Bot", "backend": "fixture", "command": "x", "cwd": dir}))
            .expect("config");
    hub.store.save_bot(&config).expect("bot");
    Fixture { hub, bot, config, dir }
}

fn say(f: &Fixture, text: &str) {
    f.hub.store.insert_entry(&Lane::main(&f.bot), EntryKind::User, 0, &json!({"text": text})).expect("message");
}

fn learn(f: &Fixture, text: &str) {
    consider(f.hub.clone(), f.config.clone(), text.to_owned(), None);
}

#[test]
fn a_correction_waits_for_review_and_is_not_memory_yet() {
    let f = fixture();
    learn(&f, "No, use pnpm, not npm.");
    let waiting = pending(&f.hub.store, &f.bot);
    assert_eq!(waiting.len(), 1, "{waiting:?}");
    assert_eq!(waiting[0].text, "Use pnpm, not npm");
    assert_eq!(waiting[0].kind, detect::Kind::Correction);
    assert!(files::read(Some(&f.bot), apply::NOTE).expect("read").is_none(), "nothing is saved before approval");
    assert!(!apply::known(&f.hub, &f.config, "Use pnpm, not npm").expect("known"));
}

#[test]
fn a_plain_request_is_not_a_preference() {
    let f = fixture();
    learn(&f, "Fix the sidebar spacing.");
    learn(&f, "What is the weather today?");
    assert!(pending(&f.hub.store, &f.bot).is_empty());
}

#[test]
fn the_same_correction_said_again_is_one_proposal_with_two_hits() {
    let f = fixture();
    learn(&f, "No, use pnpm, not npm.");
    say(&f, "No, use pnpm, not npm.");
    learn(&f, "No, use pnpm, not npm.");
    let waiting = pending(&f.hub.store, &f.bot);
    assert_eq!(waiting.len(), 1, "deduplicated: {waiting:?}");
    assert_eq!(waiting[0].hits, 2);
}

#[test]
fn approving_writes_one_feedback_fact_and_clears_the_inbox() {
    let f = fixture();
    learn(&f, "No, use pnpm, not npm.");
    let id = pending(&f.hub.store, &f.bot)[0].id.clone();
    let answer = call(&f.hub, "approveLearning", &json!({"botId": f.bot, "id": id})).expect("approved");
    assert_eq!(answer["proposals"].as_array().expect("list").len(), 0);
    let note = files::read(Some(&f.bot), apply::NOTE).expect("read").expect("note");
    assert!(note.contains("type: feedback"), "{note}");
    assert!(note.contains("confidence: high"), "{note}");
    assert!(note.contains("- Use pnpm, not npm"), "{note}");
    // Said again, it is known memory: no second proposal.
    learn(&f, "No, use pnpm, not npm.");
    assert!(pending(&f.hub.store, &f.bot).is_empty());
}

#[test]
fn dismissing_remembers_the_fact_so_it_is_not_proposed_again() {
    let f = fixture();
    learn(&f, "I always want the tests run before a commit.");
    let id = pending(&f.hub.store, &f.bot)[0].id.clone();
    call(&f.hub, "dismissLearning", &json!({"botId": f.bot, "id": id})).expect("dismissed");
    assert!(files::read(Some(&f.bot), apply::NOTE).expect("read").is_none());
    learn(&f, "I always want the tests run before a commit.");
    assert!(pending(&f.hub.store, &f.bot).is_empty(), "dismissed facts stay out");
}

#[test]
fn after_the_configured_repeats_a_fact_is_saved_without_review() {
    let f = fixture();
    f.hub.update_bot(&json!({"id": f.bot, "learning": {"enabled": true, "autoApproveAfter": 2}})).expect("settings");
    let config = f.hub.store.bot(&f.bot).expect("bot").expect("row").config;
    assert_eq!(config.learning.auto_approve_after, 2);
    say(&f, "Please run the tests before committing anything.");
    consider(f.hub.clone(), config.clone(), "Run the tests before committing anything.".into(), None);
    assert!(pending(&f.hub.store, &f.bot).is_empty(), "saved at the second saying");
    let note = files::read(Some(&f.bot), apply::NOTE).expect("read").expect("note");
    assert!(note.contains("Run the tests before committing anything"), "{note}");
}

#[test]
fn learning_can_be_turned_off_for_one_bot() {
    let f = fixture();
    f.hub.update_bot(&json!({"id": f.bot, "learning": {"enabled": false, "autoApproveAfter": 0}})).expect("settings");
    let config = f.hub.store.bot(&f.bot).expect("bot").expect("row").config;
    consider(f.hub.clone(), config, "No, use pnpm, not npm.".into(), None);
    assert!(pending(&f.hub.store, &f.bot).is_empty());
}

#[test]
fn settings_are_saved_for_the_bot_and_clamped() {
    let f = fixture();
    call(&f.hub, "saveLearningSettings", &json!({"botId": f.bot, "enabled": true, "autoApproveAfter": 99}))
        .expect("saved");
    let saved = f.hub.store.bot(&f.bot).expect("bot").expect("row").config.learning;
    assert!(saved.enabled);
    assert_eq!(saved.auto_approve_after, MAX_AUTO);
}
