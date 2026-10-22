//! Every change to a bot's instructions (its description) is kept as a numbered version with
//! who made it and why, so a change can be compared and undone. An edit can name the version it
//! started from (`ifInstructionsVersion`): when someone changed the instructions since, it's
//! refused with a conflict instead of silently overwriting them.
//!
//! Idea from rakazo's `packages/core/src/instruction-diff.ts` (Apache-2.0); no code copied.

use crate::LockExt;
use crate::store::{BotConfig, Store, now_ms};
use anyhow::{Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fmt::Write as _;
use std::sync::{Mutex, MutexGuard};

/// Versions kept per bot (the oldest go first).
const KEPT: usize = 50;
/// Line pairs a diff compares before it shows the whole text as replaced.
const MAX_DIFF_CELLS: usize = 4_000_000;
/// Patch keys that describe an edit rather than set a field.
pub const IF_VERSION: &str = "ifInstructionsVersion";
pub const AUTHOR: &str = "instructionsAuthor";
pub const REASON: &str = "instructionsReason";

/// One bot update at a time, from reading the bot to saving it: two edits can't both pass the
/// version check, and an edit of something else can't save a stale copy of the instructions.
static EDIT: Mutex<()> = Mutex::new(());

/// Held by `Hub::update_bot` around its read, check, save and record.
pub fn lock() -> MutexGuard<'static, ()> {
    EDIT.locked()
}

/// Who changed the instructions.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Author {
    /// The user, in the bot's settings or through the API.
    #[default]
    User,
    /// The bot itself (`set_instructions`).
    Bot,
    /// The bot that created it, with its fleet tools (the reason names it; see `fleet`).
    Fleet,
    /// What the bot had before its history started.
    Earlier,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Revision {
    pub version: u32,
    pub text: String,
    pub author: Author,
    #[serde(default)]
    pub reason: Option<String>,
    /// Unix milliseconds; 0 for an `Earlier` version.
    pub at: i64,
}

fn key(bot: &str) -> String {
    format!("instructions.history.{bot}")
}

fn history(store: &Store, bot: &str) -> Result<Vec<Revision>> {
    Ok(match store.kv_read(&key(bot))? {
        Some(raw) => serde_json::from_str(&raw)?,
        None => vec![],
    })
}

/// The version `text` is now: the last recorded one, or 1 for instructions older than the history.
fn current(versions: &[Revision], text: &str) -> u32 {
    versions.last().map_or(u32::from(!text.is_empty()), |v| v.version)
}

/// An edit of `bot`'s instructions, from the version check until it's recorded (under [`lock`]).
pub struct Edit {
    author: Author,
    reason: Option<String>,
    /// The bot and its history as it was before `record`, for `undo`.
    before: Option<(String, Option<String>)>,
}

/// Checks a patch that may change `old`'s instructions; `None` when it doesn't touch them.
pub fn begin(old: &BotConfig, new: &BotConfig, patch: &Value, store: &Store) -> Result<Option<Edit>> {
    let expected = patch.get(IF_VERSION).filter(|v| !v.is_null());
    if new.description == old.description && expected.is_none() {
        return Ok(None);
    }
    let versions = history(store, &old.id)?;
    let now = current(&versions, &old.description);
    if let Some(expected) = expected {
        let expected = expected.as_u64().ok_or_else(|| anyhow!("{IF_VERSION} is a version number"))?;
        if expected != u64::from(now) {
            let by = versions.last().map_or(Author::Earlier, |v| v.author);
            bail!(
                "The instructions changed since version {expected}: they're at version {now} (by {by:?}). Read them again with instruction_history before changing them."
            );
        }
    }
    let author = serde_json::from_value(patch[AUTHOR].clone()).unwrap_or_default();
    let reason = patch[REASON].as_str().map(str::trim).filter(|r| !r.is_empty()).map(|r| r.chars().take(300).collect());
    Ok(Some(Edit { author, reason, before: None }))
}

impl Edit {
    /// Records the change before it's saved (`old` becomes version 1 if the history was empty);
    /// [`Edit::undo`] takes it back when the save fails.
    pub fn record(&mut self, store: &Store, old: &BotConfig, new: &BotConfig) -> Result<()> {
        if new.description == old.description {
            return Ok(());
        }
        let raw = store.kv_read(&key(&old.id))?;
        let mut versions: Vec<Revision> = raw.as_deref().map(serde_json::from_str).transpose()?.unwrap_or_default();
        if versions.is_empty() && !old.description.is_empty() {
            versions.push(Revision {
                version: 1,
                text: old.description.clone(),
                author: Author::Earlier,
                reason: None,
                at: 0,
            });
        }
        let version = current(&versions, &old.description) + 1;
        versions.push(Revision {
            version,
            text: new.description.clone(),
            author: self.author,
            reason: self.reason.clone(),
            at: now_ms(),
        });
        let skip = versions.len().saturating_sub(KEPT);
        store.kv_set(&key(&old.id), &serde_json::to_string(&versions[skip..])?)?;
        self.before = Some((old.id.clone(), raw));
        Ok(())
    }

    /// Puts the history back as it was before [`Edit::record`].
    pub fn undo(self, store: &Store) -> Result<()> {
        match self.before {
            Some((bot, Some(raw))) => store.kv_set(&key(&bot), &raw),
            Some((bot, None)) => store.kv_delete(&key(&bot)),
            None => Ok(()),
        }
    }
}

/// `bot`'s versions, newest first, without their text; `current` is the version in use.
pub fn list(store: &Store, bot: &BotConfig) -> Result<Value> {
    let versions = history(store, &bot.id)?;
    let items: Vec<Value> = versions
        .iter()
        .rev()
        .map(|v| json!({"version": v.version, "author": v.author, "reason": v.reason, "at": v.at, "chars": v.text.chars().count()}))
        .collect();
    Ok(json!({"current": current(&versions, &bot.description), "versions": items}))
}

/// The text of `version`.
pub fn text(store: &Store, bot: &str, version: u32) -> Result<String> {
    history(store, bot)?
        .into_iter()
        .find(|v| v.version == version)
        .map(|v| v.text)
        .ok_or_else(|| anyhow!("No version {version} of these instructions is kept"))
}

/// What changed from version `from` to `to`, line by line (`-` removed, `+` added, ` ` kept).
pub fn diff(store: &Store, bot: &str, from: u32, to: u32) -> Result<String> {
    Ok(lines_diff(&text(store, bot, from)?, &text(store, bot, to)?))
}

/// A line diff by longest common subsequence, O(n·m) up to `MAX_DIFF_CELLS`.
fn lines_diff(old: &str, new: &str) -> String {
    let (a, b): (Vec<&str>, Vec<&str>) = (old.lines().collect(), new.lines().collect());
    // Past this many line pairs the table costs too much: show the whole text replaced.
    if a.len().saturating_mul(b.len()) > MAX_DIFF_CELLS {
        return a.iter().map(|l| format!("- {l}\n")).chain(b.iter().map(|l| format!("+ {l}\n"))).collect();
    }
    let mut common = vec![vec![0u32; b.len() + 1]; a.len() + 1];
    for i in (0..a.len()).rev() {
        for j in (0..b.len()).rev() {
            common[i][j] = if a[i] == b[j] { common[i + 1][j + 1] + 1 } else { common[i + 1][j].max(common[i][j + 1]) };
        }
    }
    let (mut i, mut j, mut out) = (0, 0, String::new());
    while i < a.len() || j < b.len() {
        if i < a.len() && j < b.len() && a[i] == b[j] {
            let _ = writeln!(out, "  {}", a[i]);
            (i, j) = (i + 1, j + 1);
        } else if i < a.len() && (j == b.len() || common[i + 1][j] >= common[i][j + 1]) {
            let _ = writeln!(out, "- {}", a[i]);
            i += 1;
        } else {
            let _ = writeln!(out, "+ {}", b[j]);
            j += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bot(text: &str) -> BotConfig {
        serde_json::from_value(json!({"id":"bot","name":"Bot","backend":"x","cwd":"/tmp","description":text})).unwrap()
    }

    #[test]
    fn diffs_show_removed_and_added_lines() {
        assert_eq!(lines_diff("a\nb\nc", "a\nB\nc"), "  a\n- b\n+ B\n  c\n");
        assert_eq!(lines_diff("", "new"), "+ new\n");
        // Too long to compare line by line: the whole text shows as replaced.
        let long = (0..3000).map(|n| n.to_string()).collect::<Vec<_>>().join("\n");
        assert!(lines_diff(&long, &long.replace("1\n", "x\n")).starts_with("- 0\n"));
    }

    #[test]
    fn edits_are_versioned_and_stale_ones_refused() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let (v1, v2, v3) = (bot("Reviews PRs."), bot("Reviews PRs.\nBe brief."), bot("Writes tests."));
        let mut edit =
            begin(&v1, &v2, &json!({AUTHOR: "bot", REASON: "user asked for brevity"}), &store).unwrap().unwrap();
        edit.record(&store, &v1, &v2).unwrap();
        let listed = list(&store, &v2).unwrap();
        assert_eq!(listed["current"], 2);
        assert_eq!(listed["versions"][0]["author"], "bot");
        assert_eq!(listed["versions"][1]["author"], "earlier");
        assert!(diff(&store, "bot", 1, 2).unwrap().contains("+ Be brief."));
        // Someone who read version 1 can't overwrite version 2.
        assert!(begin(&v2, &v3, &json!({IF_VERSION: 1}), &store).is_err());
        let mut edit = begin(&v2, &v3, &json!({IF_VERSION: 2}), &store).unwrap().unwrap();
        edit.record(&store, &v2, &v3).unwrap();
        assert_eq!(text(&store, "bot", 3).unwrap(), "Writes tests.");
        // A save that failed takes its version back.
        edit.undo(&store).unwrap();
        assert!(text(&store, "bot", 3).is_err());
        assert_eq!(list(&store, &v2).unwrap()["current"], 2);
        assert!(begin(&v3, &v3, &json!({}), &store).unwrap().is_none());
    }
}
