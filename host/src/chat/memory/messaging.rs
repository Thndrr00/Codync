//! Messaging connections: facts kept per linked place, and one tool that posts to an allowed place.
//! From `OpenBot` messaging docs: a fact belongs to the connection (a WhatsApp number, a Telegram
//! bot, a Slack workspace, a Discord server), not to one conversation, and is never put in the
//! agent's own chat. The host keeps them in its `kv` store beside channel notes and sends through
//! the connections the owner configured; the decisions here are pure and tested below.

use crate::hub::Hub;
use crate::store::Store;
use anyhow::{Result, anyhow, bail};
use serde_json::{Value, json};
use std::sync::Arc;

/// OpenBot's per-connection caps: 32 facts of at most 500 characters each.
pub const MAX_FACTS: usize = 32;
pub const MAX_FACT_CHARS: usize = 500;
/// A post is prompt data, so it stays bounded before it reaches a channel.
const MAX_POST_CHARS: usize = 8_000;
const MAX_MENTIONS: usize = 10;

/// A connection, as the agent names it in a tool call: `provider:connection`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Place<'a> {
    pub provider: &'a str,
    pub connection: &'a str,
}

/// Parses one of the four places. The connection is the linked account, bot name, workspace or
/// server id, never a secret; `messaging_list_places` is the only way the agent learns these.
pub fn place(value: &str) -> Result<Place<'_>> {
    let value = value.trim();
    let (provider, connection) = value.split_once(':').unwrap_or(("", ""));
    if !matches!(provider, "whatsapp" | "telegram" | "slack" | "discord")
        || connection.is_empty()
        || connection.len() > 200
    {
        bail!("Use a place from messaging_list_places: a channel, a colon, then its connection.");
    }
    Ok(Place { provider, connection })
}

fn slot(provider: &str, connection: &str) -> String {
    format!("messaging.facts.{provider}.{connection}")
}

/// The facts of one connection, in the order they were learned.
pub fn facts(store: &Store, provider: &str, connection: &str) -> Vec<String> {
    store
        .kv_get(&slot(provider, connection))
        .and_then(|raw| serde_json::from_str::<Vec<String>>(&raw).ok())
        .unwrap_or_default()
}

fn save(store: &Store, provider: &str, connection: &str, list: &[String]) -> Result<()> {
    store.kv_set(&slot(provider, connection), &serde_json::to_string(list)?)
}

/// The decision behind `messaging_remember`: one trimmed, redacted, single-line fact, no copy of
/// one already there, and no new fact once `cap` is reached. Pure so the caps and the refusal have
/// tests of their own (OpenBot messaging.md).
pub fn messaging_fact(text: &str, existing: &[String], cap: usize) -> Result<String> {
    let fact = super::budget::redact_secrets(text.trim()).trim().to_owned();
    if fact.is_empty() {
        bail!("Write the fact first.");
    }
    if fact.contains('\n') || fact.contains('\r') {
        bail!("Write one fact, on one line.");
    }
    if fact.chars().count() > MAX_FACT_CHARS {
        bail!("A fact is at most {MAX_FACT_CHARS} characters.");
    }
    let lower = fact.to_lowercase();
    if existing.iter().any(|old| old.to_lowercase() == lower) {
        bail!("That fact is already here.");
    }
    if existing.len() >= cap {
        bail!("This connection has {cap} facts; forget one first.");
    }
    Ok(fact)
}

pub fn remember(store: &Store, provider: &str, connection: &str, text: &str) -> Result<Vec<String>> {
    let mut list = facts(store, provider, connection);
    list.push(messaging_fact(text, &list, MAX_FACTS)?);
    save(store, provider, connection, &list)?;
    Ok(list)
}

pub fn forget(store: &Store, provider: &str, connection: &str, index: usize) -> Result<Vec<String>> {
    let mut list = facts(store, provider, connection);
    if index >= list.len() {
        bail!("Unknown fact; use an index from the tool's last answer.");
    }
    list.remove(index);
    save(store, provider, connection, &list)?;
    Ok(list)
}

/// The block a channel prompt gets, and nothing else. Pure: the store call is in [`with_facts`].
pub fn opening(list: &[String]) -> String {
    if list.is_empty() {
        return String::new();
    }
    let mut out = String::from("[Connection memory]\n");
    for fact in list {
        out.push_str("- ");
        out.push_str(fact);
        out.push('\n');
    }
    out
}

/// Prepends a connection's facts to a channel prompt. Called only from the four channel ingress
/// builders, so a bot's own chat and its routines never see another connection's facts.
pub fn with_facts(store: &Store, provider: &str, connection: &str, text: String) -> String {
    let block = opening(&facts(store, provider, connection));
    if block.is_empty() { text } else { format!("{block}\n{text}") }
}

/// A place the owner configured, with the destinations the agent may use on it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlaceInfo {
    pub place: String,
    /// Chat ids, JIDs or channel ids, never tokens or names of people who did not opt in.
    pub allowed: Vec<String>,
    /// Bots this connection is bound to; empty means the number's default bot.
    pub bots: Vec<String>,
}

fn push_places(out: &mut Vec<PlaceInfo>, provider: &str, rows: Result<Vec<(String, Vec<String>, Vec<String>)>>) {
    if let Ok(rows) = rows {
        for (connection, allowed, bots) in rows {
            out.push(PlaceInfo { place: format!("{provider}:{connection}"), allowed, bots });
        }
    }
}

/// Every configured place, read from each channel's own settings. A failed read leaves that
/// connection out rather than letting a stale or corrupt setting widen what the agent can post to.
pub fn places(hub: &Hub) -> Vec<PlaceInfo> {
    let mut out = Vec::new();
    push_places(&mut out, "whatsapp", crate::whatsapp::places(&hub.store));
    push_places(&mut out, "telegram", crate::telegram::places(&hub.store));
    push_places(&mut out, "slack", crate::slack::places(&hub.store));
    push_places(&mut out, "discord", crate::discord::places(&hub.store));
    out
}

fn owned(place: &PlaceInfo, bot: &str) -> bool {
    place.bots.is_empty() || place.bots.iter().any(|id| id == bot)
}

fn known(hub: &Hub, bot: &str, value: &str) -> Result<PlaceInfo> {
    place(value)?;
    places(hub)
        .into_iter()
        .find(|known| known.place == value.trim() && owned(known, bot))
        .ok_or_else(|| anyhow!("That messaging connection is not configured for this bot."))
}

pub fn places_json(hub: &Hub) -> Value {
    let places = places(hub).into_iter().map(|p| json!({"place": p.place, "targets": p.allowed})).collect::<Vec<_>>();
    json!({"places": places})
}

/// The validated arguments of one post, before any I/O.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Target {
    pub provider: String,
    pub connection: String,
    pub to: String,
    pub text: String,
    pub reply_to: String,
    pub mentions: Vec<String>,
}

fn digits(value: &str) -> bool {
    (1..=32).contains(&value.len()) && value.bytes().all(|b| b.is_ascii_digit())
}

fn slack_id(value: &str, prefixes: &[u8]) -> bool {
    (1..=80).contains(&value.len())
        && value.as_bytes().first().is_some_and(|b| prefixes.contains(b))
        && value.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
}

/// The pure decision behind `messaging_post`: what place, destination, text, reply and mentions
/// the host may actually use. Shapes are checked per channel and inline credentials are redacted;
/// the allowlist check needs the store and is in [`prepare`].
pub fn post_target(value: &str, args: &Value) -> Result<Target> {
    let place = place(value)?;
    let to = args["target"].as_str().unwrap_or_default().trim();
    let reply_to = args["replyTo"].as_str().unwrap_or_default().trim();
    let to_ok = match place.provider {
        "whatsapp" => crate::whatsapp::delivery::jid(to),
        "telegram" => to.parse::<i64>().is_ok(),
        "slack" => slack_id(to, b"CGD"),
        _ => digits(to),
    };
    if !to_ok {
        bail!("Use a destination id from messaging_list_places.");
    }
    let reply_ok = if reply_to.is_empty() {
        true
    } else {
        match place.provider {
            "whatsapp" => !reply_to.contains(char::is_whitespace) && reply_to.len() <= 120,
            "telegram" => reply_to.parse::<i64>().is_ok(),
            "slack" => {
                let mut parts = reply_to.split('.');
                parts.next().is_some_and(|s| s.bytes().all(|b| b.is_ascii_digit()))
                    && parts.next().is_some_and(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
                    && parts.next().is_none()
            }
            _ => digits(reply_to),
        }
    };
    if !reply_ok {
        bail!("replyTo must be a message id from this place.");
    }
    let mut mentions = Vec::new();
    for mention in args["mentions"].as_array().into_iter().flatten() {
        let Some(given) = mention.as_str() else { bail!("mentions must be strings") };
        let id = given.trim();
        if id.is_empty() {
            continue;
        }
        let ok = match place.provider {
            "whatsapp" => crate::whatsapp::delivery::jid(id),
            "telegram" => {
                let username = id.trim_start_matches('@');
                id.parse::<i64>().is_ok()
                    || ((5..=32).contains(&username.len())
                        && username.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'))
            }
            "slack" => slack_id(id, b"UW"),
            _ => digits(id),
        };
        if !ok {
            bail!("A mention is not a user id on this place.");
        }
        if !mentions.iter().any(|old| old == id) {
            mentions.push(id.to_owned());
        }
        if mentions.len() > MAX_MENTIONS {
            bail!("At most {MAX_MENTIONS} mentions in one post.");
        }
    }
    let text = super::budget::redact_secrets(args["text"].as_str().unwrap_or_default().trim());
    if text.is_empty() {
        bail!("Write the message first.");
    }
    if text.chars().count() > MAX_POST_CHARS {
        bail!("A post is at most {MAX_POST_CHARS} characters.");
    }
    Ok(Target {
        provider: place.provider.to_owned(),
        connection: place.connection.to_owned(),
        to: to.to_owned(),
        text,
        reply_to: reply_to.to_owned(),
        mentions,
    })
}

/// A destination is usable only when the owner put it on this connection's allowlist.
pub fn allowed_target(target: &Target, allowed: &[String]) -> bool {
    allowed.iter().any(|id| id == &target.to)
}

/// Replaces saved secret values with a length marker, so a post can never carry a bot's key.
pub fn redact_values(text: &str, values: &[String]) -> String {
    let mut out = text.to_owned();
    for value in values {
        if value.len() >= 3 && out.contains(value) {
            out = out.replace(value, &format!("<redacted {} chars>", value.chars().count()));
        }
    }
    out
}

/// Mentions and replies the bridge can't attach to a message go in its text; the other three
/// platforms get them as native reply parameters.
pub fn render_text(target: &Target) -> String {
    let mut text = target.text.clone();
    if !target.mentions.is_empty() {
        let names = target
            .mentions
            .iter()
            .map(|id| match target.provider.as_str() {
                "telegram" => format!("@{}", id.trim_start_matches('@')),
                "slack" | "discord" => format!("<@{id}>"),
                _ => format!("@{}", id.split('@').next().unwrap_or(id)),
            })
            .collect::<Vec<_>>()
            .join(" ");
        text = format!("{names} {text}");
    }
    if target.provider == "whatsapp" && !target.reply_to.is_empty() {
        text = format!("[Replying to {}]\n{text}", target.reply_to);
    }
    text
}

/// Validates a call on a bot's own chat or a routine, with the same guard both paths use.
fn prepare(hub: &Hub, bot: &str, value: &str, args: &Value) -> Result<Target> {
    let known = known(hub, bot, value)?;
    let mut target = post_target(value, args)?;
    if !allowed_target(&target, &known.allowed) {
        bail!("That destination is not an allowed contact on this connection.");
    }
    let saved = crate::market::bot_secrets::for_bot(&hub.store, bot).unwrap_or_default();
    let values = saved
        .into_iter()
        .flat_map(|secret| {
            let totp = secret.totp().map(str::to_owned);
            let mut values = vec![secret.value];
            if let Some(totp) = totp {
                values.push(totp);
            }
            values
        })
        .filter(|value| value.len() >= 3)
        .collect::<Vec<_>>();
    target.text = redact_values(&target.text, &values);
    Ok(target)
}

/// Sends through the connection's existing outgoing path, never a new HTTP client here.
pub async fn send(hub: &Arc<Hub>, target: &Target) -> Result<()> {
    let text = render_text(target);
    match target.provider.as_str() {
        "whatsapp" => crate::whatsapp::delivery::post(hub, &target.connection, &target.to, &text).await,
        "telegram" => {
            let chat = target.to.parse::<i64>()?;
            crate::telegram::post(&hub.store, &target.connection, chat, &text, &target.reply_to).await
        }
        "slack" => crate::slack::post(&hub.store, &target.to, &text, &target.reply_to).await,
        "discord" => crate::discord::post(&hub.store, &target.to, &text, &target.reply_to).await,
        other => bail!("Unknown messaging place {other}."),
    }
}

/// The built-in `memory` server's messaging tools.
pub fn tools() -> Vec<Value> {
    vec![
        json!({
            "name": "messaging_list_places",
            "description": "List the WhatsApp, Telegram, Slack and Discord connections set up on this computer and the destinations each lets you post to. Call this before messaging_post or a connection-memory tool; never invent a place or a destination.",
            "inputSchema": {"type": "object", "properties": {}},
            "annotations": {"readOnlyHint": true},
        }),
        json!({
            "name": "messaging_remember",
            "description": "Save one lasting fact (at most 500 characters) about a messaging connection, seen in every conversation on that connection but never in your own chat. At most 32 facts per connection; use messaging_forget_memory to make room. The connection belongs to you.",
            "inputSchema": {"type": "object", "properties": {
                "place": {"type": "string", "description": "A place from messaging_list_places."},
                "fact": {"type": "string", "description": "One short fact; secrets are redacted."}
            }, "required": ["place", "fact"]},
        }),
        json!({
            "name": "messaging_forget_memory",
            "description": "Take one fact out of a messaging connection's memory by its index from the last messaging_remember answer.",
            "inputSchema": {"type": "object", "properties": {
                "place": {"type": "string"}, "index": {"type": "integer", "minimum": 0}
            }, "required": ["place", "index"]},
        }),
        json!({
            "name": "messaging_post",
            "description": "Post Markdown to a connected WhatsApp, Telegram, Slack or Discord destination, from your own chat or a routine. Use a place and a destination from messaging_list_places; replyTo answers a message id there and mentions @-names people. Secrets from your vault and inline credentials are redacted before the post leaves the host.",
            "inputSchema": {"type": "object", "properties": {
                "place": {"type": "string", "description": "A place from messaging_list_places."},
                "target": {"type": "string", "description": "An allowed chat, JID or channel id on that place."},
                "text": {"type": "string", "description": "The Markdown to post."},
                "replyTo": {"type": "string", "description": "The id of a message on that place to answer."},
                "mentions": {"type": "array", "items": {"type": "string"}, "description": "User ids or @usernames to mention."}
            }, "required": ["place", "target", "text"]},
        }),
    ]
}

/// Routines get the same posting path, but not the connection-memory tools: a routine runs in the
/// bot's own chat, where those facts are never shown.
pub fn routine_tools() -> Vec<Value> {
    tools()
        .into_iter()
        .filter(|tool| matches!(tool["name"].as_str(), Some("messaging_post" | "messaging_list_places")))
        .collect()
}

/// Called from the built-in `memory` server, on any of this bot's sessions.
pub async fn call(hub: &Arc<Hub>, bot: &str, name: &str, args: &Value) -> Result<Value> {
    match name {
        "messaging_list_places" => Ok(places_json(hub)),
        "messaging_remember" => {
            let value = args["place"].as_str().unwrap_or_default();
            known(hub, bot, value)?;
            let selected = place(value)?;
            Ok(json!({"facts": remember(&hub.store, selected.provider, selected.connection,
                args["fact"].as_str().unwrap_or_default())?}))
        }
        "messaging_forget_memory" => {
            let value = args["place"].as_str().unwrap_or_default();
            known(hub, bot, value)?;
            let selected = place(value)?;
            let index = usize::try_from(args["index"].as_i64().unwrap_or(-1))
                .map_err(|_| anyhow!("index must be a fact's index"))?;
            Ok(json!({"facts": forget(&hub.store, selected.provider, selected.connection, index)?}))
        }
        "messaging_post" => {
            let value = args["place"].as_str().unwrap_or_default();
            let target = prepare(hub, bot, value, args)?;
            send(hub, &target).await?;
            Ok(json!({"posted": true, "place": value, "target": target.to}))
        }
        other => bail!("unknown messaging tool: {other}"),
    }
}

/// The routine bridge is synchronous; it checks the same guard here and posts on the host runtime
/// so a run can report before a slow channel call finishes.
pub fn routine_call(hub: &Arc<Hub>, bot: &str, name: &str, args: &Value) -> Result<Value> {
    match name {
        "messaging_list_places" => Ok(places_json(hub)),
        "messaging_post" => {
            let value = args["place"].as_str().unwrap_or_default();
            let target = prepare(hub, bot, value, args)?;
            let handle = tokio::runtime::Handle::try_current()
                .map_err(|_| anyhow!("A messaging post needs the host's async runtime."))?;
            let hub = hub.clone();
            let to = target.to.clone();
            handle.spawn(async move {
                if let Err(error) = send(&hub, &target).await {
                    tracing::warn!(%error, place = %target.place(), "a routine's messaging post was not sent");
                }
            });
            Ok(json!({"queued": true, "place": value, "target": to}))
        }
        other => bail!("unknown routine messaging tool: {other}"),
    }
}

impl Target {
    fn place(&self) -> String {
        format!("{}:{}", self.provider, self.connection)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(value: serde_json::Value) -> Value {
        value
    }

    #[test]
    fn a_fact_is_trimmed_redacted_bounded_and_never_duplicated() {
        let existing = vec!["Prefers short answers".to_owned()];
        assert_eq!(messaging_fact("  Prefers morning calls  ", &[], MAX_FACTS).unwrap(), "Prefers morning calls");
        assert_eq!(
            messaging_fact("api_key = \"sk_live_1234567890abcdef\"", &[], MAX_FACTS).unwrap(),
            "api_key= <redacted 24 chars>",
            "a credential does not become a fact"
        );
        assert!(messaging_fact("   ", &[], MAX_FACTS).is_err());
        assert!(messaging_fact("two\nlines", &[], MAX_FACTS).is_err());
        assert!(messaging_fact(&"x".repeat(MAX_FACT_CHARS + 1), &[], MAX_FACTS).is_err());
        assert_eq!(
            messaging_fact(&"x".repeat(MAX_FACT_CHARS), &[], MAX_FACTS).unwrap().chars().count(),
            MAX_FACT_CHARS
        );
        assert!(messaging_fact("PREFERS SHORT ANSWERS", &existing, MAX_FACTS).is_err());
        assert!(messaging_fact("new", &existing, 1).is_err(), "a full connection makes room first");
        assert_eq!(messaging_fact("new", &existing, 2).unwrap(), "new");
    }

    #[test]
    fn a_place_names_exactly_one_configured_connection() {
        assert_eq!(place(" telegram:home ").unwrap().provider, "telegram");
        assert_eq!(place("whatsapp:5215@s.whatsapp.net").unwrap().connection, "5215@s.whatsapp.net");
        for bad in ["", "telegram", "signal:x", "telegram:", ":home", "slack:workspace:extra"] {
            assert!(place(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn a_post_target_checks_each_platforms_ids_reply_and_mentions() {
        let slack = post_target("slack:T1", &args(json!({"target":"C0123","text":"**done**"}))).unwrap();
        assert_eq!(slack.text, "**done**", "Markdown goes out as written");
        assert!(allowed_target(&slack, &["C0123".to_owned()]));
        assert!(!allowed_target(&slack, &["C999".to_owned()]));
        let replied = post_target(
            "slack:T1",
            &args(json!({"target":"C0123","text":"hi","replyTo":"1712.0004","mentions":["U1","U1"]})),
        )
        .unwrap();
        assert_eq!(replied.reply_to, "1712.0004");
        assert_eq!(replied.mentions, ["U1"], "a mention is not repeated");
        assert!(render_text(&replied).starts_with("<@U1> hi"));

        let telegram = post_target(
            "telegram:home",
            &args(json!({"target":"-10042","text":"hi","replyTo":"77","mentions":["@alice_bot"]})),
        )
        .unwrap();
        assert!(render_text(&telegram).starts_with("@alice_bot hi"));
        assert!(post_target("telegram:home", &args(json!({"target":"abc","text":"hi"}))).is_err());

        let discord =
            post_target("discord:42", &args(json!({"target":"123456","text":"hi","replyTo":"999","mentions":["123"]})))
                .unwrap();
        assert!(render_text(&discord).starts_with("<@123> hi"));
        assert!(post_target("discord:42", &args(json!({"target":"12 3","text":"hi"}))).is_err());

        let whatsapp = post_target(
            "whatsapp:5215@s.whatsapp.net",
            &args(json!({"target":"5216@s.whatsapp.net","text":"hi","replyTo":"BAE123","mentions":["5217@s.whatsapp.net"]})),
        )
        .unwrap();
        assert_eq!(render_text(&whatsapp), "[Replying to BAE123]\n@5217 hi");
        assert!(
            post_target("whatsapp:5215@s.whatsapp.net", &args(json!({"target":"5216","text":"hi"}))).is_err(),
            "a bare number is not a WhatsApp destination"
        );
    }

    #[test]
    fn an_invalid_post_is_refused_before_any_send() {
        for bad in [
            json!({"place":"slack:T1","target":"C1","text":""}),
            json!({"place":"slack:T1","target":"C1","text":"x","replyTo":"not-a-ts"}),
            json!({"place":"slack:T1","target":"C1","text":"x","mentions":["not an id"]}),
            json!({"place":"slack:T1","target":"C1","text":"x","mentions":["U1","U2","U3","U4","U5","U6","U7","U8","U9","U10","U11"]}),
            json!({"place":"slack:T1","target":"C1","text":"x".repeat(MAX_POST_CHARS + 1)}),
            json!({"place":"slack:T1","target":"C1","text":"x","mentions":[5]}),
        ] {
            let place = bad["place"].as_str().unwrap_or_default();
            assert!(post_target(place, &bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn inline_credentials_and_saved_secret_values_do_not_leave_the_host() {
        let redacted =
            post_target("slack:T1", &args(json!({"target":"C1","text":"token = ABCDEFGH and password: hunter22"})))
                .unwrap();
        assert!(!redacted.text.contains("ABCDEFGH"));
        assert!(!redacted.text.contains("hunter22"));
        let saved = vec!["sup3r-s3cret-value".to_owned(), "no".to_owned()];
        assert_eq!(
            redact_values("key sup3r-s3cret-value, word no", &saved),
            "key <redacted 18 chars>, word no",
            "values too short to be a secret are left alone"
        );
    }

    #[test]
    fn facts_are_scoped_to_a_connection_in_the_host_store() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let first = remember(&store, "telegram", "home", "Prefers evening calls").unwrap();
        assert_eq!(first, ["Prefers evening calls"]);
        assert!(facts(&store, "telegram", "work").is_empty(), "another bot is a different connection");
        assert!(facts(&store, "slack", "home").is_empty(), "another platform is a different connection");
        let added = remember(&store, "telegram", "home", "Ships on Fridays").unwrap();
        assert_eq!(added.len(), 2);
        assert_eq!(opening(&added), "[Connection memory]\n- Prefers evening calls\n- Ships on Fridays\n");
        assert_eq!(with_facts(&store, "telegram", "home", "hi".into()), format!("{}\nhi", opening(&added)));
        assert_eq!(with_facts(&store, "telegram", "work", "hi".into()), "hi");
        assert_eq!(forget(&store, "telegram", "home", 0).unwrap(), ["Ships on Fridays"]);
        assert!(forget(&store, "telegram", "home", 9).is_err());
    }

    #[test]
    fn routines_get_the_post_tool_but_not_the_connection_memory_tools() {
        let names = tools().into_iter().filter_map(|t| t["name"].as_str().map(str::to_owned)).collect::<Vec<_>>();
        assert!(names.contains(&"messaging_post".to_owned()));
        assert!(names.contains(&"messaging_remember".to_owned()));
        let routine =
            routine_tools().into_iter().filter_map(|t| t["name"].as_str().map(str::to_owned)).collect::<Vec<_>>();
        assert_eq!(routine, ["messaging_list_places", "messaging_post"]);
    }
}
