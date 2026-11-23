//! A question or a plan of a terminal agent, answered from any Codync client.
//!
//! The `PreToolUse` hook installed to wait (`hook --ask`, matching `AskUserQuestion` and `ExitPlanMode`) posts the
//! tool's input here. Each question becomes a card whose options are the question's own; the picked labels go back
//! as `updatedInput.answers`, which is how a hook answers `AskUserQuestion`. A plan becomes an Approve / Reject card.
//! With no answer in time, or for anything this does not understand, the reply is silence and the terminal asks.
//!
//! Following OpenBot nightly (`8d94d815e`), a question that asks for `multiSelect` takes several options:
//! the card collects them up to [`MAX_SELECTED`] and commits them when the person taps *Done*. Claude's
//! native multi-select questions already carry the flag; the shared `ask_user` tool now carries it on
//! every other provider too, so a card can accept several answers across providers.

use super::event::Event;
use super::permission::{target, wait_card};
use super::{Decision, tool};
use crate::hub::Hub;
use serde_json::{Map, Value, json};
use std::sync::Arc;

/// Claude Code asks at most this many questions at once, each with at most this many options.
const MAX_QUESTIONS: usize = 4;
const MAX_OPTIONS: usize = 4;
/// The most options one multi-answer card collects. A question has at most [`MAX_OPTIONS`] anyway.
pub(super) const MAX_SELECTED: usize = MAX_OPTIONS;

const APPROVE: &str = "allow";
const REJECT: &str = "deny";
/// The extra option of a multi-answer card that commits the options the person picked.
pub(super) const DONE: &str = "done";

/// Whether a card may take several selected options. `provider` is the hook's agent tag (`""` is
/// Claude Code); the card records the provider that asked, so a tap cannot answer another agent's
/// card. Claude's native `AskUserQuestion` and the shared `ask_user` tool on every other provider
/// both carry `multiSelect`, so the choice is driven by the card, not by the provider. Pure, tested.
pub(super) fn multi_answer_allowed(provider: &str, card: &Value) -> bool {
    card["answerAgent"].as_str() == Some(provider)
        && card["multiSelect"].as_bool() == Some(true)
        && card["options"]
            .as_array()
            .is_some_and(|options| options.iter().filter(|o| o["optionId"].as_str() != Some(DONE)).count() >= 2)
}

/// The verdict to print for the hook, or `None` when nobody answered and the terminal should ask.
pub(super) async fn ask(hub: &Arc<Hub>, ev: &Event) -> Option<Value> {
    if !matches!(ev.tool.as_str(), "AskUserQuestion" | "ExitPlanMode") {
        return None;
    }
    let (bot, config) = target(hub, ev)?;
    if ev.tool == "ExitPlanMode" {
        let data = plan_card(ev, &bot, &config.cwd);
        return match wait_card(hub, ev, &bot, &config, &data, "Approve the plan").await? {
            Decision::Allow => Some(json!({"permissionDecision": "allow", "updatedInput": ev.input})),
            Decision::Deny => {
                Some(json!({"permissionDecision": "deny", "reason": "The plan was rejected from Codync."}))
            }
            Decision::Pick(_) | Decision::Picks(_) => None,
        };
    }
    let questions = questions(&ev.input)?;
    let mut answers = Map::new();
    for question in &questions {
        let mut data = question_card(question, &bot, &config.cwd);
        // The card keeps the agent that asked, so the answer is only ever committed to that one.
        data["answerAgent"] = ev.agent.as_str().into();
        data["multiSelect"] = question.multi_select.into();
        let multi = multi_answer_allowed(&ev.agent, &data);
        if multi {
            data["options"]
                .as_array_mut()
                .expect("a question has options")
                .push(json!({"optionId": DONE, "name": "Done", "kind": "allow_once"}));
            data["selected"] = json!([]);
        }
        let decision = wait_card(hub, ev, &bot, &config, &data, &question.text).await?;
        let picks = match decision {
            Decision::Pick(index) => vec![index],
            Decision::Picks(picks) => picks,
            _ => return None,
        };
        let labels: Option<Vec<String>> =
            picks.iter().map(|i| question.options.get(*i).map(|o| o.label.clone())).collect();
        let labels = labels?;
        if labels.is_empty() {
            return None;
        }
        // A single-answer card keeps the string Claude's hook expects; a multi-answer card sends an
        // array of the labels the person picked, in tap order, capped by the card's own decision.
        let answer =
            if multi { Value::Array(labels.into_iter().map(Value::from).collect()) } else { labels[0].clone().into() };
        answers.insert(question.text.clone(), answer);
    }
    let mut input = ev.input.clone();
    input["answers"] = Value::Object(answers);
    Some(json!({"permissionDecision": "allow", "updatedInput": input}))
}

struct Question {
    text: String,
    header: String,
    options: Vec<Choice>,
    /// The question asked for more than one answer (Claude's `AskUserQuestion` or another provider's
    /// `ask_user` with `multiSelect`).
    multi_select: bool,
}

struct Choice {
    label: String,
    description: String,
}

/// The questions of a terminal agent's ask, when each has two to four options and there are at most four
/// questions. A `multiSelect` question is answerable too: its card collects several options up to
/// [`MAX_SELECTED`]. Anything else is left to the terminal.
fn questions(input: &Value) -> Option<Vec<Question>> {
    let list = input.get("questions")?.as_array().filter(|q| (1..=MAX_QUESTIONS).contains(&q.len()))?;
    list.iter()
        .map(|q| {
            let multi_select = q.get("multiSelect").and_then(Value::as_bool) == Some(true);
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
                multi_select,
            })
        })
        .collect()
}

/// A question as an approval card: the options keep their labels, their ids are their positions.
fn question_card(question: &Question, bot: &str, cwd: &str) -> Value {
    let detail: Vec<String> = question
        .options
        .iter()
        .filter(|o| !o.description.is_empty())
        .map(|o| format!("{}: {}", o.label, o.description))
        .collect();
    let options: Vec<Value> = question
        .options
        .iter()
        .enumerate()
        .map(|(i, o)| json!({"optionId": i.to_string(), "name": o.label, "kind": "allow_once"}))
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
        let card = question_card(&list[0], "bot", "/work");
        assert_eq!(card["title"], "Which database?");
        assert_eq!(card["options"][1], json!({"optionId": "1", "name": "Postgres", "kind": "allow_once"}));
        assert_eq!(card["detail"], "SQLite: one file\nPostgres: a server");
    }

    fn card(provider: &str, multi: bool) -> Value {
        let mut question = pick();
        question["multiSelect"] = multi.into();
        let list = questions(&input(&question)).expect("answerable");
        let mut card = question_card(&list[0], "bot", "/work");
        card["answerAgent"] = provider.into();
        card["multiSelect"] = multi.into();
        if multi_answer_allowed(provider, &card) {
            card["options"]
                .as_array_mut()
                .expect("options")
                .push(json!({"optionId": DONE, "name": "Done", "kind": "allow_once"}));
            card["selected"] = json!([]);
        }
        card
    }

    #[test]
    fn what_a_tap_cannot_answer_is_left_to_the_terminal() {
        let one_option = json!({"question": "Sure?", "options": [{"label": "Yes"}]});
        let too_many: Vec<Value> = (0..5).map(|_| pick()).collect();
        for input in [
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
    fn a_multi_select_question_is_answerable_on_every_provider() {
        let mut many = pick();
        many["multiSelect"] = true.into();
        let list = questions(&input(&many)).expect("a multi-select question is answerable");
        assert!(list[0].multi_select);
        for provider in ["", "gemini", "antigravity", "codex", "copilot", "my-tool"] {
            let card = card(provider, true);
            assert!(multi_answer_allowed(provider, &card), "{provider:?} can take several answers");
            assert_eq!(card["options"].as_array().map(Vec::len), Some(3), "the options plus Done");
        }
    }

    #[test]
    fn a_single_answer_card_keeps_claudes_shape() {
        let single = card("", false);
        assert!(!multi_answer_allowed("", &single));
        assert_eq!(single["options"].as_array().map(Vec::len), Some(2));
        assert!(single["selected"].is_null());
    }

    #[test]
    fn only_the_agent_the_card_belongs_to_can_take_several_answers() {
        assert!(!multi_answer_allowed("gemini", &card("", true)), "another agent's card is not ours");
        let mut malformed = card("", true);
        malformed["options"] = json!([{"optionId": "0"}]);
        assert!(!multi_answer_allowed("", &malformed), "one choice is not a multi-answer question");
    }

    #[test]
    fn up_to_four_questions_are_asked() {
        let four: Vec<Value> = (0..4).map(|_| pick()).collect();
        assert_eq!(questions(&json!({"questions": four})).map(|q| q.len()), Some(4));
    }
}
