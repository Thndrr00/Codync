//! Setting an MCP server's HTTP headers from a key the person typed in the chat.
//!
//! Ported from OpenBot 0.74.0: an agent can set a header such as `Authorization: Bearer <token>`
//! when it is given the key in the local chat. The approval card names only the headers (the value
//! is masked), and the real value is held in the encrypted connector vault until the person allows
//! it. A key that came from an external chat (Telegram, Slack, Discord, WhatsApp) is refused: that
//! chat is not a safe place to hand over a credential.

use crate::{hub::Hub, store::now_ms};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub(super) const TOOL: &str = "set_mcp_headers";
/// Pending values live in the encrypted connector vault, never in the chat entry the client sees.
const SLOT: &str = "mcpHeaderRequests";
/// Headers a server needs are small; keep a generous but bounded limit.
const MAX_VALUE: usize = 8 * 1024;
const MAX_HEADERS: usize = 16;
/// Headers the host or the transport owns; a person may never overwrite them with a key.
const RESERVED: [&str; 5] = ["host", "cookie", "content-length", "transfer-encoding", "connection"];
/// Chats whose messages carry a `source` in their entry; a key from one of these is refused.
const EXTERNAL: [&str; 5] = ["slack", "telegram", "discord", "whatsapp", "native-channel"];

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Pending {
    connector_id: String,
    headers: BTreeMap<String, String>,
    at: i64,
}

pub fn tool() -> Value {
    json!({
        "name": TOOL,
        "description": "Set HTTP headers on an installed remote MCP server, e.g. Authorization: Bearer <token>, with a key the user gives you in this chat. Only works in the user's local chat, never from Telegram, Slack, Discord or WhatsApp. The user approves a card that names the headers that change; the key is masked in the card, in logs and in this result, and is never returned to you. Never ask for a key in ordinary chat or paste one into a tool argument yourself.",
        "inputSchema": {"type": "object", "properties": {
            "connectorId": {"type": "string", "description": "The installed MCP server's id from list_connectors."},
            "headers": {"type": "object", "additionalProperties": {"type": "string"}, "description": "Header name to full header value, e.g. {\"Authorization\": \"Bearer sk-…\"}."},
            "reason": {"type": "string", "description": "Why these headers are needed."}
        }, "required": ["connectorId", "headers", "reason"]},
    })
}

/// Whether `source` marks a message delivered by an external chat, where a key must not be accepted.
pub(super) fn external_source(source: &str) -> bool {
    EXTERNAL.contains(&source)
}

/// Keeps just enough of a value to recognize which key changed, never enough to use it.
pub(super) fn mask(value: &str) -> String {
    let chars: Vec<char> = value.chars().collect();
    if chars.len() <= 8 {
        return "*".repeat(chars.len());
    }
    let head: String = chars[..4].iter().collect();
    let tail: String = chars[chars.len() - 2..].iter().collect();
    format!("{head}…{tail}")
}

/// Why a header name is refused, in the words the form and the agent both show.
/// Ported from OpenBot nightly triage-13174: a name with a space or a symbol is not
/// an HTTP header name; the key or token belongs in the value, not the name.
pub(crate) const HEADER_NAME_MESSAGE: &str = "Header names cannot contain spaces or symbols such as : ( ) /. Use a name such as Authorization, and put the key or token in Value.";

/// Whether `name` is an HTTP token (RFC 9110): letters, digits and a fixed set of
/// separators, with no space, colon, slash or other symbol. The form skips blank
/// rows before saving; the host refuses an empty name.
pub(crate) fn valid_header_name(name: &str) -> bool {
    const TOKEN: &str = "!#$%&'*+.^_`|~-";
    !name.is_empty()
        && name.len() <= 64
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || TOKEN.as_bytes().contains(&b))
}

/// A header a person may set: a valid token, not one the transport or host owns.
pub(super) fn check_header(name: &str) -> Result<()> {
    if valid_header_name(name) && !RESERVED.contains(&name.to_ascii_lowercase().as_str()) {
        Ok(())
    } else {
        bail!("{HEADER_NAME_MESSAGE}")
    }
}

/// Every name of a saved or about-to-be-saved header set must be usable. Used when
/// a connector is added or tested, so a name saved by an older client is refused
/// the next time it is edited or checked too.
pub(crate) fn check_headers(headers: &BTreeMap<String, String>) -> Result<()> {
    for name in headers.keys() {
        check_header(name)?;
    }
    Ok(())
}

/// Splits the agent's headers into the real values (stored encrypted) and a masked copy (shown).
pub(super) fn prepare(headers: &Value) -> Result<(BTreeMap<String, String>, Value)> {
    let object = headers.as_object().context("headers must be an object")?;
    if object.is_empty() || object.len() > MAX_HEADERS {
        bail!("Give between 1 and {MAX_HEADERS} headers");
    }
    let mut real = BTreeMap::new();
    let mut shown = serde_json::Map::new();
    for (name, value) in object {
        check_header(name)?;
        let value = value.as_str().context("header values must be text")?;
        if value.is_empty() || value.len() > MAX_VALUE {
            bail!("A header value must be 1 to {MAX_VALUE} characters");
        }
        real.insert(name.clone(), value.to_owned());
        shown.insert(name.clone(), json!(mask(value)));
    }
    Ok((real, Value::Object(shown)))
}

/// Refuses a key that reached the agent from an external chat. A turn with no tracked user message
/// (a routine or a host-started turn) has no key from a chat and is not subject to this gate.
pub(super) fn local_chat(hub: &Hub, bot: &str) -> Result<()> {
    let Some(id) = hub.runtime(bot).message.clone() else {
        return Ok(());
    };
    if let Some(entry) = hub.store.entry(&id)
        && external_source(entry.data["source"].as_str().unwrap_or_default())
    {
        bail!(
            "A key from an external chat (Telegram, Slack, Discord or WhatsApp) can't set MCP server headers. Ask for it in your local chat instead."
        );
    }
    Ok(())
}

fn all_pending(store: &crate::store::Store) -> Result<BTreeMap<String, Pending>> {
    Ok(match super::vault::read(store, SLOT)? {
        Some(raw) if !raw.is_empty() => serde_json::from_str(&raw)?,
        _ => BTreeMap::new(),
    })
}

/// Posts the approval card (masked) and keeps the real values encrypted until the person decides.
pub(super) fn request(
    hub: &Hub,
    bot: &str,
    connector_id: &str,
    reason: &str,
    real: BTreeMap<String, String>,
    masked: Value,
) -> Result<Value> {
    let connector = super::connectors(&hub.store)?
        .into_iter()
        .find(|c| c.id == connector_id && c.url.is_some())
        .context("Install a remote MCP server before setting its headers")?;
    let card = json!({
        "kind": super::requests::RequestKind::Headers,
        "status": super::requests::RequestStatus::Pending,
        "title": connector.name,
        "connectorId": connector_id,
        "botId": bot,
        "headers": masked,
    });
    let answer = super::requests::post(hub, bot, reason, &card)?;
    let id = answer["requestId"].as_str().context("Could not save the request")?.to_owned();
    let mut pending = all_pending(&hub.store)?;
    pending.retain(|_, item| now_ms() - item.at < super::requests::MAX_AGE);
    pending.insert(id, Pending { connector_id: connector_id.to_owned(), headers: real, at: now_ms() });
    super::vault::write(&hub.store, SLOT, &serde_json::to_string(&pending)?)?;
    Ok(answer)
}

fn take(hub: &Hub, id: &str) -> Result<Option<Pending>> {
    let mut pending = all_pending(&hub.store)?;
    let found = pending.remove(id);
    super::vault::write(&hub.store, SLOT, &serde_json::to_string(&pending)?)?;
    Ok(found)
}

/// Applies the approved headers to the server. Called by `requests::finish` after the card is
/// answered; the card itself only ever held masked values.
pub(super) fn apply(hub: &Hub, id: &str) -> Result<()> {
    let Some(pending) = take(hub, id)? else {
        bail!("This header request expired. Ask for it again.");
    };
    super::update_connectors(&hub.store, |all| {
        let connector = all.iter_mut().find(|c| c.id == pending.connector_id).context("Unknown connector")?;
        connector.headers.extend(pending.headers);
        Ok(())
    })
}

/// Drops pending values when the person cancels or the card expires.
pub(super) fn forget(hub: &Hub, id: &str) -> Result<()> {
    let _one = hub.store.connector_lock.locked();
    take(hub, id).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_local_chats_may_supply_a_key() {
        for source in ["", "user", "voice"] {
            assert!(!external_source(source), "`{source}` is a local chat");
        }
        for source in EXTERNAL {
            assert!(external_source(source), "`{source}` is an external chat");
        }
    }

    #[test]
    fn masking_never_shows_enough_of_a_key_to_use_it() {
        let key = "Bearer sk_live_1234567890abcdef";
        let shown = mask(key);
        assert_eq!(shown, "Bear…ef");
        assert!(!shown.contains(key) && !shown.contains("sk_live"));
        assert_eq!(mask("abc"), "***");
        assert_eq!(mask("12345678"), "********");
    }

    #[test]
    fn only_named_secret_headers_are_accepted() {
        assert!(check_header("Authorization").is_ok());
        assert!(check_header("X-Api-Key").is_ok());
        assert!(check_header("Host").is_err());
        assert!(check_header("bad header").is_err());
        assert!(check_header("").is_err());
    }

    #[test]
    fn a_name_with_a_space_or_symbol_gets_the_form_message() {
        for bad in ["Authorization Bearer", "X:Key", "X/Key", "X(Key)", "X@Key", ""] {
            assert!(!valid_header_name(bad), "`{bad}` is not an HTTP token");
            let error = check_header(bad).unwrap_err().to_string();
            assert_eq!(error, HEADER_NAME_MESSAGE);
            assert!(error.contains("Authorization") && error.contains("Value"), "the message points at the value");
        }
        for good in ["Authorization", "X-Api-Key", "X_API_KEY", "X.Api.Key", "X~Key", "X1"] {
            assert!(valid_header_name(good), "`{good}` is an HTTP token");
        }
        let saved = BTreeMap::from([("Authorization Bearer".to_owned(), "sk-live".to_owned())]);
        assert!(check_headers(&saved).is_err(), "a bad saved header is refused on test or edit");
        let fine = BTreeMap::from([("Authorization".to_owned(), "Bearer sk-live".to_owned())]);
        assert!(check_headers(&fine).is_ok());
    }

    #[test]
    fn prepared_headers_hide_every_value() {
        let (real, shown) =
            prepare(&json!({"Authorization": "Bearer verysecretkey", "X-Api-Key": "0123456789abcdef"})).unwrap();
        assert_eq!(real["Authorization"], "Bearer verysecretkey");
        let shown = shown.to_string();
        assert!(!shown.contains("verysecretkey"));
        assert!(!shown.contains("0123456789abcdef"));
        assert!(shown.contains("Authorization"));
    }
}
