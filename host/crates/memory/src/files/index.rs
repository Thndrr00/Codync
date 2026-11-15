//! The index of notes an opening carries: each note a bot sees, with its description and size.

use rusqlite::Connection;

use super::{DEPTH_MAX, FILE_CAP, FOLDER_CAP, File, MAIN, layer};
use crate::check;
use crate::layers::{Layers, Scope};
use crate::store::Result;

/// The notes an opening lists: the live ones of each layer the bot sees. `MEMORY.md` is in the opening itself.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Index {
    /// The bot the opening is for.
    pub owner: String,
    /// Its brain's notes; `None` when the bot works in no brain.
    pub brain: Option<Brain>,
    pub bot: Vec<File>,
}

/// A brain's notes in an index.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Brain {
    pub name: String,
    /// The key the brain's files are stored under (`None`: the shared folder, for the Fleet brain).
    pub layer: Option<String>,
    pub files: Vec<File>,
}

pub fn index(conn: &Connection, layers: &Layers) -> Result<Index> {
    let notes = |scope: Scope| -> Result<Vec<File>> {
        Ok(layer(conn, layers.layer(scope))?.into_iter().filter(|f| !f.deleted && f.name != MAIN).collect())
    };
    let brain = match layers.brain.as_ref() {
        Some(brain) => {
            Some(Brain { name: brain.name.clone(), layer: brain.layer.clone(), files: notes(Scope::Brain)? })
        }
        None => None,
    };
    Ok(Index { owner: layers.bot.clone(), brain, bot: notes(Scope::Personal)? })
}

impl Index {
    /// The section of the opening, with each layer's folder as `folder` names it. Files and folders over their caps
    /// are marked, never cut.
    pub fn render(&self, folder: impl Fn(Option<&str>) -> String) -> String {
        let mut lines = vec![format!(
            "Notes: one topic each, for detail too long for MEMORY.md (lists, itineraries, histories), in topic \
             folders up to {DEPTH_MAX} deep, like a project's docs (people/family.md, health/allergies.md). Read a \
             note when its topic comes up. Each starts with frontmatter whose description: line says when to read \
             it; names are lowercase-with-dashes."
        )];
        lines.push(format!("This bot's own, in {}:", folder(Some(&self.owner))));
        lines.extend(Self::folder_lines(&self.bot));
        if let Some(brain) = &self.brain {
            let about = if brain.layer.is_none() {
                "Shared with the user's other bots".to_owned()
            } else {
                format!("The brain \"{}\", shared with the other bots working in it", brain.name)
            };
            lines.push(format!("{about}, in {}:", folder(brain.layer.as_deref())));
            lines.extend(Self::folder_lines(&brain.files));
        }
        lines.join("\n")
    }

    fn folder_lines(files: &[File]) -> Vec<String> {
        if files.is_empty() {
            return vec!["(none yet)".into()];
        }
        let mut lines: Vec<String> = files.iter().map(index_line).collect();
        if files.len() > FOLDER_CAP {
            lines.push(format!("({} notes, over the {FOLDER_CAP}-note cap: merge some.)", files.len()));
        }
        lines
    }
}

fn index_line(file: &File) -> String {
    let about = match &file.description {
        Some(text) => match check::content(text) {
            Ok(()) => text.clone(),
            Err(refusal) => format!("(description withheld: {refusal})"),
        },
        None => "(no description)".into(),
    };
    let size = if file.tokens > FILE_CAP {
        format!("{} tokens, over the {FILE_CAP} cap: shorten it", file.tokens)
    } else {
        format!("{} tokens", file.tokens)
    };
    format!("- {}: {about} ({size})", file.name)
}
