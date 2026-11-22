//! A short follow-up ("continue", "keep going", "go on") means "finish what you were doing", not
//! "start a new request". It reuses the existing bounded self-check on the same ACP session
//! (`super::self_check`) instead of beginning another agent loop. The matcher is pure: questions
//! and longer sentences never count as continuations.
//!
//! OpenBot nightly triage-14327 adds the other half: an agent that only *says* it will do the work
//! ("I'll research the options now") in a turn that used no tool still gets the bounded check, so
//! the request does not end on the promise. [`promises_work`] is that pure matcher. A short closing
//! ("Let me know if you need anything"), a refusal and a question are not promises.

use super::self_check::MAX_SELF_CHECKS;

/// The whole set of short phrases OpenBot's nightly treats as continuing unfinished work.
const PHRASES: [&str; 3] = ["continue", "keep going", "go on"];

/// Longest answer in which a promise is looked for (OpenBot's `NEXT_STEP_ANSWER_MAX`). A pasted
/// document must not turn a closing line into a promise.
const NEXT_STEP_ANSWER_MAX: usize = 400;

/// Ways to say "I will do it" in English and Portuguese. Each is a promise only when followed by a
/// verb; the table keeps the matcher small and testable without a regex dependency.
const PROMISES: [&str; 13] = [
    "i'll",
    "i will",
    "let me",
    "i'm going to",
    "i am going to",
    "vou",
    "eu vou",
    "irei",
    "eu irei",
    "deixa eu",
    "deixe eu",
    "deixa-me",
    "deixe-me",
];

/// Words right after a promise that mean the agent is waiting, not working. "Let me know",
/// "I'll wait", "vou aguardar" and the like end the request (triage-14327).
const WAITING: [&str; 14] = [
    "not", "never", "know", "wait", "hold", "pause", "stop", "be", "aguardar", "esperar", "ficar", "avisar", "retorno",
    "volto",
];

/// Filler a promise may carry between the marker and the verb ("I'll *now* check…").
const FILLER: [&str; 10] = ["now", "first", "next", "then", "also", "quickly", "agora", "já", "primeiro", "logo"];

/// A refusal before the Portuguese "vou"/"irei" ("Não vou apagar isso") is not a promise.
const REFUSAL: [&str; 3] = ["não", "nao", "nunca"];

/// Whether the text is exactly one of the continuation phrases (case and surrounding space aside).
/// A trailing `.` or `!` is harmless, but a `?` turns it into a question, which is never a
/// continuation: the agent must answer it, not carry on with old work.
pub(super) fn is_continuation(text: &str) -> bool {
    let text = text.trim();
    if text.ends_with('?') {
        return false;
    }
    let phrase = text.trim_end_matches(['.', '!']).trim_end().to_ascii_lowercase();
    PHRASES.contains(&phrase.as_str())
}

/// Whether the follow-up may be carried on by a self-check within the per-request budget.
pub(super) fn continues(text: &str, checks_done: u32) -> bool {
    is_continuation(text) && checks_done < MAX_SELF_CHECKS
}

/// Curly apostrophes and case aside, so the English promises match either spelling.
fn normalised(text: &str) -> String {
    text.replace('\u{2019}', "'").to_lowercase()
}

/// Drops a trailing "nothing (else) (needed/required) from you" clause (English and Portuguese). It
/// closes a request rather than promising work, so it must not hide an earlier promise from the last
/// sentence (OpenBot's `NOTHING_FROM_USER`).
fn strip_nothing_from_user(text: &str) -> &str {
    const TAILS: [&str; 12] = [
        "nothing else needed from you",
        "nothing else required from you",
        "nothing needed from you",
        "nothing required from you",
        "nothing from you",
        "nothing for you to do",
        "no action needed from you",
        "no action required from you",
        "de você nada",
        "de voce nada",
        "da sua parte nada",
        "nada da sua parte",
    ];
    let text = text.trim_end().trim_end_matches(['.', '!', '…']).trim_end();
    let lower = normalised(text);
    for tail in TAILS {
        if lower.ends_with(tail) {
            let cut = &text[..text.len() - tail.len()];
            return cut.trim_end().trim_end_matches(['.', '!', '…']).trim_end();
        }
    }
    text
}

/// The last sentence of an answer, after the markdown marks and the "nothing from you" tail are
/// gone. Sentence boundaries are `. `, `! `, `… ` and newlines.
fn last_sentence(text: &str) -> String {
    let plain: String = text.chars().filter(|c| !matches!(*c, '*' | '_' | '`' | '>' | '#')).collect();
    let plain = strip_nothing_from_user(plain.trim_end());
    let mut last = String::new();
    let mut current = String::new();
    let mut boundary = false;
    for ch in plain.chars() {
        if ch == '\n' {
            if !current.trim().is_empty() {
                last = std::mem::take(&mut current);
            }
            boundary = false;
            continue;
        }
        if boundary && ch.is_whitespace() {
            if !current.trim().is_empty() {
                last = std::mem::take(&mut current);
            }
            boundary = false;
            continue;
        }
        boundary = matches!(ch, '.' | '!' | '…');
        current.push(ch);
    }
    if current.trim().is_empty() { last } else { current }
}

/// A question, or an answer that says it will wait, is not a promise. Asking "shall I continue?"
/// is the one question that is not a stop (OpenBot's `waitsForUser`).
fn waits_for_user(answer: &str) -> bool {
    let line = normalised(answer.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or(""));
    let asks_to_continue = [
        "shall i continue",
        "should i continue",
        "can i continue",
        "want me to continue",
        "would you like me to continue",
        "shall i keep going",
        "shall i go on",
        "do you want me to go on",
    ]
    .iter()
    .any(|p| line.contains(p));
    let question = (line.contains('?') || line.contains('？')) && !asks_to_continue;
    let waiting = [
        "i'll wait",
        "i will wait",
        "i'm going to wait",
        "i am going to wait",
        "let me wait",
        "let me hold",
        "let me pause",
        "let me stop",
        "waiting for you",
        "waiting on your",
        "vou aguardar",
        "vou esperar",
    ]
    .iter()
    .any(|p| line.contains(p));
    question || waiting
}

/// The word just before a Portuguese promise: "não vou"/"nunca irei" is a refusal (triage-14327).
fn refused_before(sentence: &str, at: usize) -> bool {
    sentence[..at].trim_end().rsplit_whitespace().next().is_some_and(|w| REFUSAL.contains(&w))
}

/// Whether one promise marker in `sentence` is followed by a verb (and not a "wait" word or a
/// refusal). Mirrors OpenBot's `PROMISES_WORK` without pulling in a regex crate.
fn matches_promise(sentence: &str, marker: &str) -> bool {
    for (at, _) in sentence.match_indices(marker) {
        // The marker must start a word, not sit inside one ("will" in "willingly").
        if sentence[..at].chars().next_back().is_some_and(|c| c.is_alphabetic() || c == '\'') {
            continue;
        }
        let rest = &sentence[at + marker.len()..];
        if !rest.starts_with(|c: char| c.is_whitespace()) {
            continue;
        }
        if (marker.starts_with("vou") || marker.contains("irei")) && refused_before(sentence, at) {
            continue;
        }
        let mut words = rest.split_whitespace();
        let mut word = words.next().unwrap_or("");
        while FILLER.contains(&word) {
            word = words.next().unwrap_or("");
        }
        let verb: String = word.chars().take_while(|c: &char| c.is_alphabetic()).collect();
        if verb.is_empty() || WAITING.contains(&verb.as_str()) {
            continue;
        }
        return true;
    }
    false
}

/// Whether a short final answer promises work in a turn that used no tool, so the bounded
/// completion check should run (OpenBot nightly triage-14327). A closing, a refusal and a question
/// all return false. Pure, so every case has a test without agent I/O.
pub(super) fn promises_work(answer: &str) -> bool {
    let text = answer.trim();
    if text.is_empty() || text.chars().count() > NEXT_STEP_ANSWER_MAX || waits_for_user(text) {
        return false;
    }
    let sentence = normalised(&last_sentence(text));
    PROMISES.iter().any(|marker| matches_promise(&sentence, marker))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_exact_phrases_are_continuations() {
        for phrase in PHRASES {
            assert!(is_continuation(phrase), "`{phrase}` continues");
            assert!(is_continuation(&format!("  {}  ", phrase.to_uppercase())), "case and space are ignored");
            assert!(is_continuation(&format!("{phrase}.")), "`{phrase}.` still continues");
            assert!(is_continuation(&format!("{phrase}!")), "`{phrase}!` still continues");
        }
    }

    #[test]
    fn questions_are_not_continuations() {
        for question in ["continue?", "keep going?", "go on?", "Should I continue?", "continue? please"] {
            assert!(!is_continuation(question), "`{question}` is a question, not a continuation");
        }
    }

    #[test]
    fn longer_or_other_text_is_not_a_continuation() {
        for text in ["", "   ", "please continue", "continue the work", "go on then", "no, continue", "resume"] {
            assert!(!is_continuation(text), "`{text}` is not one of the exact phrases");
        }
    }

    #[test]
    fn a_continuation_uses_the_bounded_self_check_budget() {
        for done in 0..MAX_SELF_CHECKS {
            assert!(continues("continue", done), "check {} is still in budget", done + 1);
        }
        assert!(!continues("continue", MAX_SELF_CHECKS), "no fourth check for one request");
        assert!(!continues("a question?", 0));
    }

    #[test]
    fn a_promise_to_work_gets_the_finish_check() {
        // triage-14327: words only, no tool, but work is promised.
        assert!(promises_work("Sure, I'll research the three options now."));
        assert!(promises_work("Pode deixar, vou pesquisar isso agora."));
        assert!(promises_work("I will check the logs."));
        assert!(promises_work("Let me look into it."));
        assert!(promises_work("I'm going to fix that."));
        assert!(promises_work("Deixa eu ver o que aconteceu."));
        assert!(
            promises_work("Done with the first file. Now I'll review the second."),
            "only the last sentence counts"
        );
        assert!(promises_work("**I'll** research it."), "markdown marks do not hide a promise");
        assert!(promises_work("Eu vou agora verificar."), "filler between marker and verb");
    }

    #[test]
    fn a_closing_a_wait_a_refusal_or_a_question_is_not_a_promise() {
        // triage-14327: a short closing still ends the request.
        assert!(!promises_work("Let me know if you need anything else."));
        assert!(!promises_work("I'll wait for your answer."));
        assert!(!promises_work("Vou aguardar sua resposta."));
        assert!(!promises_work("Should I continue?"));
        assert!(!promises_work("What should I research?"));
        assert!(!promises_work("I will not delete the files."), "a refusal is not work");
        assert!(!promises_work("I'll never share that."));
        assert!(!promises_work("Não vou apagar isso."));
        assert!(!promises_work("Nothing else needed from you."));
        assert!(!promises_work(""));
    }

    #[test]
    fn a_long_answer_is_not_scanned_for_a_promise() {
        let long = format!("{} I'll research it.", "x ".repeat(NEXT_STEP_ANSWER_MAX));
        assert!(!promises_work(&long), "a pasted document must not become a promise");
    }
}
