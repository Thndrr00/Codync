//! A stable, opaque deep link to one message: `bot:<bot id>/entry:<entry id>`.
//!
//! The host owns the format so every client copies the same string; a unit test parses it back to
//! reject ids that could make a link ambiguous (an id never contains `/`). The desktop builds the
//! same link locally when it copies one from a message row.

use crate::hub::Hub;
use anyhow::{Context, Result, anyhow};
use serde_json::{Value, json};

const BOT: &str = "bot:";
const ENTRY: &str = "/entry:";

/// The link a client copies for one message. Both ids stay as the host wrote them.
pub(super) fn format(bot_id: &str, entry_id: &str) -> Result<String> {
    if bot_id.is_empty() || entry_id.is_empty() || bot_id.contains('/') || entry_id.contains('/') {
        return Err(anyhow!("A bot and message id are needed for a link"));
    }
    Ok(format!("{BOT}{bot_id}{ENTRY}{entry_id}"))
}

/// The two ids a link was made from, or nothing when the string isn't one of ours.
#[cfg(test)]
pub(super) fn parse(link: &str) -> Result<(&str, &str)> {
    let rest = link.strip_prefix(BOT).ok_or_else(|| anyhow!("Not a message link"))?;
    let (bot_id, entry_id) = rest.split_once(ENTRY).ok_or_else(|| anyhow!("Not a message link"))?;
    if bot_id.is_empty() || entry_id.is_empty() || entry_id.contains('/') {
        return Err(anyhow!("Not a message link"));
    }
    Ok((bot_id, entry_id))
}

/// Answers `messageLink {botId, entryId}` with the link, after checking the message is in that chat.
pub(super) fn call(hub: &Hub, b: &Value) -> Result<Value> {
    let bot_id = b["botId"].as_str().context("`botId` is required")?;
    let entry_id = b["entryId"].as_str().context("`entryId` is required")?;
    let entry = hub.store.entry(entry_id).context("Unknown message")?;
    if entry.bot_id != bot_id {
        return Err(anyhow!("That message is in another chat"));
    }
    Ok(json!({"link": format(bot_id, entry_id)?}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_link_names_one_bot_and_one_message() {
        assert_eq!(format("pacer", "e-42").expect("format"), "bot:pacer/entry:e-42");
        assert_eq!(parse("bot:pacer/entry:e-42").expect("parse"), ("pacer", "e-42"));
    }

    #[test]
    fn a_link_refuses_ids_that_would_make_it_ambiguous() {
        for (bot, entry) in [("", "e"), ("b", ""), ("b/c", "e"), ("b", "e/f")] {
            assert!(format(bot, entry).is_err(), "{bot}/{entry}");
        }
        for link in ["http://pacer/entry/e", "bot:pacer", "bot:/entry:e", "bot:pacer/entry:", "bot:pacer/entry:e/more"]
        {
            assert!(parse(link).is_err(), "{link}");
        }
    }
}
