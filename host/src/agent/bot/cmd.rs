//! What reaches a bot's actor ([`Cmd`]) and the turns that wait in its queue ([`Queued`]).

use crate::chat::group::GroupTurn;
use crate::store::{BotConfig, Lane};
use anyhow::Result;
use serde_json::Value;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Instant;

pub enum Cmd {
    /// Apply the whole native Hermes route together after the current turn.
    HermesRuntimeChanged,
    /// Revoke only input from the replaced connection generation or unpaired sender.
    NativeAuthorizationChanged {
        connection: String,
        generation: Option<String>,
        user: Option<String>,
    },
    RefreshTools,
    /// The computer is short of memory: an idle agent gives its memory back now, without waiting out its idle time.
    FreeMemory,
    /// A cloud computer's keeper started again, forgetting the grants the agent was started with.
    KeeperRestarted,
    Routine(String),
    /// The user wrote in the bot's chat.
    Send {
        lane: Lane,
        entry_id: String,
        text: String,
    },
    /// The user rewrote a waiting message before it went out.
    EditQueued {
        entry_id: String,
        text: String,
        reply: tokio::sync::oneshot::Sender<Result<()>>,
    },
    /// The user wants a queued message of theirs handled now (`steer`).
    Steer {
        entry_id: String,
    },
    /// What the agent made of a steered message (`_session/steering`).
    Steered {
        entry_id: String,
        outcome: Result<Value>,
    },
    /// The user took back a queued message of theirs (`unqueue`).
    Unqueue {
        entry_id: String,
    },
    /// A serialized snapshot of user messages that have not started, in actor queue order.
    QueuedMessages {
        reply: tokio::sync::oneshot::Sender<Vec<String>>,
    },
    /// Removes one queued user message without stopping the current turn or other queued work.
    CancelQueued {
        entry_id: String,
        reply: tokio::sync::oneshot::Sender<Result<bool>>,
    },
    BotRequest(crate::chat::team::BotRequest),
    /// Cancel only the synchronous request owned by the caller that stopped waiting.
    CancelAsk {
        id: String,
    },
    /// How a request this bot sent came out: a turn of its own, with any others that arrive meanwhile.
    BotReply(crate::chat::team::BotReply),
    /// A turn the host starts in the bot's own chat, with no message of the user's (the lead opening a call): its
    /// reply is that call's (`chat::calls`).
    Prompt {
        text: String,
        call: Option<String>,
    },
    /// `send_message` from the bot's `chat` MCP server: a message for the user, now.
    SendToUser {
        text: String,
        reply: tokio::sync::oneshot::Sender<Result<()>>,
    },
    /// `react` from the bot's `chat` MCP server: an emoji on the message the turn answers.
    React {
        emoji: String,
        /// In a room: whose message (see `chat::reactions::in_room`).
        to: Option<String>,
        reply: tokio::sync::oneshot::Sender<Result<()>>,
    },
    SendFile {
        path: String,
        name: Option<String>,
        cancelled: Arc<std::sync::atomic::AtomicBool>,
        reply: tokio::sync::oneshot::Sender<Result<()>>,
    },
    /// An artifact the bot just published (`chat::artifacts`): its card (the entry's `data`) in the turn's chat. Answers
    /// the card's entry id.
    PostArtifact {
        data: Value,
        reply: tokio::sync::oneshot::Sender<Result<String>>,
    },
    /// Its turn in a group's room turn.
    Group(GroupTurn),
    /// The group's room turns stopped: drop its queued turns, stop a running one.
    CancelGroup {
        chat: String,
    },
    Stop,
    Permission {
        entry_id: String,
        option_id: Option<String>,
    },
    /// A Codync tool asks the user before it acts (`chat::manage`), on the agent's kind of approval card.
    Approve {
        title: String,
        detail: String,
        reply: tokio::sync::oneshot::Sender<bool>,
    },
    NewSession,
    Reconfigure(Box<BotConfig>),
    /// The user wants the bot's own agent again (`switchBack`).
    SwitchBack,
    /// A voice call ended (`chat::calls`): its replies show in the chat again, and dreaming reads it.
    CallEnded {
        call: String,
    },
    Shutdown,
}

pub(super) enum Queued {
    Routine(String),
    User {
        lane: Lane,
        entry_id: String,
        text: String,
        /// When the message arrived, so a wait behind other work can be measured.
        queued_at: Instant,
    },
    BotRequest(crate::chat::team::BotRequest),
    BotReply(crate::chat::team::BotReply),
    Prompt {
        text: String,
        call: Option<String>,
    },
    /// A compaction the host decided on (see `compaction`); `idle` while the user is away.
    Compact {
        idle: bool,
    },
    Group(GroupTurn),
}

impl Queued {
    pub(super) fn user_entry_id(&self) -> Option<&str> {
        match self {
            Self::User { entry_id, .. } => Some(entry_id),
            _ => None,
        }
    }

    /// Drop every copy of this message, preserving the order of all unrelated work.
    pub(super) fn remove_user(queue: &mut VecDeque<Self>, entry_id: &str) -> bool {
        let before = queue.len();
        queue.retain(|queued| queued.user_entry_id() != Some(entry_id));
        before != queue.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user(id: &str) -> Queued {
        Queued::User { lane: Lane::main("bot"), entry_id: id.into(), text: id.into(), queued_at: Instant::now() }
    }

    #[test]
    fn cancelling_one_message_preserves_other_user_messages_and_background_work_in_order() {
        let mut queue =
            VecDeque::from([user("first"), Queued::Routine("routine".into()), user("cancel"), user("last")]);
        assert!(Queued::remove_user(&mut queue, "cancel"));
        assert_eq!(queue.iter().filter_map(Queued::user_entry_id).collect::<Vec<_>>(), ["first", "last"]);
        assert!(matches!(&queue[1], Queued::Routine(id) if id == "routine"));
        assert_eq!(queue.len(), 3);
    }

    #[test]
    fn a_message_that_already_started_or_was_removed_cannot_be_cancelled_again() {
        let mut queue = VecDeque::from([user("waiting")]);
        assert!(!Queued::remove_user(&mut queue, "already-started"));
        assert_eq!(queue.len(), 1);
        assert!(Queued::remove_user(&mut queue, "waiting"));
        assert!(!Queued::remove_user(&mut queue, "waiting"));
    }

    #[test]
    fn snapshots_include_only_user_messages_in_the_actual_fifo_order() {
        let queue = VecDeque::from([user("z"), Queued::Routine("r".into()), user("a")]);
        assert_eq!(queue.iter().filter_map(Queued::user_entry_id).collect::<Vec<_>>(), ["z", "a"]);
    }
}
