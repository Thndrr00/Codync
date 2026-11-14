//! What a turn is reminded of: lines from earlier in this chat and from the bot's pages that match what the
//! person just said, within a character budget.
//!
//! The conversation the agent already holds is the last messages; this adds the *relevant* older ones, each
//! saying when it was said and by whom. The budget is the bot's (`BotConfig::recall_chars`), and a line that
//! does not fit is dropped, never cut in half: half a snippet is worse than none.
//!
//! Idea from agentes.chat's run memory (`packages/engine/src/context/tools.ts`, AGPL-3.0-only): a fixed core
//! never cut, then what matched the request while the budget lasts, each line naming its source and date. The
//! behavior, not the code; written against Codync's own search.

use crate::store::Store;
use anyhow::Result;
use serde_json::Value;

/// The budget when the bot has none: about 250 tokens.
pub const DEFAULT_CHARS: u32 = 900;
/// The most a bot may ask for.
pub const MAX_CHARS: u32 = 4_000;
/// Hits looked at for one turn; the budget cuts most of them anyway.
const CANDIDATES: u32 = 12;
/// The most of one hit shown: a snippet, not the message.
const LINE_CHARS: usize = 220;
/// How many of the chat's newest entries are already in front of the agent.
const IN_CONTEXT: i64 = 40;
/// Shorter messages (an emoji, "ok") recall nothing worth the budget.
const MIN_QUERY: usize = 3;

/// Entry kinds whose words are worth recalling: what a person or the bot said, and a plan. Thoughts, tool
/// traces, permission prompts and notices are the agent's working noise.
const WORTH_RECALLING: [&str; 3] = ["user", "agent", "plan"];

/// The block a turn carries, or an empty string. `budget` is the bot's cap; 0 turns recall off.
pub fn block(store: &Store, bot: &str, budget: u32, text: &str) -> Result<String> {
    let budget = budget.min(MAX_CHARS) as usize;
    if budget == 0 || text.trim().chars().count() < MIN_QUERY {
        return Ok(String::new());
    }
    let in_context: Vec<String> = store.history(bot, i64::MAX, IN_CONTEXT)?.into_iter().map(|entry| entry.id).collect();
    let hits = store.search(bot, text, CANDIDATES)?;
    let mut lines = Vec::new();
    let mut used = HEADER.chars().count();
    for hit in hits.iter().filter(|hit| worth_it(hit, &in_context)) {
        let Some(line) = line(hit) else { continue };
        let width = line.chars().count() + 1;
        if used + width > budget {
            break;
        }
        used += width;
        lines.push(line);
    }
    Ok(if lines.is_empty() { String::new() } else { format!("{HEADER}\n{}", lines.join("\n")) })
}

const HEADER: &str = "[Codync: recalled from earlier in this chat and its pages, best match first. Use what is relevant, \
don't read it out; open a page with get_document(id).]";

/// Whether a hit belongs in a turn: a message worth reading and not already in front of the agent.
fn worth_it(hit: &Value, in_context: &[String]) -> bool {
    let Some(id) = hit["id"].as_str() else { return false };
    match hit["kind"].as_str() {
        Some("document") => hit["title"].as_str().is_some_and(|title| !title.trim().is_empty()),
        Some("message") => {
            !in_context.iter().any(|seen| seen == id)
                && hit["entryKind"].as_str().is_some_and(|kind| WORTH_RECALLING.contains(&kind))
        }
        _ => false,
    }
}

/// One line with its source and date, or none when the hit has no words.
fn line(hit: &Value) -> Option<String> {
    let words = squash(hit["snippet"].as_str()?);
    if words.is_empty() {
        return None;
    }
    let words = crate::agent::acp::truncate(&words, LINE_CHARS);
    let when = date(hit["at"].as_i64().unwrap_or_default());
    let line = if hit["kind"] == "document" {
        let title = squash(hit["title"].as_str().unwrap_or_default());
        format!("- page \"{}\" (edited {when}): {words} (id {})", title, hit["id"].as_str().unwrap_or_default())
    } else {
        let who = match hit["entryKind"].as_str() {
            Some("agent") => "the bot",
            Some("plan") => "a plan",
            _ => "the user",
        };
        format!("- {who}, {when}: {words}")
    };
    Some(line)
}

/// The snippet on one line: the marker stays, the whitespace does not.
fn squash(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// When it was said, on this computer's clock, as the rest of the host reads dates.
fn date(ms: i64) -> String {
    chrono::DateTime::from_timestamp_millis(ms)
        .map_or_else(|| "unknown date".into(), |t| t.with_timezone(&chrono::Local).format("%Y-%m-%d").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{DocumentDraft, DocumentWrite, EntryKind, Lane};
    use serde_json::json;

    fn said(store: &Store, bot: &str, text: &str) -> String {
        store.insert_entry(&Lane::main(bot), EntryKind::User, 1, &json!({"text": text})).expect("an entry").id
    }

    fn bot(store: &Store, id: &str) {
        store.save_bot(&serde_json::from_value(json!({"id": id, "name": id})).unwrap()).unwrap();
    }

    #[test]
    fn an_old_message_is_recalled_with_when_it_was_said() {
        let store = crate::store::tests::temp_store();
        bot(&store, "b1");
        said(&store, "b1", "the deploy needs a rollback plan");
        for turn in 0..IN_CONTEXT {
            said(&store, "b1", &format!("filler {turn}"));
        }
        let block = block(&store, "b1", 500, "deploy").unwrap();
        assert!(block.contains("[deploy]"), "the match is marked: {block}");
        assert!(block.contains("rollback plan"), "{block}");
        assert!(block.contains("the user"), "it says who: {block}");
    }

    #[test]
    fn what_the_agent_already_holds_is_left_out() {
        let store = crate::store::tests::temp_store();
        bot(&store, "b1");
        said(&store, "b1", "the deploy needs a rollback plan");
        assert!(block(&store, "b1", 500, "deploy").unwrap().is_empty(), "the newest message is in context");
        for turn in 0..IN_CONTEXT {
            said(&store, "b1", &format!("filler {turn}"));
        }
        assert_ne!(block(&store, "b1", 500, "deploy").unwrap().len(), 0);
    }

    #[test]
    fn a_page_is_recalled_by_its_title_and_where_to_open_it() {
        let store = crate::store::tests::temp_store();
        bot(&store, "b1");
        let draft = DocumentDraft {
            id: None,
            parent_id: None,
            title: "Rollback runbook".into(),
            body: "if the deploy fails, roll back".into(),
            if_revision: 0,
        };
        let DocumentWrite::Saved(page) = store.save_document("b1", draft).unwrap() else { panic!("a page") };
        let block = block(&store, "b1", 500, "deploy").unwrap();
        assert!(block.contains("get_document(id)"), "{block}");
        assert!(block.contains(&page.id), "the page is named: {block}");
        assert!(block.contains("Rollback runbook"), "{block}");
    }

    #[test]
    fn tool_traces_stay_out() {
        let store = crate::store::tests::temp_store();
        bot(&store, "b1");
        store.insert_entry(&Lane::main("b1"), EntryKind::Tool, 1, &json!({"title": "grep deploy"})).unwrap();
        for turn in 0..IN_CONTEXT {
            said(&store, "b1", &format!("filler {turn}"));
        }
        assert_eq!(block(&store, "b1", 500, "deploy").unwrap().len(), 0);
    }

    #[test]
    fn the_budget_stops_the_lines_and_zero_turns_it_off() {
        let store = crate::store::tests::temp_store();
        bot(&store, "b1");
        for turn in 0..5 {
            said(&store, "b1", &format!("deploy note {turn} {}", "x".repeat(300)));
        }
        for turn in 0..IN_CONTEXT {
            said(&store, "b1", &format!("filler {turn}"));
        }
        let small = block(&store, "b1", 400, "deploy").unwrap();
        let large = block(&store, "b1", 4_000, "deploy").unwrap();
        assert!(small.chars().count() <= 400, "{}", small.chars().count());
        assert!(large.lines().count() > small.lines().count());
        assert_eq!(block(&store, "b1", 0, "deploy").unwrap().len(), 0);
    }
}
