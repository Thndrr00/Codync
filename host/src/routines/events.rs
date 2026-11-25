//! The event triggers an editor may offer, and the filters each source takes.
//!
//! The host keeps the catalog so desktop, Android and the bot describe the same events. Stored
//! triggers stay valid when a source later loses an event: [`validate`] is used by the editor's
//! save, not by [`super::Trigger::validate`], which must keep older triggers savable.

use anyhow::{Result, bail};
use base64::Engine as _;
use hmac::{KeyInit as _, Mac as _};
use serde_json::{Value, json};

pub struct Filter {
    pub key: &'static str,
    /// The field's label, translated by the clients.
    pub label: &'static str,
    /// What to type, shown under the field.
    pub hint: &'static str,
}

pub struct Event {
    /// The `event` a payload carries: `<name>.<action>`, or `*` for every one.
    pub id: &'static str,
    pub label: &'static str,
}

pub struct Source {
    pub id: &'static str,
    pub label: &'static str,
    pub events: &'static [Event],
    pub filters: &'static [Filter],
}

const ANY: Event = Event { id: "*", label: "Any event" };
const MESSAGE_CONTAINS: Filter =
    Filter { key: "messageContains", label: "Text contains", hint: "Only when the text has this" };

pub const SOURCES: &[Source] = &[
    Source {
        id: "github",
        label: "GitHub",
        events: &[
            ANY,
            Event { id: "issues.opened", label: "Issue opened" },
            Event { id: "issues.closed", label: "Issue closed" },
            Event { id: "issue_comment.created", label: "Comment on an issue" },
            Event { id: "pull_request.opened", label: "Pull request opened" },
            Event { id: "pull_request.closed", label: "Pull request closed" },
            Event { id: "pull_request_review.submitted", label: "Pull request review" },
            Event { id: "pull_request_review_comment.created", label: "Comment on a review" },
            Event { id: "check_run.completed", label: "Checks finished" },
            Event { id: "push", label: "Push" },
            Event { id: "release.published", label: "Release published" },
            Event { id: "discussion.created", label: "Discussion opened" },
        ],
        filters: &[
            Filter { key: "repo", label: "Repository", hint: "owner/name" },
            Filter { key: "sender", label: "Sender", hint: "A GitHub login" },
            Filter { key: "conclusion", label: "Conclusion", hint: "failure for failed checks" },
            MESSAGE_CONTAINS,
        ],
    },
    Source {
        id: "slack",
        label: "Slack",
        events: &[ANY, Event { id: "app_mention", label: "Mention" }],
        filters: &[
            Filter { key: "channel", label: "Channel", hint: "A channel id, such as C0123ABCD" },
            Filter { key: "user", label: "Sender", hint: "A Slack user id" },
            MESSAGE_CONTAINS,
        ],
    },
    Source {
        id: "telegram",
        label: "Telegram",
        events: &[ANY, Event { id: "mention", label: "Mention" }],
        filters: &[
            Filter { key: "chat", label: "Chat", hint: "A chat id" },
            Filter { key: "user", label: "Sender", hint: "A Telegram user id" },
            MESSAGE_CONTAINS,
        ],
    },
    Source {
        id: "discord",
        label: "Discord",
        events: &[
            ANY,
            Event { id: "message.created", label: "Message posted" },
            Event { id: "message.mention", label: "Mention" },
        ],
        filters: &[
            Filter { key: "channel", label: "Channel", hint: "A channel id" },
            Filter { key: "user", label: "Sender", hint: "A Discord user id" },
            MESSAGE_CONTAINS,
        ],
    },
    Source {
        id: "whatsapp",
        label: "WhatsApp",
        events: &[
            ANY,
            Event { id: "message", label: "Message" },
            Event { id: "groupMention", label: "Mention in a group" },
        ],
        filters: &[
            Filter { key: "chat", label: "Chat", hint: "A chat id, such as 5215555555555@s.whatsapp.net" },
            Filter { key: "from", label: "Sender", hint: "A WhatsApp id" },
            MESSAGE_CONTAINS,
        ],
    },
];

pub fn source(id: &str) -> Option<&'static Source> {
    SOURCES.iter().find(|source| source.id == id)
}

/// Why `source`, `event` and `filters` can't be saved, or `None` when they can.
pub fn problem(source_id: &str, event: &str, filters: &std::collections::BTreeMap<String, String>) -> Option<String> {
    let Some(source) = source(source_id) else {
        // A source the catalog doesn't know (an older or bot-set trigger) still saves: its event and
        // filters can't be checked here, so they pass through.
        if source_id.is_empty() || event.is_empty() {
            return Some("Choose a service for this trigger.".into());
        }
        return filters
            .iter()
            .find(|(key, value)| key.trim().is_empty() || value.trim().is_empty())
            .map(|(key, _)| format!("The {key} filter is empty."));
    };
    if !source.events.iter().any(|known| known.id == event) {
        return Some(format!("Choose an event {} can send.", source.label));
    }
    if filters.len() > 20 {
        return Some("Use at most 20 filters.".into());
    }
    for (key, value) in filters {
        if !source.filters.iter().any(|filter| filter.key == key) {
            return Some(format!("{} has no filter named {key}.", source.label));
        }
        if value.trim().is_empty() || value.len() > 4000 {
            return Some(format!("The {key} filter is empty or too long."));
        }
    }
    None
}

/// Checks [`problem`] for a save or a preview.
pub fn validate(source_id: &str, event: &str, filters: &std::collections::BTreeMap<String, String>) -> Result<()> {
    match problem(source_id, event, filters) {
        Some(why) => bail!("{why}"),
        None => Ok(()),
    }
}

/// Whether a filter matches `payload`; the special keys read where the services put them.
pub fn matches(payload: &Value, key: &str, value: &str) -> bool {
    match key {
        "messageContains" => payload["text"].as_str().is_some_and(|text| text.contains(value)),
        "conclusion" => {
            ["check_run", "check_suite"].iter().any(|kind| payload[*kind]["conclusion"].as_str() == Some(value))
        }
        _ => payload[key].as_str() == Some(value),
    }
}

/// How far from the host's clock a Standard Webhooks delivery may be; older ones are replayed ones.
const WEBHOOK_SKEW_MS: i64 = 5 * 60 * 1000;
const MAX_WEBHOOK_ID: usize = 200;
const MAX_WEBHOOK_SIGNATURE: usize = 2048;

/// The bytes a Standard Webhooks sender signs: `<webhook-id>.<webhook-timestamp>.<body>`.
fn signed_content(id: &str, timestamp: &str, body: &[u8]) -> Vec<u8> {
    let mut content = Vec::with_capacity(id.len() + timestamp.len() + body.len() + 2);
    content.extend_from_slice(id.as_bytes());
    content.push(b'.');
    content.extend_from_slice(timestamp.as_bytes());
    content.push(b'.');
    content.extend_from_slice(body);
    content
}

/// A `whsec_<base64>` secret is the form Standard Webhooks libraries hand out; a plain string is
/// accepted too, so a sender that names its own secret can paste it as-is.
fn secret_bytes(secret: &str) -> Vec<u8> {
    secret
        .strip_prefix("whsec_")
        .and_then(|encoded| base64::engine::general_purpose::STANDARD.decode(encoded).ok())
        .unwrap_or_else(|| secret.as_bytes().to_vec())
}

/// The signature a test delivery carries, in the same form the sender's library produces.
pub fn sign_sender(secret: &str, id: &str, timestamp: &str, body: &[u8]) -> Option<String> {
    let mut mac = hmac::Hmac::<sha2::Sha256>::new_from_slice(&secret_bytes(secret)).ok()?;
    mac.update(&signed_content(id, timestamp, body));
    Some(format!("v1,{}", base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes())))
}

/// Whether a delivery was signed with the sender's own Standard Webhooks secret and is recent.
/// The body is part of the signed content, so the decision needs it; no error names the secret.
/// Pure, so the host's verification path and its tests use the same rule.
pub fn sender_secret_ok(headers: &super::hooks::Headers, secret: &str, body: &[u8], now_ms: i64) -> bool {
    let Some(id) = headers.get("webhook-id") else { return false };
    let Some(timestamp) = headers.get("webhook-timestamp") else { return false };
    let Some(signatures) = headers.get("webhook-signature") else { return false };
    if secret.trim().is_empty()
        || id.is_empty()
        || id.len() > MAX_WEBHOOK_ID
        || timestamp.len() > 20
        || signatures.len() > MAX_WEBHOOK_SIGNATURE
    {
        return false;
    }
    let Ok(at) = timestamp.parse::<i64>() else { return false };
    let Some(at_ms) = at.checked_mul(1000) else { return false };
    // Reject replays and clocks far ahead, without overflowing at i64's ends.
    if !now_ms.checked_sub(at_ms).is_some_and(|skew| (-WEBHOOK_SKEW_MS..=WEBHOOK_SKEW_MS).contains(&skew)) {
        return false;
    }
    let Ok(mut mac) = hmac::Hmac::<sha2::Sha256>::new_from_slice(&secret_bytes(secret)) else { return false };
    mac.update(&signed_content(id, timestamp, body));
    let expected = mac.finalize().into_bytes();
    signatures.split_ascii_whitespace().any(|entry| {
        entry
            .strip_prefix("v1,")
            .and_then(|encoded| base64::engine::general_purpose::STANDARD.decode(encoded).ok())
            .is_some_and(|given| crate::remote::crypto::ct_eq(&given, &expected))
    })
}

/// A sample delivery for the routine's event triggers, so the editor's test uses the same filters a
/// real sender has to match. A webhook-only routine gets the plain `{source:"test"}` event.
pub fn test_event(routine: &super::Routine, now_ms: i64) -> Value {
    let mut event = json!({"source": "test", "event": "test", "text": "Routine webhook test", "at": now_ms});
    for trigger in &routine.triggers {
        if let super::Trigger::Event { source, event: name, filters } = trigger {
            event["source"] = json!(source);
            event["event"] = json!(if name == "*" { "test" } else { name });
            for (key, value) in filters {
                // `messageContains` needs text that contains the wanted substring; an exact filter
                // is satisfied by its own value (the source puts ids and names in these fields).
                event[key.as_str()] = json!(if key == "messageContains" || !value.is_empty() { value } else { "test" });
            }
            break;
        }
    }
    event
}

/// The catalog the editors offer.
pub fn catalog() -> Value {
    json!({
        "sources": SOURCES.iter().map(|source| json!({
            "id": source.id,
            "label": source.label,
            "events": source.events.iter().map(|event| json!({"id": event.id, "label": event.label})).collect::<Vec<_>>(),
            "filters": source.filters.iter().map(|filter| json!({"key": filter.key, "label": filter.label, "hint": filter.hint})).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    })
}

/// A readable trigger line for `source` and `event`, when the catalog knows them.
pub fn describe(source_id: &str, event: &str) -> Option<String> {
    let source = source(source_id)?;
    let event = source.events.iter().find(|known| known.id == event)?;
    Some(format!("When {}: {}", source.label, event.label.to_lowercase()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine as _;
    use std::collections::BTreeMap;

    #[test]
    fn every_source_offers_any_event_and_known_filters() {
        for source in SOURCES {
            assert!(source.events.iter().any(|event| event.id == "*"), "{}", source.id);
            assert!(!source.filters.is_empty(), "{}", source.id);
        }
        assert!(source("github").is_some());
        assert!(source("nope").is_none());
    }

    #[test]
    fn validate_names_the_broken_field() {
        let empty = BTreeMap::new();
        assert!(validate("github", "issues.opened", &empty).is_ok());
        assert!(validate("github", "issues.assigned", &empty).is_err());
        assert!(validate("github", "issues.opened", &BTreeMap::from([("channel".into(), "C1".into())])).is_err());
        assert!(validate("github", "issues.opened", &BTreeMap::from([("repo".into(), " ".into())])).is_err());
        assert!(validate("slack", "*", &BTreeMap::from([("channel".into(), "C1".into())])).is_ok());
        assert!(
            validate("whatsapp", "message", &BTreeMap::from([("chat".into(), "5215555555555@s.whatsapp.net".into())]))
                .is_ok()
        );
        // A source outside the catalog keeps its event and filters as they are.
        assert!(validate("origin", "pull_request.opened", &BTreeMap::from([("repo".into(), "o/r".into())])).is_ok());
        assert!(validate("origin", "", &empty).is_err());
        assert!(validate("pagerduty", "incident.triggered", &empty).is_ok());
    }

    #[test]
    fn filters_read_the_payload_where_the_service_puts_them() {
        let failure = json!({"text": "Build failed", "repo": "o/r", "check_run": {"conclusion": "failure"}});
        assert!(matches(&failure, "repo", "o/r"));
        assert!(matches(&failure, "messageContains", "failed"));
        assert!(matches(&failure, "conclusion", "failure"));
        assert!(!matches(&failure, "conclusion", "success"));
        assert!(!matches(&failure, "repo", "o/other"));
    }

    #[test]
    fn description_uses_the_catalog() {
        assert_eq!(describe("github", "pull_request.opened").as_deref(), Some("When GitHub: pull request opened"));
        assert!(describe("github", "issues.assigned").is_none());
    }

    use super::super::hooks::Headers;

    fn signed_headers(secret: &str, id: &str, timestamp: &str, body: &[u8]) -> Headers {
        let mut headers = Headers::new();
        headers.insert("webhook-id".into(), id.into());
        headers.insert("webhook-timestamp".into(), timestamp.into());
        headers.insert("webhook-signature".into(), sign_sender(secret, id, timestamp, body).expect("a signature"));
        headers
    }

    #[test]
    fn a_sender_secret_accepts_only_a_recent_signature_of_the_exact_body() {
        const SECRET: &str = "senders-own-secret";
        let now = 1_700_000_000_000;
        let timestamp = "1700000000";
        let body = br#"{"action":"opened"}"#;
        let id = "msg-1";
        let headers = signed_headers(SECRET, id, timestamp, body);
        assert!(sender_secret_ok(&headers, SECRET, body, now));
        assert!(!sender_secret_ok(&headers, "another-secret", body, now));
        assert!(!sender_secret_ok(&headers, SECRET, br#"{"action":"closed"}"#, now), "the body is signed");
        assert!(!sender_secret_ok(&headers, "", body, now), "an empty secret proves nothing");

        let mut stale = signed_headers(SECRET, id, "1699999640", body); // six minutes ago
        assert!(!sender_secret_ok(&stale, SECRET, body, now));
        stale = signed_headers(SECRET, id, "1700000360", body); // six minutes ahead
        assert!(!sender_secret_ok(&stale, SECRET, body, now));
        let recent = signed_headers(SECRET, id, "1699999700", body); // five minutes ago is still in
        assert!(sender_secret_ok(&recent, SECRET, body, now));
    }

    #[test]
    fn a_sender_secret_needs_the_standard_headers_and_a_well_formed_signature() {
        let now = 1_700_000_000_000;
        let timestamp = "1700000000";
        let body = b"{}";
        let headers = signed_headers("secret", "msg-1", timestamp, body);
        assert!(sender_secret_ok(&headers, "secret", body, now));

        let mut missing = headers.clone();
        missing.remove("webhook-id");
        assert!(!sender_secret_ok(&missing, "secret", body, now));
        let mut malformed = headers.clone();
        malformed.insert("webhook-signature".into(), "v1,not-base64".into());
        assert!(!sender_secret_ok(&malformed, "secret", body, now));
        let mut unknown_version = headers.clone();
        unknown_version.insert("webhook-signature".into(), "v2,AAAA".into());
        assert!(!sender_secret_ok(&unknown_version, "secret", body, now));
        let mut no_time = headers.clone();
        no_time.insert("webhook-timestamp".into(), "yesterday".into());
        assert!(!sender_secret_ok(&no_time, "secret", body, now));
        // Rotating keys in flight: a sender may list several signatures, one of which is current.
        let mut rotated = headers.clone();
        let other = sign_sender("old-secret", "msg-1", timestamp, body).expect("a signature");
        rotated.insert("webhook-signature".into(), format!("{other} {}", headers["webhook-signature"]));
        assert!(sender_secret_ok(&rotated, "secret", body, now));

        // Libraries hand out `whsec_<base64>`; the same bytes must verify.
        let encoded = base64::engine::general_purpose::STANDARD.encode("secret");
        let whsec = format!("whsec_{encoded}");
        let with_prefix = signed_headers(&whsec, "msg-1", timestamp, body);
        assert!(sender_secret_ok(&with_prefix, &whsec, body, now));
    }

    #[test]
    fn a_test_event_matches_the_routines_event_filters() {
        let mut routine = super::super::tests::routine();
        routine.triggers = vec![super::super::Trigger::Event {
            source: "github".into(),
            event: "pull_request.opened".into(),
            filters: [("repo".into(), "o/r".into()), ("messageContains".into(), "fix".into())].into(),
        }];
        let event = test_event(&routine, 7_000);
        assert!(routine.triggers[0].matches(&event));
        assert_eq!(event["source"], "github");
        assert_eq!(event["event"], "pull_request.opened");
        assert_eq!(event["at"], 7_000);

        routine.triggers = vec![super::super::Trigger::Event {
            source: "slack".into(),
            event: "*".into(),
            filters: [("channel".into(), "C1".into())].into(),
        }];
        assert!(routine.triggers[0].matches(&test_event(&routine, 0)), "any event still matches a sample");

        routine.triggers = vec![super::super::Trigger::Webhook];
        assert_eq!(test_event(&routine, 0)["source"], "test", "a plain webhook gets a plain test event");
    }
}
