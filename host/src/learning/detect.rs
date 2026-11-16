//! What in a person's own message is worth remembering: a correction ("No, use pnpm"), a stated preference
//! ("I always want the tests run before a commit") or the same instruction given again. Nothing else: a question, a
//! one-off task, code, a URL or anything that looks like a secret is left alone.

use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

/// Why a sentence is a candidate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// The person said the bot had it wrong.
    Correction,
    /// A durable preference or rule, stated once.
    Instruction,
    /// The same instruction came up again.
    Repetition,
}

/// A durable fact the person stated, before it is stored.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub text: String,
    pub kind: Kind,
    /// How many times the person said it (1 unless repeated).
    pub hits: u32,
}

const MIN_CHARS: usize = 12;
const MAX_CHARS: usize = 240;
const MAX_WORDS: usize = 40;

/// Words at the start of a sentence that mark a correction; the marker is dropped from the fact.
const CORRECTIONS: &[&str] = &[
    "no,",
    "nope,",
    "nope ",
    "actually,",
    "actually ",
    "i said ",
    "i meant ",
    "that's wrong",
    "that is wrong",
    "instead,",
    "instead ",
    "en realidad,",
    "en realidad ",
];

/// Markers of a preference or a rule that are worth keeping when the person says them once.
const DURABLE: &[&str] = &[
    "i prefer",
    "i'd rather",
    "i would rather",
    "i always",
    "i never",
    "always ",
    "never ",
    "from now on",
    "remember",
    "my name is",
    "i live",
    "i work",
    "prefiero",
    "siempre ",
    "nunca ",
    "recuerda",
];

/// Imperative openers that read as an instruction to the bot.
const INSTRUCTIONS: &[&str] = &[
    "use ",
    "run ",
    "don't ",
    "do not ",
    "always ",
    "never ",
    "make sure",
    "keep ",
    "write ",
    "reply ",
    "answer ",
    "ask ",
    "commit ",
    "name ",
    "format ",
    "avoid ",
    "include ",
    "put ",
    "call ",
    "send ",
    "prefer ",
];

/// The candidates in `message`, using `history` (older user messages, newest last) to spot an instruction given
/// again. Each text is unique and keeps the order it was said in.
pub fn candidates(message: &str, history: &[String]) -> Vec<Candidate> {
    let mut found: Vec<Candidate> = Vec::new();
    let mut counts: std::collections::HashMap<String, (String, u32)> = std::collections::HashMap::new();
    let current = message.to_owned();
    for (index, text) in history.iter().chain(std::iter::once(&current)).enumerate() {
        let latest = index == history.len();
        for sentence in sentences(text) {
            let Some(normal) = keepable(sentence) else { continue };
            let normal = clean(&normal);
            if normal.chars().count() < MIN_CHARS {
                continue;
            }
            let key = normalize(&normal);
            let entry = counts.entry(key).or_insert_with(|| (normal.clone(), 0));
            entry.1 += 1;
            if !latest {
                continue;
            }
            // A single message is learned from only when it corrects the bot or states a preference: an ordinary
            // one-off command is not a rule. Repetition (below) catches the rest.
            let kind = if correction(sentence) {
                Some(Kind::Correction)
            } else if durable(sentence) {
                Some(Kind::Instruction)
            } else {
                None
            };
            if let Some(kind) = kind {
                push(&mut found, Candidate { text: normal.clone(), kind, hits: 1 });
            }
        }
    }
    // The same instruction, stated in two turns: the repeat is what makes it worth proposing.
    for (normal, hits) in counts.values().filter(|entry| entry.1 >= 2) {
        if instruction(normal) || durable(normal) {
            push(&mut found, Candidate { text: normal.clone(), kind: Kind::Repetition, hits: *hits });
        }
    }
    found
}

fn push(found: &mut Vec<Candidate>, candidate: Candidate) {
    if let Some(existing) = found.iter_mut().find(|c| normalize(&c.text) == normalize(&candidate.text)) {
        existing.hits = existing.hits.max(candidate.hits);
        // A correction says more about what the person wants than a plain instruction.
        if candidate.kind == Kind::Correction {
            existing.kind = Kind::Correction;
        }
        return;
    }
    found.push(candidate);
}

/// A lower-case form for comparing the same fact said twice.
pub fn normalize(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut space = true;
    for c in text.chars().flat_map(char::to_lowercase) {
        if c.is_alphanumeric() {
            out.push(c);
            space = false;
        } else if !space {
            out.push(' ');
            space = true;
        }
    }
    let mut words: Vec<&str> = out.split_whitespace().collect();
    // A repeated instruction often loses its politeness before the second saying.
    let filler = |word: &&str| matches!(*word, "please" | "can" | "could" | "would" | "you" | "por" | "favor");
    while words.first().is_some_and(filler) {
        words.remove(0);
    }
    words.join(" ")
}

/// The id the apps and the store use for a fact.
pub fn fingerprint(text: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(normalize(text).as_bytes());
    crate::chat::files::digest_hex(&digest.finalize()).chars().take(16).collect()
}

/// The sentences of a message, without code blocks or quoted lines.
fn sentences(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut fence = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            fence = !fence;
            continue;
        }
        if fence || line.trim_start().starts_with(['>', '|']) {
            continue;
        }
        for sentence in line.split_inclusive(['.', '!', '?', '\n', ';']) {
            let sentence = sentence.trim();
            if !sentence.is_empty() {
                out.push(sentence);
            }
        }
    }
    out
}

/// A sentence that may become a fact: long enough, not a question, no code, URL, email, number-like account or
/// secret. Returns the sentence without its opening list marker.
fn keepable(sentence: &str) -> Option<String> {
    let text = sentence.trim().trim_start_matches(['-', '*', '•', '"', '\'', '`', ' ']).trim();
    if sentence.trim_end().ends_with('?') {
        return None;
    }
    let text = text.trim_end_matches(['.', '!', ';', ',']).trim();
    let chars = text.chars().count();
    if !(MIN_CHARS..=MAX_CHARS).contains(&chars) || text.split_whitespace().count() > MAX_WORDS {
        return None;
    }
    if looks_sensitive(text) {
        return None;
    }
    Some(text.to_owned())
}

fn looks_sensitive(text: &str) -> bool {
    let lower = text.to_lowercase();
    let secret = [
        "password",
        "passwd",
        "api key",
        "api_key",
        "secret",
        "bearer ",
        "sk-",
        "ghp_",
        "xoxb-",
        "xoxp-",
        "-----begin",
        "private key",
        "credit card",
    ];
    if secret.iter().any(|word| lower.contains(word)) {
        return true;
    }
    if text.contains("```") || lower.contains("http://") || lower.contains("https://") {
        return true;
    }
    // An email or a long number is an identifier, not a preference.
    if lower.split_whitespace().any(|word| {
        let word = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '@' && c != '.' && c != '_');
        word.contains('@') && word.contains('.')
    }) {
        return true;
    }
    lower.chars().filter(char::is_ascii_digit).count() >= 12
}

fn starts(text: &str, markers: &[&str]) -> bool {
    let lower = text.trim().to_lowercase();
    markers.iter().any(|marker| lower.starts_with(*marker))
}

fn correction(sentence: &str) -> bool {
    starts(sentence, CORRECTIONS)
}

fn durable(sentence: &str) -> bool {
    let lower = sentence.to_lowercase();
    DURABLE.iter().any(|marker| lower.contains(*marker))
}

fn instruction(sentence: &str) -> bool {
    starts(sentence, INSTRUCTIONS) && sentence.split_whitespace().count() >= 3
}

/// The fact without the correction prefix, a leading "please" or a trailing thank-you.
fn clean(sentence: &str) -> String {
    let mut text = sentence.trim().to_owned();
    let prefixes = [
        "no,",
        "no ",
        "nope,",
        "nope ",
        "actually,",
        "actually ",
        "i said ",
        "i meant ",
        "instead,",
        "instead ",
        "en realidad,",
        "en realidad ",
        "remember that ",
        "remember: ",
        "remember ",
        "please ",
        "por favor ",
    ];
    loop {
        let lower = text.to_lowercase();
        let Some(prefix) = prefixes.iter().find(|prefix| lower.starts_with(**prefix)) else { break };
        text = text[prefix.len()..].trim().to_owned();
    }
    for suffix in [", thanks", ", thank you", " thanks", " please", " por favor"] {
        if text.to_lowercase().ends_with(suffix) {
            text.truncate(text.len() - suffix.len());
            break;
        }
    }
    let text = text.trim().trim_matches(['"', '\'', ' ']).to_owned();
    let mut text = text.trim_end_matches(['.', '!', ';', ',']).trim().to_owned();
    if let Some(first) = text.chars().next() {
        text.replace_range(0..first.len_utf8(), &first.to_uppercase().to_string());
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(message: &str) -> Vec<Candidate> {
        candidates(message, &[])
    }

    #[test]
    fn a_correction_becomes_the_thing_to_do_instead() {
        let found = one("No, use pnpm here, not npm.");
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].kind, Kind::Correction);
        assert_eq!(found[0].text, "Use pnpm here, not npm");
    }

    #[test]
    fn a_stated_preference_is_kept_and_a_question_is_not() {
        assert_eq!(one("I always want the tests run before a commit")[0].kind, Kind::Instruction);
        assert!(one("What is the weather today?").is_empty());
        assert!(one("Fix the sidebar spacing.").is_empty(), "a one-off task is not a preference");
        assert!(one("Use the blue button.").is_empty(), "too short to be a rule");
    }

    #[test]
    fn the_same_instruction_in_two_turns_is_worth_proposing() {
        let history = vec!["Please run the tests before committing anything.".into(), "Can you fix the header?".into()];
        let found = candidates("Run the tests before committing anything.", &history);
        assert!(found.iter().any(|c| c.kind == Kind::Repetition && c.hits >= 2), "{found:?}");
    }

    #[test]
    fn code_urls_emails_secrets_and_account_numbers_are_left_alone() {
        for text in [
            "Remember to open https://example.com later",
            "No, email ada@example.com instead",
            "Always put the API key sk-123 in the file",
            "Remember the account 1234567890123",
            "```\nno, use this\n```",
        ] {
            assert!(one(text).is_empty(), "{text:?} became {:?}", one(text));
        }
    }

    #[test]
    fn the_same_fact_has_one_fingerprint_whatever_its_punctuation() {
        assert_eq!(fingerprint("Use pnpm, not npm."), fingerprint("use pnpm not npm"));
        assert_ne!(fingerprint("Use pnpm"), fingerprint("Use yarn"));
    }
}
