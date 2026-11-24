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
use crate::hub::{BotStatus, Hub};
use crate::store::{Entry, EntryKind};
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

/// The parts of a question card the overlap decision needs: its own id, the chat it sits in and
/// whether it is still open. A routine run's question and the agent's earlier question are two
/// cards — even when a person sees them in the same chat (OpenBot nightly triage-15338).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct CardRef<'a> {
    pub id: &'a str,
    pub chat: &'a str,
    pub open: bool,
}

/// Whether the posting or answer of `incoming` may close `existing`. Each card owns its lifecycle:
/// only the card the event names may close, so a routine question never expires the agent's earlier
/// one in the same chat, and answering either leaves the other waiting. A card in another chat is a
/// different card even if an id happens to match. A repeat answer replaces the named card with the
/// same closed status — the host's `waiting` sender is taken once, so it commits nothing twice. Pure.
pub(super) fn question_can_close(existing: CardRef<'_>, incoming: CardRef<'_>) -> bool {
    existing.id == incoming.id && existing.chat == incoming.chat
}

fn card_ref(entry: &Entry) -> CardRef<'_> {
    CardRef {
        id: &entry.id,
        chat: &entry.bot_id,
        open: matches!(entry.data["status"].as_str(), Some("pending" | "expired")),
    }
}

/// The still-open question cards in the same main chat, oldest first. Routine questions reach the
/// same card path as an agent's own, so a card here may have been posted by either.
fn open_questions(hub: &Hub, bot: &str) -> Vec<Entry> {
    hub.store
        .entries_since(0, 500)
        .unwrap_or_default()
        .into_iter()
        .filter(|e| {
            e.bot_id == bot
                && e.thread_id.is_none()
                && e.kind == EntryKind::Permission.as_str()
                && e.data["question"] == true
                && card_ref(e).open
        })
        .collect()
}

/// Called when a card is posted. An earlier question in the chat is not closed by this one: it
/// stays in the card list and the bot still needs input, with a hint that another card waits.
pub(super) fn on_post(hub: &Hub, entry: &Entry, title: &str) {
    let incoming = card_ref(entry);
    // Only this card may be taken out of the open set; a routine question in the same chat stays.
    let earlier: Vec<Entry> =
        open_questions(hub, &entry.bot_id).into_iter().filter(|e| !question_can_close(card_ref(e), incoming)).collect();
    hub.set_runtime(&entry.bot_id, |r| {
        r.status = BotStatus::NeedsInput;
        r.activity = if earlier.is_empty() {
            format!("Needs approval: {title}")
        } else {
            format!("Needs approval: {title} (+{} earlier)", earlier.len())
        };
    });
}

/// Called when a card settles or its hook goes away. If another question is still open in the chat
/// it takes over the "needs input" state, so the earlier card reappears as answerable; the `fallback`
/// is the activity to show when this was the last card.
pub(super) fn after_card(hub: &Hub, bot: &str, closed: &str, fallback: &str) {
    let incoming = CardRef { id: closed, chat: bot, open: true };
    // Only the settled card may leave the open set; its siblings in the chat stay answerable.
    let remaining: Vec<Entry> =
        open_questions(hub, bot).into_iter().filter(|e| !question_can_close(card_ref(e), incoming)).collect();
    hub.set_runtime(bot, |r| {
        if let Some(next) = remaining.first() {
            r.status = BotStatus::NeedsInput;
            r.activity = format!("Needs approval: {}", next.data["title"].as_str().unwrap_or("a question"));
        } else {
            r.status = BotStatus::Working;
            r.activity = fallback.to_owned();
        }
    });
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

    fn card(id: &str, chat: &str, open: bool) -> CardRef<'_> {
        CardRef { id, chat, open }
    }

    #[test]
    fn two_overlapping_questions_in_one_chat_each_keep_their_own_lifecycle() {
        let agent = card("agent-q", "bot", true);
        let routine = card("routine-q", "bot", true);
        assert!(!question_can_close(agent, routine), "a routine question cannot close the agent's");
        assert!(!question_can_close(routine, agent), "nor the other way round");
        assert!(question_can_close(agent, agent), "a card may still close itself");
        assert!(question_can_close(routine, routine));
    }

    #[test]
    fn answering_either_card_first_leaves_the_other_waiting() {
        let agent = card("agent-q", "bot", true);
        let routine = card("routine-q", "bot", true);
        // The routine is answered first: the agent's earlier question is untouched and answerable.
        assert!(question_can_close(routine, routine));
        assert!(!question_can_close(agent, card("routine-q", "bot", false)));
        // The agent answers first: the routine question is untouched instead.
        assert!(question_can_close(agent, agent));
        assert!(!question_can_close(routine, card("agent-q", "bot", false)));
        // A second answer names the same card again, but its status is already closed.
        assert!(question_can_close(card("routine-q", "bot", false), card("routine-q", "bot", false)));
        assert!(!card_ref(&entry("routine-q", "bot", "answered")).open);
    }

    #[test]
    fn a_card_in_another_chat_is_never_closed_by_a_post_or_an_answer() {
        let here = card("q", "bot-a", true);
        let there = card("q", "bot-b", true);
        assert!(!question_can_close(here, there));
        assert!(!question_can_close(there, here));
        assert!(question_can_close(here, here));
    }

    fn entry(id: &str, bot: &str, status: &str) -> Entry {
        Entry {
            id: id.into(),
            seq: 1,
            bot_id: bot.into(),
            rev: 1,
            kind: EntryKind::Permission.as_str().into(),
            turn: 1,
            data: json!({"status": status, "question": true, "title": "Which database?"}),
            created_at: 1,
            updated_at: 1,
            thread_id: None,
        }
    }

    #[test]
    fn an_expired_routine_question_still_cannot_close_the_agent_question() {
        let agent = card("agent-q", "bot", true);
        let expired = card_ref(&entry("routine-q", "bot", "expired"));
        assert!(expired.open, "expired is still an open, answerable card");
        assert!(!question_can_close(agent, expired));
        assert!(question_can_close(expired, expired));
        let answered = card_ref(&entry("routine-q", "bot", "answered"));
        assert!(!answered.open, "an answered card cannot be closed twice");
        assert!(!question_can_close(agent, answered));
    }
}
