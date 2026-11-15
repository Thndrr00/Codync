//! A bot's missions: a goal that outlives a turn, the steps it takes, and how far it got. The sheet is
//! the TUI's half of the same panel the desktop inspector and the phone apps carry.

use super::super::app::{After, App, Editor, Overlay};
use super::{Page, Reply};
use crate::store::{Mission, MissionStatus, MissionStepStatus};
use crossterm::event::{KeyCode, KeyEvent};
use serde_json::{Value, json};

#[derive(Clone)]
pub enum MissionReply {
    /// The list for a bot: open ones, or every one when `finished`.
    List(String, bool),
    /// A change went through: the list is read again.
    Changed(String),
}

/// What an editor open over the sheet is for.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MissionEdit {
    New,
    Step,
    Checkpoint,
    Abandon,
}

pub struct MissionSheet {
    pub bot: String,
    pub items: Option<Vec<Mission>>,
    pub cursor: usize,
    /// The step picked inside the selected mission.
    pub step: usize,
    /// Keys move between a mission's steps instead of the list.
    pub focus_steps: bool,
    /// Every mission, finished ones too.
    pub finished: bool,
    pub edit: Option<(MissionEdit, Editor)>,
    pub busy: bool,
    pub error: Option<String>,
}

impl MissionSheet {
    fn current(&self) -> Option<&Mission> {
        self.items.as_ref()?.get(self.cursor)
    }

    fn current_step(&self) -> Option<&crate::store::MissionStep> {
        self.current()?.steps.get(self.step)
    }

    /// The selected mission's id, when there is one and it is not busy.
    fn target(&self) -> Option<&str> {
        self.current().map(|m| m.id.as_str())
    }

    fn open_editor(&mut self, what: MissionEdit, text: &str) {
        self.edit = Some((what, Editor::with(text)));
    }
}

impl App {
    pub fn open_missions(&mut self) {
        let Some(b) = self.bot().filter(|b| !b.group) else {
            self.flash("Missions belong to a bot, not a group");
            return;
        };
        let bot = b.id.clone();
        self.read_missions(&bot, false);
        self.overlays.push(Overlay::Missions(Box::new(MissionSheet {
            bot,
            items: None,
            cursor: 0,
            step: 0,
            focus_steps: false,
            finished: false,
            edit: None,
            busy: false,
            error: None,
        })));
    }

    /// Reads a bot's missions; `finished` keeps the closed ones in the same list.
    pub(in crate::tui) fn read_missions(&mut self, bot: &str, finished: bool) {
        self.call(
            "missions",
            json!({"botId": bot, "open": !finished}),
            After::Sheet(Reply::Missions(MissionReply::List(bot.to_owned(), finished))),
        );
    }

    /// A change on mission `id`: the same call path as any other sheet.
    fn mission_call(&mut self, bot: &str, method: &'static str, body: Value) {
        self.call(method, body, After::Sheet(Reply::Missions(MissionReply::Changed(bot.to_owned()))));
    }

    pub(in crate::tui) fn missions_key(&mut self, mut m: MissionSheet, k: KeyEvent) -> Option<Overlay> {
        if let Some(page) = Page::Missions.step(k) {
            self.open_page(page);
            return None;
        }
        if let Some((what, ed)) = m.edit.as_mut() {
            match k.code {
                KeyCode::Esc => m.edit = None,
                KeyCode::Enter => {
                    let text = ed.text.trim().to_owned();
                    let (what, id) = (what.to_owned(), m.target().map(str::to_owned));
                    m.edit = None;
                    self.submit_mission(&m.bot, what, id, &text);
                }
                _ => {
                    ed.key(k);
                }
            }
            return Some(Overlay::Missions(Box::new(m)));
        }
        let n = m.items.as_ref().map_or(0, Vec::len);
        let steps = m.current().map_or(0, |mission| mission.steps.len());
        match k.code {
            KeyCode::Esc | KeyCode::Char('q' | 'M') => return None,
            KeyCode::Tab => m.focus_steps = !m.focus_steps,
            KeyCode::Down | KeyCode::Char('j') if m.focus_steps => m.step = (m.step + 1).min(steps.saturating_sub(1)),
            KeyCode::Up | KeyCode::Char('k') if m.focus_steps => m.step = m.step.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => {
                m.cursor = (m.cursor + 1).min(n.saturating_sub(1));
                m.step = 0;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                m.cursor = m.cursor.saturating_sub(1);
                m.step = 0;
            }
            KeyCode::Char('f') => {
                m.finished = !m.finished;
                m.items = None;
                self.read_missions(&m.bot, m.finished);
            }
            KeyCode::Char('n') => m.open_editor(MissionEdit::New, ""),
            KeyCode::Char('c') => {
                let text = m.current().map(|mission| mission.checkpoint.clone()).unwrap_or_default();
                m.open_editor(MissionEdit::Checkpoint, &text);
            }
            KeyCode::Char('s') => m.open_editor(MissionEdit::Step, ""),
            KeyCode::Char('x') if !m.focus_steps => m.open_editor(MissionEdit::Abandon, ""),
            KeyCode::Char('a') => {
                if let Some((id, status)) = m.current().map(|mission| (mission.id.clone(), mission.status)) {
                    let to = step_status(status);
                    self.mission_call(&m.bot, "updateMission", json!({"id": id, "status": to}));
                }
            }
            KeyCode::Char('p') => {
                if let Some(id) = m.target().map(str::to_owned) {
                    self.mission_call(&m.bot, "updateMission", json!({"id": id, "status": "waiting"}));
                }
            }
            KeyCode::Char('d') => {
                if let Some(id) = m.target().map(str::to_owned) {
                    self.mission_call(&m.bot, "updateMission", json!({"id": id, "status": "done"}));
                }
            }
            // A step: Enter marks it done, x skips it, r puts it back to pending.
            KeyCode::Enter | KeyCode::Char(' ') if m.focus_steps => {
                if let Some((id, step)) = m.current_step().map(|step| (step.mission_id.clone(), step.id.clone())) {
                    self.mission_call(&m.bot, "missionStep", json!({"id": id, "stepId": step, "status": "done"}));
                }
            }
            KeyCode::Char('x') if m.focus_steps => {
                if let Some((id, step)) = m.current_step().map(|step| (step.mission_id.clone(), step.id.clone())) {
                    self.mission_call(&m.bot, "missionStep", json!({"id": id, "stepId": step, "status": "skipped"}));
                }
            }
            KeyCode::Char('r') if m.focus_steps => {
                if let Some((id, step)) = m.current_step().map(|step| (step.mission_id.clone(), step.id.clone())) {
                    self.mission_call(&m.bot, "missionStep", json!({"id": id, "stepId": step, "status": "pending"}));
                }
            }
            _ => {}
        }
        Some(Overlay::Missions(Box::new(m)))
    }

    /// Sends what an editor held: a new mission, a step, the checkpoint, or an abandonment.
    fn submit_mission(&mut self, bot: &str, what: MissionEdit, id: Option<String>, text: &str) {
        match what {
            MissionEdit::New if !text.is_empty() => {
                self.mission_call(bot, "createMission", json!({"botId": bot, "title": text}));
            }
            MissionEdit::Step => {
                if let Some(id) = id.filter(|_| !text.is_empty()) {
                    self.mission_call(bot, "missionStep", json!({"id": id, "text": text}));
                }
            }
            MissionEdit::Checkpoint => {
                if let Some(id) = id {
                    self.mission_call(bot, "updateMission", json!({"id": id, "checkpoint": text}));
                }
            }
            MissionEdit::Abandon => {
                if let Some(id) = id.filter(|_| !text.is_empty()) {
                    self.mission_call(
                        bot,
                        "updateMission",
                        json!({"id": id, "status": "abandoned", "stopReason": text}),
                    );
                }
            }
            MissionEdit::New => {}
        }
    }

    /// The host's answer to a list read.
    pub(in crate::tui) fn missions_loaded(&mut self, bot: &str, finished: bool, v: &Value) {
        let items: Vec<Mission> = v["missions"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|mission| serde_json::from_value(mission.clone()).ok())
            .collect();
        for o in &mut self.overlays {
            if let Overlay::Missions(m) = o
                && m.bot == bot
                && m.finished == finished
            {
                m.cursor = m.cursor.min(items.len().saturating_sub(1));
                m.step = 0;
                m.items = Some(items.clone());
            }
        }
    }

    /// A mission changed: the open sheet reads its list again.
    pub(in crate::tui) fn mission_changed(&mut self, bot: &str) {
        let Some(Overlay::Missions(m)) = self.overlays.last() else { return };
        let finished = m.finished;
        self.read_missions(bot, finished);
    }
}

/// Where `a` takes a mission: on along its way, or back from a wait.
fn step_status(status: MissionStatus) -> &'static str {
    match status {
        MissionStatus::Active => "waiting",
        MissionStatus::Proposed | MissionStatus::Waiting | MissionStatus::Done | MissionStatus::Abandoned => "active",
    }
}

/// `x` on a step's status glyph, for the draw.
pub fn mark(status: MissionStepStatus) -> &'static str {
    match status {
        MissionStepStatus::Pending => "·",
        MissionStepStatus::Active => "▸",
        MissionStepStatus::Done => "✓",
        MissionStepStatus::Skipped => "–",
    }
}

/// The word a status shows as.
pub fn status_label(status: MissionStatus) -> &'static str {
    match status {
        MissionStatus::Proposed => "proposed",
        MissionStatus::Active => "running",
        MissionStatus::Waiting => "waiting",
        MissionStatus::Done => "done",
        MissionStatus::Abandoned => "abandoned",
    }
}

#[cfg(test)]
mod tests;
