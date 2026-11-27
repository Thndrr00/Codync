//! A support snapshot on demand: versions, health, counts, which channels are set up, and the end of the host log with
//! tokens and keys taken out. The user copies it themselves; nothing is sent anywhere.

use crate::hub::Hub;
use anyhow::Result;
use serde_json::{Value, json};
use std::path::Path;

/// Lines of the host log kept in a snapshot.
const LOG_LINES: usize = 80;

pub fn snapshot(hub: &Hub) -> Result<Value> {
    let bots = hub.store.bots()?;
    let devices = hub.store.devices()?;
    let log = tail(&crate::service::data_dir().join("host.log"), LOG_LINES);
    Ok(json!({
        "generatedAt": crate::store::now_ms(),
        "hostVersion": env!("CARGO_PKG_VERSION"),
        "health": crate::health::value(hub),
        "bots": bots.iter().filter(|bot| !bot.deleted && !bot.config.is_group()).count(),
        "groups": bots.iter().filter(|bot| !bot.deleted && bot.config.is_group()).count(),
        "devices": devices.len(),
        "channels": {
            "telegram": crate::telegram::configuration_state(&hub.store),
            "slack": crate::slack::configuration_state(&hub.store),
            "discord": crate::discord::configuration_state(&hub.store),
        },
        "log": log,
    }))
}

fn tail(path: &Path, lines: usize) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(path) else { return Vec::new() };
    let all: Vec<&str> = text.lines().collect();
    let start = all.len().saturating_sub(lines);
    all[start..].iter().map(|line| redact(line)).collect()
}

/// Whether a `name=value` field is a credential. Matching is on whole name segments, so a public
/// header like `X-Portkey-Provider` or a word like `monkey` keeps its value (OpenBot 0.80).
fn secret_field(name: &str) -> bool {
    name.to_ascii_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .any(|part| matches!(part, "key" | "apikey" | "token" | "secret" | "password" | "authorization"))
}

/// Takes out anything that looks like a credential: what follows `Bearer`, a `token=`/`key=`/`secret=` value, a
/// provider key, or a long unbroken random-looking run.
fn redact(line: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut after_bearer = false;
    for word in line.split(' ') {
        if after_bearer {
            out.push("[redacted]".into());
            after_bearer = false;
            continue;
        }
        if word.eq_ignore_ascii_case("bearer") {
            out.push(word.to_owned());
            after_bearer = true;
            continue;
        }
        if let Some((name, _)) = word.split_once('=') {
            // OpenBot 0.80: only a real credential field hides its value. A substring such as
            // `monkey` or the public header `X-Portkey-Provider` must keep its value.
            if secret_field(name) {
                out.push(format!("{name}=[redacted]"));
                continue;
            }
        }
        let random_looking =
            word.len() >= 32 && word.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '+'));
        if random_looking || word.starts_with("sk-") || word.starts_with("ghp_") || word.starts_with("xoxb-") {
            out.push("[redacted]".into());
            continue;
        }
        out.push(word.to_owned());
    }
    out.join(" ")
}

#[cfg(test)]
mod tests {
    use super::redact;

    #[test]
    fn credentials_never_reach_the_snapshot() {
        assert_eq!(redact("authorization: Bearer abc.def"), "authorization: Bearer [redacted]");
        assert_eq!(redact("GET /hooks?token=s3cr3t ok"), "GET /hooks?token=[redacted] ok");
        assert_eq!(redact("api_key=abcd"), "api_key=[redacted]");
        assert_eq!(redact("key sk-1234567890"), "key [redacted]");
        assert_eq!(redact("id 9f8e7d6c5b4a39281706f5e4d3c2b1a0"), "id [redacted]");
    }

    #[test]
    fn a_public_header_or_word_with_key_in_its_name_keeps_its_value() {
        assert_eq!(secret_field("api_key"), true);
        assert_eq!(secret_field("token"), true);
        assert_eq!(secret_field("monkey"), false);
        assert_eq!(secret_field("X-Portkey-Provider"), false);
        assert_eq!(redact("header X-Portkey-Provider=anthropic"), "header X-Portkey-Provider=anthropic");
        assert_eq!(redact("note monkey=banana"), "note monkey=banana");
        assert_eq!(redact("api_key=abcd"), "api_key=[redacted]");
    }

    #[test]
    fn ordinary_log_lines_are_kept() {
        assert_eq!(
            redact("codync-host listening version=2.39.0 port=19223"),
            "codync-host listening version=2.39.0 port=19223"
        );
        assert_eq!(redact("bot 0689a99e turn usage input=6"), "bot 0689a99e turn usage input=6");
        assert_eq!(redact(""), "");
    }
}
