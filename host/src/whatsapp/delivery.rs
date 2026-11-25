//! Routine delivery uses the existing link, pinned to its account and authorization generation.

use crate::hub::Hub;
use crate::routines::Output;
use crate::store::Store;
use anyhow::Result;
use serde_json::{Value, json};
use std::sync::Arc;

/// The linked account is still authorized, but nothing has entered the bridge yet.
#[derive(Debug)]
pub(crate) struct Unsent(pub anyhow::Error);

impl std::fmt::Display for Unsent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WhatsApp was not sent: {}", self.0)
    }
}
impl std::error::Error for Unsent {}

pub(super) fn unsent(error: anyhow::Error, bound: bool) -> anyhow::Error {
    if bound { anyhow::Error::new(Unsent(error)) } else { error }
}

/// The linked number as a messaging place: its writable allowlist, plus the owner's own chat, and
/// the bot it routes to (empty when the number still uses the default). A number with no allowlist
/// answers anyone but may only post to its own chat, so an agent can never pick an unallowed
/// contact. The link's generation and tokens never appear here.
pub(crate) fn places(store: &Store) -> Result<Vec<(String, Vec<String>, Vec<String>)>> {
    let Some(account) = store.kv_read("whatsapp.link.account")? else { return Ok(Vec::new()) };
    if account.is_empty() {
        return Ok(Vec::new());
    }
    let mut targets = super::settings::read_allowed(store)?
        .into_iter()
        .filter(|entry| super::settings::can_reply(Some(entry)))
        .map(|entry| entry.jid)
        .collect::<Vec<_>>();
    if !targets.iter().any(|jid| jid == &account) {
        targets.push(account.clone());
    }
    let bots = store.kv_read("whatsapp.bot")?.filter(|bot| !bot.is_empty()).into_iter().collect();
    Ok(vec![(account, targets, bots)])
}

pub(crate) fn matches(store: &Store, account: &str, expected: &Value) -> Result<bool> {
    let _configuration = crate::LockExt::locked(&store.connector_lock);
    matches_locked(store, account, expected)
}

/// Caller holds the connector lock through validation and receipt publication.
pub(super) fn matches_locked(store: &Store, account: &str, expected: &Value) -> Result<bool> {
    let current = store.kv_read("whatsapp.link.account")?;
    let epoch = store.kv_read("whatsapp.link.generation")?;
    Ok(!account.is_empty()
        && current.as_deref() == Some(account)
        && epoch.as_deref().is_some_and(|epoch| !epoch.is_empty())
        && expected == &json!([current, epoch]))
}

pub(crate) fn authorized(hub: &Hub, output: &Output, event: &Value) -> Result<bool> {
    matches(&hub.store, output.account.as_deref().unwrap_or_default(), &event["whatsappBinding"])
}

pub(crate) fn connected(hub: &Hub, account: Option<&str>) -> bool {
    hub.whatsapp.state.try_lock().is_ok_and(|state| state.state == "connected" && state.me.as_deref() == account)
}

pub(crate) async fn send(hub: &Arc<Hub>, output: &Output, event: &Value, text: &str) -> Result<()> {
    hub.whatsapp
        .send_bound_text(
            &hub.store,
            output.account.as_deref().unwrap_or_default(),
            &event["whatsappBinding"],
            output.target.trim(),
            text,
        )
        .await
}

/// Posts from an agent or routine through the same linked number a reply uses, with the same
/// generation check. The bridge sends plain text, so reply and mention markers are put in the text
/// by the caller; nothing is sent before the link still matches.
pub(crate) async fn post(hub: &Arc<Hub>, account: &str, to: &str, text: &str) -> Result<()> {
    let binding = {
        let _configuration = crate::LockExt::locked(&hub.store.connector_lock);
        hub.store.snapshot_whatsapp_output(&json!({}))?["whatsappBinding"].clone()
    };
    if !matches(&hub.store, account, &binding)? {
        anyhow::bail!("That WhatsApp number is no longer linked; nothing was posted.");
    }
    hub.whatsapp.send_bound_text(&hub.store, account, &binding, to, text).await
}

pub(crate) fn jid(value: &str) -> bool {
    let Some((id, domain)) = value.split_once('@') else { return false };
    !id.is_empty()
        && id.len() <= 80
        && id.split('-').all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
        && (domain != "s.whatsapp.net" || !id.contains('-'))
        && matches!(domain, "s.whatsapp.net" | "g.us")
}
