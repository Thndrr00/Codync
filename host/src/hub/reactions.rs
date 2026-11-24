//! Emoji reactions on chat messages: the user's (`data.reactions`, toggled from any client) and
//! the bots' (`data.botReactions`, one `{bot, emoji}` per bot, set with the `react` tool).

use super::{BotStatus, Hub};
use crate::agent::bot::Cmd;
use crate::store::{Entry, EntryKind};
use anyhow::{Result, bail};
use serde_json::json;

/// The user's reactions per message: a handful of distinct emoji.
const MAX_REACTIONS: usize = 8;
const ALLOW: &str = "allow";
const DENY: &str = "deny";
const FIRST: &str = "0";

/// What a reaction can answer at the moment of the tap (OpenBot v0.72.0). `Some(true)` is an
/// approval card, `Some(false)` a choice card, `None` a question asked in plain chat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PendingAnswer {
    pub waiting: bool,
    pub approval: Option<bool>,
    /// How many choices the open card shows; zero for a question asked in plain chat.
    pub choices: usize,
}

/// The card choice a gesture names. A thumbs up takes the first choice; a thumbs down cancels a
/// question or denies an approval; a number key takes that numbered choice. Pure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CardAnswer {
    Allow,
    Deny,
    Pick(usize),
}

fn reaction_choice(emoji: &str) -> Option<CardAnswer> {
    match emoji {
        "👍" | "✅" | "👌" | "🙌" => Some(CardAnswer::Allow),
        "👎" | "❌" | "🙅" => Some(CardAnswer::Deny),
        "1️⃣" => Some(CardAnswer::Pick(0)),
        "2️⃣" => Some(CardAnswer::Pick(1)),
        "3️⃣" => Some(CardAnswer::Pick(2)),
        "4️⃣" => Some(CardAnswer::Pick(3)),
        _ => None,
    }
}

/// Whether this emoji answers the bot's pending question. A bot that isn't waiting never takes an
/// answer from a reaction; on a card the gesture must name one of its choices. Pure, tested.
pub fn reaction_is_answer(emoji: &str, pending: PendingAnswer) -> bool {
    if !pending.waiting {
        return false;
    }
    match pending.approval {
        None => true,
        Some(approval) => match reaction_choice(emoji) {
            Some(CardAnswer::Allow) => pending.choices >= 2,
            Some(CardAnswer::Deny) => true,
            Some(CardAnswer::Pick(i)) => !approval && i < pending.choices,
            None => false,
        },
    }
}

impl Hub {
    /// Toggles the user's `emoji` reaction on a chat message (`data.reactions`, oldest first).
    /// A message keeps `reactions: []` once reacted to, so it never turns unread again.
    pub fn react(&self, id: &str, emoji: &str) -> Result<Entry> {
        let emoji = one_emoji(emoji)?;
        let entry = self.edit_entry(id, |e| {
            chat_message(e)?;
            let mut reactions: Vec<String> = serde_json::from_value(e.data["reactions"].clone()).unwrap_or_default();
            if let Some(i) = reactions.iter().position(|r| r == emoji) {
                reactions.remove(i);
            } else if reactions.len() >= MAX_REACTIONS {
                bail!("a message takes at most {MAX_REACTIONS} reactions");
            } else {
                reactions.push(emoji.to_owned());
            }
            e.data["reactions"] = json!(reactions);
            Ok(true)
        })?;
        // OpenBot 0.72.0: reacting to a bot's answer to your question is answering it. Only a final
        // bot reply counts, and only while that bot still waits (see `answer_reaction`).
        if entry.kind == EntryKind::Agent.as_str() && entry.data["final"] == true {
            self.answer_reaction(&entry, emoji);
        }
        Ok(entry)
    }

    /// A gesture on a waiting bot's reply takes the place of tapping its card, or, with no card,
    /// arrives as the answer to the question it asked in chat. A tap that removed a reaction, an
    /// idle bot and another bot's card all do nothing: the reaction itself always stands.
    fn answer_reaction(&self, reply: &Entry, emoji: &str) {
        let added =
            reply.data["reactions"].as_array().is_some_and(|list| list.iter().any(|r| r.as_str() == Some(emoji)));
        if !added {
            return;
        }
        let bot = reply.bot_id.as_str();
        let Ok(Some(row)) = self.store.bot(bot) else { return };
        if row.config.is_group() || self.runtime(bot).status != BotStatus::NeedsInput {
            return;
        }
        // A question card and an approval card take different option ids; nothing else can be answered.
        let card = self.waiting_card(bot);
        let pending = PendingAnswer {
            waiting: true,
            approval: card.as_ref().map(|(_, approval, _)| *approval),
            choices: card.as_ref().map_or(0, |(_, _, choices)| *choices),
        };
        if !reaction_is_answer(emoji, pending) {
            return;
        }
        let Some((card_id, approval, _)) = card else {
            // A plain question has no card to close, so the emoji goes in as the turn's answer
            // through the same queued-input path the other host-initiated messages use.
            if let Err(error) = crate::api::apps_runtime::dispatch_bot_turn(self, bot, emoji, "reaction") {
                tracing::warn!(bot, error = format!("{error:#}"), "couldn't send the reaction as an answer");
            }
            return;
        };
        let choice = reaction_choice(emoji);
        let option = match choice {
            Some(CardAnswer::Allow) => Some(if approval { ALLOW } else { FIRST }.to_owned()),
            Some(CardAnswer::Deny) => approval.then(|| DENY.to_owned()),
            Some(CardAnswer::Pick(i)) if !approval => Some(i.to_string()),
            _ => return,
        };
        // A terminal session's hook is waiting on its own card; a bot actor's is answered over its
        // command queue. Both go through the card's existing answer path, which closes it once.
        if self.external.answer(self, &card_id, option.as_deref()) {
            return;
        }
        if let Err(error) = self.send_cmd(bot, Cmd::Permission { entry_id: card_id, option_id: option }) {
            tracing::warn!(bot, error = format!("{error:#}"), "couldn't answer the card from a reaction");
        }
    }

    /// The oldest still-open card of `bot`'s chat that its own reply left waiting, with whether it
    /// asks for approval (Allow / Deny) rather than a numbered choice.
    fn waiting_card(&self, bot: &str) -> Option<(String, bool, usize)> {
        self.store
            .entries_since(0, 500)
            .ok()?
            .into_iter()
            .rev()
            .find(|e| {
                e.kind == EntryKind::Permission.as_str()
                    && e.data["status"] == "pending"
                    && e.data["welcomeQuestion"] != true
                    && e.data["author"].as_str().unwrap_or(&e.bot_id) == bot
            })
            .map(|e| {
                let options = e.data["options"].as_array();
                let approval = options.is_some_and(|options| {
                    options.iter().any(|option| option["optionId"] == ALLOW || option["optionId"] == DENY)
                });
                (e.id, approval, options.map_or(0, Vec::len))
            })
    }

    /// `bot` reacts to a chat message with `emoji`, replacing its earlier reaction there.
    /// The message keeps the rev of its content (`contentRev`), so a message the user read stays read.
    pub fn bot_react(&self, id: &str, bot: &str, emoji: &str) -> Result<Entry> {
        let emoji = one_emoji(emoji)?;
        self.edit_entry(id, |e| {
            chat_message(e)?;
            let mut reactions = e.data["botReactions"].as_array().cloned().unwrap_or_default();
            if reactions.iter().any(|r| r["bot"] == bot && r["emoji"] == emoji) {
                return Ok(false);
            }
            reactions.retain(|r| r["bot"] != bot);
            reactions.push(json!({"bot": bot, "emoji": emoji}));
            e.data["botReactions"] = json!(reactions);
            if e.data["contentRev"].is_null() {
                e.data["contentRev"] = e.rev.into();
            }
            Ok(true)
        })
    }
}

/// Only a message the user sees takes reactions: theirs, or a bot's final one.
fn chat_message(e: &Entry) -> Result<()> {
    let visible =
        e.kind == EntryKind::User.as_str() || (e.kind == EntryKind::Agent.as_str() && e.data["final"] == true);
    if !visible {
        bail!("only chat messages take reactions");
    }
    Ok(())
}

fn one_emoji(emoji: &str) -> Result<&str> {
    let emoji = emoji.trim();
    if emoji.is_empty() || emoji.len() > 32 || emoji.chars().any(|c| c.is_alphanumeric() || c.is_whitespace()) {
        bail!("`emoji` must be one emoji");
    }
    Ok(emoji)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pending(approval: Option<bool>) -> PendingAnswer {
        PendingAnswer { waiting: true, approval, choices: 3 }
    }

    #[test]
    fn an_idle_bot_never_takes_a_reaction_as_an_answer() {
        for emoji in ["👍", "👎", "2️⃣", "❤️"] {
            assert!(
                !reaction_is_answer(emoji, PendingAnswer { waiting: false, approval: None, choices: 0 }),
                "{emoji}"
            );
            assert!(
                !reaction_is_answer(emoji, PendingAnswer { waiting: false, approval: Some(true), choices: 2 }),
                "{emoji}"
            );
        }
    }

    #[test]
    fn an_approval_takes_a_yes_or_no_but_not_a_number() {
        let card = pending(Some(true));
        assert!(reaction_is_answer("👍", card));
        assert!(reaction_is_answer("✅", card));
        assert!(reaction_is_answer("👎", card));
        assert!(reaction_is_answer("❌", card));
        assert!(!reaction_is_answer("2️⃣", card), "a number isn't an approval choice");
        assert!(!reaction_is_answer("❤️", card), "a bare emoji isn't a choice");
        assert_eq!(reaction_choice("👍"), Some(CardAnswer::Allow));
        assert_eq!(reaction_choice("👎"), Some(CardAnswer::Deny));
    }

    #[test]
    fn a_question_card_takes_its_numbered_choices() {
        let card = pending(Some(false));
        assert!(reaction_is_answer("1️⃣", card));
        assert!(reaction_is_answer("4️⃣", card));
        assert!(reaction_is_answer("👍", card), "a yes takes the first option");
        assert!(!reaction_is_answer("❤️", card));
        assert_eq!(reaction_choice("3️⃣"), Some(CardAnswer::Pick(2)));
        assert!(
            !reaction_is_answer("4️⃣", PendingAnswer { waiting: true, approval: Some(false), choices: 2 }),
            "a number the card doesn't show is no answer"
        );
    }

    #[test]
    fn a_question_asked_in_chat_takes_any_reaction_as_its_answer() {
        let plain = pending(None);
        assert!(reaction_is_answer("❤️", plain));
        assert!(reaction_is_answer("🎉", plain));
        assert!(reaction_is_answer("👍", plain));
    }
}
