//! A new bot's defaults, for a client's `createBot` and a bot's `create_bot` (`chat::manage`) alike.

use crate::hub::{BotStatus, Hub};
use crate::market;
use crate::store::{BotConfig, EntryKind, Lane};
use anyhow::{Context, Result, anyhow};
use serde_json::{Value, json};
use std::sync::Arc;

/// The bot or group `b` asks for, with the defaults for what the caller left out.
pub async fn with_defaults(hub: &Arc<Hub>, mut b: Value) -> Result<BotConfig> {
    b["id"] = "".into();
    // Only the host makes the lead (`chat::lead`).
    b["lead"] = false.into();
    // Connectors are on for a new bot unless the client picked them.
    if b["connectors"].is_null() && b["kind"] != "group" {
        b["connectors"] = default_connectors(hub).await?;
    }
    // Skills too: every installed one is on unless the client picked them, but a bot's skill stays with that bot.
    if b["skills"].is_null() && b["kind"] != "group" {
        b["skills"] = json!(market::Installed::Skills.ids(&hub.store)?);
    }
    // And the agent, when the client names none.
    if b["backend"].as_str().is_none_or(str::is_empty) && b["kind"] != "group" {
        let agents = tokio::task::spawn_blocking(super::backends::list).await?;
        b["backend"] = default_backend(&agents).into();
    }
    serde_json::from_value(b).context("invalid bot")
}

/// The question a new agent asks on its own, and the three first tasks it offers. OpenBot 0.66.0:
/// a new agent opens with a question of its own instead of an empty chat. Pure, so the wording for
/// each language can be tested without a host.
pub struct WelcomeQuestion {
    pub title: String,
    pub header: String,
    pub tasks: [String; 3],
}

pub fn welcome_question(lang: &str) -> WelcomeQuestion {
    // A region tag ("es-ES", "zh_Hant") still reads as the base language.
    let lang = lang.trim().to_ascii_lowercase();
    let lang = lang.split(['-', '_']).next().unwrap_or("en");
    let (title, header, tasks) = match lang {
        "es" => (
            "¿Por dónde empezamos?",
            "Primera tarea",
            ["Resumir un documento", "Redactar un mensaje", "Crear una rutina"],
        ),
        "zh" => ("我们先从哪件事开始？", "第一个任务", ["总结一份文档", "写一则消息", "设置一个定时任务"]),
        _ => (
            "What should we work on first?",
            "First task",
            ["Summarize a document", "Draft a message", "Set up a routine"],
        ),
    };
    WelcomeQuestion { title: title.to_owned(), header: header.to_owned(), tasks: tasks.map(str::to_owned) }
}

/// The same question as a card: the three tasks are its options, so answering it uses Codync's
/// ordinary question-card path. `welcomeTasks` keeps the prompt each option stands for.
fn welcome_card(cfg: &BotConfig, question: &WelcomeQuestion) -> Value {
    json!({
        "title": question.title,
        "header": question.header,
        "toolKind": "question",
        "question": true,
        "welcomeQuestion": true,
        "command": Value::Null,
        "detail": "",
        "diffs": [],
        "cwd": cfg.cwd,
        "options": question.tasks.iter().enumerate().map(|(i, task)| {
            json!({"optionId": i.to_string(), "name": task, "kind": "allow_once"})
        }).collect::<Vec<_>>(),
        "status": "pending",
        "selected": Value::Null,
        "author": cfg.id,
        "welcomeTasks": question.tasks,
    })
}

/// Posts the new agent's welcome question once, beside the bot it just created.
pub fn post_welcome(hub: &Hub, cfg: &BotConfig, lang: &str) -> Result<()> {
    let question = welcome_question(lang);
    let data = welcome_card(cfg, &question);
    hub.add_entry(&Lane::main(&cfg.id), EntryKind::Permission, hub.store.max_turn(&cfg.id), &data)
        .ok_or_else(|| anyhow!("couldn't post the welcome question"))?;
    hub.set_runtime(&cfg.id, |r| {
        r.status = BotStatus::NeedsInput;
        r.activity = format!("Needs approval: {}", question.title);
    });
    Ok(())
}

/// The prompt behind a tap on a welcome card, when the tap names one of its three tasks. Pure.
pub fn welcome_task(data: &Value, option_id: Option<&str>) -> Option<String> {
    if data["welcomeQuestion"] != true {
        return None;
    }
    let index = option_id?.parse::<usize>().ok()?;
    data["welcomeTasks"].as_array()?.get(index)?.as_str().map(str::to_owned)
}

/// Every connector a new bot gets. With none saved the credential store stays shut, which is what keeps the
/// lead bot of a new computer from opening it at start-up.
async fn default_connectors(hub: &Arc<Hub>) -> Result<Value> {
    if !market::connectors_saved(&hub.store)? {
        return Ok(json!([]));
    }
    // Without a credential store (headless Linux) no connector can run, so the bot starts with none.
    Ok(match market::vault::unlock(hub.clone()).await {
        Ok(()) => json!(market::connector_ids(&hub.store)?),
        Err(e) => {
            tracing::warn!(error = format!("{e:#}"), "credential store unavailable; new bot starts without connectors");
            json!([])
        }
    })
}

/// A new bot's agent (`agents` as `backends::list` gives them): Claude when it runs here, else the
/// first one Codync knows that does. A registry agent nobody chose is never the default.
fn default_backend(agents: &[Value]) -> &str {
    let runnable = || agents.iter().filter(|a| a["available"] == true);
    if runnable().any(|a| a["id"] == "claude") {
        return "claude";
    }
    runnable().find(|a| a["curated"] == true).and_then(|a| a["id"].as_str()).unwrap_or("claude")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_agent_welcomes_you_with_three_tasks_in_your_language() {
        let english = welcome_question("en-US");
        assert_eq!(english.tasks.len(), 3);
        assert!(english.title.contains("first"), "{}", english.title);
        let spanish = welcome_question("es");
        assert_eq!(spanish.title, "¿Por dónde empezamos?");
        assert_eq!(spanish.tasks[0], "Resumir un documento");
        assert_ne!(english.tasks, spanish.tasks);
        assert_eq!(welcome_question("zh_Hant").tasks[2], "设置一个定时任务");
        assert_eq!(welcome_question("").title, english.title, "an unknown language reads English");
    }

    #[test]
    fn a_welcome_card_answer_is_the_task_the_person_picked() {
        let question = welcome_question("es");
        let cfg: BotConfig = serde_json::from_value(json!({"id": "b", "name": "B", "backend": "claude"})).expect("bot");
        let mut card = welcome_card(&cfg, &question);
        assert_eq!(card["title"], question.title);
        assert_eq!(card["question"], true);
        assert_eq!(card["options"][1]["name"], question.tasks[1]);
        assert_eq!(welcome_task(&card, Some("1")), Some(question.tasks[1].clone()));
        assert_eq!(welcome_task(&card, Some("9")), None, "an option the card didn't show is no task");
        assert_eq!(welcome_task(&card, None), None);
        card["welcomeQuestion"].take();
        assert_eq!(welcome_task(&card, Some("0")), None, "an ordinary card isn't a welcome");
    }

    #[test]
    fn the_default_agent_is_claude_then_the_first_known_one() {
        assert_eq!(default_backend(&[]), "claude", "nothing installed");
        let mut agents = vec![
            json!({"id": "goose", "available": true, "curated": false}),
            json!({"id": "codex", "available": true, "curated": true}),
            json!({"id": "claude", "available": false, "curated": true}),
        ];
        assert_eq!(default_backend(&agents), "codex");
        agents[2]["available"] = true.into();
        assert_eq!(default_backend(&agents), "claude");
    }
}
