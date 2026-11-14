//! A bot's turn and the skills (`market::skill_usage`): a tool call that touches a skill's folder is a use of that
//! skill, reading its `SKILL.md` is a load of its text (counted for Usage), and a long turn is a chance to tell the
//! user the bot can keep what it just did as a skill.

use super::{Actor, NoticeStyle};
use serde_json::Value;

impl Actor {
    /// Notes the use of a skill when this tool call names a file inside the skills folder, and counts a load of its
    /// text when a finished call reads the skill's `SKILL.md`. A turn loads each skill once, however many reads it makes.
    pub(super) fn note_skill_use(&mut self, update: &Value) {
        if !matches!(update["sessionUpdate"].as_str(), Some("tool_call" | "tool_call_update")) {
            return;
        }
        let root = crate::market::skills_root();
        let input = &update["rawInput"];
        let finished = update["status"].as_str() == Some("completed");
        let writes = matches!(update["kind"].as_str(), Some("edit" | "delete" | "move"));
        let read = finished.then(|| crate::market::skill_read(input, &root)).flatten().filter(|_| !writes);
        if let Some(id) = read {
            if self.turn_skills.insert(id.clone()) {
                let tokens = crate::market::skill_load_tokens(&id);
                let cost = tokens.and_then(|tokens| {
                    crate::usage::estimate_cost(tokens, &self.cfg.backend, self.cfg.model.as_deref())
                });
                crate::market::note_skill_load(
                    &self.hub.store,
                    &self.cfg.id,
                    &id,
                    tokens,
                    cost,
                    crate::store::now_ms(),
                );
            }
        } else if let Some(id) = crate::market::skill_touched(input, &root) {
            crate::market::note_skill_use(&self.hub.store, &id, crate::store::now_ms());
        }
    }

    /// After a long turn, tells the user once a day that this bot can save it as a skill (`save_skill`).
    pub(super) fn offer_skill(&mut self) {
        let calls = self.watchdog.tool_calls();
        if crate::market::offer_skill(&self.hub.store, &self.cfg.id, calls, crate::store::now_ms()) {
            self.notice(
                &format!("That took {calls} steps. If you'll want it again, ask this bot to save it as a skill."),
                NoticeStyle::Info,
            );
        }
    }
}
