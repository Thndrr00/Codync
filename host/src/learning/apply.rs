//! Saving an approved fact: it becomes one line of the bot's `preferences.md` note, written as the person (so a bot
//! may change or forget it later without the strict outside-a-turn rules). The same fact is not written twice, in the
//! note or in `MEMORY.md`.

use super::detect::normalize;
use crate::chat::memory::files::{self, Writer};
use crate::hub::Hub;
use crate::store::BotConfig;
use anyhow::Result;
use codync_mem::files::belief::{self, Belief, Confidence, Kind};
use codync_mem::files::{self as mf, Author, Cause};
use std::sync::Arc;

/// The note approved facts go to. Its header says what it holds, so the bot reads it as feedback rather than a fact
/// about a project.
pub const NOTE: &str = "preferences.md";
const DESCRIPTION: &str = "How the user wants the bot to work";

/// Whether the bot's memory already says `text` (in the note or in `MEMORY.md`), so it is not proposed or saved again.
pub fn known(hub: &Hub, cfg: &BotConfig, text: &str) -> Result<bool> {
    files::scan(hub, cfg);
    let want = normalize(text);
    if want.is_empty() {
        return Ok(true);
    }
    for name in [mf::MAIN, NOTE] {
        if let Some(content) = files::read(Some(&cfg.id), name)?
            && mf::facts(&content).iter().any(|fact| normalize(fact) == want)
        {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Adds `text` to the bot's `preferences.md` as the person's, unless it is already there. Returns whether it wrote.
pub fn save(hub: &Arc<Hub>, cfg: &BotConfig, text: &str) -> Result<bool> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(false);
    }
    files::scan(hub, cfg);
    let layer = Some(cfg.id.as_str());
    let before = files::read(layer, NOTE)?;
    if before.as_deref().is_some_and(|content| mf::facts(content).iter().any(|fact| normalize(fact) == normalize(text)))
    {
        return Ok(false);
    }
    let stamp = Belief { kind: Kind::Feedback, confidence: Confidence::High, confirmed: Some(today()) };
    let body = match &before {
        Some(content) => {
            let mut next = content.trim_end().to_owned();
            next.push_str("\n- ");
            next.push_str(text);
            next.push('\n');
            next
        }
        None => format!("---\ndescription: {DESCRIPTION}\n---\n- {text}\n"),
    };
    let content = belief::stamp(&body, &stamp);
    let expected = before.as_deref().map(mf::hash);
    files::write(
        hub,
        layer,
        NOTE,
        Some(&content),
        expected.as_deref(),
        Writer { author: Author::User, bot: None, cause: Cause::Edit, message: None, channel: false },
    )?;
    Ok(true)
}

fn today() -> String {
    crate::chat::memory::ymd(crate::store::now_ms())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_preference_note_says_it_is_feedback() {
        let content = belief::stamp(
            &format!("---\ndescription: {DESCRIPTION}\n---\n- Use pnpm, not npm\n"),
            &Belief { kind: Kind::Feedback, confidence: Confidence::High, confirmed: Some("2026-11-16".into()) },
        );
        assert_eq!(belief::read(&content).kind, Kind::Feedback);
        assert_eq!(belief::read(&content).confidence, Confidence::High);
        assert_eq!(mf::facts(&content), ["Use pnpm, not npm"]);
    }
}
