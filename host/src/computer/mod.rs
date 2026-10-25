//! Capability gates for Codync tools, not an OS sandbox for an agent harness.
use crate::{LockExt, hub::Hub, store::BotConfig};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Mutex};
use tokio::sync::watch;

pub mod panel;
mod terminal;
#[cfg(test)]
mod tests;
mod workspace;

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct Permissions {
    pub browser: bool,
    pub files: bool,
    pub shell: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Capability {
    Browser,
    Files,
    Shell,
}
impl Capability {
    fn enabled(self, config: &BotConfig) -> bool {
        config.computer
            && !config.is_group()
            && match self {
                Self::Browser => config.computer_permissions.browser,
                Self::Files => config.computer_permissions.files,
                Self::Shell => config.computer_permissions.shell,
            }
    }
}

#[derive(Default)]
pub struct Access {
    epochs: Mutex<HashMap<(String, Capability), watch::Sender<u64>>>,
}
#[derive(Clone)]
pub struct Lease {
    receiver: watch::Receiver<u64>,
    epoch: u64,
}
impl Lease {
    pub fn check(&self) -> Result<()> {
        if *self.receiver.borrow() != self.epoch {
            bail!("Computer capability was revoked; the request was cancelled.");
        }
        Ok(())
    }
    pub async fn cancelled(&mut self) {
        while self.check().is_ok() && self.receiver.changed().await.is_ok() {}
    }
}
impl Access {
    pub fn begin(&self, hub: &Hub, bot: &str, capability: Capability) -> Result<Lease> {
        let mut epochs = self.epochs.locked();
        if hub.screen.entry_protected() {
            bail!("Protected entry is active. Codync computer tools are paused until the owner finishes.");
        }
        let row = hub.store.bot(bot)?.filter(|row| !row.deleted).context("unknown bot")?;
        if !capability.enabled(&row.config) {
            bail!("This computer capability is turned off for this bot.");
        }
        let sender = epochs.entry((bot.into(), capability)).or_insert_with(|| watch::channel(0).0);
        let receiver = sender.subscribe();
        let epoch = *receiver.borrow();
        Ok(Lease { receiver, epoch })
    }
    pub fn changed(&self, bot: &str, before: &BotConfig, after: &BotConfig) {
        for capability in [Capability::Browser, Capability::Files, Capability::Shell] {
            if capability.enabled(before) && !capability.enabled(after) {
                self.revoke(bot, capability);
            }
        }
    }
    pub fn revoke(&self, bot: &str, capability: Capability) {
        if let Some(sender) = self.epochs.locked().get(&(bot.into(), capability)) {
            sender.send_modify(|epoch| *epoch = epoch.wrapping_add(1));
        }
    }
    pub fn revoke_all(&self, bot: &str) {
        for capability in [Capability::Browser, Capability::Files, Capability::Shell] {
            self.revoke(bot, capability);
        }
    }
}
