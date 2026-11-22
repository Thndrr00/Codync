//! Where a chat-app approval card shows (OpenBot nightly `triage-13781`).
//!
//! A turn from WhatsApp, Telegram, Slack, Discord or a native channel runs in a side conversation,
//! but clients only show pending approvals in the agent's main chat and sidebar. Keeping the card
//! in the messaging thread left the agent "working" with no card to answer. The pure decisions here
//! give the card the agent's main lane while preserving the channel thread that the messaging
//! bridge needs to find the chat link.

use crate::store::Lane;

/// Thread roots that are messaging side conversations, not documents (see `session::side_slot`).
pub const MESSAGING_PREFIXES: [&str; 4] = ["slack-", "discord-", "whatsapp-", "native-channel-"];

/// Whether `thread` is a messaging conversation rather than a document or the main chat.
pub fn is_messaging_thread(thread: Option<&str>) -> bool {
    thread.is_some_and(|t| MESSAGING_PREFIXES.iter().any(|prefix| t.starts_with(prefix)))
}

/// The lane an approval card is saved in. A messaging thread's turn still answers in its own side
/// conversation, but its card goes to the agent's main chat so the apps and sidebar show it. All
/// other lanes (main chat, room, document thread) keep their card where it is.
pub fn approval_lane(lane: &Lane, agent: &str) -> Lane {
    if lane.chat == agent && is_messaging_thread(lane.thread.as_deref()) { Lane::main(agent) } else { lane.clone() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_messaging_threads_count() {
        assert!(is_messaging_thread(Some("whatsapp-+15551234@group")));
        assert!(is_messaging_thread(Some("slack-C123/1234.5")));
        assert!(is_messaging_thread(Some("discord-99")));
        assert!(is_messaging_thread(Some("native-channel-abc")));
        assert!(!is_messaging_thread(Some("doc:abcd")));
        assert!(!is_messaging_thread(None));
    }

    #[test]
    fn a_chat_app_approval_moves_to_main_but_other_cards_stay_put() {
        let thread = Lane::in_thread("a", "whatsapp-+15551234@group");
        assert_eq!(approval_lane(&thread, "a"), Lane::main("a"));
        // A card in someone else's chat (a group) or a document isn't a chat-app approval.
        let group = Lane::main("g");
        assert_eq!(approval_lane(&group, "a"), group);
        let doc = Lane::in_thread("a", "doc:abcd");
        assert_eq!(approval_lane(&doc, "a"), doc);
    }
}
