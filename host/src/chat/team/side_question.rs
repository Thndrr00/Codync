//! A bounded recent transcript for a side question. Ported from OpenBot nightly's bounded
//! side-question prompt: when one bot asks another agent, the recipient gets the last few messages of
//! the asking chat so its answer fits, without being handed the whole history. The transcript is
//! context only, never new instructions, and reuses the store's history helper.

use crate::hub::Hub;
use crate::store::{BotConfig, EntryKind, Lane};

/// The most recent messages of the asking chat that go to the recipient.
pub(super) const MESSAGES: i64 = 6;
/// The most characters of that transcript one answer may use.
pub(super) const CHARS: usize = 6000;

/// The asking chat's last [`MESSAGES`] messages, oldest first, as `(who, text)`. Text is escaped so a
/// line in the chat can't forge a harness header in the recipient's prompt.
pub(super) fn lines(hub: &Hub, from: &BotConfig) -> Vec<(String, String)> {
    hub.store
        .messages_after(&Lane::main(&from.id), 0, MESSAGES)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|e| {
            let who = if e.kind == EntryKind::User.as_str() {
                "User".to_owned()
            } else {
                hub.bot_name(e.data["author"].as_str().unwrap_or(&from.id))
            };
            let text = crate::agent::envelope::escape_envelope(e.data["text"].as_str().unwrap_or_default().trim());
            (!text.is_empty()).then_some((who, text))
        })
        .collect()
}

/// The transcript section for the recipient: at most the last [`MESSAGES`] lines and [`CHARS`]
/// characters in total, oldest first. Pure so the bound has a test without a store or a bot.
pub(super) fn transcript(lines: &[(String, String)]) -> String {
    let mut kept: Vec<String> = Vec::new();
    let mut used = 0usize;
    for (who, text) in lines.iter().rev().take(MESSAGES as usize) {
        let line = format!("{who}: {text}");
        let chars = line.chars().count();
        if used + chars <= CHARS {
            used += chars;
            kept.push(line);
        } else if kept.is_empty() {
            // The newest line alone is over budget: keep as much of it as fits.
            let mut short: String = line.chars().take(CHARS.saturating_sub(1)).collect();
            short.push('…');
            kept.push(short);
            break;
        } else {
            break;
        }
    }
    kept.reverse();
    kept.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(n: usize, chars: usize) -> Vec<(String, String)> {
        (0..n).map(|i| (format!("bot{i}"), "x".repeat(chars))).collect()
    }

    #[test]
    fn only_the_last_six_messages_of_the_chat_go_to_the_recipient() {
        let all = lines(9, 10);
        let out = transcript(&all);
        assert_eq!(out.lines().count(), MESSAGES as usize, "older messages are dropped");
        assert!(out.starts_with("bot3: "), "the last six are bat3 through bot8");
        assert!(out.ends_with(&format!("bot8: {}", "x".repeat(10))));
    }

    #[test]
    fn the_transcript_never_exceeds_its_character_budget() {
        let out = transcript(&lines(9, 2000));
        assert!(out.chars().count() <= CHARS, "the six messages are capped in total");
        // A single over-long newest line is kept, clipped to the budget.
        let one = transcript(&lines(1, CHARS + 500));
        assert!(one.chars().count() <= CHARS);
        assert!(one.ends_with('…'));
        assert!(transcript(&[]).is_empty(), "no messages is no transcript");
    }
}
