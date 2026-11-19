//! One permalink per message (agentes.chat ADR 0138).
//!
//! A link names the bot and the entry's sequence in its chat, so an agent can cite where a person
//! asked it to work and a client can jump straight there. Nothing is stored: the link is derived
//! from the identifiers the store already gives every entry.

use serde_json::{Value, json};

/// A client-deep link to one message. The scheme is the app's own; clients already open `codync://`.
pub fn permalink(bot: &str, entry_id: &str) -> String {
    format!("codync://message/{bot}/{entry_id}")
}

/// The structured form an agent puts in a reply so the client can draw a pill.
pub fn cite(bot: &str, entry_id: &str, label: &str) -> Value {
    json!({
        "type": "message_link",
        "url": permalink(bot, entry_id),
        "botId": bot,
        "entryId": entry_id,
        "label": label,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_link_is_stable_and_carries_its_target() {
        let link = permalink("b1", "e42");
        assert_eq!(link, "codync://message/b1/e42");
        let c = cite("b1", "e42", "the request");
        assert_eq!(c["url"], link);
        assert_eq!(c["entryId"], "e42");
        assert_eq!(c["type"], "message_link");
    }
}
