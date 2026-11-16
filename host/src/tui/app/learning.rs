//! Learned preferences in the terminal: what the bot proposes from the person's corrections and repeated
//! instructions. `a` remembers one, `x` dismisses it for good, `t` turns learning off or on, and `,` `.` change how
//! many repeats save a fact without review.

use crossterm::event::{KeyCode, KeyEvent};
use serde_json::{Value, json};

use super::{After, App, Overlay};

#[derive(Default)]
pub struct LearningSheet {
    pub bot_id: String,
    pub bot_name: String,
    /// `None` until the host answered.
    pub inbox: Option<Value>,
    pub sel: usize,
}

const REPEATS: [u64; 4] = [0, 2, 3, 5];

pub fn inbox_title(count: usize) -> String {
    if count > 0 { format!("Learned preferences ({count})") } else { "Learned preferences".to_owned() }
}

/// `Correction`, `Said again · 3 times` or `Preference`.
pub fn kind_label(proposal: &Value) -> String {
    let hits = proposal["hits"].as_u64().unwrap_or(1);
    match proposal["kind"].as_str().unwrap_or("instruction") {
        "correction" => "Correction".to_owned(),
        "repetition" if hits > 1 => format!("Said again · {hits} times"),
        "repetition" => "Said again".to_owned(),
        _ => "Preference".to_owned(),
    }
}

impl App {
    pub(super) fn open_learning(&mut self) {
        let Some(bot) = self.bot().filter(|b| !b.group) else {
            self.flash("Pick a bot first");
            return;
        };
        let sheet = LearningSheet { bot_id: bot.id.clone(), bot_name: bot.name.clone(), ..LearningSheet::default() };
        self.call("learning", json!({"botId": sheet.bot_id}), After::Learning);
        self.overlays.push(Overlay::Learning(Box::new(sheet)));
    }

    pub(super) fn learning_key(&mut self, mut s: Box<LearningSheet>, k: KeyEvent) -> Option<Overlay> {
        let count = s.inbox.as_ref().and_then(|v| v["proposals"].as_array()).map_or(0, Vec::len);
        let picked = s
            .inbox
            .as_ref()
            .and_then(|v| v["proposals"].as_array())
            .and_then(|items| items.get(s.sel))
            .and_then(|p| p["id"].as_str())
            .map(str::to_owned);
        let enabled = s.inbox.as_ref().and_then(|v| v["enabled"].as_bool()).unwrap_or(true);
        match k.code {
            KeyCode::Esc | KeyCode::Char('q') => return None,
            KeyCode::Down | KeyCode::Char('j') => s.sel = (s.sel + 1).min(count.saturating_sub(1)),
            KeyCode::Up | KeyCode::Char('k') => s.sel = s.sel.saturating_sub(1),
            KeyCode::Char('r') => self.call("learning", json!({"botId": s.bot_id}), After::Learning),
            KeyCode::Char('t') => {
                let body = json!({"botId": s.bot_id, "enabled": !enabled});
                self.call("saveLearningSettings", body, After::Learning);
            }
            KeyCode::Char(step @ (',' | '.')) => {
                let current = s.inbox.as_ref().and_then(|v| v["autoApproveAfter"].as_u64()).unwrap_or(0);
                let index = REPEATS.iter().position(|n| *n == current).unwrap_or(0);
                let next = if step == ',' { index.saturating_sub(1) } else { (index + 1).min(REPEATS.len() - 1) };
                let body = json!({"botId": s.bot_id, "autoApproveAfter": REPEATS[next]});
                self.call("saveLearningSettings", body, After::Learning);
            }
            KeyCode::Char('a') => {
                if let Some(id) = picked {
                    self.call("approveLearning", json!({"botId": s.bot_id, "id": id}), After::Learning);
                }
            }
            KeyCode::Char('x') => {
                if let Some(id) = picked {
                    self.call("dismissLearning", json!({"botId": s.bot_id, "id": id}), After::Learning);
                }
            }
            _ => {}
        }
        Some(Overlay::Learning(s))
    }

    /// Every call of the sheet answers with the inbox as it is now.
    pub(super) fn learning_loaded(&mut self, v: &Value) {
        if let Some(Overlay::Learning(s)) = self.overlays.last_mut() {
            let count = v["proposals"].as_array().map_or(0, Vec::len);
            s.sel = s.sel.min(count.saturating_sub(1));
            s.inbox = Some(v.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_inbox_title_and_a_card_s_reason_read_as_words() {
        assert_eq!(inbox_title(0), "Learned preferences");
        assert_eq!(inbox_title(2), "Learned preferences (2)");
        assert_eq!(kind_label(&json!({"kind": "correction"})), "Correction");
        assert_eq!(kind_label(&json!({"kind": "repetition", "hits": 3})), "Said again · 3 times");
        assert_eq!(kind_label(&json!({"kind": "instruction"})), "Preference");
    }
}
