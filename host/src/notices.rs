//! The interruption policy for everything a bot raises on its own (proactive check-ins, routine
//! and memory notices): findings raised together reach the person as one notice, a topic already
//! waiting for an answer is not raised again, and it may return after a reply or its cooldown.

use crate::hub::Hub;
use crate::store::{EntryKind, Lane, ProactiveSettings, Store, now_ms};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;

/// The most findings one notice holds; the rest wait for the next notice.
pub const MAX_FINDINGS: usize = 6;
const MAX_TOPIC: usize = 120;
const MAX_TEXT: usize = 400;
const STYLE: &str = "info";

/// One thing worth telling the person about.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    /// Its identity for the policy: the same topic is not raised twice while it waits.
    pub topic: String,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    /// Entry fields a notice of this finding carries (a memory notice's undo data).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra: Option<Value>,
}

impl Finding {
    pub fn new(topic: impl Into<String>, text: impl Into<String>) -> Self {
        Self { topic: topic.into(), text: text.into(), style: None, extra: None }
    }

    pub fn error(topic: impl Into<String>, text: impl Into<String>) -> Self {
        Self { style: Some("error".into()), ..Self::new(topic, text) }
    }

    pub fn with_extra(mut self, extra: Value) -> Self {
        self.extra = Some(extra);
        self
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct Topic {
    at: i64,
    /// The notice's seq; a user message after it answers the topic.
    seq: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Open {
    entry: String,
    seq: i64,
    last_at: i64,
    topics: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct State {
    #[serde(default)]
    topics: BTreeMap<String, Topic>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    open: Option<Open>,
}

fn key(bot: &str) -> String {
    format!("notices.{bot}")
}

fn load(store: &Store, bot: &str) -> State {
    store.kv_read(&key(bot)).ok().flatten().and_then(|raw| serde_json::from_str(&raw).ok()).unwrap_or_default()
}

fn save(store: &Store, bot: &str, state: &State) -> Result<()> {
    store.kv_set(&key(bot), &serde_json::to_string(state)?)
}

fn settings(hub: &Hub, bot: &str) -> ProactiveSettings {
    hub.store.bot(bot).ok().flatten().map_or_else(ProactiveSettings::default, |row| row.config.proactive)
}

fn cooldown(hub: &Hub, bot: &str) -> i64 {
    i64::from(settings(hub, bot).notice_cooldown_hours.max(1)) * 3_600_000
}

fn batch(hub: &Hub, bot: &str) -> i64 {
    i64::from(settings(hub, bot).notice_batch_minutes.max(1)) * 60_000
}

/// A topic is free again after the person replies to its notice, or once its cooldown passes.
fn released(hub: &Hub, bot: &str, topic: &Topic, now: i64) -> Result<bool> {
    if now - topic.at >= cooldown(hub, bot) {
        return Ok(true);
    }
    if topic.seq > 0 {
        hub.store.user_replied_after(bot, topic.seq)
    } else {
        hub.store.user_replied_since(bot, topic.at)
    }
}

fn clean_topic(raw: &str) -> Result<String> {
    let topic = raw.trim().to_lowercase();
    ensure!(
        !topic.is_empty() && topic.chars().count() <= MAX_TOPIC,
        "a finding needs a topic of 1–{MAX_TOPIC} characters"
    );
    Ok(topic)
}

fn clean_text(raw: &str) -> Result<String> {
    let text = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    ensure!(!text.is_empty(), "a finding needs text");
    Ok(text.chars().take(MAX_TEXT).collect())
}

fn clean(finding: &Finding) -> Result<Finding> {
    Ok(Finding {
        topic: clean_topic(&finding.topic)?,
        text: clean_text(&finding.text)?,
        style: finding.style.as_deref().map(|style| if style == "error" { "error" } else { STYLE }.to_owned()),
        extra: finding.extra.clone(),
    })
}

fn text_of(findings: &[Finding]) -> String {
    match findings {
        [one] => one.text.clone(),
        many => {
            let lines: Vec<String> = many.iter().map(|finding| format!("• {}", finding.text)).collect();
            format!("{} findings:\n{}", many.len(), lines.join("\n"))
        }
    }
}

/// Raises findings, grouped into the open notice when one is still within its batch window.
/// Nothing is raised for a topic that is already waiting for an answer.
pub fn raise(hub: &Hub, bot: &str, findings: Vec<Finding>) -> Option<String> {
    raise_at(hub, bot, findings, now_ms(), true)
}

/// A single notice that is never merged with another finding (a memory change keeps its own undo).
pub fn tell(hub: &Hub, bot: &str, finding: Finding) -> Option<String> {
    raise_at(hub, bot, vec![finding], now_ms(), false)
}

/// Whether a topic may be raised now; the proactive scheduler uses it for its own check-in topic.
pub fn may_raise(hub: &Hub, bot: &str, topic: &str) -> bool {
    let now = now_ms();
    let Ok(topic) = clean_topic(topic) else { return false };
    let mut state = load(&hub.store, bot);
    let Some(raised) = state.topics.get(&topic).cloned() else { return true };
    match released(hub, bot, &raised, now) {
        Ok(true) => {
            state.topics.remove(&topic);
            let _ = save(&hub.store, bot, &state);
            true
        }
        _ => false,
    }
}

/// Whether the last check-in said something the person has not answered yet: the next check-in
/// waits for a reply, or for the topic's cooldown, instead of repeating the same points.
pub fn check_in_waiting(hub: &Hub, bot: &str, check_in_at: i64) -> bool {
    if check_in_at <= 0 {
        return false;
    }
    let Some(spoken) = hub.store.last_spoken_at(bot) else { return false };
    if spoken <= check_in_at || now_ms() - spoken >= cooldown(hub, bot) {
        return false;
    }
    !hub.store.user_replied_since(bot, spoken).unwrap_or(false)
}

/// Records that a topic was just raised outside a notice entry (a proactive check-in).
pub fn mark_raised(hub: &Hub, bot: &str, topic: &str) {
    let Ok(topic) = clean_topic(topic) else { return };
    let now = now_ms();
    let mut state = load(&hub.store, bot);
    state.topics.insert(topic, Topic { at: now, seq: 0 });
    if let Err(error) = save(&hub.store, bot, &state) {
        tracing::warn!(bot, error = format!("{error:#}"), "couldn't save the notice policy");
    }
}

fn raise_at(hub: &Hub, bot: &str, findings: Vec<Finding>, now: i64, group: bool) -> Option<String> {
    let findings: Vec<Finding> = findings.into_iter().filter_map(|finding| clean(&finding).ok()).collect();
    if findings.is_empty() {
        return None;
    }
    let mut state = load(&hub.store, bot);
    let topics: Vec<String> = state.topics.keys().cloned().collect();
    for topic in topics {
        let Some(raised) = state.topics.get(&topic).cloned() else { continue };
        let replied = raised.seq > 0 && hub.store.user_replied_after(bot, raised.seq).unwrap_or(false);
        match released(hub, bot, &raised, now) {
            Ok(true) => {
                state.topics.remove(&topic);
                let in_open = state.open.as_ref().is_some_and(|open| open.topics.contains(&topic));
                if let Some(open) = state.open.as_mut() {
                    open.topics.retain(|waiting| waiting != &topic);
                }
                // An answer ends the notice it answered: a topic raised later starts one of its own.
                if replied && in_open {
                    state.open = None;
                }
            }
            Ok(false) => {}
            Err(error) => tracing::warn!(bot, error = format!("{error:#}"), "couldn't check for a reply"),
        }
    }
    if state.open.as_ref().is_some_and(|open| now - open.last_at > batch(hub, bot)) {
        state.open = None;
    }
    let open_topics = state.open.as_ref().map(|open| open.topics.clone()).unwrap_or_default();
    let mut accepted: Vec<Finding> = Vec::new();
    for finding in findings {
        let waiting = state.topics.contains_key(&finding.topic) || open_topics.contains(&finding.topic);
        if !waiting && !accepted.iter().any(|kept| kept.topic == finding.topic) {
            accepted.push(finding);
        }
    }
    if accepted.is_empty() {
        // Everything is still waiting: nothing new reaches the person.
        return None;
    }
    accepted.truncate(MAX_FINDINGS);
    let style = if accepted.iter().any(|finding| finding.style.as_deref() == Some("error")) { "error" } else { STYLE };
    let text = text_of(&accepted);
    let notices: Vec<Value> = accepted
        .iter()
        .map(|finding| json!({"topic": finding.topic, "text": finding.text, "style": finding.style}))
        .collect();
    let reusable = group
        && state.open.as_ref().is_some_and(|open| {
            !open.topics.is_empty()
                && now - open.last_at <= batch(hub, bot)
                && open.topics.len() + accepted.len() <= MAX_FINDINGS
        });
    if reusable {
        let open = state.open.clone().expect("the open notice was checked");
        let appended = hub.edit_entry(&open.entry, |entry| {
            entry.data["text"] = text.clone().into();
            entry.data["style"] = style.into();
            entry.data["notices"] = entry.data["notices"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .chain(notices.clone())
                .collect::<Vec<Value>>()
                .into();
            Ok(true)
        });
        if appended.is_ok() {
            finish(&hub.store, bot, &mut state, &open.entry, open.seq, &accepted, now, true);
            return Some(open.entry);
        }
    }
    let mut data = json!({
        "text": text,
        "style": style,
        "notices": notices,
        "noticePolicy": {"grouped": accepted.len() > 1},
    });
    if let [one] = accepted.as_slice()
        && let Some(Value::Object(extra)) = &one.extra
    {
        if let Value::Object(fields) = &mut data {
            for (key, value) in extra {
                fields.insert(key.clone(), value.clone());
            }
        }
    }
    let turn = hub.store.max_turn(bot);
    let entry = hub.add_entry(&Lane::main(bot), EntryKind::Notice, turn, &data)?;
    let entry_id = entry.id.clone();
    finish(&hub.store, bot, &mut state, &entry_id, entry.seq, &accepted, now, false);
    Some(entry_id)
}

#[allow(clippy::too_many_arguments)] // the policy's state, the entry, and what it now holds
fn finish(
    store: &Store,
    bot: &str,
    state: &mut State,
    entry: &str,
    seq: i64,
    accepted: &[Finding],
    now: i64,
    appended: bool,
) {
    let mut topics =
        if appended { state.open.as_ref().map(|open| open.topics.clone()).unwrap_or_default() } else { Vec::new() };
    for finding in accepted {
        state.topics.insert(finding.topic.clone(), Topic { at: now, seq });
        if !topics.contains(&finding.topic) {
            topics.push(finding.topic.clone());
        }
    }
    state.open = Some(Open { entry: entry.to_owned(), seq, last_at: now, topics });
    if let Err(error) = save(store, bot, state) {
        tracing::warn!(bot, error = format!("{error:#}"), "couldn't save the notice policy");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::BotConfig;
    use std::sync::Arc;

    fn hub() -> (Arc<Hub>, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("codync-notices-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let store = Store::open(std::path::Path::new(":memory:")).expect("store");
        let identity = crate::remote::identity::Identity::load_or_create(&dir).expect("identity");
        let hub = Hub::new(store, "notices-test".into(), identity, "t".into(), 0);
        let mut cfg: BotConfig =
            serde_json::from_value(json!({"id": "b1", "name": "B", "backend": "claude"})).expect("bot");
        cfg.proactive.notice_cooldown_hours = 6;
        hub.store.save_bot(&cfg).expect("saved");
        (hub, dir)
    }

    fn notices(hub: &Hub) -> Vec<crate::store::Entry> {
        hub.store.entries_since(0, 50).expect("entries").into_iter().filter(|entry| entry.kind == "notice").collect()
    }

    #[test]
    fn three_findings_arrive_in_one_notice() {
        let (hub, dir) = hub();
        let id = raise(
            &hub,
            "b1",
            vec![
                Finding::new("deploy", "The deploy failed."),
                Finding::new("tests", "Two tests are flaky."),
                Finding::error("disk", "The disk is almost full."),
            ],
        )
        .expect("a notice");
        let listed = notices(&hub);
        assert_eq!(listed.len(), 1, "one notice, not three");
        assert_eq!(listed[0].id, id);
        assert_eq!(listed[0].data["notices"].as_array().map(Vec::len), Some(3));
        assert_eq!(listed[0].data["style"].as_str(), Some("error"), "the strongest style wins");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_waiting_topic_is_not_raised_again_until_a_reply_or_the_deadline() {
        let (hub, dir) = hub();
        let first = raise(&hub, "b1", vec![Finding::new("deploy", "The deploy failed.")]).expect("a notice");
        assert_eq!(notices(&hub).len(), 1);
        // The same topic within the batch window is suppressed, not a second notice.
        assert_eq!(raise(&hub, "b1", vec![Finding::new("deploy", "Still failing.")]), None);
        assert_eq!(notices(&hub).len(), 1);
        assert_eq!(notices(&hub)[0].id, first.clone());
        // Another topic joins the open notice.
        raise(&hub, "b1", vec![Finding::new("tests", "A test is flaky.")]);
        assert_eq!(notices(&hub).len(), 1);
        assert_eq!(notices(&hub)[0].data["notices"].as_array().map(Vec::len), Some(2));

        // A reply releases it: the topic may be raised again, as a notice of its own.
        let line = hub.store.insert_entry(&Lane::main("b1"), EntryKind::User, 1, &json!({"text": "looking"})).unwrap();
        assert!(line.seq > 0);
        let again = raise(&hub, "b1", vec![Finding::new("deploy", "The deploy failed again.")]).expect("a new notice");
        assert_ne!(again, first);
        assert_eq!(notices(&hub).len(), 2);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn the_cooldown_brings_a_topic_back_without_a_reply() {
        let (hub, dir) = hub();
        let now = now_ms();
        let first = raise_at(&hub, "b1", vec![Finding::new("deploy", "The deploy failed.")], now, true);
        assert!(first.is_some());
        assert!(raise_at(&hub, "b1", vec![Finding::new("deploy", "Still failing.")], now + 60_000, true).is_none());
        assert_eq!(notices(&hub).len(), 1, "still the same waiting topic");
        let later = now + 7 * 3_600_000;
        let raised = raise_at(&hub, "b1", vec![Finding::new("deploy", "The deploy failed again.")], later, true);
        assert!(raised.is_some());
        assert_eq!(notices(&hub).len(), 2, "the deadline released it");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_single_notice_can_carry_its_own_entry_fields() {
        let (hub, dir) = hub();
        let undo = json!({"memoryUndo": {"file": "MEMORY.md", "undone": false}});
        let id = tell(&hub, "b1", Finding::new("memory:MEMORY.md", "I learned something").with_extra(undo))
            .expect("a notice");
        let entry = hub.store.entry(&id).expect("saved");
        assert_eq!(entry.data["memoryUndo"]["file"].as_str(), Some("MEMORY.md"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_check_in_that_spoke_waits_for_the_person_to_answer() {
        let (hub, dir) = hub();
        let checked = now_ms() - 1_000;
        assert!(!check_in_waiting(&hub, "b1", checked), "nothing was said yet");
        hub.store
            .insert_entry(&Lane::main("b1"), EntryKind::Agent, 1, &json!({"text": "The deploy failed.", "final": true}))
            .unwrap();
        assert!(check_in_waiting(&hub, "b1", checked), "an unanswered check-in holds the next one");
        hub.store.insert_entry(&Lane::main("b1"), EntryKind::User, 2, &json!({"text": "on it"})).unwrap();
        assert!(!check_in_waiting(&hub, "b1", checked), "the reply releases it");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_proactive_check_in_waits_for_a_reply_or_its_cooldown() {
        let (hub, dir) = hub();
        assert!(may_raise(&hub, "b1", "check-in"));
        mark_raised(&hub, "b1", "check-in");
        assert!(!may_raise(&hub, "b1", "check-in"));
        hub.store.insert_entry(&Lane::main("b1"), EntryKind::User, 1, &json!({"text": "thanks"})).unwrap();
        assert!(may_raise(&hub, "b1", "check-in"), "a reply opens the topic again");
        let _ = std::fs::remove_dir_all(dir);
    }
}
