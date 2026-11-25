//! Fixed fields a trigger gives to the steps that follow it, ported from OpenBot nightly.
//!
//! A schedule gives `weekday` and `local_date`, a GitHub event gives `title`, `author` and `labels`,
//! and a chat mention gives `channel`, `text` and the sender as `author`. They are checked answer
//! values, so a condition such as `author = ana` works in a step with no model before it, and a
//! `{{weekday}}` in a later instruction needs no model step to fill it. The fields are computed once
//! from the run's event and frozen on the run, so a retry or a "run from this step" sees the same
//! values. Everything here is pure; `engine` calls it when a run starts and when a step reads its
//! answers.

use super::answers::{Answer, AnswerValue};
use crate::routines::{Trigger, local_zone};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::BTreeMap;

/// A field a trigger publishes to later steps, with the type clients show in their `{{` picker.
/// Paths are read from the event Codync's webhook and channel ingress build; the first non-empty
/// one wins. Ported from OpenBot's `ipc-workflow-trigger-fields.ts` (format 3).
struct Field {
    name: &'static str,
    ty: &'static str,
    paths: &'static [&'static [&'static str]],
}

const fn field(name: &'static str, ty: &'static str, paths: &'static [&'static [&'static str]]) -> Field {
    Field { name, ty, paths }
}

/// The 24 typed fields of a GitHub trigger, in the OpenBot contract's order.
const GITHUB_FIELDS: &[Field] = &[
    field("repository", "text", &[&["repo"], &["payload", "repository", "full_name"]]),
    field("actor", "text", &[&["sender"], &["payload", "sender", "login"]]),
    field("action", "text", &[&["action"], &["payload", "action"]]),
    field("number", "integer", &[&["payload", "issue", "number"], &["payload", "pull_request", "number"]]),
    field("pull_request", "yes/no", &[&["payload", "pull_request"], &["pullRequest"]]),
    field(
        "url",
        "text",
        &[
            &["url"],
            &["payload", "comment", "html_url"],
            &["payload", "review", "html_url"],
            &["payload", "pull_request", "html_url"],
            &["payload", "issue", "html_url"],
        ],
    ),
    field("title", "text", &[&["title"], &["payload", "issue", "title"], &["payload", "pull_request", "title"]]),
    field(
        "body",
        "text",
        &[
            &["payload", "issue", "body"],
            &["payload", "pull_request", "body"],
            &["payload", "comment", "body"],
            &["text"],
        ],
    ),
    field(
        "author",
        "text",
        &[
            &["author"],
            &["payload", "issue", "user", "login"],
            &["payload", "pull_request", "user", "login"],
            &["payload", "comment", "user", "login"],
        ],
    ),
    field("state", "text", &[&["payload", "issue", "state"], &["payload", "pull_request", "state"]]),
    field("labels", "list", &[&["labels"], &["payload", "issue", "labels"], &["payload", "pull_request", "labels"]]),
    field("label", "text", &[&["payload", "label", "name"], &["label"]]),
    field("assignee", "text", &[&["payload", "assignee", "login"], &["assignee"]]),
    field(
        "reviewer",
        "text",
        &[&["payload", "requested_reviewer", "login"], &["payload", "review", "user", "login"], &["reviewer"]],
    ),
    field("comment_author", "text", &[&["payload", "comment", "user", "login"], &["comment", "author"]]),
    field("comment_body", "text", &[&["payload", "comment", "body"], &["comment", "body"]]),
    field("comment_url", "text", &[&["payload", "comment", "html_url"], &["comment", "url"]]),
    field("review_author", "text", &[&["payload", "review", "user", "login"], &["review", "author"]]),
    field("review_state", "text", &[&["payload", "review", "state"], &["review", "state"]]),
    field("review_body", "text", &[&["payload", "review", "body"], &["review", "body"]]),
    field("review_url", "text", &[&["payload", "review", "html_url"], &["review", "url"]]),
    field("check_conclusion", "text", &[&["payload", "check_run", "conclusion"], &["checks", "conclusion"]]),
    field("check_app", "text", &[&["payload", "check_run", "app", "name"], &["checks", "app"]]),
    field(
        "head_branch",
        "text",
        &[
            &["payload", "pull_request", "head", "ref"],
            &["payload", "check_run", "head_branch"],
            &["checks", "headBranch"],
        ],
    ),
];

/// Discord, Telegram and Slack publish the shared message fields plus their place's ids.
const CHAT_FIELDS: &[Field] = &[
    field("text", "text", &[&["text"]]),
    field("author", "text", &[&["author"], &["user"], &["sender"], &["username"]]),
    field("author_id", "text", &[&["userId"], &["user"], &["sender"]]),
    field("message_id", "text", &[&["messageId"], &["message_id"]]),
    field("channel", "text", &[&["channel"], &["channelId"], &["chat"]]),
    field("channel_id", "text", &[&["channelId"], &["channel"]]),
    field("server", "text", &[&["connection"], &["server"]]),
    field("chat", "text", &[&["chat"], &["channel"]]),
    field("chat_id", "text", &[&["chatId"], &["chat"]]),
    field("workspace", "text", &[&["connection"], &["workspace"]]),
    field("thread_id", "text", &[&["thread"], &["threadId"]]),
];

/// Reads the first non-empty string at one of the dotted paths of `value`.
fn first_str(value: &Value, paths: &[&[&str]]) -> Option<String> {
    paths.iter().find_map(|path| {
        let mut at = value;
        for key in *path {
            at = &at[*key];
        }
        at.as_str().map(str::trim).filter(|text| !text.is_empty()).map(str::to_owned)
    })
}

/// The labels of a GitHub issue or pull request, from an array of names or of label objects.
fn labels_at(value: &Value) -> Vec<String> {
    let Some(items) = value.as_array() else { return Vec::new() };
    items.iter().filter_map(|item| item.as_str().or_else(|| item["name"].as_str())).map(str::to_owned).collect()
}

/// The checked value of one typed trigger field. Integer and yes/no fields read numbers and the
/// presence of an object; a list reads labels from any of its paths. Pure, so the picker and a run
/// agree on the type of every field.
fn typed_value(event: &Value, spec: &Field) -> Option<AnswerValue> {
    match spec.name {
        "number" => {
            spec.paths.iter().find_map(|path| at(event, path).and_then(Value::as_i64)).map(AnswerValue::Integer)
        }
        "pull_request" => {
            spec.paths.iter().find_map(|path| at(event, path)).map(|value| AnswerValue::YesNo(!value.is_null()))
        }
        "labels" => {
            let labels = spec
                .paths
                .iter()
                .flat_map(|path| at(event, path).map(labels_at).unwrap_or_default())
                .collect::<Vec<_>>();
            (!labels.is_empty()).then_some(AnswerValue::List(labels))
        }
        _ => first_str(event, spec.paths).map(AnswerValue::Text),
    }
}

fn at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut at = value;
    for key in path {
        at = at.get(*key)?;
    }
    Some(at)
}

fn publish(answer: &mut Answer, event: &Value, fields: &'static [Field]) {
    for spec in fields {
        if let Some(value) = typed_value(event, spec) {
            answer.entry(spec.name.to_owned()).or_insert(value);
        }
    }
}

/// The zone a trigger names, or this computer's zone for an interval or one-off schedule.
fn schedule_zone(triggers: &[Trigger]) -> chrono_tz::Tz {
    triggers
        .iter()
        .find_map(|trigger| match trigger {
            Trigger::Cron { time_zone, .. } => time_zone.parse::<chrono_tz::Tz>().ok(),
            _ => None,
        })
        .unwrap_or_else(|| local_zone().parse::<chrono_tz::Tz>().unwrap_or(chrono_tz::Tz::UTC))
}

fn is_schedule(triggers: &[Trigger]) -> bool {
    triggers
        .iter()
        .any(|trigger| matches!(trigger, Trigger::Interval { .. } | Trigger::Once { .. } | Trigger::Cron { .. }))
}

/// The fields a trigger makes available to later steps, for the client's `{{` field picker. The
/// names match `fixed_answers`: a schedule gives `weekday`/`local_date`, GitHub gives
/// `title`/`author`/`labels`, and a chat mention gives `channel`/`text`/`author`. Each is a plain
/// text field; the fixed values depend on the actual event and arrive at run time.
pub fn fixed_field_names(triggers: &[Trigger]) -> Vec<(String, String)> {
    fn push(names: &mut Vec<(String, String)>, name: &str, ty: &str) {
        if let Some(existing) = names.iter_mut().find(|(n, _)| n == name) {
            // A typed field refines the plain text entry an older client first offered.
            if ty != "text" {
                existing.1 = ty.to_owned();
            }
        } else {
            names.push((name.to_owned(), ty.to_owned()));
        }
    }
    let mut names: Vec<(String, String)> = Vec::new();
    for trigger in triggers {
        match trigger {
            Trigger::Once { .. } | Trigger::Interval { .. } | Trigger::Cron { .. } => {
                push(&mut names, "weekday", "text");
                push(&mut names, "local_date", "text");
            }
            Trigger::Webhook => {
                for spec in CHAT_FIELDS {
                    if spec.name == "text" || spec.name == "author" {
                        push(&mut names, spec.name, spec.ty);
                    }
                }
                push(&mut names, "title", "text");
            }
            Trigger::Event { source, .. } => {
                // Keep the legacy names so a workflow saved before types existed still fills them.
                for name in ["title", "author", "labels", "text", "channel"] {
                    push(&mut names, name, "text");
                }
                let fields = match source.as_str() {
                    "github" => GITHUB_FIELDS,
                    "discord" | "telegram" | "slack" | "whatsapp" => CHAT_FIELDS,
                    _ => &[],
                };
                for spec in fields {
                    push(&mut names, spec.name, spec.ty);
                }
            }
            Trigger::Watch { .. } => {}
        }
    }
    names
}

/// The fixed fields of `event` and `triggers`, keyed by field name. Only non-empty values are given:
/// a step that uses `{{labels}}` without a GitHub trigger keeps its braces instead of sending an
/// empty placeholder.
pub fn fixed_answers(run_at: i64, triggers: &[Trigger], event: &Value) -> Answer {
    let mut answer = Answer::new();
    let source = event["source"].as_str().unwrap_or_default();
    let payload = &event["payload"];
    if source == "github" {
        let title = first_str(
            event,
            &[
                &["title"],
                &["text"],
                &["payload", "issue", "title"],
                &["payload", "pull_request", "title"],
                &["payload", "discussion", "title"],
            ],
        );
        let author = first_str(
            event,
            &[
                &["author"],
                &["sender"],
                &["payload", "sender", "login"],
                &["payload", "issue", "user", "login"],
                &["payload", "pull_request", "user", "login"],
            ],
        );
        let labels = labels_at(&event["labels"])
            .into_iter()
            .chain(labels_at(&payload["issue"]["labels"]))
            .chain(labels_at(&payload["pull_request"]["labels"]))
            .collect::<Vec<_>>();
        if let Some(title) = title {
            answer.insert("title".into(), AnswerValue::Text(title));
        }
        if let Some(author) = author {
            answer.insert("author".into(), AnswerValue::Text(author));
        }
        if !labels.is_empty() {
            answer.insert("labels".into(), AnswerValue::List(labels));
        }
        // The full typed table (24 fields in OpenBot) adds `number`, `pull_request`, `body`,
        // `review_*`, `check_*` and the rest without overwriting the legacy values above.
        publish(&mut answer, event, GITHUB_FIELDS);
    }
    if ["slack", "telegram", "discord", "whatsapp"].contains(&source) {
        let channel = first_str(event, &[&["channel"], &["chat"]]);
        let text = first_str(event, &[&["text"]]);
        let author = first_str(event, &[&["author"], &["user"], &["sender"]]);
        if let Some(channel) = channel {
            answer.insert("channel".into(), AnswerValue::Text(channel));
        }
        if let Some(text) = text {
            answer.insert("text".into(), AnswerValue::Text(text));
        }
        if let Some(author) = author {
            answer.insert("author".into(), AnswerValue::Text(author));
        }
        publish(&mut answer, event, CHAT_FIELDS);
    }
    // Workflow webhook deliveries publish the body's text as a trigger field too.
    if source == "webhook" || source == "test" {
        publish(&mut answer, event, CHAT_FIELDS);
    }
    if is_schedule(triggers)
        && let Some(at) =
            DateTime::<Utc>::from_timestamp_millis(run_at).map(|at| at.with_timezone(&schedule_zone(triggers)))
    {
        answer.insert("weekday".into(), AnswerValue::Text(at.format("%A").to_string()));
        answer.insert("local_date".into(), AnswerValue::Text(at.format("%Y-%m-%d").to_string()));
    }
    answer
}

/// Adds the trigger's fields to a step's checked answer without overwriting what a model answered.
pub fn merged_answer(step_answer: &mut Answer, trigger: &Answer) {
    for (name, value) in trigger {
        step_answer.entry(name.clone()).or_insert_with(|| value.clone());
    }
}

/// Adds the trigger's fields to the `{{name}}` values of a step's instruction, keeping any value a
/// supplier step already gave the same name.
pub fn merged_values(values: &mut BTreeMap<String, String>, trigger: &Answer) {
    for (name, value) in trigger {
        values.entry(name.clone()).or_insert_with(|| super::answers::display_value(value));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_github_event_gives_title_author_and_labels() {
        let event = json!({
            "source": "github", "event": "issues.opened", "text": "Fix login",
            "sender": "ana",
            "payload": {"issue": {"labels": [{"name": "bug"}, "urgent"]}}
        });
        let fields = fixed_answers(0, &[], &event);
        assert_eq!(fields["title"], AnswerValue::Text("Fix login".into()));
        assert_eq!(fields["author"], AnswerValue::Text("ana".into()));
        assert_eq!(fields["labels"], AnswerValue::List(vec!["bug".into(), "urgent".into()]));
    }

    #[test]
    fn a_chat_mention_gives_channel_text_and_sender() {
        let event = json!({"source": "discord", "channel": "C1", "text": "hello there", "user": "ana"});
        let fields = fixed_answers(0, &[], &event);
        assert_eq!(fields["channel"], AnswerValue::Text("C1".into()));
        assert_eq!(fields["text"], AnswerValue::Text("hello there".into()));
        assert_eq!(fields["author"], AnswerValue::Text("ana".into()));
    }

    #[test]
    fn a_schedule_gives_the_weekday_and_local_date_in_its_zone() {
        let schedule = vec![Trigger::Cron { expression: "0 9 * * *".into(), time_zone: "Asia/Taipei".into() }];
        // 2026-11-22 00:30 UTC is already Sunday 08:30 in Taipei.
        let at = chrono::DateTime::parse_from_rfc3339("2026-11-22T00:30:00Z").expect("a time").timestamp_millis();
        let fields = fixed_answers(at, &schedule, &json!({"source": "test"}));
        assert_eq!(fields["weekday"], AnswerValue::Text("Sunday".into()));
        assert_eq!(fields["local_date"], AnswerValue::Text("2026-11-22".into()));
        assert!(fixed_answers(at, &[], &json!({"source": "test"})).is_empty(), "no schedule, no date fields");
    }

    #[test]
    fn a_model_answer_wins_over_the_trigger_field() {
        let mut step = Answer::from([("author".to_owned(), AnswerValue::Text("bo".into()))]);
        let trigger = Answer::from([
            ("author".to_owned(), AnswerValue::Text("ana".into())),
            ("weekday".into(), AnswerValue::Text("Sunday".into())),
        ]);
        merged_answer(&mut step, &trigger);
        assert_eq!(step["author"], AnswerValue::Text("bo".into()));
        assert_eq!(step["weekday"], AnswerValue::Text("Sunday".into()));
        let mut values = BTreeMap::from([("author".to_owned(), "bo".to_owned())]);
        merged_values(&mut values, &trigger);
        assert_eq!(values["author"], "bo");
        assert_eq!(values["weekday"], "Sunday");
    }

    #[test]
    fn the_field_picker_lists_each_trigger_kind_once() {
        let schedule = fixed_field_names(&[Trigger::Cron { expression: "* * * * *".into(), time_zone: "UTC".into() }]);
        assert!(schedule.iter().any(|(name, _)| name == "weekday"));
        assert!(schedule.iter().any(|(name, _)| name == "local_date"));
        let github = fixed_field_names(&[Trigger::Event {
            source: "github".into(),
            event: "issues".into(),
            filters: BTreeMap::new(),
        }]);
        for name in ["title", "author", "labels", "channel"] {
            assert!(github.iter().any(|(n, _)| n == name), "{name} is offered");
        }
        let both = fixed_field_names(&[Trigger::Once { at: 0 }, Trigger::Webhook]);
        assert_eq!(both.iter().filter(|(n, _)| n == "author").count(), 1, "no duplicates");
        assert!(
            fixed_field_names(&[Trigger::Watch { kind: super::super::watch::WatchKind::Files, every_minutes: 1 }])
                .is_empty()
        );
    }

    #[test]
    fn a_github_trigger_publishes_all_twenty_four_typed_fields() {
        let trigger = Trigger::Event { source: "github".into(), event: "pr.merged".into(), filters: BTreeMap::new() };
        let names = fixed_field_names(&[trigger]);
        assert_eq!(GITHUB_FIELDS.len(), 24);
        for spec in GITHUB_FIELDS {
            let (_, ty) = names.iter().find(|(name, _)| name == spec.name).unwrap_or_else(|| panic!("{}", spec.name));
            assert_eq!(ty, spec.ty, "{} has its OpenBot type", spec.name);
        }
        let event = json!({
            "source":"github","event":"pull_request","repo":"o/r","sender":"ana","text":"Fix login",
            "payload":{"action":"closed","number":12,
                "pull_request":{"number":12,"state":"closed","merged":true,"title":"Fix login",
                    "body":"Details","head":{"ref":"topic"},"user":{"login":"ana"},
                    "labels":[{"name":"bug"}]},
                "review":{"user":{"login":"kai"},"state":"APPROVED","body":"LGTM","html_url":"https://r"},
                "check_run":{"conclusion":"success","app":{"name":"CI"}}}
        });
        let fields = fixed_answers(0, &[], &event);
        assert_eq!(fields["number"], AnswerValue::Integer(12));
        assert_eq!(fields["pull_request"], AnswerValue::YesNo(true));
        assert_eq!(fields["labels"], AnswerValue::List(vec!["bug".into()]));
        assert_eq!(fields["head_branch"], AnswerValue::Text("topic".into()));
        assert_eq!(fields["review_author"], AnswerValue::Text("kai".into()));
        assert_eq!(fields["review_state"], AnswerValue::Text("APPROVED".into()));
        assert_eq!(fields["check_conclusion"], AnswerValue::Text("success".into()));
        assert_eq!(fields["actor"], AnswerValue::Text("ana".into()));
    }

    #[test]
    fn chat_mentions_publish_their_connection_message_and_thread_ids() {
        let trigger = Trigger::Event { source: "slack".into(), event: "app_mention".into(), filters: BTreeMap::new() };
        let names = fixed_field_names(&[trigger]);
        for name in ["author_id", "message_id", "channel_id", "workspace", "thread_id"] {
            assert!(names.iter().any(|(n, _)| n == name), "{name} is offered");
        }
        let event = json!({"source":"slack","event":"app_mention","text":"please review",
            "connection":"T1","channel":"C1","user":"U1","messageId":"1712.0004","thread":"1712.0000"});
        let fields = fixed_answers(0, &[], &event);
        assert_eq!(fields["workspace"], AnswerValue::Text("T1".into()));
        assert_eq!(fields["message_id"], AnswerValue::Text("1712.0004".into()));
        assert_eq!(fields["thread_id"], AnswerValue::Text("1712.0000".into()));
    }
}
