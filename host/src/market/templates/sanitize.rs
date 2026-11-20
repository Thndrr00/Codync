//! Sanitizing untrusted text. Deliberately conservative: losing a sentence is
//! better than sharing a key, a path or an account.

use super::{DEFAULT_COLOR, DEFAULT_SHAPE, MAX_ID, MAX_INSTRUCTIONS};
use crate::store::{AVATAR_COLORS, AVATAR_SHAPES};
use serde_json::Value;

const REDACTED: &str = "[redacted]";

/// Names that introduce or hold a secret.
const SECRET_WORDS: &[&str] = &[
    "bearer",
    "token",
    "apikey",
    "api-key",
    "api_key",
    "password",
    "passwd",
    "secret",
    "authorization",
    "credential",
    "credentials",
    "privatekey",
    "private-key",
    "private_key",
    "session",
    "mnemonic",
    "seed",
    "key",
];
/// Substrings in a `key=value` name or a URL that mark the value as private.
const SECRET_HINTS: &[&str] = &[
    "token",
    "key",
    "secret",
    "password",
    "passwd",
    "credential",
    "authorization",
    "auth",
    "seed",
    "mnemonic",
    "session",
    "private",
];
const TOKEN_PREFIXES: &[&str] = &[
    "sk-",
    "sk_",
    "ghp_",
    "gho_",
    "ghu_",
    "ghs_",
    "ghr_",
    "github_pat_",
    "xoxb-",
    "xoxa-",
    "xoxp-",
    "xoxr-",
    "akia",
    "asia",
    "aiza",
    "eyj",
];

fn normalized(token: &str) -> String {
    token.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '-').to_ascii_lowercase()
}

fn is_key_block(line: &str, begin: bool) -> bool {
    let upper = line.to_ascii_uppercase();
    upper.contains(if begin { "BEGIN" } else { "END" }) && upper.contains("PRIVATE KEY")
}

fn is_secret_word(token: &str) -> bool {
    let word = normalized(token);
    SECRET_WORDS.contains(&word.as_str())
}

/// `PASSWORD=hunter2` or `token: abc` becomes `PASSWORD=[redacted]`.
fn secret_assignment(token: &str) -> Option<(&str, &str)> {
    let at = token.find('=').or_else(|| token.find(':'))?;
    let key = &token[..at];
    let rest = &token[at..];
    let separator = rest.chars().next()?;
    let value = &rest[separator.len_utf8()..];
    if value.is_empty() || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return None;
    }
    let lower = key.to_ascii_lowercase();
    if SECRET_HINTS.iter().any(|hint| lower.contains(hint)) { Some((key, value)) } else { None }
}

fn is_secret_token(token: &str) -> bool {
    let value = token.trim_matches(|c: char| !c.is_ascii_alphanumeric() && !matches!(c, '_' | '-' | '+' | '/' | '='));
    let lower = value.to_ascii_lowercase();
    if TOKEN_PREFIXES.iter().any(|prefix| lower.starts_with(prefix)) {
        return true;
    }
    value.len() >= 32 && value.bytes().any(|b| b.is_ascii_digit()) && value.bytes().any(|b| b.is_ascii_alphabetic())
}

fn is_path(token: &str) -> bool {
    let value = token.trim_matches(|c: char| {
        !c.is_ascii_alphanumeric() && !matches!(c, '/' | '\\' | '.' | '~' | ':' | '\'' | '"' | '(' | ')')
    });
    if value.contains('\\')
        || value.starts_with('/')
        || value.starts_with("~/")
        || value.starts_with("./")
        || value.starts_with("../")
    {
        return true;
    }
    let bytes = value.as_bytes();
    bytes.len() >= 3 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' && (bytes[2] == b'\\' || bytes[2] == b'/')
}

fn is_account(token: &str) -> bool {
    let value = token
        .trim_matches(|c: char| !c.is_ascii_alphanumeric() && !matches!(c, '@' | '.' | '/' | ':' | '?' | '=' | '&'));
    if value.contains('@') && value.contains('.') {
        return true;
    }
    if let Some((_, rest)) = value.split_once("://") {
        let authority = rest.split('/').next().unwrap_or("");
        if authority.contains('@') {
            return true;
        }
        let lower = value.to_ascii_lowercase();
        return SECRET_HINTS.iter().any(|hint| lower.contains(hint));
    }
    false
}

/// Removes anything that could be a secret, a credential, an account or a
/// computer path from free text. Key blocks are dropped whole; a word that
/// introduces a secret redacts the token after it.
pub fn redact(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_key_block = false;
    for line in text.lines() {
        if is_key_block(line, true) {
            in_key_block = true;
            continue;
        }
        if in_key_block {
            // The whole block is private, including its base64 body.
            if is_key_block(line, false) {
                in_key_block = false;
            }
            continue;
        }
        let mut drop_next = false;
        let mut first = true;
        for token in line.split_whitespace() {
            if !first {
                out.push(' ');
            }
            first = false;
            if drop_next {
                drop_next = false;
                out.push_str(REDACTED);
            } else if is_secret_word(token) {
                drop_next = true;
                out.push_str(token);
            } else if let Some((key, _)) = secret_assignment(token) {
                out.push_str(key);
                out.push('=');
                out.push_str(REDACTED);
            } else if is_secret_token(token) || is_path(token) || is_account(token) {
                out.push_str(REDACTED);
            } else {
                out.push_str(token);
            }
        }
        out.push('\n');
    }
    while out.ends_with('\n') {
        out.pop();
    }
    out
}

pub(crate) fn clean_line(text: &str, max: usize) -> String {
    let one_line = redact(text).split_whitespace().collect::<Vec<_>>().join(" ");
    one_line.chars().take(max).collect()
}

pub(crate) fn clean_instructions(text: &str) -> String {
    redact(text).chars().take(MAX_INSTRUCTIONS).collect()
}

pub(crate) fn is_safe_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_ID
        && id.as_bytes()[0].is_ascii_lowercase()
        && id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

pub(crate) fn safe_catalog_id(value: Option<&str>) -> Option<String> {
    let id = value?.trim();
    if id.is_empty()
        || id.len() > MAX_ID
        || !id.as_bytes()[0].is_ascii_lowercase()
        || !id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'-' | b'_' | b'.' | b':'))
    {
        return None;
    }
    Some(id.to_owned())
}

pub(crate) fn avatar(color: Option<&str>, shape: Option<&str>) -> (String, String) {
    let color = color.filter(|c| AVATAR_COLORS.contains(c)).unwrap_or(DEFAULT_COLOR);
    let shape = shape.filter(|s| AVATAR_SHAPES.contains(s)).unwrap_or(DEFAULT_SHAPE);
    (color.to_owned(), shape.to_owned())
}

/// Keeps only well-formed, distinct ids, skipping anything else.
pub(crate) fn safe_ids(value: &Value, cap: usize) -> Vec<String> {
    let mut out = Vec::new();
    for id in value.as_array().into_iter().flatten().filter_map(Value::as_str) {
        if is_safe_id(id) && !out.iter().any(|kept| kept == id) {
            out.push(id.to_owned());
            if out.len() == cap {
                break;
            }
        }
    }
    out
}

pub(crate) fn positive_or(value: Option<i64>, fallback: i64) -> i64 {
    value.filter(|v| *v > 0).unwrap_or(fallback)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn sanitization_strips_secrets_paths_and_accounts() {
        let text = concat!(
            "Keep this sentence.\n",
            "API_KEY=sk-live-1234567890abcdefghij\n",
            "Authorization: Bearer ghp_abcdefghijklmnopqrstuvwxyz\n",
            "token: abc123XYZ789tokenabc123XYZ789tokenabc\n",
            "Read C:\\Users\\Ada\\Documents\\notes.txt and /home/ada/.ssh/id_rsa\n",
            "Contact ada@example.com or https://u:p@private.example/x\n",
            "-----BEGIN OPENSSH PRIVATE KEY-----\n",
            "some base64 body\n",
            "-----END OPENSSH PRIVATE KEY-----\n",
            "Back to normal text.\n",
        );
        let safe = redact(text);
        assert!(safe.contains("Keep this sentence."));
        assert!(safe.contains("Back to normal text."), "text after a key block survives");
        for secret in [
            "sk-live-1234567890abcdefghij",
            "ghp_abcdefghijklmnopqrstuvwxyz",
            "abc123XYZ789tokenabc123XYZ789tokenabc",
            "PRIVATE KEY",
            "some base64 body",
            "C:\\Users\\Ada",
            "/home/ada/.ssh/id_rsa",
            "ada@example.com",
            "https://u:p@private.example",
        ] {
            assert!(!safe.contains(secret), " leaked {secret}: {safe}");
        }
        assert!(safe.contains("API_KEY="));
        assert!(safe.contains(REDACTED));
    }
}
