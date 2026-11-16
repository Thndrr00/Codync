//! A channel's memory: one `MEMORY.md` layer a group keeps beside its members' own and the shared one. Every
//! member reads it in the room, and a member's own chats never do. `channel_remember` saves a line and
//! `channel_forget_memory` takes one out; with the channel's "Keep memories in this channel" on, a member's own
//! memory writes in the room go here instead of into its own layer.

use super::api;
use super::files::{self, Writer};
use crate::hub::Hub;
use crate::store::BotConfig;
use anyhow::{Context, Result, anyhow, bail};
use codync_mem::files::{self as mf, Author, Cause, MAIN};
use serde_json::{Value, json};

/// The most one channel memory holds, in characters (the prompt states the limit, so a refusal can too).
pub const MAX_CHARS: usize = 500;

/// The group the bot is answering in now, when it is one of its members and still exists.
pub fn current(hub: &Hub, bot: &str) -> Result<BotConfig> {
    let lane = hub.runtime(bot).lane.filter(|lane| lane.chat != bot);
    let Some(lane) = lane else {
        bail!("This works in a channel: call it while answering one, or keep a memory of your own with edit_memory");
    };
    let group = hub
        .store
        .bot(&lane.chat)?
        .filter(|row| !row.deleted)
        .map(|row| row.config)
        .filter(BotConfig::is_group)
        .ok_or_else(|| anyhow!("That channel is gone."))?;
    if !group.members.iter().any(|member| member == bot) {
        bail!("You're not a member of {}: ask the user to add you to it, then save the memory there.", group.name);
    }
    Ok(group)
}

/// The channel layer a member's own memory write goes to, when the channel keeps its memories.
pub fn write_layer(hub: &Hub, cfg: &BotConfig) -> Option<String> {
    current(hub, &cfg.id).ok().filter(|group| group.keep_memories).map(|group| group.id)
}

/// Saves one channel memory, or corrects the one with the same text: the channel never holds it twice.
pub fn remember(hub: &Hub, cfg: &BotConfig, raw: &str) -> Result<Value> {
    let group = current(hub, &cfg.id)?;
    files::scan_channel(hub, &group.id);
    let text = bare(raw);
    if text.is_empty() {
        bail!("text is required: the memory to keep, in one short line");
    }
    let chars = text.chars().count();
    if chars > MAX_CHARS {
        bail!(
            "A channel memory holds at most {MAX_CHARS} characters, and this one has {chars}: save the point in \
             fewer words."
        );
    }
    let layer = Some(group.id.as_str());
    let before = files::read(layer, MAIN)?;
    let mut lines: Vec<String> = before.as_deref().unwrap_or_default().lines().map(str::to_owned).collect();
    let wanted = format!("- {text}");
    let corrected = match lines.iter_mut().find(|line| same(line, &wanted)) {
        Some(existing) if *existing == wanted => {
            return Ok(json!({"status": "kept", "channelId": group.id, "channel": group.name}));
        }
        Some(existing) => {
            *existing = wanted;
            true
        }
        None => {
            lines.push(wanted);
            false
        }
    };
    let after = body(&lines);
    let cap = mf::cap(layer);
    let used = codync_mem::tokens::estimate(&after);
    if used > cap {
        bail!(
            "This channel's memory holds at most {cap} tokens and is full: forget a memory first, or keep this \
             point shorter."
        );
    }
    let expected = before.as_deref().map(mf::hash);
    let writer = Writer { author: Author::Bot, bot: Some(cfg), cause: Cause::Edit, message: None, channel: true };
    files::write(hub, layer, MAIN, Some(&after), expected.as_deref(), writer)?;
    Ok(json!({
        "status": "saved", "corrected": corrected, "channelId": group.id, "channel": group.name,
        "used": used, "cap": cap,
    }))
}

/// Takes one channel memory out, matched by its text (exactly, or ignoring case when typed from memory).
pub fn forget(hub: &Hub, cfg: &BotConfig, raw: &str) -> Result<Value> {
    let group = current(hub, &cfg.id)?;
    files::scan_channel(hub, &group.id);
    let text = raw.trim();
    if text.is_empty() {
        bail!("text is required: the memory to forget, exactly as the channel lists it");
    }
    let layer = Some(group.id.as_str());
    let before = files::read(layer, MAIN)?.context("This channel has no memories yet.")?;
    let mut lines: Vec<String> = before.lines().map(str::to_owned).collect();
    let at = lines
        .iter()
        .position(|line| bare(line) == bare(text))
        .or_else(|| lines.iter().position(|line| bare(line).eq_ignore_ascii_case(bare(text))))
        .ok_or_else(|| {
            anyhow!(
                "No channel memory matches that text: give one exactly as the channel lists it, or leave it as it is."
            )
        })?;
    lines.remove(at);
    let after = body(&lines);
    let content = (!after.is_empty()).then_some(after.as_str());
    let expected = before.as_deref().map(mf::hash);
    let writer = Writer { author: Author::Bot, bot: Some(cfg), cause: Cause::Delete, message: None, channel: true };
    files::write(hub, layer, MAIN, content, expected.as_deref(), writer)?;
    Ok(json!({"status": "forgotten", "channelId": group.id, "channel": group.name}))
}

/// The channel's memories for a member's prompt, only when they changed since it last saw them (or in a new
/// session, whose context no longer holds them).
pub fn block(hub: &Hub, group: &BotConfig, member: &str) -> Result<Option<String>> {
    files::scan_channel(hub, &group.id);
    let facts = hub.store.memory_read(|c| mf::facts_of(c, Some(group.id.as_str())))?;
    if facts.is_empty() {
        return Ok(None);
    }
    let revision = super::current_revision(&hub.store, &group.id);
    let session = hub.runtime(member).session.unwrap_or_default();
    let key = seen_key(&group.id, member, &session);
    if !revision.is_empty() && hub.store.kv_get(&key).as_deref() == Some(revision.as_str()) {
        return Ok(None);
    }
    if let Err(error) = hub.store.kv_set(&key, &revision) {
        tracing::warn!(group = %group.id, error = format!("{error:#}"), "couldn't note a member's channel memory");
    }
    let mut out =
        vec!["Memory kept in this channel (every member reads it here; your other chats don't have it):".to_owned()];
    out.extend(facts.iter().map(|fact| format!("- {}", fact.text)));
    Ok(Some(out.join("\n")))
}

/// The channel's memories as its settings show them, read-only.
pub fn view(hub: &Hub, group: &str) -> Result<Value> {
    files::scan_channel(hub, group);
    let (facts, content) = hub
        .store
        .memory_read(|c| Ok((mf::facts_of(c, Some(group))?, mf::current(c, Some(group), MAIN)?.unwrap_or_default())))?;
    let mut seen = std::collections::HashSet::new();
    let entries: Vec<Value> = facts
        .iter()
        .filter(|fact| seen.insert(mf::fact_id(&fact.text)))
        .map(|fact| api::entry(Some(group), fact))
        .collect();
    Ok(json!({"entries": entries, "used": codync_mem::tokens::estimate(&content), "cap": mf::cap(Some(group))}))
}

/// Where a member last saw the channel's memory: per channel, member and session.
fn seen_key(group: &str, member: &str, session: &str) -> String {
    format!("memory.channel.{group}.{member}.{session}")
}

/// The channel's `MEMORY.md` from its lines, ending in one newline (empty when none are left).
fn body(lines: &[String]) -> String {
    let joined = lines.join("\n");
    let trimmed = joined.trim_end();
    if trimmed.is_empty() { String::new() } else { format!("{trimmed}\n") }
}

/// Two memory lines with the same text, list marker aside (a second save corrects the first, not a duplicate).
fn same(line: &str, other: &str) -> bool {
    bare(line).to_lowercase() == bare(other).to_lowercase()
}

/// A memory line's text, without its indentation or list marker.
fn bare(line: &str) -> &str {
    line.trim().trim_start_matches("- ").trim()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::identity::Identity;
    use crate::store::{Lane, Store};

    fn setup(name: &str) -> (std::sync::Arc<Hub>, std::path::PathBuf, BotConfig, BotConfig) {
        let dir = std::env::temp_dir().join(format!("codync-channel-{}-{name}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::open(&dir.join("t.db")).unwrap();
        let member: BotConfig = serde_json::from_value(json!({
            "id": "5f0c7a52-0000-4000-8000-0000000000aa", "name": "Ann", "backend": "claude", "cwd": "/tmp",
        }))
        .unwrap();
        let group: BotConfig = serde_json::from_value(json!({
            "id": "5f0c7a52-0000-4000-8000-0000000000bb", "kind": "group", "name": "Ship room",
            "members": [member.id], "keepMemories": true,
        }))
        .unwrap();
        store.save_bot(&member).unwrap();
        store.save_bot(&group).unwrap();
        let hub = Hub::new(store, "test".into(), Identity::load_or_create(&dir).unwrap(), "token".into(), 0);
        hub.set_runtime(&member.id, |r| {
            r.lane = Some(Lane::main(&group.id));
            r.session = Some("s1".into());
        });
        (hub, dir, member, group)
    }

    #[test]
    fn a_member_saves_and_reads_a_channel_memory_and_its_own_chat_does_not() {
        let (hub, dir, member, group) = setup("save");
        remember(&hub, &member, "The release ships on Fridays").unwrap();
        let shown = block(&hub, &group, &member.id).unwrap().expect("a member reads it");
        assert!(shown.contains("- The release ships on Fridays"), "{shown}");
        assert!(block(&hub, &group, &member.id).unwrap().is_none(), "told once per session");

        // In its own chat the member's turn has no channel memory.
        hub.set_runtime(&member.id, |r| r.lane = Some(Lane::main(&member.id)));
        assert!(current(&hub, &member.id).is_err());
        files::remove_bot(&group.id);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn the_same_text_again_corrects_the_memory_and_the_cap_is_stated() {
        let (hub, dir, member, group) = setup("cap");
        remember(&hub, &member, "Ships on Fridays").unwrap();
        remember(&hub, &member, "ships on fridays").unwrap();
        let saved = files::read(Some(group.id.as_str()), MAIN).unwrap().unwrap_or_default();
        assert_eq!(saved.matches("Fridays").count() + saved.matches("fridays").count(), 1, "{saved}");

        let long = "x".repeat(MAX_CHARS + 1);
        let error = remember(&hub, &member, &long).unwrap_err().to_string();
        assert!(error.contains(&MAX_CHARS.to_string()), "{error}");
        files::remove_bot(&group.id);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn forgetting_takes_one_memory_out() {
        let (hub, dir, member, group) = setup("forget");
        remember(&hub, &member, "Ships on Fridays").unwrap();
        remember(&hub, &member, "The leads are Cara and Ben").unwrap();
        forget(&hub, &member, "ships on fridays").unwrap();
        let saved = files::read(Some(group.id.as_str()), MAIN).unwrap().unwrap_or_default();
        assert!(!saved.to_lowercase().contains("fridays"), "{saved}");
        assert!(saved.contains("Cara and Ben"), "{saved}");
        assert!(forget(&hub, &member, "not there").is_err());
        files::remove_bot(&group.id);
        std::fs::remove_dir_all(dir).ok();
    }
}
