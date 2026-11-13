//! Webhook deliveries, from the local endpoint or the cloud's public one (spec §7.8): who may
//! send one (the routine's bearer key, a GitHub-style HMAC of the body, or a Standard Webhooks
//! signature such as Composio and Svix send), and the event the routine's triggers match and its
//! run sees.
//!
//! The cloud checks the same proof before it queues a delivery; the host checks again because
//! it alone decides what runs.
//!
//! The Standard Webhooks half is ported from `openbot-nightly/src/main/webhook-security.ts`
//! (Apache-2.0, see `docs/features/routines.md`): `v1,<base64 HMAC-SHA256>` over
//! `<webhook-id>.<webhook-timestamp>.<body>`, a five-minute clock window and, for a `whsec_…`
//! secret, the decoded bytes as well as the secret itself.

use base64::Engine as _;
use hmac::{KeyInit as _, Mac as _};
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub const MAX_BODY: usize = 64 * 1024;
const MAX_DELIVERY_ID: usize = 200;
/// A signature outside this window is a replay, even when the secret is right.
const MAX_CLOCK_SKEW_MS: i64 = 5 * 60 * 1000;
const STANDARD_SECRET: &str = "whsec_";
/// A key shorter than this is too easy to guess to be a signature key.
const MIN_STANDARD_KEY_BYTES: usize = 16;
const SHA256_BYTES: usize = 32;

/// Request headers by lowercase name.
pub type Headers = BTreeMap<String, String>;

/// Why a delivery didn't start a run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refused {
    /// Unknown routine or wrong key: told apart nowhere, so ids can't be probed.
    Unauthorized,
    Paused,
    /// The routine is already running; the sender (or the cloud queue) tries again later.
    Busy,
    Invalid(String),
}

impl Refused {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Unauthorized => "unauthorized",
            Self::Paused => "paused",
            Self::Busy => "busy",
            Self::Invalid(_) => "invalid",
        }
    }

    pub fn retry(&self) -> bool {
        matches!(self, Self::Busy)
    }
}

impl std::fmt::Display for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unauthorized => f.write_str("invalid webhook key or signature"),
            Self::Paused => f.write_str("routine is paused"),
            Self::Busy => f.write_str("routine already has an active run"),
            Self::Invalid(why) => f.write_str(why),
        }
    }
}

impl std::error::Error for Refused {}

/// `Authorization: Bearer <key>`, `X-Hub-Signature-256: sha256=<hex HMAC-SHA256(key, body)>`, or
/// the `webhook-id` / `webhook-timestamp` / `webhook-signature` headers of a Standard Webhooks
/// sender. `now` is milliseconds since the epoch; only the Standard form uses it.
pub fn authorized(key: &str, headers: &Headers, body: &[u8], now: i64) -> bool {
    if key.is_empty() {
        return false;
    }
    if let Some(given) = headers.get("authorization").and_then(|v| v.strip_prefix("Bearer "))
        && crate::remote::crypto::ct_eq(given.as_bytes(), key.as_bytes())
    {
        return true;
    }
    if standard(key, headers, body, now) {
        return true;
    }
    let Some(sig) = headers.get("x-hub-signature-256").and_then(|v| v.strip_prefix("sha256=")).and_then(hex) else {
        return false;
    };
    let Ok(mut mac) = hmac::Hmac::<sha2::Sha256>::new_from_slice(key.as_bytes()) else { return false };
    mac.update(body);
    mac.verify_slice(&sig).is_ok()
}

/// A Standard Webhooks delivery: the base64 HMAC-SHA256 of `<id>.<timestamp>.<body>` in one of the
/// space-separated `v1,` values, with the timestamp inside the clock window.
fn standard(key: &str, headers: &Headers, body: &[u8], now: i64) -> bool {
    let (Some(id), Some(timestamp), Some(signature)) =
        (headers.get("webhook-id"), headers.get("webhook-timestamp"), headers.get("webhook-signature"))
    else {
        return false;
    };
    if !delivery_id_ok(id) || !timestamp_ok(timestamp, now) {
        return false;
    }
    let supplied: Vec<Vec<u8>> = signature
        .split(' ')
        .filter_map(|entry| entry.strip_prefix("v1,"))
        .filter_map(decode)
        .filter(|bytes| bytes.len() == SHA256_BYTES)
        .collect();
    if supplied.is_empty() {
        return false;
    }
    let signed = format!("{id}.{timestamp}.");
    standard_keys(key).into_iter().any(|key| {
        let Ok(mut mac) = hmac::Hmac::<sha2::Sha256>::new_from_slice(&key) else { return false };
        mac.update(signed.as_bytes());
        mac.update(body);
        let digest = mac.finalize().into_bytes();
        supplied.iter().any(|given| crate::remote::crypto::ct_eq(digest.as_slice(), given))
    })
}

/// The HMAC keys a Standard Webhooks secret stands for: the secret's own bytes, and — for the
/// `whsec_` form the specification defines — the bytes it encodes. `whsec_` secrets older senders
/// made in base64url also count, since a sender signs with the same bytes either way.
fn standard_keys(key: &str) -> Vec<Vec<u8>> {
    let mut keys = vec![key.as_bytes().to_vec()];
    let Some(encoded) = key.strip_prefix(STANDARD_SECRET) else { return keys };
    for engine in [&base64::engine::general_purpose::STANDARD, &base64::engine::general_purpose::STANDARD_NO_PAD] {
        let Ok(bytes) = engine.decode(encoded) else { continue };
        if bytes.len() >= MIN_STANDARD_KEY_BYTES && !keys.contains(&bytes) {
            keys.push(bytes);
        }
    }
    let Ok(bytes) = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(encoded) else { return keys };
    if bytes.len() >= MIN_STANDARD_KEY_BYTES && !keys.contains(&bytes) {
        keys.push(bytes);
    }
    keys
}

fn decode(value: &str) -> Option<Vec<u8>> {
    base64::engine::general_purpose::STANDARD
        .decode(value)
        .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(value))
        .ok()
}

/// Unix seconds as ASCII digits, no more than twelve of them, close enough to `now`.
fn timestamp_ok(timestamp: &str, now: i64) -> bool {
    if timestamp.is_empty() || timestamp.len() > 12 || !timestamp.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    timestamp.parse::<i64>().is_ok_and(|seconds| (seconds * 1000 - now).abs() <= MAX_CLOCK_SKEW_MS)
}

fn delivery_id_ok(id: &str) -> bool {
    !id.is_empty() && id.len() <= MAX_DELIVERY_ID && id.bytes().all(|b| b.is_ascii_graphic())
}

fn hex(s: &str) -> Option<Vec<u8>> {
    if s.len() != 64 {
        return None;
    }
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok()).collect()
}

/// The sender's id for this delivery, so a retry never runs twice.
pub fn delivery_id(headers: &Headers) -> Result<Option<String>, Refused> {
    let id =
        ["x-delivery-id", "x-github-delivery", "webhook-id", "idempotency-key"].iter().find_map(|h| headers.get(*h));
    match id {
        None => Ok(None),
        Some(id) if delivery_id_ok(id) => Ok(Some(id.clone())),
        Some(_) => Err(Refused::Invalid("invalid delivery id".into())),
    }
}

/// What a routine sees and its event triggers match; `None` for a GitHub `ping`, which only
/// checks the connection.
///
/// A JSON object is the event as-is; GitHub deliveries become `{source:"github", event:
/// "<event>.<action>", repo, sender, text, url, payload}`; anything else is `{text}`.
pub fn event(headers: &Headers, body: &[u8]) -> Result<Option<Value>, Refused> {
    if body.len() > MAX_BODY {
        return Err(Refused::Invalid("event too large".into()));
    }
    let parsed: Option<Value> =
        if body.iter().all(u8::is_ascii_whitespace) { Some(json!({})) } else { serde_json::from_slice(body).ok() };
    if let Some(name) = headers.get("x-github-event") {
        if name == "ping" {
            return Ok(None);
        }
        let payload = parsed.ok_or_else(|| Refused::Invalid("GitHub deliveries must use application/json".into()))?;
        return Ok(Some(github(name, &payload)));
    }
    Ok(Some(match parsed {
        Some(v @ Value::Object(_)) => v,
        Some(other) => json!({ "payload": other }),
        None => json!({ "text": String::from_utf8_lossy(body) }),
    }))
}

fn github(name: &str, payload: &Value) -> Value {
    let event = payload["action"].as_str().map_or_else(|| name.to_owned(), |action| format!("{name}.{action}"));
    let first = |paths: &[&[&str]]| {
        paths.iter().find_map(|path| {
            let v = path.iter().fold(payload, |v, k| &v[*k]);
            v.as_str().filter(|s| !s.is_empty()).map(str::to_owned)
        })
    };
    let text = first(&[
        &["pull_request", "title"],
        &["issue", "title"],
        &["discussion", "title"],
        &["release", "name"],
        &["release", "tag_name"],
        &["comment", "body"],
        &["review", "body"],
        &["head_commit", "message"],
        &["workflow_run", "name"],
    ]);
    let url = first(&[
        &["comment", "html_url"],
        &["review", "html_url"],
        &["pull_request", "html_url"],
        &["issue", "html_url"],
        &["discussion", "html_url"],
        &["release", "html_url"],
        &["workflow_run", "html_url"],
        &["compare"],
    ]);
    json!({
        "source": "github",
        "event": event,
        "repo": payload["repository"]["full_name"],
        "sender": payload["sender"]["login"],
        "text": text,
        "url": url,
        "payload": payload,
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    const KEY: &str = "0123456789abcdef0123456789abcdef";
    const NOW: i64 = 1_775_000_000_000;
    const STAMP: &str = "1775000000";

    fn headers(pairs: &[(&str, &str)]) -> Headers {
        pairs.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect()
    }

    fn signature(key: &str, body: &[u8]) -> String {
        let mut mac = hmac::Hmac::<sha2::Sha256>::new_from_slice(key.as_bytes()).unwrap();
        mac.update(body);
        let bytes = mac.finalize().into_bytes();
        bytes.iter().fold(String::from("sha256="), |mut s, b| {
            use std::fmt::Write as _;
            let _ = write!(s, "{b:02x}");
            s
        })
    }

    /// What a Standard Webhooks sender signs and sends.
    fn standard(key: &[u8], id: &str, body: &[u8]) -> Headers {
        use base64::Engine as _;
        let mut mac = hmac::Hmac::<sha2::Sha256>::new_from_slice(key).unwrap();
        mac.update(format!("{id}.{STAMP}.").as_bytes());
        mac.update(body);
        let digest = base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes());
        headers(&[("webhook-id", id), ("webhook-timestamp", STAMP), ("webhook-signature", &format!("v1,{digest}"))])
    }

    #[test]
    fn bearer_key_or_signature_of_the_exact_body() {
        let body = br#"{"action":"opened"}"#;
        assert!(authorized(KEY, &headers(&[("authorization", &format!("Bearer {KEY}"))]), body, NOW));
        assert!(!authorized(KEY, &headers(&[("authorization", "Bearer nope")]), body, NOW));
        assert!(!authorized(KEY, &headers(&[]), body, NOW));
        let sig = signature(KEY, body);
        assert!(authorized(KEY, &headers(&[("x-hub-signature-256", &sig)]), body, NOW));
        assert!(!authorized(KEY, &headers(&[("x-hub-signature-256", &sig)]), b"{}", NOW));
        assert!(!authorized(KEY, &headers(&[("x-hub-signature-256", &signature("other", body))]), body, NOW));
        assert!(!authorized(KEY, &headers(&[("x-hub-signature-256", "sha256=zz")]), body, NOW));
        assert!(!authorized("", &headers(&[("authorization", "Bearer ")]), body, NOW), "a cleared key never matches");
    }

    #[test]
    fn a_standard_webhooks_signature_from_the_routines_own_secret() {
        let body = br#"{"type":"composio.trigger.message"}"#;
        assert!(authorized(KEY, &standard(KEY.as_bytes(), "msg_1", body), body, NOW));
        assert!(!authorized(KEY, &standard(KEY.as_bytes(), "msg_1", body), b"{}", NOW), "the body is signed");
        assert!(!authorized(KEY, &standard(KEY.as_bytes(), "msg_2", body), body, NOW), "so is the delivery id");
        assert!(!authorized(KEY, &standard(b"another-key-another-key", "msg_1", body), body, NOW));
        let late = standard(KEY.as_bytes(), "msg_1", body);
        assert!(authorized(KEY, &late, body, NOW + 4 * 60 * 1000), "four minutes late is still fresh");
        assert!(!authorized(KEY, &late, body, NOW + 6 * 60 * 1000), "six minutes late is a replay");
        let wrong_stamp =
            headers(&[("webhook-id", "msg_1"), ("webhook-timestamp", "soon"), ("webhook-signature", "v1,x")]);
        assert!(!authorized(KEY, &wrong_stamp, body, NOW));
        let other_version =
            headers(&[("webhook-id", "msg_1"), ("webhook-timestamp", STAMP), ("webhook-signature", "v2,abc")]);
        assert!(!authorized(KEY, &other_version, body, NOW));
    }

    #[test]
    fn a_pasted_sender_secret_signs_with_the_bytes_it_encodes() {
        use base64::Engine as _;
        let bytes = [7u8; 32];
        let secret = format!("whsec_{}", base64::engine::general_purpose::STANDARD.encode(bytes));
        let body = br#"{"type":"invoice.paid"}"#;
        assert!(authorized(&secret, &standard(&bytes, "msg_9", body), body, NOW), "the decoded secret");
        assert!(authorized(&secret, &standard(secret.as_bytes(), "msg_9", body), body, NOW), "the secret as typed");
        let unpadded = format!("whsec_{}", base64::engine::general_purpose::STANDARD_NO_PAD.encode(bytes));
        assert!(authorized(&unpadded, &standard(&bytes, "msg_9", body), body, NOW));
        let url_safe = format!("whsec_{}", base64::engine::general_purpose::URL_SAFE_NO_PAD.encode([251u8; 32]));
        assert!(authorized(&url_safe, &standard(&[251u8; 32], "msg_9", body), body, NOW));
    }

    #[test]
    fn github_deliveries_become_events_triggers_can_match() {
        let body = br#"{"action":"opened","pull_request":{"title":"Fix login","html_url":"https://github.com/o/r/pull/1"},"repository":{"full_name":"o/r"},"sender":{"login":"kai"}}"#;
        let e = event(&headers(&[("x-github-event", "pull_request")]), body).unwrap().unwrap();
        assert_eq!(e["source"], "github");
        assert_eq!(e["event"], "pull_request.opened");
        assert_eq!(e["repo"], "o/r");
        assert_eq!(e["text"], "Fix login");
        assert_eq!(e["url"], "https://github.com/o/r/pull/1");
        assert_eq!(e["payload"]["sender"]["login"], "kai");
        let push = event(&headers(&[("x-github-event", "push")]), br#"{"head_commit":{"message":"m"},"compare":"c"}"#)
            .unwrap()
            .unwrap();
        assert_eq!(
            (push["event"].as_str(), push["text"].as_str(), push["url"].as_str()),
            (Some("push"), Some("m"), Some("c"))
        );
        assert_eq!(event(&headers(&[("x-github-event", "ping")]), b"{}").unwrap(), None);
        assert!(event(&headers(&[("x-github-event", "push")]), b"payload=%7B%7D").is_err());
    }

    #[test]
    fn other_bodies_keep_their_content() {
        let none = headers(&[]);
        assert_eq!(event(&none, br#"{"a":1}"#).unwrap(), Some(json!({"a":1})));
        assert_eq!(event(&none, b"[1]").unwrap(), Some(json!({"payload":[1]})));
        assert_eq!(event(&none, b"disk full").unwrap(), Some(json!({"text":"disk full"})));
        assert_eq!(event(&none, b"  ").unwrap(), Some(json!({})));
        assert!(event(&none, &vec![b'x'; MAX_BODY + 1]).is_err());
    }

    #[test]
    fn delivery_ids_come_from_the_usual_headers() {
        assert_eq!(delivery_id(&headers(&[])), Ok(None));
        assert_eq!(delivery_id(&headers(&[("x-github-delivery", "g-1")])), Ok(Some("g-1".into())));
        assert_eq!(delivery_id(&headers(&[("x-delivery-id", "a"), ("idempotency-key", "b")])), Ok(Some("a".into())));
        assert_eq!(delivery_id(&headers(&[("webhook-id", "msg_1")])), Ok(Some("msg_1".into())));
        assert!(delivery_id(&headers(&[("x-delivery-id", "has space")])).is_err());
        assert!(delivery_id(&headers(&[("x-delivery-id", &"x".repeat(201))])).is_err());
    }
}
