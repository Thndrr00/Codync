//! A bot's memory, read-only as in the other apps: what it knows about you (shared by your bots) and on its own,
//! its notes, its dream log, and Claude Code's own memory where the user kept it on. The user changes memory by
//! telling the bot.

use super::super::app::{App, Confirm, ConfirmAct, Editor, Overlay, OwnMemory};
use super::super::l10n::tr;
use super::s;
use super::{Fields, Input, Page, Reply, Submit, ctrl};
use crossterm::event::{KeyCode, KeyEvent};
use serde_json::{Value, json};
use std::cell::Cell;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    Yours,
    Bot,
    /// Both layers' long-form files.
    Notes,
    Dreams,
    /// Claude Code's own memory (its `MEMORY.md`), read-only, where the user kept it on.
    Harness,
}

impl Section {
    const ALL: [Self; 5] = [Self::Yours, Self::Bot, Self::Notes, Self::Dreams, Self::Harness];

    pub fn label(self) -> &'static str {
        match self {
            Self::Yours => "About you",
            Self::Bot => "This bot",
            Self::Notes => "Notes",
            Self::Dreams => "Dreams",
            Self::Harness => "Claude Code",
        }
    }
}

pub struct MemorySheet {
    pub bot: String,
    /// The host's `memory` reply.
    pub view: Option<Value>,
    pub section: Section,
    /// The row picked in the section's list.
    pub cursor: usize,
    /// The open note's, or Claude Code's memory's, scroll.
    pub scroll: Scroll,
    /// The host's `brains` reply, once the brain list is open.
    pub brains: Option<Vec<Value>>,
    /// The brain the picker is on.
    pub brain_cursor: usize,
    pub mode: MemoryMode,
    pub error: Option<String>,
    pub busy: bool,
}

pub enum MemoryMode {
    Browse,
    /// The brains the bot can work in: pick one (Enter), make one (`n`), rename (`r`), delete (`d`), export
    /// (`x`) or import (`i`).
    Brains,
    /// One note: what it holds (`detail`, once fetched).
    File {
        name: String,
        shared: bool,
        detail: Option<Value>,
    },
}

/// A read-only text's scroll, by wrapped row; the last draw sets how far it goes and a page's height.
#[derive(Default)]
pub struct Scroll {
    pub pos: usize,
    pub max: Cell<usize>,
    pub page: Cell<usize>,
}

impl Scroll {
    /// ↑↓, PageUp/PageDown and Ctrl-d/u; false for a key it doesn't take.
    fn key(&mut self, k: KeyEvent) -> bool {
        let max = self.max.get();
        let half = (self.page.get() / 2).max(1);
        // A refresh or a resize can leave the position past the end.
        let pos = self.pos.min(max);
        self.pos = match k.code {
            KeyCode::Down | KeyCode::Char('j') => (pos + 1).min(max),
            KeyCode::Up | KeyCode::Char('k') => pos.saturating_sub(1),
            KeyCode::PageDown => (pos + half).min(max),
            KeyCode::PageUp => pos.saturating_sub(half),
            KeyCode::Char('d') if ctrl(k) => (pos + half).min(max),
            KeyCode::Char('u') if ctrl(k) => pos.saturating_sub(half),
            _ => return false,
        };
        true
    }
}

impl MemorySheet {
    pub fn body(&self) -> Value {
        json!({"botId": self.bot})
    }

    /// The note sheet's fetch, when one is open.
    pub fn file_body(&self) -> Option<Value> {
        match &self.mode {
            MemoryMode::File { name, shared, .. } => Some(json!({"botId": self.bot, "file": name, "shared": shared})),
            MemoryMode::Browse | MemoryMode::Brains => None,
        }
    }

    /// The brains the picker lists, the Fleet one first.
    pub fn brain_rows(&self) -> Vec<Value> {
        self.brains.clone().unwrap_or_default()
    }

    /// The brain the picker is on.
    pub fn picked_brain(&self) -> Option<Value> {
        self.brain_rows().get(self.brain_cursor).cloned()
    }

    /// The current section's rows: entries, notes (deleted ones left out), or dream runs.
    pub fn rows(&self) -> Vec<Value> {
        let Some(view) = &self.view else { return Vec::new() };
        let list = match self.section {
            Section::Yours => &view["yours"]["entries"],
            Section::Bot => &view["bot"]["entries"],
            Section::Dreams => &view["dreams"],
            Section::Notes => {
                // This bot's own first, then the shared ones; each row says which.
                let mut rows = Vec::new();
                for (layer, shared) in [("bot", false), ("yours", true)] {
                    for file in
                        view["files"][layer]["files"].as_array().into_iter().flatten().filter(|f| f["deleted"] != true)
                    {
                        let mut file = file.clone();
                        file["shared"] = shared.into();
                        rows.push(file);
                    }
                }
                return rows;
            }
            Section::Harness => return Vec::new(),
        };
        list.as_array().cloned().unwrap_or_default()
    }

    /// Takes a fresh `memory` reply, keeping the cursor on the row it was on: pinning or archiving reorders them.
    pub fn show(&mut self, view: Value) {
        let key = |row: &Value| json!([row["id"], row["name"], row["shared"]]);
        let was = self.rows().get(self.cursor).map(key);
        self.view = Some(view);
        if !self.sections().contains(&self.section) {
            // Claude Code's memory was just turned off.
            self.section = Section::Bot;
        }
        let rows = self.rows();
        self.cursor = was
            .and_then(|k| rows.iter().position(|row| key(row) == k))
            .unwrap_or_else(|| self.cursor.min(rows.len().saturating_sub(1)));
        // The picker opens on the brain the bot works in.
        let current = self.view.as_ref().and_then(|v| v["brain"]["id"].as_str()).map(str::to_owned);
        let brains = self.brain_rows();
        self.brain_cursor = current
            .and_then(|id| brains.iter().position(|b| b["id"] == id.as_str()))
            .unwrap_or_else(|| self.brain_cursor.min(brains.len().saturating_sub(1)));
    }

    /// Sections worth showing: Claude Code's memory only when it is kept on.
    pub fn sections(&self) -> Vec<Section> {
        let harness = self.view.as_ref().is_some_and(|v| v["projectMemory"].is_object());
        Section::ALL.into_iter().filter(|s| *s != Section::Harness || harness).collect()
    }

    fn step(&mut self, by: isize) {
        let sections = self.sections();
        let at = sections.iter().position(|s| *s == self.section).unwrap_or(0);
        self.section = sections[at.saturating_add_signed(by).min(sections.len() - 1)];
        self.cursor = 0;
        self.scroll = Scroll::default();
    }
}

impl App {
    pub fn open_memory(&mut self) {
        let Some(bot) = self.bot().filter(|b| !b.group).map(|b| b.id.clone()) else {
            self.flash("A group has no memory of its own");
            return;
        };
        let m = MemorySheet {
            bot: bot.clone(),
            view: None,
            section: Section::Bot,
            cursor: 0,
            scroll: Scroll::default(),
            brains: None,
            brain_cursor: 0,
            mode: MemoryMode::Browse,
            error: None,
            busy: true,
        };
        self.sheet("memory", m.body(), Reply::Memory(bot));
        self.overlays.push(Overlay::Memory(m));
    }

    /// The name of the bot's harness as the agent list gives it ("Claude Code").
    pub fn harness_name(&self, bot: &str) -> String {
        self.bots.get(bot).map(|b| self.agent_name(&b.backend)).unwrap_or_default()
    }

    /// A bot's memory changed somewhere (its `memoryRev` moved): an open sheet fetches it again.
    pub(in crate::tui) fn memory_moved(&mut self, bot: &Value) {
        let mut stale = Vec::new();
        for o in &self.overlays {
            if let Overlay::Memory(m) = o
                && bot["id"] == m.bot.as_str()
                && m.view.as_ref().is_some_and(|v| v["memoryRev"] != bot["memoryRev"])
            {
                stale.push(m.bot.clone());
            }
        }
        for id in stale {
            self.sheet("memory", json!({"botId": id}), Reply::Memory(id.clone()));
        }
    }

    pub(in crate::tui) fn memory_key(&mut self, mut m: MemorySheet, k: KeyEvent) -> Option<Overlay> {
        if let MemoryMode::File { .. } = m.mode {
            if matches!(k.code, KeyCode::Esc | KeyCode::Char('q')) {
                m.mode = MemoryMode::Browse;
            } else {
                m.scroll.key(k);
            }
            return Some(Overlay::Memory(m));
        }
        if let MemoryMode::Brains = m.mode {
            return self.brain_key(m, k);
        }
        if let Some(page) = Page::Memory.step(k) {
            self.open_page(page);
            return None;
        }
        // Claude Code's memory scrolls; the other sections move between rows.
        if m.section == Section::Harness && m.scroll.key(k) {
            return Some(Overlay::Memory(m));
        }
        let rows = m.rows();
        match k.code {
            KeyCode::Esc | KeyCode::Char('q' | 'M') => return None,
            KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => self.step_section(&mut m, 1),
            KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => self.step_section(&mut m, -1),
            KeyCode::Down | KeyCode::Char('j') => m.cursor = (m.cursor + 1).min(rows.len().saturating_sub(1)),
            KeyCode::Up | KeyCode::Char('k') => m.cursor = m.cursor.saturating_sub(1),
            KeyCode::Char('r') => {
                m.busy = true;
                self.sheet("memory", m.body(), Reply::Memory(m.bot.clone()));
            }
            KeyCode::Char('b') if !m.busy => {
                m.mode = MemoryMode::Brains;
                self.sheet("brains", json!({}), Reply::Brains(m.bot.clone()));
            }
            KeyCode::Enter if m.section == Section::Notes => {
                if let Some(file) = rows.get(m.cursor) {
                    m.mode = MemoryMode::File { name: s(file, "name"), shared: file["shared"] == true, detail: None };
                    m.scroll = Scroll::default();
                    if let Some(body) = m.file_body() {
                        self.sheet("memoryFile", body, Reply::MemoryFile(m.bot.clone()));
                    }
                }
            }
            KeyCode::Char('m') if !m.busy => return self.harness_memory_key(m),
            _ => {}
        }
        Some(Overlay::Memory(m))
    }

    /// The brain picker: Enter gives the bot the highlighted brain, `n` makes one, `r` renames it, `d` deletes it,
    /// `x` saves it to a file and `i` brings one in from a file.
    fn brain_key(&mut self, mut m: MemorySheet, k: KeyEvent) -> Option<Overlay> {
        let rows = m.brain_rows();
        match k.code {
            KeyCode::Esc | KeyCode::Char('q') => m.mode = MemoryMode::Browse,
            KeyCode::Down | KeyCode::Char('j') => {
                m.brain_cursor = (m.brain_cursor + 1).min(rows.len().saturating_sub(1));
            }
            KeyCode::Up | KeyCode::Char('k') => m.brain_cursor = m.brain_cursor.saturating_sub(1),
            KeyCode::Enter if !m.busy => {
                if let Some(brain) = m.picked_brain() {
                    let current = m.view.as_ref().and_then(|v| v["brain"]["id"].as_str()) == brain["id"].as_str();
                    if !current {
                        self.sheet(
                            "joinBrain",
                            json!({"botId": m.bot, "brainId": s(&brain, "id")}),
                            Reply::MemorySaved(m.bot.clone()),
                        );
                    }
                }
            }
            KeyCode::Char('n') if !m.busy => {
                let submit = Submit::BrainName { bot: m.bot.clone(), id: None };
                return self.push_all(Self::brain_fields(m, submit, ""));
            }
            KeyCode::Char('r') if !m.busy => {
                if let Some(brain) = m.picked_brain().filter(|b| b["shared"] != true) {
                    let submit = Submit::BrainName { bot: m.bot.clone(), id: Some(s(&brain, "id")) };
                    return self.push_all(Self::brain_fields(m, submit, &s(&brain, "name")));
                }
            }
            KeyCode::Char('d') if !m.busy => {
                if let Some(brain) = m.picked_brain().filter(|b| b["shared"] != true) {
                    let name = s(&brain, "name");
                    let c = Confirm {
                        title: tr("Delete the brain {name}?").replace("{name}", &name),
                        detail: "Its bots go back to the Fleet brain and keep their own memory.".into(),
                        note: "What the brain holds is gone.".into(),
                        button: "Delete",
                        act: ConfirmAct::DeleteBrain { bot: m.bot.clone(), id: s(&brain, "id") },
                    };
                    self.overlays.push(Overlay::Memory(m));
                    self.overlays.push(Overlay::Confirm(c));
                    return None;
                }
            }
            KeyCode::Char('x') if !m.busy => {
                if let Some(brain) = m.picked_brain() {
                    self.sheet("exportBrain", json!({"id": s(&brain, "id")}), Reply::BrainExported(s(&brain, "name")));
                }
            }
            KeyCode::Char('i') if !m.busy => {
                let submit = Submit::BrainImport { bot: m.bot.clone() };
                return self.push_all(Self::brain_fields(m, submit, ""));
            }
            _ => {}
        }
        Some(Overlay::Memory(m))
    }

    /// Pushes `overlays` and keeps none on the stack: the picker holds its state in its own overlay.
    fn push_all(&mut self, overlays: Vec<Overlay>) -> Option<Overlay> {
        self.overlays.extend(overlays);
        None
    }

    /// A text field over the picker: a brain's name, or the file to import one from.
    fn brain_fields(m: MemorySheet, submit: Submit, value: &str) -> Vec<Overlay> {
        let (title, label, note) = match &submit {
            Submit::BrainImport { .. } => (
                "Import a brain",
                "File",
                "The path of a brain file this app saved. Its files go through the same checks as the bot's.",
            ),
            Submit::BrainName { id: Some(_), .. } => ("Rename the brain", "Name", "What the bots in it share."),
            _ => (
                "New brain",
                "Name",
                "What the bots in it share. The Fleet brain is the memory every bot that uses the shared memory reads.",
            ),
        };
        let fields = Fields {
            title: title.into(),
            note: note.into(),
            choices: vec![],
            choice: 0,
            inputs: vec![Input {
                name: "value".into(),
                label: label.into(),
                secret: false,
                required: true,
                multiline: false,
                placeholder: String::new(),
                ed: Editor::with(value),
            }],
            cursor: 0,
            error: None,
            saving: false,
            submit,
        };
        vec![Overlay::Memory(m), Overlay::Fields(Box::new(fields))]
    }

    /// Moves to another section. Claude Code writes its notes outside Codync, so nothing moves `memoryRev`: its
    /// section reads them again on the way in.
    fn step_section(&mut self, m: &mut MemorySheet, by: isize) {
        m.step(by);
        if m.section == Section::Harness {
            m.busy = true;
            self.sheet("memory", m.body(), Reply::Memory(m.bot.clone()));
        }
    }

    /// `m`: turns the harness's own memory off, or asks first, saying what it costs, before turning it on.
    fn harness_memory_key(&mut self, mut m: MemorySheet) -> Option<Overlay> {
        match self.bots.get(&m.bot).and_then(|b| b.harness_memory) {
            Some(OwnMemory::Switch(true)) => {
                m.busy = true;
                self.sheet(
                    "updateBot",
                    json!({"id": m.bot, "harnessMemory": false}),
                    Reply::MemorySaved(m.bot.clone()),
                );
            }
            Some(OwnMemory::Switch(false)) => {
                // One line each in the confirmation: the info sheets' points, short.
                let c = Confirm {
                    title: tr("Turn on {name}'s own memory?").replace("{name}", &self.harness_name(&m.bot)),
                    detail: "It may store things twice; the two memories can disagree.".into(),
                    note: "Forgetting something in Codync doesn't reach it.".into(),
                    button: "Turn on",
                    act: ConfirmAct::KeepHarnessMemory(m.bot.clone()),
                };
                self.overlays.push(Overlay::Memory(m));
                self.overlays.push(Overlay::Confirm(c));
                return None;
            }
            Some(OwnMemory::Shared) | None => {}
        }
        Some(Overlay::Memory(m))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cursor_follows_its_entry_when_pinning_reorders_them() {
        let view = |ids: [i64; 3]| json!({"bot": {"entries": ids.map(|id| json!({"id": id}))}});
        let mut m = MemorySheet {
            bot: "b".into(),
            view: Some(view([1, 2, 3])),
            section: Section::Bot,
            cursor: 2,
            scroll: Scroll::default(),
            brains: None,
            brain_cursor: 0,
            mode: MemoryMode::Browse,
            error: None,
            busy: false,
        };
        m.show(view([3, 1, 2]));
        assert_eq!(m.cursor, 0, "entry 3 was pinned to the top");
        m.show(view([1, 2, 4]));
        assert_eq!(m.cursor, 0, "gone: the cursor stays in range");
    }

    #[test]
    fn a_memory_change_keeps_an_open_notes_scroll_and_its_row() {
        let files = |names: [&str; 2]| json!({"files": {"bot": {"files": names.map(|n| json!({"name": n}))}}});
        let mut m = MemorySheet {
            bot: "b".into(),
            view: Some(files(["a", "n"])),
            section: Section::Notes,
            cursor: 1,
            scroll: Scroll { pos: 10, ..Scroll::default() },
            brains: None,
            brain_cursor: 0,
            mode: MemoryMode::File { name: "n".into(), shared: false, detail: None },
            error: None,
            busy: false,
        };
        m.show(files(["n", "a"]));
        assert_eq!((m.cursor, m.scroll.pos), (0, 10), "the list keeps its row, the note its scroll");
    }

    #[test]
    fn scrolling_stops_at_the_end_the_draw_set() {
        let key = |code| KeyEvent::from(code);
        let mut scroll = Scroll { pos: 10, ..Scroll::default() };
        scroll.max.set(30);
        scroll.page.set(20);
        assert!(scroll.key(key(KeyCode::PageDown)));
        assert_eq!(scroll.pos, 20);
        assert!(scroll.key(key(KeyCode::PageDown)));
        assert_eq!(scroll.pos, 30, "no further than the last wrapped row");
        scroll.max.set(5);
        assert!(scroll.key(key(KeyCode::Up)));
        assert_eq!(scroll.pos, 4, "a shorter text: the first ↑ moves from its end");
        assert!(!scroll.key(key(KeyCode::Enter)));
    }

    #[test]
    fn turning_claude_codes_memory_off_leaves_its_section() {
        let entries = json!({"entries": [{"id": 1}, {"id": 2}]});
        let mut m = MemorySheet {
            bot: "b".into(),
            view: Some(json!({"bot": entries, "projectMemory": {"path": "p", "text": "a\nb\nc\nd"}})),
            section: Section::Harness,
            cursor: 0,
            scroll: Scroll { pos: 3, ..Scroll::default() },
            brains: None,
            brain_cursor: 0,
            mode: MemoryMode::Browse,
            error: None,
            busy: false,
        };
        m.show(json!({"bot": entries, "projectMemory": null}));
        assert_eq!((m.section, m.cursor), (Section::Bot, 0));
    }
}
