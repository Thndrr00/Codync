//! A bounded, agent-facing map from "where is X?" to the settings path that already exists.
//!
//! OpenBot 0.7x (15947/16129/16151/15623/15922) found that an agent asked where to change a
//! setting answered "I can't" even though the path was on screen. This is a fixed catalog: a
//! small set of keywords, a fixed answer per topic, no new settings surface and no browsing of
//! the app. It is appended to the MCP instructions so the answer travels with the tools an agent
//! already has. Nothing here reads a question from chat or sends a click.

use std::borrow::Cow;

/// The one topic a question is about. Keyword matches are case-insensitive and ordered so the
/// most specific topic wins; anything else gets no answer rather than a guessed path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SettingTopic {
    Vault,
    Import,
    ClaudeSetup,
    Account,
    Provider,
}

/// The exact settings path for `topic`, in the desktop app's own words.
pub(crate) fn setting_path(topic: SettingTopic) -> &'static str {
    match topic {
        SettingTopic::Vault => "Settings → Marketplace → Credentials: saved logins and the encrypted vault.",
        SettingTopic::Import => "Settings → Import an agent: bring another agent's folder into a new bot.",
        SettingTopic::ClaudeSetup => {
            "Settings → Import an agent: use a Claude Code setup you already have on this computer."
        }
        SettingTopic::Account => "Settings → Account: the signed-in account and this computer's access.",
        SettingTopic::Provider => "Settings → Providers: the agent CLIs on this computer, their updates and usage.",
    }
}

/// Which topic `question` asks about, if any. Bounded: only the first 400 characters are read,
/// so an enormous message cannot turn this into a search over arbitrary text.
pub(crate) fn setting_topic(question: &str) -> Option<SettingTopic> {
    let text = question.chars().take(400).collect::<String>().to_ascii_lowercase();
    let has = |words: &[&str]| words.iter().any(|word| text.contains(word));
    if has(&["claude code", "use my claude", "claude setup"]) {
        Some(SettingTopic::ClaudeSetup)
    } else if has(&["vault", "password", "saved login", "credential"]) {
        Some(SettingTopic::Vault)
    } else if has(&["import", "bring over", "migrate"]) {
        Some(SettingTopic::Import)
    } else if has(&["provider", "agent cli", "which agent", "usage"]) {
        Some(SettingTopic::Provider)
    } else if has(&["account", "signed in", "sign in"]) {
        Some(SettingTopic::Account)
    } else {
        None
    }
}

/// The fixed help line added to MCP instructions; no question text ever enters it.
pub(crate) const SETTINGS_HELP: &str = "If someone asks where a setting is, answer from this fixed map instead of saying you cannot: \
vault or saved passwords → Settings, Marketplace, Credentials; import another agent or use my Claude Code setup → \
Settings, Import an agent; account or sign-in → Settings, Account; agent providers and usage → Settings, Providers. \
Do not invent a path for anything else.";

/// Adds the catalog to an instruction string without changing who owns the original string.
pub(crate) fn with_catalog(base: &'static str) -> Cow<'static, str> {
    Cow::Owned(format!("{base}\n{SETTINGS_HELP}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_question_finds_its_settings_path() {
        for (question, topic) in [
            ("Where is the vault?", SettingTopic::Vault),
            ("how do I import my old agent", SettingTopic::Import),
            ("use my Claude Code setup", SettingTopic::ClaudeSetup),
            ("which account am I signed in as", SettingTopic::Account),
            ("where are the providers?", SettingTopic::Provider),
        ] {
            assert_eq!(setting_topic(question), Some(topic), "{question}");
            assert!(setting_path(topic).starts_with("Settings →"));
        }
    }

    #[test]
    fn an_unknown_question_gets_no_invented_path() {
        assert_eq!(setting_topic("what is the weather"), None);
        assert_eq!(setting_topic(""), None);
        assert_eq!(setting_topic(&format!("{} vault", "x".repeat(2_000))), None, "the question is bounded");
    }

    #[test]
    fn the_help_is_fixed_and_travels_with_the_instructions() {
        assert!(SETTINGS_HELP.contains("Settings"));
        let joined = with_catalog("base instructions");
        assert!(joined.starts_with("base instructions\n"));
        assert!(joined.contains("Import an agent"));
        assert!(!SETTINGS_HELP.contains("http"));
    }
}
