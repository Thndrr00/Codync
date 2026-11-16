//! A channel's request tree over the composer: "Waiting for sub-tasks" as the apps show it. Each row is a member's
//! assignment; Stop pauses it and its branch, and a stopped row offers Continue, Reassign (a member picker) and
//! Cancel. A stopped row keeps the channel's messages when it is cancelled.

use super::{After, App, Task};
use crossterm::event::{KeyCode, KeyEvent};
use serde_json::json;

#[derive(Default)]
pub struct TasksBand {
    /// The row the keys act on.
    pub cursor: usize,
    /// The stopped row whose new member is being picked (`A`).
    pub assign: Option<String>,
    /// The member the picker is on.
    pub member: usize,
}

/// How many rows the band shows at once.
pub const BAND_ROWS: usize = 3;

impl App {
    /// The selected channel's request tree, as its band shows it.
    pub fn tasks(&self) -> Vec<&Task> {
        let Some(bot) = self.bot().filter(|bot| bot.group) else { return vec![] };
        bot.tasks.iter().collect()
    }

    /// The row the keys act on.
    pub fn task_picked(&self) -> Option<&Task> {
        let tasks = self.tasks();
        tasks.get(self.tasks_band.cursor.min(tasks.len().saturating_sub(1))).cloned()
    }

    /// Handles a key the sub-task band owns; `true` when it took it.
    pub(super) fn tasks_key(&mut self, key: KeyEvent) -> bool {
        let Some(bot) = self.bot().filter(|bot| bot.group) else { return false };
        if bot.tasks.is_empty() {
            return false;
        }
        let (bot_id, tasks): (String, Vec<Task>) = (bot.id.clone(), bot.tasks.clone());
        if let Some(task) = self.tasks_band.assign.clone() {
            return self.assign_key(key, &bot_id, &task);
        }
        let count = tasks.len();
        self.tasks_band.cursor = self.tasks_band.cursor.min(count.saturating_sub(1));
        let picked = tasks.get(self.tasks_band.cursor).cloned();
        match key.code {
            KeyCode::Char(',') => self.tasks_band.cursor = self.tasks_band.cursor.saturating_sub(1),
            KeyCode::Char('.') => {
                self.tasks_band.cursor = (self.tasks_band.cursor + 1).min(count.saturating_sub(1));
            }
            KeyCode::Char('W') => {
                if let Some(task) = picked {
                    self.task_call(&bot_id, &task.id, "stop", None);
                }
            }
            KeyCode::Char('O') => {
                if let Some(task) = picked {
                    self.task_call(&bot_id, &task.id, "continue", None);
                }
            }
            KeyCode::Char('A') => {
                let Some(task) = picked.filter(Task::stopped) else {
                    self.flash("Stop a sub-task before giving it to another member");
                    return true;
                };
                self.tasks_band.assign = Some(task.id);
                self.tasks_band.member = 0;
            }
            KeyCode::Char('B') => {
                if let Some(task) = picked {
                    self.task_call(&bot_id, &task.id, "cancel", None);
                }
            }
            _ => return false,
        }
        true
    }

    /// The member picker of Reassign: `j`/`k` or a digit moves, Enter gives the row to that member.
    fn assign_key(&mut self, key: KeyEvent, bot_id: &str, task: &str) -> bool {
        let Some(bot) = self.bot().cloned() else { return false };
        let members: Vec<String> = bot.members.clone();
        match key.code {
            KeyCode::Esc => self.tasks_band.assign = None,
            KeyCode::Char('j') | KeyCode::Down => {
                self.tasks_band.member = (self.tasks_band.member + 1).min(members.len().saturating_sub(1));
            }
            KeyCode::Char('k') | KeyCode::Up => self.tasks_band.member = self.tasks_band.member.saturating_sub(1),
            KeyCode::Char(c @ '1'..='9') => {
                self.tasks_band.member = ((c as usize) - ('1' as usize)).min(members.len().saturating_sub(1));
            }
            KeyCode::Enter => {
                if let Some(member) = members.get(self.tasks_band.member).cloned() {
                    self.tasks_band.assign = None;
                    self.task_call(bot_id, task, "reassign", Some(member));
                }
            }
            _ => return false,
        }
        true
    }

    fn task_call(&mut self, bot: &str, task: &str, action: &str, owner: Option<String>) {
        let mut body = json!({"botId": bot, "taskId": task, "action": action});
        if let Some(owner) = owner {
            body["ownerId"] = owner.into();
        }
        self.call("groupTask", body, After::Nothing);
    }

    /// The member the picker is on, for the band.
    pub fn task_member(&self) -> Option<(String, usize, usize)> {
        let bot = self.bot()?;
        let member = bot.members.get(self.tasks_band.member)?;
        Some((self.author_name(Some(member.as_str())), self.tasks_band.member, bot.members.len()))
    }
}

#[cfg(test)]
mod tests {
    use super::super::Msg;
    use serde_json::json;

    fn app_with_tasks() -> super::App {
        let mut app = super::super::test_app();
        app.on_msg(Msg::Event(json!({"type": "bot", "bot": {
            "id": "g", "name": "Crew", "kind": "group", "members": ["a", "b"],
            "tasks": [
                {"id": "t1", "instruction": "Ship it", "ownerName": null, "state": "paused"},
                {"id": "t2", "instruction": "Check CI", "ownerName": "Bob", "state": "paused"},
            ],
        }})));
        app.selected = Some("g".into());
        app
    }

    #[test]
    fn a_stopped_row_can_be_stopped_and_moved_to_another_member() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let mut app = app_with_tasks();
        assert_eq!(app.tasks().len(), 2);
        app.tasks_band.cursor = 1;
        assert!(app.task_picked().is_some_and(|t| t.stopped()));
        assert!(app.tasks_key(KeyEvent::new(KeyCode::Char('A'), KeyModifiers::SHIFT)));
        assert_eq!(app.tasks_band.assign.as_deref(), Some("t2"));
        app.tasks_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
        assert!(app.tasks_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)));
        assert!(app.tasks_band.assign.is_none(), "the picker closes on the member it hands the row to");
    }

    #[test]
    fn a_row_that_is_still_working_offers_no_reassign() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let mut app = super::super::test_app();
        app.on_msg(Msg::Event(json!({"type": "bot", "bot": {
            "id": "g", "name": "Crew", "kind": "group", "members": ["a", "b"],
            "tasks": [{"id": "t1", "instruction": "Ship it", "state": "running"}],
        }})));
        app.selected = Some("g".into());
        app.tasks_band.cursor = 0;
        assert!(app.tasks_key(KeyEvent::new(KeyCode::Char('A'), KeyModifiers::SHIFT)));
        assert!(app.tasks_band.assign.is_none());
    }
}
