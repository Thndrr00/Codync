//! A question or a plan of Claude Code in a terminal, answered from any Codync client.
//!
//! The `PreToolUse` hook installed to wait (`hook --ask`, matching `AskUserQuestion` and `ExitPlanMode`) posts the
//! tool's input here. Each question becomes a card whose options are the question's own; the picked labels go back
//! as `updatedInput.answers`, which is how a hook answers `AskUserQuestion`. A plan becomes an Approve / Reject card.
//! With no answer in time, or for anything this does not understand, the reply is silence and the terminal asks.

use super::event::Event;
use super::permission::{target, wait_card};
use super::{Decision, tool};
use crate::hub::Hub;
use serde_json::{Map, Value, json};
use std::sync::Arc;

/// Claude Code asks at most this many questions at once, each with at most this many options.
const MAX_QUESTIONS: usize = 4;
const MAX_OPTIONS: usize = 4;

const APPROVE: &str = "allow";
const REJECT: &str = "deny";
/// The answer that goes back to the question before this one. It is a card option, so every client
/// answers it like any other.
const BACK: &str = "__back";
/// Whether an option id is the way back to the question before this one.
pub(super) fn is_back(id: &str) -> bool {
    id == BACK
}

/// The verdict to print for the hook, or `None` when nobody answered and the terminal should ask.
pub(super) async fn ask(hub: &Arc<Hub>, ev: &Event) -> Option<Value> {
    if !matches!(ev.tool.as_str(), "AskUserQuestion" | "ExitPlanMode") {
        return None;
    }
    let (bot, config) = target(hub, ev)?;
    if ev.tool == "ExitPlanMode" {
        let data = plan_card(ev, &bot, &config.cwd);
        // The plan starts the work by itself after the computer's time, or waits for the person.
        let after = super::plan::saved(&hub.store);
        let starts_at = after.map(|ms| crate::store::now_ms() + ms);
        return match wait_card(hub, ev, &bot, &config, &data, "Approve the plan", starts_at, super::plan::wait(after))
            .await?
        {
            Decision::Allow => Some(json!({"permissionDecision": "allow", "updatedInput": ev.input})),
            Decision::Deny => {
                Some(json!({"permissionDecision": "deny", "reason": "The plan was rejected from Codync."}))
            }
            Decision::Pick(_) => None,
        };
    }
    let questions = questions(&ev.input)?;
    // One card per question, in order. Back re-asks the one before, keeping the answers already given
    // in the card's data, so the person sees where they are and the summary before the last answer.
    let mut answers: Vec<Option<String>> = vec![None; questions.len()];
    let mut step = 0;
    loop {
        let data = question_card(&questions, step, &answers, &bot, &config.cwd);
        let Decision::Pick(index) =
            wait_card(hub, ev, &bot, &config, &data, &questions[step].text, None, super::DECISION_WAIT).await?
        else {
            return None;
        };
        // The Back row sits after the question's own options.
        if index == questions[step].options.len() {
            step = step.saturating_sub(1);
            continue;
        }
        let option = questions[step].options.get(index)?;
        answers[step] = Some(option.label.clone());
        if step + 1 < questions.len() {
            step += 1;
            continue;
        }
        break;
    }
    let mut answers_out = Map::new();
    for (question, answer) in questions.iter().zip(answers) {
        answers_out.insert(question.text.clone(), answer?.into());
    }
    let mut input = ev.input.clone();
    input["answers"] = Value::Object(answers_out);
    Some(json!({"permissionDecision": "allow", "updatedInput": input}))
}

struct Question {
    text: String,
    header: String,
    options: Vec<Choice>,
}

struct Choice {
    label: String,
    description: String,
}

/// The questions of an `AskUserQuestion`, when every one can be answered by tapping a single option. A multi-select
/// question, one without two to four options, or too many questions are left to the terminal.
fn questions(input: &Value) -> Option<Vec<Question>> {
    let list = input.get("questions")?.as_array().filter(|q| (1..=MAX_QUESTIONS).contains(&q.len()))?;
    list.iter()
        .map(|q| {
            if q.get("multiSelect").and_then(Value::as_bool) == Some(true) {
                return None;
            }
            let options: Vec<Choice> = q
                .get("options")?
                .as_array()?
                .iter()
                .filter_map(|o| {
                    let label = o.get("label")?.as_str()?.trim();
                    (!label.is_empty()).then(|| Choice {
                        label: label.to_owned(),
                        description: o.get("description").and_then(Value::as_str).unwrap_or_default().to_owned(),
                    })
                })
                .collect();
            let text = q.get("question")?.as_str()?.trim().to_owned();
            ((2..=MAX_OPTIONS).contains(&options.len()) && !text.is_empty()).then(|| Question {
                text,
                header: q.get("header").and_then(Value::as_str).unwrap_or_default().to_owned(),
                options,
            })
        })
        .collect()
}

/// A question as an approval card: the options keep their labels, their ids are their positions.
/// The card carries the whole set, which question it asks, the answers already given and, after the
/// first, a Back option, so every client can show one question at a time with a summary.
fn question_card(questions: &[Question], step: usize, answers: &[Option<String>], bot: &str, cwd: &str) -> Value {
    let question = &questions[step];
    let detail: Vec<String> = question
        .options
        .iter()
        .filter(|o| !o.description.is_empty())
        .map(|o| format!("{}: {}", o.label, o.description))
        .collect();
    let mut options: Vec<Value> = question
        .options
        .iter()
        .enumerate()
        .map(|(i, o)| json!({"optionId": i.to_string(), "name": o.label, "kind": "allow_once"}))
        .collect();
    if step > 0 {
        options.push(json!({"optionId": BACK, "name": "Back", "kind": "other"}));
    }
    let answered: Vec<Value> = questions
        .iter()
        .zip(answers)
        .take(step)
        .filter_map(|(question, answer)| answer.as_ref().map(|label| json!({"text": question.text, "label": label})))
        .collect();
    json!({
        "title": question.text,
        "toolKind": "question",
        "question": true,
        "header": question.header,
        "command": Value::Null,
        "detail": detail.join("\n"),
        "diffs": [],
        "cwd": cwd,
        "options": options,
        "questions": questions
            .iter()
            .map(|question| json!({
                "text": question.text,
                "header": question.header,
                "options": question.options.iter().map(|o| json!({"label": o.label, "description": o.description})).collect::<Vec<_>>(),
            }))
            .collect::<Vec<_>>(),
        "questionIndex": step,
        "questionCount": questions.len(),
        "answers": answered,
        "status": "pending",
        "selected": Value::Null,
        "author": bot,
    })
}

fn plan_card(ev: &Event, bot: &str, cwd: &str) -> Value {
    json!({
        "title": tool::title(&ev.tool, &ev.input),
        "toolKind": "plan",
        "plan": true,
        "command": Value::Null,
        "detail": ev.input.get("plan").and_then(Value::as_str).unwrap_or_default(),
        "diffs": [],
        "cwd": cwd,
        "options": [
            {"optionId": APPROVE, "name": "Approve", "kind": "allow_once"},
            {"optionId": REJECT, "name": "Reject", "kind": "reject_once"},
        ],
        "status": "pending",
        "selected": Value::Null,
        "author": bot,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(question: &Value) -> Value {
        json!({"questions": [question]})
    }

    fn pick() -> Value {
        json!({"question": "Which database?", "header": "DB", "multiSelect": false,
            "options": [{"label": "SQLite", "description": "one file"}, {"label": "Postgres", "description": "a server"}]})
    }

    #[test]
    fn a_single_choice_question_becomes_a_card_with_its_own_options() {
        let list = questions(&input(&pick())).expect("answerable");
        let card = question_card(&list, 0, &[None], "bot", "/work");
        assert_eq!(card["title"], "Which database?");
        assert_eq!(card["options"][1], json!({"optionId": "1", "name": "Postgres", "kind": "allow_once"}));
        assert_eq!(card["detail"], "SQLite: one file\nPostgres: a server");
        assert_eq!(card["questionIndex"], 0);
        assert_eq!(card["questionCount"], 1);
        assert_eq!(card["questions"][0]["options"][0]["label"], "SQLite");
        assert_eq!(card["answers"], json!([]));
        // The first question has nowhere to go back to.
        assert!(card["options"].as_array().unwrap().iter().all(|option| option["optionId"] != "__back"));
    }

    #[test]
    fn a_later_question_carries_the_answers_and_a_way_back() {
        let list = questions(&json!({"questions": [pick(), pick()]})).expect("answerable");
        let card = question_card(&list, 1, &[Some("SQLite".to_owned()), None], "bot", "/work");
        assert_eq!(card["questionIndex"], 1);
        assert_eq!(card["questionCount"], 2);
        assert_eq!(card["answers"][0], json!({"text": "Which database?", "label": "SQLite"}));
        assert_eq!(card["options"][2]["optionId"], "__back");
        assert_eq!(card["options"][2]["name"], "Back");
    }

    #[test]
    fn what_a_tap_cannot_answer_is_left_to_the_terminal() {
        let mut many = pick();
        many["multiSelect"] = true.into();
        let one_option = json!({"question": "Sure?", "options": [{"label": "Yes"}]});
        let too_many: Vec<Value> = (0..5).map(|_| pick()).collect();
        for input in [
            input(&many),
            input(&one_option),
            json!({"questions": too_many}),
            json!({"questions": []}),
            json!({"questions": [{"options": []}]}),
            json!({}),
        ] {
            assert!(questions(&input).is_none(), "{input}");
        }
    }

    #[test]
    fn up_to_four_questions_are_asked() {
        let four: Vec<Value> = (0..4).map(|_| pick()).collect();
        assert_eq!(questions(&json!({"questions": four})).map(|q| q.len()), Some(4));
    }
}
