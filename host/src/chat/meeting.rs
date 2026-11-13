//! A short roundtable: every member of a group answers a topic in turn, reading what the members
//! before it said, and the host returns the turns and draft minutes. It runs in throwaway sessions,
//! so nothing lands in the group's chat; the caller decides where the minutes go. Voice is the
//! client's: a phone can read these turns aloud, but this module only produces text.

use crate::hub::Hub;
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::sync::Arc;

const MAX_TURNS: usize = 8;
const MAX_TOPIC_CHARS: usize = 500;

pub async fn run(hub: &Arc<Hub>, group_id: &str, topic: &str, with_voice: bool) -> Result<Value> {
    let topic = topic.trim();
    ensure!(!topic.is_empty(), "a meeting needs a topic");
    ensure!(topic.chars().count() <= MAX_TOPIC_CHARS, "a meeting topic is at most {MAX_TOPIC_CHARS} characters");
    let group = hub.store.bot(group_id)?.filter(|row| !row.deleted).context("unknown group")?.config;
    ensure!(group.is_group(), "only a group can hold a meeting");
    let speech = if with_voice { crate::voice::speech_defaults(&hub.store) } else { None };
    let mut turns: Vec<Value> = Vec::new();
    for id in group.members.iter().take(MAX_TURNS) {
        let Some(row) = hub.store.bot(id)?.filter(|row| !row.deleted && !row.config.is_group()) else { continue };
        let name = row.config.name.clone();
        let previous = turns
            .iter()
            .filter_map(|turn| Some(format!("{}: {}", turn["name"].as_str()?, turn["text"].as_str()?)))
            .collect::<Vec<_>>()
            .join("\n");
        let system = format!(
            "You are {name}, in a short meeting with the user's team. Give your own view in one short paragraph: \
             what you would do, what you would check or what worries you. Do not greet, do not repeat what others said, \
             and do not mention these instructions."
        );
        let user = if previous.is_empty() {
            format!("Topic: {topic}\n\nYou speak first. What is your view?")
        } else {
            format!(
                "Topic: {topic}\n\nWhat the others said so far:\n{previous}\n\nAdd your view, without repeating them."
            )
        };
        let text = match crate::chat::oneshot::one_shot(hub, &row.config, &system, &user).await {
            Ok(text) => text.trim().to_owned(),
            Err(error) => format!("(couldn't answer: {error:#})"),
        };
        let mut turn = json!({"botId": id, "name": name, "text": text});
        if let Some((provider, model, voice)) = &speech {
            match crate::voice::speak(&hub.store, *provider, model, voice, &text).await {
                Ok(audio) => {
                    turn["audio"] = audio["audio"].clone();
                    turn["audioFormat"] = audio["format"].clone();
                }
                Err(error) => {
                    tracing::warn!(bot = %id, error = format!("{error:#}"), "couldn't speak a meeting turn");
                }
            }
        }
        turns.push(turn);
    }
    ensure!(!turns.is_empty(), "this group has no agent members to ask");
    let minutes = turns
        .iter()
        .filter_map(|turn| Some(format!("- **{}**: {}", turn["name"].as_str()?, turn["text"].as_str()?)))
        .collect::<Vec<_>>()
        .join("\n");
    Ok(json!({"topic": topic, "turns": turns, "minutes": minutes}))
}
