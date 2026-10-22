//! Teaching a bot a workflow: the user walks it through a task once, then asks it to remember
//! how. `recorded_steps` reads back what happened in this conversation (what the user said and
//! the tool steps the bot took, in order), and `save_skill` keeps the bot's write-up as a skill
//! of its own (`market/skills.rs`), turned on for that bot only: one bot's instructions don't
//! reach the others unless the user turns them on there.
//!
//! Adapted from rakazo's `teach-playbook.ts` and `teach-recording.ts` (Apache-2.0); no code
//! copied. Rakazo records the user's own clicks on the screen; here the steps are the bot's,
//! taken while the user guided it.

use crate::hub::Hub;
use crate::store::{Entry, EntryKind, Lane};
use anyhow::{Result, anyhow, bail};
use serde_json::{Value, json};
use std::sync::Arc;

pub const TOOLS: [&str; 2] = ["recorded_steps", "save_skill"];
/// Transcript entries read back, newest first.
const READ: i64 = 500;
/// Steps returned, the latest ones.
const MAX_STEPS: usize = 200;
/// Longest piece of a user message in a step.
const MAX_SAID: usize = 400;
const MAX_SKILL_BYTES: usize = 32 * 1024;

pub fn tools() -> [Value; 2] {
    [
        json!({
            "name": "recorded_steps",
            "description": "When the user has walked you through a task and asks you to remember how: what they said and the tool steps you took in this conversation, oldest first. Write it up with save_skill.",
            "inputSchema": {"type": "object", "properties": {}},
            "annotations": {"readOnlyHint": true},
        }),
        json!({
            "name": "save_skill",
            "description": "Save a workflow the user taught you as a skill of yours: when to use it (description) and the steps, with the commands, files and checks that worked (instructions, Markdown). Only when the user asks. It's on for you; the user can turn it on for other bots.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "description": {"type": "string", "description": "When to use it, in one sentence."},
                    "instructions": {"type": "string"},
                },
                "required": ["name", "description", "instructions"],
            },
            "annotations": {"readOnlyHint": false},
        }),
    ]
}

pub async fn call(hub: &Arc<Hub>, bot: &str, name: &str, args: &Value) -> Result<Value> {
    match name {
        "recorded_steps" => steps(hub, bot),
        "save_skill" => save(hub, bot, args).await,
        _ => bail!("unknown teach tool: {name}"),
    }
}

/// One entry as a step, when it's one: the user's words, or a tool step of this bot's.
fn step(e: &Entry, bot: &str) -> Option<Value> {
    if e.kind == EntryKind::User.as_str() && e.data["from"] != "host" {
        let said: String = e.data["text"].as_str()?.chars().take(MAX_SAID).collect();
        return Some(json!({"user": said}));
    }
    // In a group, only this bot's own steps.
    let mine = e.data["author"].as_str().is_none_or(|author| author == bot);
    (e.kind == EntryKind::Tool.as_str() && mine).then(|| {
        json!({"tool": e.data["title"], "kind": e.data["toolKind"], "status": e.data["status"],
            "files": e.data["locations"]})
    })
}

fn steps(hub: &Hub, bot: &str) -> Result<Value> {
    let lane = hub.runtime(bot).lane.unwrap_or_else(|| Lane::main(bot));
    let mut entries = match &lane.thread {
        Some(root) => hub.store.thread(&lane.chat, root, READ)?,
        None => hub.store.history(&lane.chat, i64::MAX, READ)?,
    };
    entries.sort_by_key(|e| e.seq);
    let all: Vec<Value> = entries.iter().filter_map(|e| step(e, bot)).collect();
    let skip = all.len().saturating_sub(MAX_STEPS);
    Ok(json!({"steps": all[skip..].to_vec(), "earlierLeftOut": skip}))
}

async fn save(hub: &Arc<Hub>, bot: &str, args: &Value) -> Result<Value> {
    let text = |key: &str| {
        args[key].as_str().map(str::trim).filter(|s| !s.is_empty()).ok_or_else(|| anyhow!("{key} is required"))
    };
    let (name, description, instructions) = (text("name")?, text("description")?, text("instructions")?);
    if instructions.len() > MAX_SKILL_BYTES {
        bail!("instructions must be at most {} KB", MAX_SKILL_BYTES / 1024);
    }
    let body = json!({"name": name, "description": description, "instructions": instructions});
    let (hub, bot_id) = (hub.clone(), bot.to_owned());
    tokio::task::spawn_blocking(move || {
        let skill = crate::market::add_custom_skill(&hub.store, &body)?;
        let id = skill["id"].as_str().ok_or_else(|| anyhow!("the skill has no id"))?.to_owned();
        let row = hub.store.bot(&bot_id)?.ok_or_else(|| anyhow!("unknown bot"))?;
        let mut on = row.config.skills;
        on.push(id.clone());
        hub.update_bot(&json!({"id": bot_id, "skills": on}))?;
        let notice = json!({"text": format!("Saved the skill {} from this conversation", skill["name"].as_str().unwrap_or(&id))});
        hub.add_entry(&Lane::main(&bot_id), EntryKind::Notice, hub.store.max_turn(&bot_id), &notice);
        anyhow::Ok(json!({"skill": skill}))
    })
    .await?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(kind: EntryKind, seq: i64, data: Value) -> Entry {
        Entry {
            id: seq.to_string(),
            seq,
            bot_id: "a".into(),
            thread_id: None,
            rev: 0,
            kind: kind.as_str().into(),
            turn: 0,
            data,
            created_at: 0,
            updated_at: 0,
        }
    }

    #[test]
    fn steps_are_the_users_words_and_this_bots_tools() {
        let said = entry(EntryKind::User, 1, json!({"text": "Deploy it like this"}));
        let note = entry(EntryKind::User, 2, json!({"text": "[Connection setup completed]", "from": "host"}));
        let mine =
            entry(EntryKind::Tool, 3, json!({"title": "npm run build", "toolKind": "execute", "status": "completed"}));
        let theirs = entry(EntryKind::Tool, 4, json!({"title": "ls", "author": "b"}));
        let reply = entry(EntryKind::Agent, 5, json!({"text": "Done"}));
        assert_eq!(step(&said, "a").unwrap()["user"], "Deploy it like this");
        assert!(step(&note, "a").is_none(), "the host's own notes aren't the user's words");
        assert_eq!(step(&mine, "a").unwrap()["tool"], "npm run build");
        assert!(step(&theirs, "a").is_none(), "another bot's step in a group");
        assert!(step(&reply, "a").is_none());
    }
}
