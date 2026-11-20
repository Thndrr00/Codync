//! The per-bot opt-in for computer use on turns that came from a channel.
//!
//! OpenBot 0.75 lets a bot with Computer Use drive the desktop when its turn came from a channel.
//! In Codync the turn's user message carries a `source` in its entry, and that same source is what
//! keeps a key typed in an external chat out of the MCP header vault
//! (`market::mcp_headers::local_chat`). This gate only adds the computer-use switch: it never
//! changes how a source is classified for secrets, and the capability, approval-card, protected
//! entry and protected browser checks still run on a channel turn exactly as on a local one.

use crate::hub::Hub;
use anyhow::{Context, Result, bail};

/// Entry sources that mean the message reached the bot from a channel. Kept in step with the
/// secret gate in `market::mcp_headers` and the channel-history tool.
const CHANNEL_SOURCES: [&str; 5] = ["slack", "telegram", "discord", "whatsapp", "native-channel"];

fn is_channel_source(source: &str) -> bool {
    CHANNEL_SOURCES.contains(&source)
}

/// The entry source of the turn's tracked user message. Empty when the turn is local or has no
/// tracked message (a routine or a host-started turn). This is the same lookup the MCP header gate
/// uses, so the two gates can never disagree about whether a message came from a channel.
fn turn_source(hub: &Hub, bot: &str) -> String {
    hub.runtime(bot)
        .message
        .as_deref()
        .and_then(|id| hub.store.entry(id))
        .and_then(|entry| entry.data["source"].as_str().map(str::to_owned))
        .unwrap_or_default()
}

/// Pure: a local turn needs no switch; a channel turn needs the bot's `channelComputerUse` on.
/// The capability itself is enforced on every path by `computer::Access::begin`, which this gate
/// does not replace or weaken.
fn channel_permitted(source: &str, channel_computer_use: bool) -> bool {
    !is_channel_source(source) || channel_computer_use
}

/// Refuses a computer-use call whose turn came from a channel unless this bot opted in. Returns
/// `Ok` for a local turn without reading the bot's config; the existing capability, approval-card,
/// protected-entry and protected-browser checks still run for every call, whichever kind of turn.
pub(super) fn ensure_allowed(hub: &Hub, bot: &str) -> Result<()> {
    let source = turn_source(hub, bot);
    if !is_channel_source(&source) {
        return Ok(());
    }
    let row = hub.store.bot(bot)?.filter(|row| !row.deleted).context("unknown bot")?;
    if !channel_permitted(&source, row.config.channel_computer_use) {
        bail!("Computer use from a channel is turned off for this bot; ask the owner to turn it on.");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mirrors the secret gate in `market::mcp_headers`: a key from a channel is never safe, and
    /// `channelComputerUse` is deliberately not an input to that decision.
    fn secret_permitted(source: &str) -> bool {
        !is_channel_source(source)
    }

    #[test]
    fn a_local_turn_needs_no_switch_the_capability_gate_is_unchanged() {
        for source in ["", "user", "voice"] {
            assert!(!is_channel_source(source), "`{source}` is local");
            assert!(channel_permitted(source, false), "`{source}` may use computer tools");
            assert!(channel_permitted(source, true), "the switch changes nothing for `{source}`");
        }
    }

    #[test]
    fn a_channel_turn_is_allowed_only_with_the_switch() {
        for source in CHANNEL_SOURCES {
            assert!(is_channel_source(source), "`{source}` is a channel");
            assert!(!channel_permitted(source, false), "`{source}` needs the switch");
            assert!(channel_permitted(source, true), "`{source}` is allowed once opted in");
        }
    }

    #[test]
    fn secrets_are_never_allowed_from_a_channel_even_with_the_switch() {
        for source in CHANNEL_SOURCES {
            // The switch cannot make a channel source local for the secret gate.
            assert!(is_channel_source(source) && !secret_permitted(source), "no secret from `{source}`");
        }
        for source in ["", "user", "voice"] {
            assert!(secret_permitted(source), "`{source}` is local");
        }
    }
}
