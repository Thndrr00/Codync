//! An output step's own message and notification fields, ported from OpenBot nightly.
//!
//! An output step can send a message in place of the workflow result that reaches it. `{{name}}` in
//! that message — and in the title, labels and person of a notification — takes the value of a
//! field from an earlier step or from the trigger. The rendered title is what a notification shows.
//! The template is saved per workflow in one kv slot, so an older host without the field simply
//! sends the workflow result as before; every decision here is pure and tested.

use super::answers;
use crate::remote::push::AlertKind;
use crate::store::Store;
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// kv prefix of a workflow's output template, followed by the routine id.
pub const TEMPLATE_PREFIX: &str = "routineflow.output-template.";

/// The message and notification fields of one output step. Every field is optional: an empty one
/// means "use what the workflow already had".
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputTemplate {
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub labels: Option<String>,
    #[serde(default)]
    pub person: Option<String>,
}

impl OutputTemplate {
    /// Whether every field is empty, in which case the slot can be removed.
    pub fn is_empty(&self) -> bool {
        [&self.message, &self.title, &self.labels, &self.person]
            .iter()
            .all(|field| field.as_deref().is_none_or(str::is_empty))
    }
}

/// How long each template field may be. A message is bounded like a link instruction.
pub const MAX_FIELD_CHARS: usize = 2000;

/// Whether a template can be saved. Pure so the API can name the field that is too long.
pub fn check(template: &OutputTemplate) -> Result<()> {
    for (name, field) in [
        ("message", &template.message),
        ("title", &template.title),
        ("labels", &template.labels),
        ("person", &template.person),
    ] {
        if field.as_deref().is_some_and(|text| text.chars().count() > MAX_FIELD_CHARS) {
            bail!("`{name}` must be at most {MAX_FIELD_CHARS} characters");
        }
    }
    Ok(())
}

/// The saved template of `routine`, or a blank one for a workflow saved before output templates.
pub fn load(store: &Store, routine: &str) -> OutputTemplate {
    store
        .kv_get(&format!("{TEMPLATE_PREFIX}{routine}"))
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

/// Saves the template; clearing every field removes the slot again.
pub fn save(store: &Store, routine: &str, template: &OutputTemplate) -> Result<()> {
    let key = format!("{TEMPLATE_PREFIX}{routine}");
    if template.is_empty() { store.kv_delete(&key) } else { store.kv_set(&key, &serde_json::to_string(template)?) }
}

/// Replaces `{{name}}` with a field's value. A name with no value keeps its braces, so a typo is
/// visible in the sent message instead of silently sending nothing.
pub fn render(text: &str, values: &BTreeMap<String, String>) -> String {
    answers::fill_template(text, values)
}

/// The message an output step sends: its own message when set, otherwise its input.
pub fn message_or_input(template: &OutputTemplate, input: &str, values: &BTreeMap<String, String>) -> String {
    template
        .message
        .as_deref()
        .map(str::trim)
        .filter(|message| !message.is_empty())
        .map_or_else(|| input.to_owned(), |message| render(message, values))
}

/// The title a notification shows: the output's title when set, else the first line of its message.
pub fn notification_title(template: &OutputTemplate, fallback: &str, values: &BTreeMap<String, String>) -> String {
    template
        .title
        .as_deref()
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .map(|title| render(title, values))
        .unwrap_or_else(|| fallback.lines().next().unwrap_or_default().trim().chars().take(120).collect())
}

/// The labels an output sets, split on commas and each rendered from the fields of earlier steps.
pub fn notification_labels(template: &OutputTemplate, values: &BTreeMap<String, String>) -> Vec<String> {
    template
        .labels
        .as_deref()
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|label| !label.is_empty())
        .map(|label| render(label, values))
        .collect()
}

/// The person an output addresses, when it names one.
pub fn notification_person(template: &OutputTemplate, values: &BTreeMap<String, String>) -> Option<String> {
    template.person.as_deref().map(str::trim).filter(|person| !person.is_empty()).map(|person| render(person, values))
}

/// The rendered notification an output step sends: its message, title, labels and person.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notification {
    pub message: String,
    pub title: String,
    pub labels: Vec<String>,
    pub person: Option<String>,
}

/// Builds a notification from the template and the fields of earlier steps. The title is used for
/// the notification's heading, so it comes from the output step rather than the message body.
pub fn notification(template: &OutputTemplate, input: &str, values: &BTreeMap<String, String>) -> Notification {
    let message = message_or_input(template, input, values);
    Notification {
        title: notification_title(template, &message, values),
        message,
        labels: notification_labels(template, values),
        person: notification_person(template, values),
    }
}

/// The most card text a workflow alert carries in its sealed body (OpenBot 0.78.0).
pub const PUSH_BODY_CHARS: usize = 400;

/// The phone alert of a workflow's Notification output or a step waiting on the user. Only `kind`
/// is ever attached to analytics; the card text lives in `body`, which the push layer seals to the
/// phone, never in an event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkflowPush {
    pub body: String,
    pub kind: AlertKind,
}

/// Pure decision for the push a workflow step sends: blank card text carries nothing to show, so
/// no alert goes out; other text is clipped to the alert body limit instead of growing with a card.
pub fn workflow_push(text: &str, kind: AlertKind) -> Option<WorkflowPush> {
    let body = text.trim();
    (!body.is_empty()).then(|| WorkflowPush { body: crate::agent::acp::truncate(body, PUSH_BODY_CHARS), kind })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn values() -> BTreeMap<String, String> {
        BTreeMap::from([("author".to_owned(), "ana".to_owned()), ("title".to_owned(), "Fix login".to_owned())])
    }

    #[test]
    fn a_message_replaces_the_input_and_fills_fields_from_earlier_steps() {
        let template =
            OutputTemplate { message: Some("{{author}} opened {{title}}".into()), ..OutputTemplate::default() };
        assert_eq!(message_or_input(&template, "old result", &values()), "ana opened Fix login");
        let plain = OutputTemplate::default();
        assert_eq!(message_or_input(&plain, "old result", &values()), "old result");
    }

    #[test]
    fn a_notification_uses_the_output_title() {
        let template = OutputTemplate { title: Some("New: {{title}}".into()), ..OutputTemplate::default() };
        assert_eq!(notification_title(&template, "fallback first line\nrest", &values()), "New: Fix login");
        assert_eq!(
            notification_title(&OutputTemplate::default(), "fallback first line\nrest", &values()),
            "fallback first line"
        );
    }

    #[test]
    fn labels_and_person_are_rendered_from_fields() {
        let template = OutputTemplate {
            labels: Some("{{title}}, urgent".into()),
            person: Some("{{author}}".into()),
            ..OutputTemplate::default()
        };
        assert_eq!(notification_labels(&template, &values()), ["Fix login", "urgent"]);
        assert_eq!(notification_person(&template, &values()).as_deref(), Some("ana"));
    }

    #[test]
    fn a_notification_takes_its_heading_from_the_output_title() {
        let template = OutputTemplate {
            message: Some("{{author}} opened {{title}}".into()),
            title: Some("New issue: {{title}}".into()),
            labels: Some("bug".into()),
            ..OutputTemplate::default()
        };
        let note = notification(&template, "old result", &values());
        assert_eq!(note.message, "ana opened Fix login");
        assert_eq!(note.title, "New issue: Fix login");
        assert_eq!(note.labels, ["bug"]);
        assert_eq!(note.person, None);
    }

    #[test]
    fn an_unknown_field_keeps_its_braces() {
        let template = OutputTemplate { message: Some("{{missing}} stays".into()), ..OutputTemplate::default() };
        assert_eq!(message_or_input(&template, "", &values()), "{{missing}} stays");
    }

    #[test]
    fn a_too_long_field_is_named() {
        let template = OutputTemplate { title: Some("x".repeat(MAX_FIELD_CHARS + 1)), ..OutputTemplate::default() };
        let error = check(&template).expect_err("a long title is refused");
        assert!(error.to_string().contains("title"));
    }

    #[test]
    fn a_blank_card_sends_no_push_and_a_real_card_keeps_its_kind() {
        assert_eq!(workflow_push("", AlertKind::Done), None);
        assert_eq!(workflow_push("  \n ", AlertKind::NeedsInput), None);
        assert_eq!(
            workflow_push("  New issue opened  ", AlertKind::Done),
            Some(WorkflowPush { body: "New issue opened".into(), kind: AlertKind::Done })
        );
        assert_eq!(workflow_push("step paused", AlertKind::Paused).map(|push| push.kind), Some(AlertKind::Paused));
    }

    #[test]
    fn a_long_card_is_clipped_to_the_alert_body() {
        let push = workflow_push(&"x".repeat(PUSH_BODY_CHARS + 40), AlertKind::Failed).expect("a push");
        assert_eq!(push.body.chars().count(), PUSH_BODY_CHARS);
        assert_eq!(push.kind, AlertKind::Failed);
    }

    #[test]
    fn a_blank_template_is_not_saved() {
        let store = Store::open(std::path::Path::new(":memory:")).expect("a store");
        let template: OutputTemplate = serde_json::from_value(json!({
            "message": "Hi {{author}}", "labels": "urgent"
        }))
        .expect("a template");
        save(&store, "r", &template).expect("a save");
        assert_eq!(load(&store, "r"), template);
        save(&store, "r", &OutputTemplate::default()).expect("a clear");
        assert!(store.kv_get(&format!("{TEMPLATE_PREFIX}r")).is_none());
        assert_eq!(load(&store, "r"), OutputTemplate::default());
    }
}
