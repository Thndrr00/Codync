//! A button on an approval card: who may press it, which card it settles, and the answer back.
//!
//! Buttons carry `p|<entry id>|<option>|<print>`: the print is a digest of the card's title and
//! options, so a button of a card that changed cannot answer another. The first answer wins, from
//! any client; a later one finds the card settled and says so.

use super::{Arc, Config, Endpoint, Hub, Result, Value, client, json};
use crate::api::devices::Caller;
use crate::channels::approval::{self, Approval};

fn custom_id(card: &Value, option: &str) -> String {
    approval::action(card, option).unwrap_or_default()
}

/// The person and channel an interaction came from.
fn who(interaction: &Value) -> Option<(&str, &str)> {
    let user = interaction["member"]["user"]["id"].as_str().or_else(|| interaction["user"]["id"].as_str())?;
    Some((user, interaction["channel_id"].as_str()?))
}

pub(super) async fn accept(hub: &Arc<Hub>, c: &Config, interaction: &Value, ep: &Endpoint) -> Result<()> {
    if interaction["type"] != 3 {
        return Ok(());
    }
    let (Some(id), Some(token)) = (interaction["id"].as_str(), interaction["token"].as_str()) else {
        return Ok(());
    };
    let Some((user, channel)) = who(interaction) else { return Ok(()) };
    let direct = interaction["guild_id"].is_null();
    let allowed = |binding: &super::Binding| {
        binding.allowed_users.iter().any(|u| u == user) && (direct || binding.channel_id == channel)
    };
    if !c.bindings.iter().any(allowed) {
        client::interaction(ep, token, id, 4, Some("You are not allowed to answer this.")).await?;
        return Ok(());
    }
    let Some((card_id, option, print)) = interaction["data"]["custom_id"].as_str().and_then(approval::parse_action)
    else {
        client::interaction(ep, token, id, 4, Some("That button is not for this.")).await?;
        return Ok(());
    };
    let Some(card) = hub.store.entry(card_id).filter(|e| e.kind == "permission") else {
        client::interaction(ep, token, id, 4, Some("That approval is gone.")).await?;
        return Ok(());
    };
    // The bound bot has to be the one that asked, not merely one that lets this person in.
    let asking = hub.store.bot(&card.bot_id)?.is_some_and(|row| {
        !row.deleted && !row.config.is_group() && c.bindings.iter().any(|b| b.bot_id == card.bot_id && allowed(b))
    });
    if !asking {
        client::interaction(ep, token, id, 4, Some("That bot is not in this conversation.")).await?;
        return Ok(());
    }
    let Ok(card) = serde_json::to_value(&card) else { return Ok(()) };
    if approval::print(&card).as_deref() != Some(print) {
        client::interaction(ep, token, id, 4, Some("That approval changed.")).await?;
        return Ok(());
    }
    // Defer before the work: Discord expects an answer within three seconds. The card's own update
    // follows from the worker's watch of the entry, whichever client answered.
    client::interaction(ep, token, id, 6, None).await?;
    let _ =
        crate::api::dispatch(hub, &Caller::Local, "respondPermission", json!({"entryId": card_id, "optionId": option}))
            .await;
    Ok(())
}

/// A button's `custom_id` for `option`, so the worker builds what `accept` reads.
pub(super) fn button(card: &Value, option: &str) -> String {
    custom_id(card, option)
}

/// The permission's text, shared by the worker and a settled card.
pub(super) fn text(card: &Value) -> String {
    Approval::from_entry(card).map_or_else(
        || "Needs you: Approval".into(),
        |model| match model.command {
            Some(command) => format!("Needs you: {}\n`{command}`", model.title),
            None => format!("Needs you: {}", model.title),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(options: &[&str]) -> Value {
        json!({"id": "e1", "data": {"title": "Run a command", "command": "cargo test",
            "options": options.iter().map(|o| json!({"optionId": o, "name": o})).collect::<Vec<_>>()}})
    }

    #[test]
    fn a_button_names_its_card_and_option() {
        let id = button(&card(&["allow", "deny"]), "allow");
        let mut parts = id.split('|');
        assert_eq!((parts.next(), parts.next(), parts.next()), (Some("p"), Some("e1"), Some("allow")));
        assert_eq!(parts.next().map(str::len), Some(6));
        assert!(text(&card(&["allow"])).contains("cargo test"));
    }

    #[test]
    fn a_changed_card_gets_another_print() {
        assert_ne!(button(&card(&["allow", "deny"]), "allow"), button(&card(&["deny"]), "allow"));
    }
}
