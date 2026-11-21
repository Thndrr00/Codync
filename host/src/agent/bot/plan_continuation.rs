//! A short follow-up ("continue", "keep going", "go on") means "finish what you were doing", not
//! "start a new request". It reuses the existing bounded self-check on the same ACP session
//! (`super::self_check`) instead of beginning another agent loop. The matcher is pure: questions
//! and longer sentences never count as continuations.

use super::self_check::MAX_SELF_CHECKS;

/// The whole set of short phrases OpenBot's nightly treats as continuing unfinished work.
const PHRASES: [&str; 3] = ["continue", "keep going", "go on"];

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
}
