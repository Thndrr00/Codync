//! Deterministic inspection of a tool call's arguments for credentials.
//!
//! WHY THIS EXISTS. A connector call carries whatever the bot was told to send, sometimes a key
//! pasted into the conversation earlier. The call row already shows the arguments' shape, but
//! nothing says a credential went out with it.
//!
//! WHAT IT LOOKS FOR. Deliberately credentials, not general private data: a broad email or
//! phone-number matcher would flag ordinary connector work and turn a security boundary into a
//! source of false assurances.
//!
//! WHICH CALLS. Ones that can take a credential off this computer: an MCP call (`mcp__…`,
//! `server.tool`) or a terminal command. A local read or edit carries no connector's key, and its
//! file body would only reach the size limit.
//!
//! WHAT IT KEEPS. Only a category and a structural path. The matched value is never copied, so a
//! key smuggled in a property name still leaves nothing behind in the entry, the log or an audit
//! row. Ported from `CopilotKit/OpenBot`'s `content-governance.ts` (MIT; see `NOTICE.md`).

use base64::Engine;
use serde_json::{Value, json};

/// What kind of credential a finding is, in words a client can translate.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Category {
    /// A field the schema names: a password, a token, an API key.
    Field,
    /// A PEM private key block.
    PrivateKey,
    /// A key a provider's own format gives away: `sk-…`, `ghp_…`, `AKIA…`, `xoxb-…`.
    ProviderToken,
    /// An `Authorization: Basic`/`Bearer` value.
    AuthorizationHeader,
}

impl Category {
    fn wire(self) -> &'static str {
        match self {
            Self::Field => "credential_field",
            Self::PrivateKey => "private_key",
            Self::ProviderToken => "provider_token",
            Self::AuthorizationHeader => "authorization_header",
        }
    }
}

const MAX_NODES: usize = 2_000;
const MAX_DEPTH: usize = 20;
const MAX_FINDINGS: usize = 20;
const MAX_STRING_LENGTH: usize = 64 * 1024;

/// Spellings of a field already meaning a credential: `passwd` is `password`, `api_secret` is
/// `secret`, `ssh_key` is `private_key`. A call carrying one carries the same thing renamed. The
/// `x-` prefix is folded away by `normalized`, so `x-api-key` lands on `api_key` by itself.
const SENSITIVE_FIELDS: [&str; 34] = [
    "access_token",
    "accesstoken",
    "api_key",
    "api_secret",
    "apikey",
    "apisecret",
    "auth_token",
    "authorization",
    "authtoken",
    "bearer_token",
    "bearertoken",
    "client_secret",
    "clientsecret",
    "credential",
    "credentials",
    "id_token",
    "idtoken",
    "passwd",
    "password",
    "private_key",
    "privatekey",
    "pwd",
    "refresh_token",
    "refreshtoken",
    "secret",
    "secret_key",
    "secretkey",
    "session_token",
    "sessiontoken",
    "signing_key",
    "signingkey",
    "ssh_key",
    "sshkey",
    "token",
];

/// The caution written on a call's entry data, or `None` when its arguments read clean.
///
/// `{reason, findings: [{category, path}]}`; `reason` is `sensitive_content`, or
/// `inspection_limit` when the arguments were too large or deep to finish.
pub(super) fn inspect(args: &Value) -> Option<Value> {
    if !args.is_object() {
        return None;
    }
    let mut visit = Visit { nodes: 0, findings: Vec::new() };
    if !visit.value(args, "$", 0) {
        return Some(caution("inspection_limit", Vec::new()));
    }
    (!visit.findings.is_empty()).then(|| caution("sensitive_content", visit.findings))
}

/// Writes the caution for a fresh set of arguments, and drops an earlier one when they read clean:
/// an update that narrows a call's arguments must not leave the first set's warning behind. Returns
/// whether a warning that wasn't there before was written, so a call is reported once.
pub(super) fn mark(data: &mut Value, args: &Value) -> bool {
    // Some adapters hand the arguments over as a JSON string rather than an object.
    let parsed = args.as_str().and_then(|text| serde_json::from_str::<Value>(text).ok());
    let args = parsed.as_ref().unwrap_or(args);
    if !args.is_object() || !leaves_machine(data) {
        return false;
    }
    let had = data.get("caution").is_some();
    match inspect(args) {
        Some(caution) => {
            data["caution"] = caution;
            !had
        }
        None => {
            if let Value::Object(map) = data {
                map.remove("caution");
            }
            false
        }
    }
}

/// A call that can take an argument off this computer: an MCP call, whatever the adapter titles it
/// with, or a terminal command. A local read or edit stays here.
fn leaves_machine(data: &Value) -> bool {
    let title = data["title"].as_str().unwrap_or_default();
    if title.starts_with("mcp__") || data["toolKind"] == "execute" {
        return true;
    }
    title
        .split_once('.')
        .is_some_and(|(server, tool)| !server.is_empty() && !tool.is_empty() && !server.contains(char::is_whitespace))
}

fn caution(reason: &str, findings: Vec<Value>) -> Value {
    json!({ "reason": reason, "findings": findings })
}

struct Visit {
    nodes: usize,
    findings: Vec<Value>,
}

impl Visit {
    /// False when the size or depth limit was reached, which fails the whole inspection: a partial
    /// walk read part of the arguments and would read as safe when it isn't.
    fn value(&mut self, value: &Value, path: &str, depth: usize) -> bool {
        self.nodes += 1;
        if self.nodes > MAX_NODES || depth > MAX_DEPTH {
            return false;
        }
        match value {
            Value::String(text) => {
                if text.len() > MAX_STRING_LENGTH {
                    return false;
                }
                if let Some(category) = category_of(text) {
                    self.push(category, path);
                }
                true
            }
            Value::Array(items) => (0..items.len()).all(|i| self.value(&items[i], &format!("{path}[{i}]"), depth + 1)),
            Value::Object(map) => {
                for (key, child) in map {
                    if key.len() > MAX_STRING_LENGTH {
                        return false;
                    }
                    let category = category_of(key);
                    let child_path = format!("{path}.{}", segment(key, category.is_some()));
                    if let Some(category) = category {
                        self.push(category, &child_path);
                    }
                    if sensitive_field(key) && !child.is_null() && child.as_str() != Some("") {
                        self.push(Category::Field, &child_path);
                        continue;
                    }
                    if !self.value(child, &child_path, depth + 1) {
                        return false;
                    }
                }
                true
            }
            _ => true,
        }
    }

    fn push(&mut self, category: Category, path: &str) {
        if self.findings.len() < MAX_FINDINGS {
            self.findings.push(json!({ "category": category.wire(), "path": path }));
        }
    }
}

/// A path is audit metadata, so it cannot repeat arbitrary argument keys. Ordinary schema-like
/// names stay useful; everything else becomes a structural marker. A credential smuggled in a
/// property name is therefore detected but never copied into the finding that records it.
fn segment(key: &str, credential: bool) -> &str {
    if credential {
        return "[credential]";
    }
    let mut chars = key.chars();
    let starts_well = chars.next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_');
    if starts_well && key.len() <= 64 && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        key
    } else {
        "[property]"
    }
}

fn normalized(key: &str) -> String {
    let normalized: String = key
        .chars()
        .map(|c| if matches!(c, '-' | '.' | '_') || c.is_whitespace() { '_' } else { c.to_ascii_lowercase() })
        .collect();
    match normalized.strip_prefix("x_") {
        Some(rest) => rest.to_owned(),
        None => normalized,
    }
}

fn sensitive_field(key: &str) -> bool {
    SENSITIVE_FIELDS.contains(&normalized(key).as_str())
}

fn category_of(text: &str) -> Option<Category> {
    if private_key(text) {
        return Some(Category::PrivateKey);
    }
    if authorization_header(text) {
        return Some(Category::AuthorizationHeader);
    }
    provider_token(text).then_some(Category::ProviderToken)
}

/// `-----BEGIN [RSA |EC |OPENSSH |… ]PRIVATE KEY-----`.
fn private_key(text: &str) -> bool {
    /// `-----BEGIN `.
    const BEGIN: usize = 11;
    let mut from = 0;
    while let Some(at) = text[from..].find("-----BEGIN ") {
        let rest = &text[from + at + BEGIN..];
        if rest.starts_with("PRIVATE KEY-----") {
            return true;
        }
        if let Some(label) = rest.find("PRIVATE KEY-----") {
            let between = &rest[..label];
            let well_formed = !between.is_empty()
                && between.len() <= 32
                && between.ends_with(' ')
                && between.chars().all(|c| c.is_ascii_uppercase() || c == ' ');
            if well_formed {
                return true;
            }
        }
        from += at + BEGIN;
    }
    false
}

/// A `Basic` header that decodes to `user:password`, or a `Bearer` value shaped like a token.
fn authorization_header(text: &str) -> bool {
    basic_credential(text) || bearer_credential(text)
}

fn after_scheme<'a>(text: &'a str, scheme: &str) -> Option<&'a str> {
    let text = text.trim_start();
    let end = text.find(char::is_whitespace)?;
    text[..end].eq_ignore_ascii_case(scheme).then(|| text[end..].trim_start())
}

fn basic_credential(text: &str) -> bool {
    let Some(value) = after_scheme(text, "basic") else { return false };
    let end = value.find(|c: char| !(c.is_ascii_alphanumeric() || c == '+' || c == '/')).unwrap_or(value.len());
    if end == 0 {
        return false;
    }
    // `{0,2}` padding: a third `=` is left to the look-ahead and rejects the value.
    let mut padded = end;
    while value.as_bytes().get(padded) == Some(&b'=') && padded - end < 2 {
        padded += 1;
    }
    if value[padded..].chars().next().is_some_and(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '/' | '=')) {
        return false;
    }
    let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(&value[..padded]) else { return false };
    let Ok(decoded) = std::str::from_utf8(&bytes) else { return false };
    decoded.contains(':') && !decoded.chars().any(char::is_control)
}

fn bearer_credential(text: &str) -> bool {
    let Some(value) = after_scheme(text, "bearer") else { return false };
    let end = value.find(|c: char| !token_char(c)).unwrap_or(value.len());
    let mut padded = end;
    while value.as_bytes().get(padded) == Some(&b'=') {
        padded += 1;
    }
    if value[padded..].chars().next().is_some_and(|c| token_char(c) || c == '=') {
        return false;
    }
    let token = &value[..end];
    if token.chars().count() < 16 {
        return false;
    }
    // A run of letters alone reads as an ordinary word; punctuation or mixed case reads as a token.
    token.chars().any(|c| matches!(c, '0'..='9' | '.' | '_' | '~' | '+' | '/' | '=' | '-'))
        || (token.chars().any(|c| c.is_ascii_lowercase()) && token.chars().skip(1).any(|c| c.is_ascii_uppercase()))
}

fn token_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '~' | '+' | '/' | '-')
}

fn provider_token(text: &str) -> bool {
    prefixed(text, "sk-", 20, |c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
        || prefixed(text, "github_pat_", 20, |c| c.is_ascii_alphanumeric() || c == '_')
        || github(text)
        || aws(text)
        || slack(text)
}

/// `\b`, as the patterns spell it: exactly one side of the position is a word character.
fn boundary(text: &str, at: usize) -> bool {
    let before = text[..at].chars().next_back().is_some_and(word);
    let after = text[at..].chars().next().is_some_and(word);
    before != after
}

fn word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// The end of a run of at least `min` characters `ok`, at or after `from`, where a word boundary
/// follows.
fn run(text: &str, from: usize, min: usize, ok: impl Fn(char) -> bool) -> Option<usize> {
    let mut seen = 0;
    for (i, c) in text[from..].char_indices() {
        if !ok(c) {
            break;
        }
        seen += 1;
        let end = from + i + c.len_utf8();
        if seen >= min && boundary(text, end) {
            return Some(end);
        }
    }
    None
}

fn prefixed(text: &str, prefix: &str, min: usize, ok: impl Fn(char) -> bool) -> bool {
    text.match_indices(prefix).any(|(at, _)| boundary(text, at) && run(text, at + prefix.len(), min, &ok).is_some())
}

/// `gh[pousr]_…`: a GitHub token of any kind.
fn github(text: &str) -> bool {
    text.match_indices("gh").any(|(at, _)| {
        let mut rest = text[at + 2..].chars();
        let kind_ok = rest.next().is_some_and(|c| matches!(c, 'p' | 'o' | 'u' | 's' | 'r'));
        boundary(text, at)
            && kind_ok
            && rest.next() == Some('_')
            && run(text, at + 4, 20, |c| c.is_ascii_alphanumeric()).is_some()
    })
}

/// `AKIA` and sixteen capitals or digits: an AWS access key id.
fn aws(text: &str) -> bool {
    text.match_indices("AKIA").any(|(at, _)| {
        boundary(text, at) && run(text, at + 4, 16, |c| c.is_ascii_uppercase() || c.is_ascii_digit()) == Some(at + 20)
    })
}

/// `xox[baprs]-…`: a Slack token.
fn slack(text: &str) -> bool {
    text.match_indices("xox").any(|(at, _)| {
        let mut rest = text[at + 3..].chars();
        let kind_ok = rest.next().is_some_and(|c| matches!(c, 'b' | 'a' | 'p' | 'r' | 's'));
        boundary(text, at)
            && kind_ok
            && rest.next() == Some('-')
            && run(text, at + 5, 10, |c| c.is_ascii_alphanumeric() || c == '-').is_some()
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn categories(args: Value) -> Vec<String> {
        inspect(&args).unwrap()["findings"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["category"].as_str().unwrap().to_owned())
            .collect()
    }

    fn paths(args: Value) -> Vec<String> {
        inspect(&args).unwrap()["findings"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["path"].as_str().unwrap().to_owned())
            .collect()
    }

    #[test]
    fn an_ordinary_call_is_clean() {
        assert!(inspect(&json!({"query": "hello", "limit": 5, "nested": {"flag": true}})).is_none());
        assert!(inspect(&json!({"password": ""})).is_none());
        assert!(inspect(&json!({"token": null})).is_none());
        assert!(inspect(&json!("not an object")).is_none());
    }

    #[test]
    fn a_named_field_is_a_credential_without_copying_its_value() {
        let args = json!({"user": "me", "password": "hunter2", "nested": {"x-api-key": "abc"}});
        assert_eq!(paths(args), ["$.password", "$.nested.x-api-key"]);
        assert_eq!(categories(json!({"password": "hunter2"})), ["credential_field"]);
        let caution = inspect(&json!({"password": "hunter2"})).unwrap();
        assert_eq!(caution["reason"], "sensitive_content");
        assert!(!caution.to_string().contains("hunter2"));
    }

    #[test]
    fn field_spellings_meet_in_one_name() {
        for key in ["api-key", "x-api-key", "API_KEY", "Api Key", "api.key", "apiKey"] {
            assert!(sensitive_field(key), "{key}");
        }
        assert!(!sensitive_field("apiKeys"));
        assert!(!sensitive_field("tokenCount"));
    }

    #[test]
    fn a_credential_in_a_property_name_is_flagged_but_not_copied() {
        let args = json!({"sk-live-abcdefghijklmnopqrstuvwxyz012345": "value"});
        let paths = paths(args);
        assert_eq!(paths, ["$.[credential]"]);
        assert!(!paths.iter().any(|p| p.contains("sk-live")));
    }

    #[test]
    fn a_pem_private_key_is_a_credential() {
        let pem = "-----BEGIN RSA PRIVATE KEY-----\nMIIE\n-----END RSA PRIVATE KEY-----";
        assert_eq!(categories(json!({"body": pem})), ["private_key"]);
    }

    #[test]
    fn authorization_headers_are_credentials_only_when_shaped_like_them() {
        assert!(authorization_header("Basic dXNlcjpwYXNz"));
        assert!(authorization_header("Bearer eyJhbGciOi.J9.abc_DEF-12"));
        assert!(!authorization_header("Basic abc"));
        assert!(!authorization_header("Bearer abcdefghijklmnopqrst"));
        assert!(!authorization_header("Bearer abcdefghijklmnopq1"));
        assert!(authorization_header("Bearer abcdefghijklmnopq1"));
    }

    #[test]
    fn provider_formats_are_credentials() {
        for token in [
            "sk-abcdefghijklmnopqrstuvwx",
            "sk-proj-abcdefghijklmnopqrstuvwx",
            "ghp_abcdefghijklmnopqrstuvwx",
            "github_pat_abcdefghijklmnopqrstuvwx",
            "AKIAIOSFODNN7EXAMPLE",
            "xoxb-1234567890-abcdefghij",
        ] {
            assert!(provider_token(token), "{token}");
        }
        assert!(!provider_token("sk-short"));
        assert!(!provider_token("AKIAIOSFODNN7EXAMPLE1"));
    }

    #[test]
    fn a_call_too_large_or_too_deep_fails_closed() {
        let large = json!({"items": (0..2_100).map(|i| json!(i)).collect::<Vec<_>>()});
        let inspection = inspect(&large).unwrap();
        assert_eq!(inspection["reason"], "inspection_limit");
        assert_eq!(inspection["findings"].as_array().unwrap().len(), 0);

        let mut deep = json!({"leaf": "sk-abcdefghijklmnopqrstuvwx"});
        for _ in 0..22 {
            deep = json!({ "a": deep });
        }
        assert_eq!(inspect(&deep).unwrap()["reason"], "inspection_limit");

        let huge = json!({"text": "x".repeat(MAX_STRING_LENGTH + 1)});
        assert_eq!(inspect(&huge).unwrap()["reason"], "inspection_limit");
    }

    #[test]
    fn only_calls_that_leave_the_computer_are_inspected() {
        assert!(leaves_machine(&json!({"title": "mcp__gmail__send"})));
        assert!(leaves_machine(&json!({"title": "chat.send_message"})));
        assert!(leaves_machine(&json!({"title": "Terminal", "toolKind": "execute"})));
        assert!(!leaves_machine(&json!({"title": "Read", "toolKind": "read"})));
        assert!(!leaves_machine(&json!({"title": "Edit src/pace.js", "toolKind": "edit"})));
        assert!(!leaves_machine(&json!({"title": "Send email."})));
        let mut data = json!({"title": "Write", "toolKind": "edit"});
        mark(&mut data, &json!({"content": "x".repeat(MAX_STRING_LENGTH + 1)}));
        assert!(data.get("caution").is_none());
    }

    #[test]
    fn a_warning_is_reported_once_per_call() {
        let mut data = json!({"title": "mcp__gmail__send"});
        assert!(mark(&mut data, &json!({"token": "abc"})));
        assert!(!mark(&mut data, &json!({"token": "abc"})));
        assert!(!mark(&mut data, &json!({"query": "x"})));
        assert!(mark(&mut data, &json!({"token": "abc"})));
    }

    #[test]
    fn a_reinspected_call_drops_an_earlier_caution() {
        let mut data = json!({"title": "mcp__gmail__send"});
        mark(&mut data, &json!({"token": "abc"}));
        assert!(data["caution"].is_object());
        mark(&mut data, &json!({"token": "abc", "query": "x"}));
        assert!(data["caution"].is_object());
        mark(&mut data, &json!({"query": "x"}));
        assert!(data.get("caution").is_none());
        let mut data = json!({"title": "mcp__gmail__send"});
        mark(&mut data, &json!("{\"password\": \"abc\"}"));
        assert!(data["caution"].is_object());
    }

    #[test]
    fn findings_stop_at_the_limit() {
        let mut args = serde_json::Map::new();
        for i in 0..30 {
            args.insert(format!("password{i}"), json!("x"));
        }
        assert_eq!(inspect(&Value::Object(args)).unwrap()["findings"].as_array().unwrap().len(), MAX_FINDINGS);
    }
}
