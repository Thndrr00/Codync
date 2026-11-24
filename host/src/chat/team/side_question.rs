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

/// Which lane a side question reads for its transcript. Asked in a side thread, it reads that
/// thread's own messages; from a chat, the chat's main transcript. Pure so the choice has a test
/// without a store (OpenBot 0.76.0).
pub(super) fn transcript_lane(asking_lane: &Lane, is_thread: bool) -> Lane {
    if is_thread && asking_lane.thread.is_some() { asking_lane.clone() } else { Lane::main(&asking_lane.chat) }
}

/// The asking lane's last [`MESSAGES`] messages, oldest first, as `(who, text)`. Text is escaped so
/// a line in the chat can't forge a harness header in the recipient's prompt.
pub(super) fn lines(hub: &Hub, from: &BotConfig, lane: &Lane) -> Vec<(String, String)> {
    hub.store
        .messages_after(lane, 0, MESSAGES)
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
    use serde_json::json;

    fn lines(n: usize, chars: usize) -> Vec<(String, String)> {
        (0..n).map(|i| (format!("bot{i}"), "x".repeat(chars))).collect()
    }

    #[test]
    fn a_thread_question_reads_its_thread_and_a_chat_question_its_chat() {
        let main = Lane::main("a");
        assert_eq!(transcript_lane(&main, false), Lane::main("a"));
        assert_eq!(transcript_lane(&main, true), Lane::main("a"), "no thread to read");
        let thread = Lane::in_thread("a", "doc:abc");
        assert_eq!(transcript_lane(&thread, true), thread, "the thread is the transcript");
        assert_eq!(transcript_lane(&thread, false), Lane::main("a"), "a chat ask drops the thread");
        let room = Lane::main("crew");
        assert_eq!(transcript_lane(&room, false), Lane::main("crew"), "a room's own transcript");
    }

    #[test]
    fn lines_read_the_real_lane_not_the_bots_main_chat() {
        let dir = std::env::temp_dir().join(format!("codync-side-question-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("dir");
        let store = crate::store::Store::open(&dir.join("test.db")).expect("store");
        let hub = Hub::new(
            store,
            "side-question-test".into(),
            crate::remote::identity::Identity::load_or_create(&dir).expect("identity"),
            "t".into(),
            0,
        );
        let from: BotConfig =
            serde_json::from_value(json!({"id": "a", "name": "A", "backend": "fixture", "command": "x", "cwd": dir}))
                .expect("config");
        let main = Lane::main(&from.id);
        let thread = Lane::in_thread(&from.id, "doc:abc");
        hub.add_entry(&main, EntryKind::User, 1, &json!({"text": "said in main"}));
        hub.add_entry(&thread, EntryKind::User, 1, &json!({"text": "said in thread"}));

        let got = lines(&hub, &from, &transcript_lane(&thread, true));
        assert_eq!(got, [("User".to_owned(), "said in thread".to_owned())]);
        let got = lines(&hub, &from, &transcript_lane(&main, false));
        assert_eq!(got, [("User".to_owned(), "said in main".to_owned())]);
        std::fs::remove_dir_all(&dir).ok();
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
