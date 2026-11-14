//! The co-author trailer a bot adds to the git commits it writes, when the user turns it on for that bot
//! (`commitCoauthor`). OpenBot ends its agents' commits with its own trailer; Codync's is this one, documented
//! in `docs/features/commit-coauthor.md`.
//!
//! Codync never rewrites a commit's message itself: the bot's agent runs `git commit`, so the trailer reaches it
//! as an instruction in the bot's profile (`context::render`). `append` is for the few places Codync writes a
//! message itself, and it never adds the trailer twice.

/// The exact line Codync adds, as `git interpret-trailers` parses it: `Token: value`.
pub const TRAILER: &str = "Co-authored-by: Codync <bot@codync.dev>";

/// The paragraph in the bot's profile that asks its agent for the trailer. Empty when the setting is off.
pub fn section(enabled: bool) -> &'static str {
    if !enabled {
        return "";
    }
    "Commits: end the message of every git commit you create or amend, also with a message from the user, with this trailer once, on a line of its own after a blank line. Keep the user as the author, and add no other co-author line:\n\
     Co-authored-by: Codync <bot@codync.dev>"
}

/// `message` with [`TRAILER`] as its last line. A message that already ends with it (with any spacing before)
/// is returned unchanged, so a caller can pass a message that came back from the agent.
pub fn append(message: &str) -> String {
    let body = message.trim_end();
    if body.lines().rev().any(|line| line.trim() == TRAILER) {
        return body.to_owned();
    }
    if body.is_empty() {
        return TRAILER.to_owned();
    }
    format!("{body}\n\n{TRAILER}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_trailer_is_the_documented_line() {
        assert_eq!(TRAILER, "Co-authored-by: Codync <bot@codync.dev>");
        assert!(TRAILER.starts_with("Co-authored-by: "), "git reads it as a trailer");
    }

    #[test]
    fn a_message_gets_the_trailer_once_after_a_blank_line() {
        assert_eq!(append("fix(ios): keep the call bar"), format!("fix(ios): keep the call bar\n\n{TRAILER}"));
        assert_eq!(append(""), TRAILER);
        // An agent that already added it (with its own spacing) keeps one.
        let already = format!("feat(host): add it\n\n{TRAILER}");
        assert_eq!(append(&already), already);
        assert_eq!(append(&format!("feat(host): add it\n{TRAILER}  \n")), format!("feat(host): add it\n{TRAILER}"));
    }

    #[test]
    fn the_profile_asks_for_the_trailer_only_when_the_setting_is_on() {
        assert!(section(false).is_empty());
        let on = section(true);
        assert!(on.contains(TRAILER), "the exact line, so the agent copies it");
        assert!(on.starts_with("Commits: "));
    }
}
