//! Bots that stand for agent sessions running in the person's own terminals (see `external`).

use super::Hub;
use crate::LockExt;
use crate::store::BotConfig;
use anyhow::Result;

impl Hub {
    /// Saves and announces the bot of a terminal session. No agent process is started for it: the session lives
    /// in the person's terminal and only its hook events reach the host.
    pub fn add_external_bot(&self, mut cfg: BotConfig) -> Result<String> {
        cfg.id = uuid::Uuid::new_v4().to_string();
        cfg.created_at = crate::store::now_ms();
        cfg.position = self.new_bot_position()?;
        {
            let _g = self.emit_lock.locked();
            self.store.save_bot(&cfg)?;
        }
        self.emit_bot(&cfg.id);
        Ok(cfg.id)
    }
}
