//! A bot's turn and the skills (`market::skill_usage`): a tool call that touches a skill's folder is a use of that
//! skill, and a long turn is a chance to tell the user the bot can keep what it just did as a skill.

use super::{Actor, NoticeStyle};
use serde_json::Value;

impl Actor {
    /// Notes the use of a skill when this tool-call update names a file inside the skills folder.
    pub(super) fn note_skill_use(&self, update: &Value) {
        if !matches!(update["sessionUpdate"].as_str(), Some("tool_call" | "tool_call_update")) {
            return;
        }
        if let Some(id) = crate::market::skill_touched(&update["rawInput"], &crate::market::skills_root()) {
            crate::market::note_skill_use(&self.hub.store, &id, crate::store::now_ms());
        }
    }

    /// After a long turn, tells the user once a day that this bot can save it as a skill (`save_skill`).
    pub(super) fn offer_skill(&self) {
        let calls = self.watchdog.tool_calls();
        if crate::market::offer_skill(&self.hub.store, &self.cfg.id, calls, crate::store::now_ms()) {
            self.notice(
                &format!("That took {calls} steps. If you'll want it again, ask this bot to save it as a skill."),
                NoticeStyle::Info,
            );
        }
    }
}
