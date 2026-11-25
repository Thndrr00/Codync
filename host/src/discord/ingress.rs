//! One Discord message: who may write, whether the bot answers, and the prompt that is queued.
//!
//! A bound channel answers a mention of the bot, a message in a conversation it already had, and
//! (for a bound person) a direct message. Everything else is dropped without error, so the bot can
//! sit in a busy server.

use super::{Arc, Config, Context, Hub, Result, Value, json};
use sha2::{Digest, Sha256};

pub(super) fn stable_id(parts: &[&str]) -> String {
    let mut digest = Sha256::new();
    for part in parts {
        digest.update((part.len() as u64).to_le_bytes());
        digest.update(part.as_bytes());
    }
    format!("discord-{}", crate::chat::files::digest_hex(&digest.finalize()))
}

pub(super) fn accept(hub: &Arc<Hub>, c: &Config, message: &Value) -> Result<()> {
    if message["author"]["bot"] == true {
        return Ok(());
    }
    let (Some(id), Some(channel)) = (message["id"].as_str(), message["channel_id"].as_str()) else {
        return Ok(());
    };
    if !valid_id(id) || !valid_id(channel) {
        return Ok(());
    }
    let Some(author) = message["author"]["id"].as_str().filter(|u| valid_id(u)) else { return Ok(()) };
    let direct = message["guild_id"].is_null();
    let allowed = |b: &super::Binding| b.allowed_users.iter().any(|u| u == author);
    let Some(binding) = (if direct {
        // A direct message has no channel to bind: any bound person may use it, with the first bot
        // that lets them in.
        c.bindings.iter().find(|b| allowed(b))
    } else if message["guild_id"] == c.guild_id.as_str() {
        c.bindings.iter().find(|b| b.channel_id == channel && allowed(b))
    } else {
        None
    }) else {
        return Ok(());
    };
    hub.store
        .bot(&binding.bot_id)?
        .filter(|bot| !bot.deleted && !bot.config.is_group())
        .context("Bound bot unavailable")?;
    let mention =
        message["mentions"].as_array().is_some_and(|list| list.iter().any(|m| m["id"] == c.bot_user_id.as_str()));
    let thread = hub.store.discord_thread_exists(&c.generation, channel)?;
    if !mention && !direct && !thread {
        return Ok(());
    }
    let text = strip_mentions(message["content"].as_str().unwrap_or_default(), &c.bot_user_id);
    let audio = crate::channels::voice_notes::Platform::Discord.attachment(message);
    let has_audio = crate::channels::voice_notes::Platform::Discord.has_audio(message);
    if (text.is_empty() && !has_audio) || text.len() > 32 * 1024 {
        return Ok(());
    }
    // The server's facts reach every conversation here, but never the bot's own chat.
    let prompt = crate::chat::memory::messaging::with_facts(&hub.store, "discord", &c.guild_id, prompt(message, &text));
    let prompt = if has_audio && audio.is_none() { crate::channels::voice_notes::unavailable(&prompt) } else { prompt };
    // The roles a workflow trigger matches come from the guild member, never from the message text.
    let roles = member_roles(message);
    let event = json!({
        "source": "discord", "event": if mention {"message.mention"} else {"message.created"},
        "connection": c.guild_id, "channel": channel, "channelId": channel,
        "user": author, "author": author, "text": text, "messageId": id, "roles": roles,
        "thread": message["thread"].is_object(), "parentChannel": message["thread"]["parent_id"],
        "url": format!("https://discord.com/channels/{}/{channel}/{id}", c.guild_id),
    });
    // A mention goes to workflow trigger nodes by connection, channel, roles and text; the chat
    // receive path also keeps the older generic matcher working. Other messages use that matcher.
    let all_skipped = if mention {
        hub.routines.receive_chat_event(&hub.store, &event, Some(id.to_owned())).all_skipped()
    } else {
        hub.routines.receive_event(&hub.store, &event, Some(id.to_owned()));
        false
    };
    // Every node that matched skipped the mention: leave the platform's failed reaction on it.
    if all_skipped {
        let (hub, channel, message) = (hub.clone(), channel.to_owned(), id.to_owned());
        tokio::spawn(async move {
            if let Err(error) = super::react(&hub.store, &channel, &message, "❌").await {
                tracing::warn!(error = format!("{error:#}"), "couldn't mark a skipped Discord workflow mention");
            }
        });
    }
    let incoming = crate::store::discord::DiscordIncoming {
        event: id.into(),
        generation: c.generation.clone(),
        bot: binding.bot_id.clone(),
        channel: channel.into(),
        entry: stable_id(&[&c.generation, channel, id]),
        text: prompt,
        user: author.into(),
        message: id.into(),
        audio,
    };
    hub.store.enqueue_discord(&incoming)?;
    let root = stable_id(&[&c.generation, channel, "root"]);
    hub.store.map_discord_thread(&incoming, &root)?;
    hub.store.record_channel_activity(&crate::store::ChannelActivity {
        id: format!("discord-{}", incoming.entry),
        source: "discord".into(),
        bot_id: incoming.bot.clone(),
        thread_id: Some(root),
        channel: incoming.channel.clone(),
        mentioned: mention,
        at: crate::store::now_ms(),
    })?;
    Ok(())
}

/// The roles a workflow trigger checks, bounded so a guild member cannot grow the event without
/// limit. Pure so the event's shape is testable without a gateway.
fn member_roles(message: &Value) -> Vec<String> {
    message["member"]["roles"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|role| role.as_str())
        .take(50)
        .map(str::to_owned)
        .collect()
}

/// The bot's own mention is not part of what was said.
fn strip_mentions(content: &str, bot: &str) -> String {
    content.replace(&format!("<@{bot}>"), "").replace(&format!("<@!{bot}>"), "").trim().to_owned()
}

/// What the bot is given: the message, then the names of the files that came with it.
fn prompt(message: &Value, text: &str) -> String {
    let names: Vec<&str> = message["attachments"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|a| a["filename"].as_str().filter(|n| !n.is_empty() && n.len() <= 255))
        .take(10)
        .collect();
    // The agent answering a Discord contact is the one posting here, so it must not promise to pass
    // the message on (OpenBot 0.78.0).
    let body = if names.is_empty() {
        text.to_owned()
    } else {
        format!("{text}\n\n[Attached, not downloaded: {}]", names.join(", "))
    };
    format!("{body}\n\n{}", crate::chat::team::channel_returns::external_reply_rule("Discord"))
}

fn valid_id(value: &str) -> bool {
    (1..=32).contains(&value.len()) && value.bytes().all(|b| b.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workflow_mentions_carry_the_members_roles() {
        let message = json!({"member": {"roles": ["admin", "mod", 42]}});
        assert_eq!(member_roles(&message), ["admin", "mod"]);
        assert!(member_roles(&json!({})).is_empty());
        let many = json!({"member": {"roles": (0..60).map(|i| i.to_string()).collect::<Vec<_>>()}});
        assert_eq!(member_roles(&many).len(), 50, "the event stays bounded");
    }

    #[test]
    fn the_bots_mention_leaves_the_text() {
        assert_eq!(strip_mentions("<@42> deploy please", "42"), "deploy please");
        assert_eq!(strip_mentions("<@!42> hi", "42"), "hi");
        assert_eq!(strip_mentions("  hi  ", "42"), "hi");
        assert_eq!(strip_mentions("<@9> hi", "42"), "<@9> hi");
    }

    #[test]
    fn attachments_are_named_in_the_prompt() {
        let message = json!({"attachments": [{"filename": "a.png"}, {"filename": ""}]});
        let rule = crate::chat::team::channel_returns::external_reply_rule("Discord");
        assert_eq!(prompt(&message, "look"), format!("look\n\n[Attached, not downloaded: a.png]\n\n{rule}"));
        assert_eq!(prompt(&json!({}), "look"), format!("look\n\n{rule}"));
        assert!(prompt(&json!({}), "look").contains(crate::chat::team::channel_returns::EXTERNAL_REPLY_RULE));
    }
}
