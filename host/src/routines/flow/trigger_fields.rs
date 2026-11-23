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
    let mut names: Vec<(String, String)> = Vec::new();
    let mut push = |name: &str| {
        if !names.iter().any(|(n, _)| n == name) {
            names.push((name.to_owned(), "text".to_owned()));
        }
    };
    for trigger in triggers {
        match trigger {
            Trigger::Once { .. } | Trigger::Interval { .. } => {
                push("weekday");
                push("local_date");
            }
            Trigger::Cron { .. } => {
                push("weekday");
                push("local_date");
            }
            Trigger::Webhook { .. } | Trigger::Event { .. } => {
                push("title");
                push("author");
                push("labels");
                push("text");
                push("channel");
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
}
